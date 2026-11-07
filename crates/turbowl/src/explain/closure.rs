//! The closure as the search reads it: the store's own tables, in the terms
//! the user wrote, with no copy.
//!
//! The store holds each triple once, in representatives (ADR 0003), one
//! table per predicate, each sorted by subject in a few batches. A pattern
//! is answered in representatives and each answer expanded to the members
//! of the classes of its free positions: the closure of the W3C eq-* rules,
//! as the reads give it. A bound term is looked up through its
//! representative and kept as written.
//!
//! What the store cannot answer directly is built on first use and kept in
//! [`Indexes`] (in the [`ExplainCache`](super::ExplainCache)):
//!
//! - a predicate's pairs by object, when its table has no object index (or
//!   one in several batches): one sorted copy of that table;
//! - eq-ref's triples the store does not hold (switch `trivial`): every
//!   representative of a term of the store, which one pass over the store
//!   finds, less those whose `r owl:sameAs r` it holds;
//! - the base triples added since its last merge, sorted, and the base's
//!   `owl:sameAs` pairs.
//!
//! Every answer comes in an order fixed by the store's contents (a table's
//! batches are merged), so one closure gives one proof whatever the
//! threads that built it.

use super::program::UNLISTED;
use crate::Reasoner;
use crate::engine::{Canon, Members, Store, Table};
use crate::rules::datatype::map::{Reading, Value, value_of};
use crate::term::{Kind, TermId, vocab};
use rustc_hash::FxHashMap;
use std::ops::ControlFlow;
use std::sync::OnceLock;

/// Whether to go on.
pub(crate) type Flow = ControlFlow<()>;

/// The lazy indexes of one closure (see the module doc).
#[derive(Debug)]
pub(crate) struct Indexes {
    /// The store's predicates (representatives), ascending.
    predicates: Vec<u32>,
    /// By the slot of [`Indexes::predicates`]: the table's `(o, s)` pairs,
    /// sorted, once a pattern needs them and the store has none in one batch.
    by_object: Vec<OnceLock<Vec<(u32, u32)>>>,
    /// eq-ref's triples the store does not hold: the representatives `r`
    /// whose `r owl:sameAs r` the closure has and the store lacks, ascending.
    eq_ref: OnceLock<Vec<u32>>,
    /// The base triples added since its last merge, as `[p, s, o]`, sorted.
    recent: OnceLock<Vec<[u32; 3]>>,
    /// The base triples a term of which is in a class of two or more, by
    /// their triple in representatives (their image), in base order.
    images: OnceLock<FxHashMap<[u32; 3], Vec<[u32; 3]>>>,
}

/// How a free position of a pattern takes the terms of a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Free {
    /// The representative, and the member a base triple of the same image
    /// states there.
    Stated,
    /// Every member.
    Members,
    /// Bound up to its class: given as the class's representative, and
    /// matched by the representative and by the members base triples of
    /// the same image state there.
    Classed,
}

impl Indexes {
    /// The indexes of `r`'s closure, none built yet: one pass over its
    /// predicates.
    pub(crate) fn new(r: &Reasoner) -> Self {
        let mut predicates: Vec<u32> = r.store().tables().map(|(p, _)| p).collect();
        predicates.sort_unstable();
        let by_object = predicates.iter().map(|_| OnceLock::new()).collect();
        Self {
            predicates,
            by_object,
            eq_ref: OnceLock::new(),
            recent: OnceLock::new(),
            images: OnceLock::new(),
        }
    }

    /// The bytes the built indexes hold.
    pub(crate) fn bytes(&self) -> usize {
        use crate::profile::vec_bytes;
        vec_bytes(&self.predicates)
            + vec_bytes(&self.by_object)
            + self
                .by_object
                .iter()
                .filter_map(OnceLock::get)
                .map(vec_bytes)
                .sum::<usize>()
            + self.eq_ref.get().map_or(0, vec_bytes)
            + self.recent.get().map_or(0, vec_bytes)
            + self.images.get().map_or(0, |m| {
                crate::profile::hash_bytes(m.capacity(), std::mem::size_of::<([u32; 3], Vec<[u32; 3]>)>())
                    + m.values().map(vec_bytes).sum::<usize>()
            })
    }
}

/// The closure of one reasoner, read through its store and the lazy
/// indexes.
#[derive(Clone, Copy)]
pub(crate) struct Closure<'a> {
    pub(crate) reasoner: &'a Reasoner,
    store: &'a Store,
    pub(crate) canon: &'a Canon,
    ix: &'a Indexes,
    /// The representative of `owl:sameAs`.
    same: u32,
    /// eq-ref holds (switch `trivial`).
    trivial: bool,
}

/// The pairs of `part` whose first is `key`.
fn run<T: Copy>(part: &[(u32, T)], key: u32) -> &[(u32, T)] {
    let from = part.partition_point(|p| p.0 < key);
    let to = from + part[from..].partition_point(|p| p.0 == key);
    &part[from..to]
}

/// The sorted parts of a table's subject index that hold something.
fn parts(table: &Table) -> Vec<&[(u32, u32)]> {
    table.by_subject().parts().filter(|p| !p.is_empty()).collect()
}

/// Calls `f` on every pair of the sorted, disjoint `parts`, in order.
fn merged(parts: &[&[(u32, u32)]], mut f: impl FnMut(u32, u32) -> Flow) -> Flow {
    let mut at = vec![0usize; parts.len()];
    loop {
        let mut best: Option<(usize, (u32, u32))> = None;
        for (k, part) in parts.iter().enumerate() {
            if let Some(&x) = part.get(at[k])
                && best.is_none_or(|(_, b)| x < b)
            {
                best = Some((k, x));
            }
        }
        let Some((k, (a, b))) = best else {
            return Flow::Continue(());
        };
        at[k] += 1;
        f(a, b)?;
    }
}

impl<'a> Closure<'a> {
    pub(crate) fn new(reasoner: &'a Reasoner, ix: &'a Indexes) -> Self {
        let store = reasoner.store();
        let canon = store.canon();
        Self {
            reasoner,
            store,
            canon,
            ix,
            same: canon.find(vocab::OWL_SAME_AS.0),
            trivial: reasoner.switches().is_on(crate::Switch::Trivial),
        }
    }

    /// The representative of `id`.
    #[inline]
    pub(crate) fn find(&self, id: u32) -> u32 {
        self.canon.find(id)
    }

    /// The members of the class whose representative is `rep`.
    #[inline]
    pub(crate) fn members(&self, rep: u32) -> Members<'a> {
        self.canon.members(rep)
    }

    /// True when no two terms are equal.
    pub(crate) fn is_flat(&self) -> bool {
        self.canon.is_empty()
    }

    fn slot(&self, p: u32) -> Option<usize> {
        self.ix.predicates.binary_search(&p).ok()
    }

    /// eq-ref's triples the store lacks (see [`Indexes::eq_ref`]).
    fn eq_ref(&self) -> &'a [u32] {
        if !self.trivial {
            return &[];
        }
        self.ix.eq_ref.get_or_init(|| {
            let mut seen: Vec<u64> = Vec::new();
            let mut mark = |id: u32| {
                let w = id as usize / 64;
                if w >= seen.len() {
                    seen.resize(w + 1, 0);
                }
                seen[w] |= 1 << (id % 64);
            };
            let mut any = false;
            for (s, p, o) in self.store.triples() {
                mark(s);
                mark(p);
                mark(o);
                any = true;
            }
            if any {
                mark(self.same);
            }
            let stored = self.store.table(self.same).map(Table::by_subject);
            let mut out = Vec::new();
            for (w, &word) in seen.iter().enumerate() {
                let mut rest = word;
                while rest != 0 {
                    let r = w as u32 * 64 + rest.trailing_zeros();
                    rest &= rest - 1;
                    if !stored.is_some_and(|v| v.contains(&(r, r))) {
                        out.push(r);
                    }
                }
            }
            out
        })
    }

    fn is_eq_ref(&self, r: u32) -> bool {
        self.trivial && self.eq_ref().binary_search(&r).is_ok()
    }

    /// True when the store (with eq-ref's triples) holds the triple of
    /// representatives `(s, p, o)`.
    fn has(&self, s: u32, p: u32, o: u32) -> bool {
        self.store.contains(s, p, o) || (p == self.same && s == o && self.is_eq_ref(s))
    }

    /// True when the closure holds `t`, in any terms (not dt-diff's
    /// unlisted triples, which [`Closure::unlisted`] answers).
    pub(crate) fn contains(&self, t: [u32; 3]) -> bool {
        t[1] != UNLISTED && self.has(self.find(t[0]), self.find(t[1]), self.find(t[2]))
    }

    /// Each object of `(s, p, ·)`, in representatives, ascending (eq-ref's
    /// own last).
    fn objects(&self, p: u32, s: u32, f: &mut dyn FnMut(u32) -> Flow) -> Flow {
        if let Some(table) = self.store.table(p) {
            let parts = parts(table);
            if let [one] = parts[..] {
                for &(_, o) in run(one, s) {
                    f(o)?;
                }
            } else {
                let mut all: Vec<u32> = parts.iter().flat_map(|part| run(part, s)).map(|&(_, o)| o).collect();
                all.sort_unstable();
                for o in all {
                    f(o)?;
                }
            }
        }
        if p == self.same && self.is_eq_ref(s) {
            f(s)?;
        }
        Flow::Continue(())
    }

    /// The `(o, s)` pairs of predicate `p`'s table, sorted.
    fn by_object(&self, p: u32) -> &'a [(u32, u32)] {
        let Some(table) = self.store.table(p) else {
            return &[];
        };
        if let Some(os) = table.by_object() {
            let mut held = os.parts().filter(|p| !p.is_empty());
            match (held.next(), held.next()) {
                (None, _) => return &[],
                (Some(one), None) => return one,
                _ => {}
            }
        }
        let slot = self.slot(p).expect("a table's predicate has a slot");
        self.ix.by_object[slot].get_or_init(|| {
            let mut pairs: Vec<(u32, u32)> = table.by_subject().parts().flatten().map(|&(s, o)| (o, s)).collect();
            pairs.sort_unstable();
            pairs
        })
    }

    /// Each subject of `(·, p, o)`, in representatives, ascending (eq-ref's
    /// own last).
    fn subjects(&self, p: u32, o: u32, f: &mut dyn FnMut(u32) -> Flow) -> Flow {
        for &(_, s) in run(self.by_object(p), o) {
            f(s)?;
        }
        if p == self.same && self.is_eq_ref(o) {
            f(o)?;
        }
        Flow::Continue(())
    }

    /// Each pair of predicate `p`, in representatives, in order (eq-ref's
    /// own last).
    fn pairs(&self, p: u32, f: &mut dyn FnMut(u32, u32) -> Flow) -> Flow {
        if let Some(table) = self.store.table(p) {
            let parts = parts(table);
            if let [one] = parts[..] {
                for &(s, o) in one {
                    f(s, o)?;
                }
            } else {
                merged(&parts, &mut *f)?;
            }
        }
        if p == self.same {
            for &r in self.eq_ref() {
                f(r, r)?;
            }
        }
        Flow::Continue(())
    }

    /// The predicates a pattern with no predicate ranges over: the store's,
    /// and `owl:sameAs` for eq-ref's own.
    fn predicates(&self) -> Vec<u32> {
        let mut all = self.ix.predicates.clone();
        if !self.eq_ref().is_empty() && self.slot(self.same).is_none() {
            let at = all.partition_point(|&p| p < self.same);
            all.insert(at, self.same);
        }
        all
    }

    /// Each triple of the closure matching the pattern (`None`: any term),
    /// as `[s, p, o]`: a bound term as given, a free one as `free` says by
    /// position. `f` stops the walk by
    /// breaking. dt-diff's unlisted triples match only fully bound
    /// ([`Closure::unlisted`]).
    pub(crate) fn each(
        &self,
        s: Option<u32>,
        p: Option<u32>,
        o: Option<u32>,
        free: [Free; 3],
        f: &mut dyn FnMut([u32; 3]) -> Flow,
    ) -> Flow {
        if p == Some(UNLISTED) {
            if let (Some(s), Some(o)) = (s, o)
                && self.unlisted(s, o)
            {
                f([s, UNLISTED, o])?;
            }
            return Flow::Continue(());
        }
        let bound = [s, p, o];
        let class = |rep: u32, k: usize| match bound[k] {
            Some(x) => Members::One(x),
            None if free[k] == Free::Members => self.members(rep),
            None => Members::One(rep),
        };
        let stated = free
            .iter()
            .zip(&bound)
            .any(|(f, b)| (*f == Free::Stated && b.is_none()) || *f == Free::Classed)
            && !self.is_flat();
        let (rs, ro) = (s.map(|x| self.find(x)), o.map(|x| self.find(x)));
        let predicates = match p {
            Some(p) => vec![self.find(p)],
            None => self.predicates(),
        };
        for rp in predicates {
            let ps = class(rp, 1);
            let mut emit = |a: u32, b: u32| -> Flow {
                for s in class(a, 0).iter() {
                    for p in ps.iter() {
                        for o in class(b, 2).iter() {
                            f([s, p, o])?;
                        }
                    }
                }
                if stated {
                    // The members base triples of this image state where
                    // the pattern is free, where it is bound as given.
                    let image = [a, rp, b];
                    // A position bound as given keeps its term (a base
                    // triple may spell it as another member of its class);
                    // a free one takes the base triple's member, unless the
                    // members above gave it already.
                    let given = |k: usize| bound[k].is_some() && free[k] != Free::Classed;
                    let mut sent: Vec<[u32; 3]> = Vec::new();
                    for t in self.images().get(&image).into_iter().flatten() {
                        let shown = (0..3).all(|k| given(k) || free[k] == Free::Members || t[k] == image[k]);
                        if shown {
                            continue;
                        }
                        let u = [0, 1, 2].map(|k| if given(k) { bound[k].expect("given") } else { t[k] });
                        if !sent.contains(&u) {
                            sent.push(u);
                            f(u)?;
                        }
                    }
                }
                Flow::Continue(())
            };
            match (rs, ro) {
                (Some(a), Some(b)) => {
                    if self.has(a, rp, b) {
                        emit(a, b)?;
                    }
                }
                (Some(a), None) => self.objects(rp, a, &mut |b| emit(a, b))?,
                (None, Some(b)) => self.subjects(rp, b, &mut |a| emit(a, b))?,
                (None, None) => self.pairs(rp, &mut emit)?,
            }
        }
        Flow::Continue(())
    }

    /// True when `id` is a literal.
    pub(crate) fn is_literal(&self, id: u32) -> bool {
        let terms = self.reasoner.terms();
        (id as usize) < terms.len() && terms.kind(TermId(id)) == Kind::Literal
    }

    /// The base triples a term of which is in a class of two or more, by
    /// image (see [`Indexes::images`]).
    pub(crate) fn images(&self) -> &'a FxHashMap<[u32; 3], Vec<[u32; 3]>> {
        self.ix.images.get_or_init(|| {
            let mut out: FxHashMap<[u32; 3], Vec<[u32; 3]>> = FxHashMap::default();
            if self.is_flat() {
                return out;
            }
            for t in self.reasoner.base_triples() {
                let image = t.map(|x| self.find(x));
                if (0..3).any(|k| self.members(image[k]).len() > 1) {
                    out.entry(image).or_default().push(t);
                }
            }
            out
        })
    }

    /// The value of a term, when it is a literal with one.
    pub(crate) fn value(&self, id: u32) -> Option<Value<'a>> {
        let terms = self.reasoner.terms();
        if id as usize >= terms.len() || terms.kind(TermId(id)) != Kind::Literal {
            return None;
        }
        match value_of(terms.term(TermId(id))) {
            Reading::Value(v) => Some(v),
            _ => None,
        }
    }

    /// The members of `x`'s class that are literals with a value, `x`
    /// first when it is one.
    pub(crate) fn valued(&self, x: u32) -> Vec<(u32, Value<'a>)> {
        let mut out: Vec<(u32, Value<'a>)> = self.value(x).map(|v| (x, v)).into_iter().collect();
        out.extend(
            self.members(self.find(x))
                .iter()
                .filter(|&m| m != x)
                .filter_map(|m| self.value(m).map(|v| (m, v))),
        );
        out
    }

    /// True when dt-diff, carried by eq-rep-s and eq-rep-o, gives
    /// `x unlisted y`: a literal equal to `x` and one equal to `y` have
    /// different values (switch `literal-values`).
    pub(crate) fn unlisted(&self, x: u32, y: u32) -> bool {
        if !self.reasoner.switches().is_on(crate::Switch::LiteralValues) {
            return false;
        }
        let (vx, vy) = (self.valued(x), self.valued(y));
        vx.iter().any(|(_, a)| vy.iter().any(|(_, b)| a != b))
    }

    /// Each base object of `(s, p, ·)`, in the terms as written, ascending
    /// within the merged base and then among the recent triples.
    pub(crate) fn base_objects(&self, s: u32, p: u32, f: &mut dyn FnMut(u32) -> Flow) -> Flow {
        let pairs = self.reasoner.base_pairs(p);
        let from = pairs.partition_point(|q| q[0] < s);
        for q in pairs[from..].iter().take_while(|q| q[0] == s) {
            f(q[1])?;
        }
        let recent = self.ix.recent.get_or_init(|| {
            let mut all: Vec<[u32; 3]> = self.reasoner.base_recent().map(|[s, p, o]| [p, s, o]).collect();
            all.sort_unstable();
            all
        });
        if !recent.is_empty() {
            let from = recent.partition_point(|t| [t[0], t[1]] < [p, s]);
            for t in recent[from..].iter().take_while(|t| t[0] == p && t[1] == s) {
                f(t[2])?;
            }
        }
        Flow::Continue(())
    }
}
