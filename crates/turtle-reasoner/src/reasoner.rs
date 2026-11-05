//! The public `Reasoner`: the base triples, the closure of the last run,
//! and the run loop over the rule table.
//!
//! # State
//!
//! - **The dictionary**: every term seen, as a term id. Terms are never
//!   removed, so a long session of edits keeps the terms it no longer uses
//!   ([`Reasoner::clear`] starts a new one).
//! - **The base**: the triples loaded or inserted, deduplicated, as
//!   `[s, p, o]` term ids ([`Base`]): a sorted vector, plus a hash set of
//!   the triples added since its last merge, merged in once it passes an
//!   eighth of the vector. One addition costs a binary search and a hash
//!   insert, not a copy of the base. 12 bytes a triple (15 with the
//!   vector's slack at most), plus the hash set: at most an eighth as many
//!   entries, about 15 to 30 bytes each.
//! - **The closure**: an engine [`Store`] holding the base, the families'
//!   facts and every derived triple, generalized triples included (a
//!   literal subject, a predicate that is not an IRI; `docs/specs/v1.md`,
//!   Implementation Decisions). The reads leave the generalized ones out;
//!   they are counted round by round from the store's deltas, so
//!   [`Summary::inferred`] needs no pass over the closure.
//! - **The findings**: sorted and deduplicated after each run, and their
//!   public form, the [`Diagnostic`]s.
//!
//! # Runs
//!
//! [`Reasoner::reason`] does one of three things:
//!
//! - nothing, when the base has not changed since the last run (0 rounds);
//! - after additions only, an incremental run: the new base triples were
//!   queued into the stored closure as they came, and [`Store::run`] joins
//!   them semi-naively against it; the findings of earlier runs stay;
//! - after any removal (and on the first run, and after
//!   [`Reasoner::clear`]), a full run: a fresh store, every family's facts
//!   (inferred, never base; an empty base gets them too), then the base.
//!
//! Each round calls every family of the rule table in order with one
//! [`Context`], as `rules/testing.rs` does, and [`Counters`] records the
//! round's delta and what each family emitted.
//!
//! Between a change and the next `reason()`, the reads ([`Reasoner::triples`],
//! [`Reasoner::inferred`], [`Reasoner::diagnostics`], [`Reasoner::write`])
//! show the closure of the last run, told apart from the base as it now is.
//!
//! # Blank nodes
//!
//! Each loaded document has its own blank nodes, written `<label>_<n>` for
//! the n-th document loaded. Inserted blank nodes belong to no document and
//! keep their label, so `insert` then `triples()` gives the labels back; a
//! label that ends in `_` and digits gets `_0` appended, so that it never
//! takes a document node's label (`Dictionary::write_blank_label`).
//! [`Reasoner::remove`] and [`Reasoner::set_base`] name blank nodes as
//! `insert` does: they reach inserted blank nodes, not a document's.
//!
//! Loading the same document again (the same bytes, format and base IRI)
//! changes nothing while the triples it added are still in the base. Its
//! blank nodes would otherwise be new nodes, and the parsers label
//! anonymous ones (`[ ]`, lists) at random on every parse. A removal of any
//! base triple forgets which documents were loaded: loading one again after
//! a removal adds it again.

use crate::Diagnostic;
#[cfg(feature = "io")]
use crate::Error;
use crate::engine::{Relation, Store};
use crate::rules::{Context, FAMILIES, Family, Finding};
use crate::term::{Dictionary, DocId, Kind, TermId};
use oxrdf::{NamedNode, NamedOrBlankNode, Triple, TripleRef};
use rustc_hash::FxHashSet;

/// A document format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Turtle,
    NTriples,
    N3,
    RdfXml,
}

impl Format {
    /// The format of a file name's extension (`ttl`, `nt`, `n3`, `rdf`, `owl`, `xml`).
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "ttl" | "turtle" => Some(Self::Turtle),
            "nt" => Some(Self::NTriples),
            "n3" => Some(Self::N3),
            "rdf" | "owl" | "xml" => Some(Self::RdfXml),
            _ => None,
        }
    }
}

/// What a run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    /// Base triples.
    pub base: usize,
    /// Inferred triples (not base), after expanding `owl:sameAs`
    /// representatives: the RDF triples [`Reasoner::inferred`] gives.
    pub inferred: usize,
    /// Evaluation rounds this run took: 0 when nothing changed since the last run.
    pub rounds: usize,
    /// Diagnostics after the run.
    pub diagnostics: usize,
}

/// An OWL 2 RL reasoner over an in-memory graph. `Send`.
///
/// Change the base ([`Reasoner::load_str`], [`Reasoner::insert`],
/// [`Reasoner::remove`], [`Reasoner::set_base`]), then [`Reasoner::reason`];
/// the reads show the closure of the last run.
#[derive(Debug)]
pub struct Reasoner {
    /// The rule table: [`FAMILIES`], or a test's own.
    families: &'static [Family],
    terms: Dictionary,
    base: Base,
    /// The closure of the last run, plus the base triples added since,
    /// queued for the next run (unless that run is a full one).
    closure: Store,
    state: State,
    /// Triples of the closure that are not RDF (all of them inferred).
    hidden: usize,
    /// The findings of the closure, sorted and deduplicated.
    findings: Vec<Finding>,
    /// The public form of `findings`, in the same order.
    diagnostics: Vec<Diagnostic>,
    counters: Counters,
    /// The documents loaded since the last removal ([`document_key`]).
    #[cfg(feature = "io")]
    loaded: FxHashSet<u128>,
}

// The interface promises `Send`: checked here, at compile time.
const _: () = {
    const fn send<T: Send>() {}
    send::<Reasoner>()
};

/// What the next [`Reasoner::reason`] has to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Nothing: the closure is the base's.
    Current,
    /// Base triples were added (and queued in the closure): an incremental run.
    Added,
    /// A run from the base: first, after a removal, or after `clear`.
    Full,
}

impl Default for Reasoner {
    fn default() -> Self {
        Self::new()
    }
}

impl Reasoner {
    /// An empty reasoner.
    pub fn new() -> Self {
        Self::with_families(FAMILIES)
    }

    /// An empty reasoner over another rule table (tests use toy families).
    pub(crate) fn with_families(families: &'static [Family]) -> Self {
        Self {
            families,
            terms: Dictionary::new(),
            base: Base::default(),
            closure: Store::new(),
            state: State::Full,
            hidden: 0,
            findings: Vec::new(),
            diagnostics: Vec::new(),
            counters: Counters::new(families),
            #[cfg(feature = "io")]
            loaded: FxHashSet::default(),
        }
    }

    /// Adds a file's triples to the base (format from its extension). See
    /// [`Reasoner::load_str`].
    #[cfg(feature = "io")]
    pub fn load_file(&mut self, path: &std::path::Path) -> Result<(), Error> {
        let format = crate::io::format_of(path)?;
        let text = std::fs::read(path).map_err(|source| Error::Io {
            path: Some(path.into()),
            source,
        })?;
        self.load(&text, format, None, Some(path))
    }

    /// Adds a document's triples to the base. Blank nodes are scoped to this
    /// document. All or nothing: on an error the base is unchanged. Loading
    /// the same document again changes nothing, unless a base triple was
    /// removed since (see the module doc).
    #[cfg(feature = "io")]
    pub fn load_str(&mut self, text: &str, format: Format, base_iri: Option<&str>) -> Result<(), Error> {
        self.load(text.as_bytes(), format, base_iri, None)
    }

    #[cfg(feature = "io")]
    fn load(
        &mut self,
        text: &[u8],
        format: Format,
        base_iri: Option<&str>,
        path: Option<&std::path::Path>,
    ) -> Result<(), Error> {
        let key = document_key(text, format, base_iri);
        if self.loaded.contains(&key) {
            return Ok(());
        }
        let mut triples = Vec::new();
        crate::io::parse(&mut self.terms, text, format, base_iri, path, |[s, p, o]| {
            triples.push([s.0, p.0, o.0]);
        })?;
        self.add(triples);
        self.loaded.insert(key);
        Ok(())
    }

    /// Adds triples to the base. Blank nodes belong to no document: `_:x`
    /// here is `_:x` in every call, and keeps its label on output.
    pub fn insert<'a>(&mut self, triples: impl IntoIterator<Item = TripleRef<'a>>) {
        let batch: Vec<[u32; 3]> = triples.into_iter().map(|t| self.intern(t)).collect();
        self.add(batch);
    }

    /// Removes triples from the base (blank nodes named as by `insert`).
    /// Triples that are not base triples are ignored; when one was removed,
    /// the next run is a full one.
    pub fn remove<'a>(&mut self, triples: impl IntoIterator<Item = TripleRef<'a>>) {
        let gone: Vec<[u32; 3]> = triples.into_iter().filter_map(|t| self.lookup(t)).collect();
        if self.base.remove(gone) > 0 {
            self.restart();
        }
    }

    /// Makes the base exactly `triples` (blank nodes named as by `insert`).
    /// True when a base triple was removed, so the next run is a full one.
    /// An unchanged base costs nothing: the next run takes no rounds; a base
    /// that only grew runs incrementally.
    pub fn set_base<'a>(&mut self, triples: impl IntoIterator<Item = TripleRef<'a>>) -> bool {
        let mut new: Vec<[u32; 3]> = triples.into_iter().map(|t| self.intern(t)).collect();
        new.sort_unstable();
        new.dedup();
        let old = self.base.flush();
        if new == *old {
            return false;
        }
        match added(old, &new) {
            Some(more) => {
                self.queue(&more);
                self.base.replace(new);
                false
            }
            None => {
                self.base.replace(new);
                self.restart();
                true
            }
        }
    }

    /// Empties the base, the closure and the diagnostics (and the
    /// dictionary): as new.
    pub fn clear(&mut self) {
        *self = Self::with_families(self.families);
    }

    /// Runs to the fixpoint: in full, or over what changed since the last run.
    pub fn reason(&mut self) -> Summary {
        let rounds = match self.state {
            State::Current => {
                self.counters.clear();
                0
            }
            State::Added => self.run(false),
            State::Full => self.run(true),
        };
        self.state = State::Current;
        let base = self.base.len();
        debug_assert!(self.closure.len() >= base + self.hidden, "the closure holds the base");
        Summary {
            base,
            inferred: self.closure.len().saturating_sub(base + self.hidden),
            rounds,
            diagnostics: self.diagnostics.len(),
        }
    }

    /// The closure: base and inferred triples. Generalized triples (a
    /// literal subject, a predicate that is not an IRI) are left out.
    pub fn triples(&self) -> impl Iterator<Item = Triple> + '_ {
        self.closure.triples().filter_map(|(s, p, o)| self.rdf_triple(s, p, o))
    }

    /// Only the inferred triples (the closure's triples that are not base).
    pub fn inferred(&self) -> impl Iterator<Item = Triple> + '_ {
        self.closure
            .triples()
            .filter(|&(s, p, o)| !self.base.contains(&[s, p, o]))
            .filter_map(|(s, p, o)| self.rdf_triple(s, p, o))
    }

    /// The diagnostics of the last run, deduplicated on (code, terms).
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Writes the closure (or only the inferred triples) as `format`,
    /// sorted by subject so that Turtle groups each subject's triples.
    #[cfg(feature = "io")]
    pub fn write(&self, out: impl std::io::Write, format: Format, inferred_only: bool) -> Result<(), Error> {
        let mut triples: Vec<[TermId; 3]> = self
            .closure
            .triples()
            .filter(|&(s, p, _)| self.is_rdf(s, p))
            .filter(|&(s, p, o)| !inferred_only || !self.base.contains(&[s, p, o]))
            .map(|(s, p, o)| [TermId(s), TermId(p), TermId(o)])
            .collect();
        triples.sort_unstable();
        crate::io::write(&self.terms, triples, out, format).map(|_| ())
    }

    /// The counters of the last run.
    #[cfg(test)]
    pub(crate) fn counters(&self) -> &Counters {
        &self.counters
    }

    /// A triple of the caller's as term ids, interned (blank nodes of no document).
    fn intern(&mut self, t: TripleRef<'_>) -> [u32; 3] {
        [
            self.terms.intern_subject(t.subject, DocId::NONE).0,
            self.terms.intern_iri(t.predicate.as_str()).0,
            self.terms.intern_term(t.object, DocId::NONE).0,
        ]
    }

    /// A triple of the caller's as term ids, if every term is known.
    fn lookup(&self, t: TripleRef<'_>) -> Option<[u32; 3]> {
        Some([
            self.terms.get_term(t.subject.into(), DocId::NONE)?.0,
            self.terms.get_iri(t.predicate.as_str())?.0,
            self.terms.get_term(t.object, DocId::NONE)?.0,
        ])
    }

    /// Adds triples to the base, and queues the new ones for the next run.
    fn add(&mut self, batch: Vec<[u32; 3]>) {
        let new = self.base.fresh(batch);
        self.queue(&new);
        self.base.extend(new);
    }

    /// Queues new base triples into the closure for an incremental run (a
    /// full run inserts the whole base instead).
    fn queue(&mut self, new: &[[u32; 3]]) {
        if new.is_empty() || self.state == State::Full {
            return;
        }
        self.closure.extend(new.iter().map(|&[s, p, o]| (s, p, o)));
        self.state = State::Added;
    }

    /// A base triple was removed: the next run starts from the base.
    fn restart(&mut self) {
        self.state = State::Full;
        #[cfg(feature = "io")]
        self.loaded.clear();
    }

    /// Runs the rule table to the fixpoint: from the base and the facts in a
    /// fresh store (`full`), or from what was queued. Returns the rounds.
    fn run(&mut self, full: bool) -> usize {
        if full {
            // Drop the old closure before building the new one.
            self.closure = Store::new();
            self.hidden = 0;
            self.findings.clear();
            for family in self.families {
                self.closure
                    .extend(family.facts.iter().map(|&[s, p, o]| (s.0, p.0, o.0)));
            }
            self.closure.extend(self.base.iter().map(|[s, p, o]| (s, p, o)));
        }
        let families = self.families;
        let Self {
            terms,
            closure,
            hidden,
            counters,
            ..
        } = self;
        let terms: &Dictionary = terms;
        counters.clear();
        let mut found = Vec::new();
        let mut cx = Context {
            terms,
            findings: &mut found,
        };
        let mut seen = closure.len();
        let rounds = closure.run(|round| {
            let store = round.store();
            let len = store.len();
            counters.delta.push(len - seen);
            seen = len;
            *hidden += generalized(store, terms);
            for family in families {
                let before = round.emitted();
                (family.rules)(round, &mut cx);
                counters.emitted.push(round.emitted() - before);
            }
        });
        if full || !found.is_empty() {
            self.findings.append(&mut found);
            self.findings.sort_unstable();
            self.findings.dedup();
            self.diagnostics = self
                .findings
                .iter()
                .map(|f| Diagnostic::from_finding(f, &self.terms))
                .collect();
        }
        rounds
    }

    /// True when `(s, p, _)` is an RDF triple: a subject that is not a
    /// literal, an IRI predicate.
    fn is_rdf(&self, s: u32, p: u32) -> bool {
        self.terms.kind(TermId(s)) != Kind::Literal && self.terms.kind(TermId(p)) == Kind::Iri
    }

    /// The `oxrdf` triple of term ids, or `None` when it is not RDF.
    fn rdf_triple(&self, s: u32, p: u32, o: u32) -> Option<Triple> {
        if !self.is_rdf(s, p) {
            return None;
        }
        let subject: NamedOrBlankNode = match self.terms.to_oxrdf(TermId(s)) {
            oxrdf::Term::NamedNode(n) => n.into(),
            oxrdf::Term::BlankNode(b) => b.into(),
            _ => return None,
        };
        let predicate = NamedNode::new_unchecked(self.terms.iri(TermId(p))?);
        Some(Triple::new(subject, predicate, self.terms.to_oxrdf(TermId(o))))
    }
}

/// The generalized triples (not RDF) in the deltas of the store's tables:
/// called once a round, it counts each stored triple once over a run.
fn generalized(store: &Store, terms: &Dictionary) -> usize {
    let mut count = 0;
    for (p, table) in store.tables() {
        let recent = table.by_subject().recent();
        if recent.is_empty() {
            continue;
        }
        if terms.kind(TermId(p)) != Kind::Iri {
            count += recent.len();
            continue;
        }
        // Sorted by subject: one lookup per subject.
        let mut last: Option<(u32, bool)> = None;
        for &(s, _) in recent.iter() {
            let literal = match last {
                Some((l, literal)) if l == s => literal,
                _ => {
                    let literal = terms.kind(TermId(s)) == Kind::Literal;
                    last = Some((s, literal));
                    literal
                }
            };
            count += usize::from(literal);
        }
    }
    count
}

/// The triples in `new` and not in `old` (both sorted and deduplicated),
/// when `old` is a subset of `new`; `None` when it is not.
fn added(old: &[[u32; 3]], new: &[[u32; 3]]) -> Option<Vec<[u32; 3]>> {
    let mut more = Vec::with_capacity(new.len().saturating_sub(old.len()));
    let mut old = old.iter().peekable();
    for t in new {
        if old.next_if_eq(&t).is_none() {
            more.push(*t);
        }
    }
    old.peek().is_none().then_some(more)
}

/// Identifies a document for [`Reasoner::load_str`]'s "the same document
/// again": its bytes, format and base IRI, hashed twice (FxHash and
/// SipHash, 128 bits together), so that two different documents share a key
/// with a probability near 2^-128.
#[cfg(feature = "io")]
fn document_key(text: &[u8], format: Format, base_iri: Option<&str>) -> u128 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    fn feed(h: &mut impl Hasher, text: &[u8], format: Format, base_iri: Option<&str>) -> u64 {
        (format as u8).hash(h);
        base_iri.hash(h);
        text.len().hash(h);
        h.write(text);
        h.finish()
    }
    let fx = feed(&mut rustc_hash::FxHasher::default(), text, format, base_iri);
    let sip = feed(&mut DefaultHasher::new(), text, format, base_iri);
    (u128::from(fx) << 64) | u128::from(sip)
}

/// The fewest triples the base's hash set holds before it is merged into
/// the sorted vector.
const MERGE_AT_LEAST: usize = 4096;

/// The base triples, deduplicated: a sorted vector, and the triples added
/// since its last merge in a hash set (none of them in the vector). The set
/// is merged in when it passes [`MERGE_AT_LEAST`] or an eighth of the
/// vector, so an addition costs O(log n) plus O(1) amortized, and the
/// memory stays at 12 bytes a triple plus at most an eighth more.
#[derive(Debug, Default)]
struct Base {
    sorted: Vec<[u32; 3]>,
    recent: FxHashSet<[u32; 3]>,
}

impl Base {
    fn len(&self) -> usize {
        self.sorted.len() + self.recent.len()
    }

    fn contains(&self, t: &[u32; 3]) -> bool {
        self.sorted.binary_search(t).is_ok() || self.recent.contains(t)
    }

    fn iter(&self) -> impl Iterator<Item = [u32; 3]> + '_ {
        self.sorted.iter().chain(self.recent.iter()).copied()
    }

    /// The triples of `batch` that are not in the base: sorted, deduplicated.
    fn fresh(&self, mut batch: Vec<[u32; 3]>) -> Vec<[u32; 3]> {
        batch.sort_unstable();
        batch.dedup();
        batch.retain(|t| !self.contains(t));
        batch
    }

    /// Adds triples that are not in the base (as [`Base::fresh`] gives them).
    fn extend(&mut self, new: Vec<[u32; 3]>) {
        if new.is_empty() {
            return;
        }
        if self.recent.len() + new.len() <= MERGE_AT_LEAST.max(self.sorted.len() / 8) {
            self.recent.extend(new);
            return;
        }
        let mut more = new;
        more.extend(self.recent.drain());
        more.sort_unstable();
        self.merge(more);
    }

    /// Merges the hash set into the vector and returns the whole base, sorted.
    fn flush(&mut self) -> &[[u32; 3]] {
        if !self.recent.is_empty() {
            let mut more: Vec<[u32; 3]> = self.recent.drain().collect();
            more.sort_unstable();
            self.merge(more);
        }
        &self.sorted
    }

    /// Merges sorted triples that are not in `sorted` into it. The vector
    /// keeps at most a quarter of slack (a merge, like a parse buffer, can
    /// leave it twice as long as its triples).
    fn merge(&mut self, more: Vec<[u32; 3]>) {
        let sorted = std::mem::take(&mut self.sorted);
        self.sorted = Relation::from_sorted(sorted)
            .merge(Relation::from_sorted(more))
            .into_vec();
        if self.sorted.capacity() > self.sorted.len() + self.sorted.len() / 4 {
            self.sorted.shrink_to_fit();
        }
    }

    /// Makes the base exactly `sorted` (sorted and deduplicated).
    fn replace(&mut self, sorted: Vec<[u32; 3]>) {
        self.recent.clear();
        self.sorted = sorted;
    }

    /// Removes the base triples among `gone`; returns how many.
    fn remove(&mut self, mut gone: Vec<[u32; 3]>) -> usize {
        gone.retain(|t| self.contains(t));
        if gone.is_empty() {
            return 0;
        }
        gone.sort_unstable();
        gone.dedup();
        self.flush();
        let before = self.sorted.len();
        let mut next = gone.iter().peekable();
        self.sorted.retain(|t| {
            while next.next_if(|g| *g < t).is_some() {}
            next.next_if_eq(&t).is_none()
        });
        before - self.sorted.len()
    }
}

/// What the last run did, round by round: the counters experiment P0
/// (`docs/specs/experiments.md`) reads. Kept by every run; not part of the
/// public interface.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Counters {
    /// The families' names, in table order.
    pub(crate) families: Vec<&'static str>,
    /// Each round's delta: the tuples new in the store when the round
    /// starts (the store's length against the round before). The first
    /// round's is the run's input (the base and the facts in a full run,
    /// the additions in an incremental one); round r + 1's is what round r
    /// derived that was new.
    pub(crate) delta: Vec<usize>,
    /// The tuples each family emitted, duplicates included: round by round,
    /// one entry per family in table order (`emitted[round * families +
    /// family]`).
    pub(crate) emitted: Vec<usize>,
}

impl Counters {
    fn new(families: &[Family]) -> Self {
        Self {
            families: families.iter().map(|f| f.name).collect(),
            ..Self::default()
        }
    }

    fn clear(&mut self) {
        self.delta.clear();
        self.emitted.clear();
    }

    /// Rounds the last run took.
    #[cfg(test)]
    pub(crate) fn rounds(&self) -> usize {
        self.delta.len()
    }

    /// What the family called `name` emitted in each round.
    #[cfg(test)]
    pub(crate) fn emitted_by(&self, name: &str) -> Vec<usize> {
        let f = self
            .families
            .iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("no family {name:?}"));
        self.emitted
            .iter()
            .skip(f)
            .step_by(self.families.len())
            .copied()
            .collect()
    }
}

/// The reasoner over a toy rule table, so that incremental runs, findings
/// and generalized triples are tested while the real families are being
/// written. The tests that need only the public interface are in
/// `tests/reasoner.rs`.
#[cfg(all(test, feature = "io"))]
mod tests {
    use super::*;
    use crate::engine::{Pair, Round, Variable};
    use crate::term::vocab;
    use std::collections::BTreeSet;

    const PREFIXES: &str = "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .\n";

    fn ex(cx: &Context<'_>, local: &str) -> Option<u32> {
        cx.terms.get_iri(&format!("http://example.org/{local}")).map(|t| t.0)
    }

    fn all(v: &Variable<Pair>) -> Vec<Pair> {
        v.parts().flatten().copied().collect()
    }

    /// `ex:anc` is transitive: recent pairs joined with all, on both sides.
    fn transitive(round: &mut Round<'_>, cx: &mut Context<'_>) {
        let Some(anc) = ex(cx, "anc") else { return };
        let Some(v) = round.by_subject(anc) else { return };
        let (recent, all) = (v.recent().to_vec(), all(v));
        for &(x, y) in &recent {
            for &(_, z) in all.iter().filter(|p| p.0 == y) {
                round.emit(x, anc, z);
            }
        }
        for &(w, x) in &all {
            for &(_, y) in recent.iter().filter(|p| p.0 == x) {
                round.emit(w, anc, y);
            }
        }
    }

    /// cax-sco's shape, and a finding for every member of `ex:Bad`, made in
    /// every round and twice each time: duplicates on purpose.
    fn typing(round: &mut Round<'_>, cx: &mut Context<'_>) {
        let (ty, sub) = (vocab::RDF_TYPE.0, vocab::RDFS_SUB_CLASS_OF.0);
        let Some(types) = round.by_subject(ty) else { return };
        let (t_recent, t_all) = (types.recent().to_vec(), all(types));
        if let Some(subs) = round.by_subject(sub) {
            let (s_recent, s_all) = (subs.recent().to_vec(), all(subs));
            for &(x, c) in &t_recent {
                for &(_, d) in s_all.iter().filter(|p| p.0 == c) {
                    round.emit(x, ty, d);
                }
            }
            for &(x, c) in &t_all {
                for &(_, d) in s_recent.iter().filter(|p| p.0 == c) {
                    round.emit(x, ty, d);
                }
            }
        }
        if let Some(bad) = ex(cx, "Bad") {
            for &(x, _) in t_all.iter().filter(|p| p.1 == bad) {
                cx.report("toy-bad", &[x]);
                cx.report("toy-bad", &[x]);
            }
        }
    }

    /// Rules over generalized triples: `x ex:value v` gives `v ex:valueOf x`
    /// (a literal subject when `v` is a literal), which gives `x ex:seen v`;
    /// `x ex:predOf y` gives `x y x` (`y` as a predicate).
    fn general(round: &mut Round<'_>, cx: &mut Context<'_>) {
        let rule = |round: &mut Round<'_>, from: Option<u32>, to: Option<u32>| {
            let (Some(from), Some(to)) = (from, to) else { return };
            if let Some(v) = round.by_subject(from) {
                for &(x, y) in v.recent().iter() {
                    round.emit(y, to, x);
                }
            }
        };
        rule(round, ex(cx, "value"), ex(cx, "valueOf"));
        rule(round, ex(cx, "valueOf"), ex(cx, "seen"));
        if let Some(pred_of) = ex(cx, "predOf")
            && let Some(v) = round.by_subject(pred_of)
        {
            for &(x, y) in v.recent().iter() {
                round.emit(x, y, x);
            }
        }
    }

    static TOY: &[Family] = &[
        Family {
            name: "transitive",
            rules: transitive,
            facts: &[],
        },
        Family {
            name: "typing",
            rules: typing,
            facts: &[[vocab::OWL_THING, vocab::RDF_TYPE, vocab::OWL_CLASS]],
        },
        Family {
            name: "general",
            rules: general,
            facts: &[],
        },
    ];

    /// A reasoner over [`TOY`]. The toy rules' IRIs are interned first, as
    /// the real vocabulary is: a rule that found its head's IRI missing in
    /// one run would miss its derivation for good.
    fn toy() -> Reasoner {
        let mut r = Reasoner::with_families(TOY);
        for local in ["anc", "Bad", "value", "valueOf", "seen", "predOf"] {
            r.terms.intern_iri(&format!("http://example.org/{local}"));
        }
        r
    }

    fn load(r: &mut Reasoner, turtle: &str) {
        r.load_str(&format!("{PREFIXES}{turtle}"), Format::Turtle, None)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// Triples of a short Turtle text, as `oxrdf` triples (for `insert`).
    fn parse(turtle: &str) -> Vec<Triple> {
        oxttl::TurtleParser::new()
            .for_slice(format!("{PREFIXES}{turtle}").as_bytes())
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn short(term: &str) -> String {
        [
            ("http://example.org/", "ex:"),
            ("http://www.w3.org/1999/02/22-rdf-syntax-ns#", "rdf:"),
            ("http://www.w3.org/2000/01/rdf-schema#", "rdfs:"),
            ("http://www.w3.org/2002/07/owl#", "owl:"),
        ]
        .iter()
        .find_map(|(ns, prefix)| {
            term.strip_prefix('<')
                .and_then(|t| t.strip_prefix(ns))
                .map(|local| format!("{prefix}{}", local.trim_end_matches('>')))
        })
        .unwrap_or_else(|| term.to_owned())
    }

    /// Triples as `s p o` lines with short IRIs.
    fn lines(triples: impl Iterator<Item = Triple>) -> BTreeSet<String> {
        triples
            .map(|t| {
                format!(
                    "{} {} {}",
                    short(&t.subject.to_string()),
                    short(&t.predicate.to_string()),
                    short(&t.object.to_string())
                )
            })
            .collect()
    }

    /// The diagnostics as `code term ...` lines.
    fn findings(r: &Reasoner) -> BTreeSet<String> {
        r.diagnostics()
            .iter()
            .map(|d| {
                let terms: Vec<String> = d.terms.iter().map(|t| short(&t.to_string())).collect();
                format!("{} {}", d.code, terms.join(" "))
            })
            .collect()
    }

    fn set(text: &str) -> BTreeSet<String> {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn the_facts_hold_on_an_empty_base() {
        let mut r = toy();
        let summary = r.reason();
        assert_eq!(
            summary,
            Summary {
                base: 0,
                inferred: 1,
                rounds: 1,
                diagnostics: 0
            }
        );
        assert_eq!(lines(r.triples()), set("owl:Thing rdf:type owl:Class"));
        assert_eq!(lines(r.inferred()), set("owl:Thing rdf:type owl:Class"));
        // And again after `clear`, and after a removal empties the base.
        r.clear();
        assert_eq!(r.triples().count(), 0, "cleared, not yet run");
        assert_eq!(r.reason().inferred, 1);
        load(&mut r, "ex:a ex:anc ex:b .");
        r.reason();
        r.remove(parse("ex:a ex:anc ex:b .").iter().map(Triple::as_ref));
        assert_eq!(r.reason().base, 0);
        assert_eq!(lines(r.triples()), set("owl:Thing rdf:type owl:Class"));
    }

    const CHAIN: &str = "ex:a ex:anc ex:b . ex:b ex:anc ex:c . ex:c ex:anc ex:d .";

    #[test]
    fn counters_record_each_round() {
        let mut r = toy();
        load(&mut r, CHAIN);
        let s = r.reason();
        assert_eq!((s.base, s.inferred, s.rounds), (3, 3 + 1, 3));
        let c = r.counters();
        assert_eq!(c.families, ["transitive", "typing", "general"]);
        // Round 1 starts with the base and the fact; it derives a-c and b-d
        // (each twice), round 2 derives a-d (twice), round 3 nothing.
        assert_eq!(c.delta, [4, 2, 1]);
        assert_eq!(c.emitted_by("transitive"), [4, 2, 0]);
        assert_eq!(c.emitted_by("typing"), [0, 0, 0]);
        assert_eq!(c.rounds(), 3);

        // An incremental run starts from the addition alone.
        load(&mut r, "ex:d ex:anc ex:e .");
        let s = r.reason();
        assert_eq!((s.base, s.inferred, s.rounds), (4, 6 + 1, 2));
        let c = r.counters();
        assert_eq!(c.delta, [1, 3]);
        assert_eq!(c.emitted_by("transitive"), [3, 3]);

        // Nothing changed: no round.
        assert_eq!(r.reason().rounds, 0);
        assert_eq!(r.counters().rounds(), 0);
    }

    const INPUT: &str = "
        ex:a ex:anc ex:b . ex:b ex:anc ex:c . ex:c ex:anc ex:d . ex:d ex:anc ex:e .
        ex:C1 rdfs:subClassOf ex:C2 . ex:C2 rdfs:subClassOf ex:C3 . ex:C3 rdfs:subClassOf ex:Bad .
        ex:x a ex:C1 . ex:y a ex:C2 . _:n a ex:C3 . _:n ex:anc ex:a .
        ex:x ex:value \"five\" .
        ex:x ex:predOf ex:link . ex:y ex:predOf \"no\" . ex:y ex:predOf _:p .
    ";

    #[test]
    fn additions_in_batches_equal_one_run() {
        let triples = parse(INPUT);
        let mut one = toy();
        one.insert(triples.iter().map(Triple::as_ref));
        let whole = one.reason();
        let expected = (lines(one.triples()), lines(one.inferred()), findings(&one));
        assert!(whole.inferred > 15 && whole.diagnostics == 3, "{whole:?}");

        for batches in 1..=5 {
            for offset in 0..batches {
                let mut r = toy();
                let mut last = Summary::default();
                for b in 0..batches {
                    let batch = triples.iter().enumerate().filter(|(i, _)| (i + offset) % batches == b);
                    r.insert(batch.map(|(_, t)| t.as_ref()));
                    last = r.reason();
                }
                let got = (lines(r.triples()), lines(r.inferred()), findings(&r));
                assert_eq!(got, expected, "{batches} batches, offset {offset}");
                assert_eq!(
                    (last.base, last.inferred, last.diagnostics),
                    (whole.base, whole.inferred, whole.diagnostics)
                );
            }
        }
    }

    #[test]
    fn a_removal_removes_its_inferences() {
        let mut r = toy();
        load(&mut r, CHAIN);
        r.reason();
        assert!(lines(r.inferred()).contains("ex:a ex:anc ex:d"));

        // Removing what is not a base triple (here an inferred one) changes nothing.
        r.remove(
            parse("ex:a ex:anc ex:d . ex:q ex:anc ex:r .")
                .iter()
                .map(Triple::as_ref),
        );
        assert_eq!(r.reason().rounds, 0);

        r.remove(parse("ex:b ex:anc ex:c .").iter().map(Triple::as_ref));
        let s = r.reason();
        assert_eq!(
            r.counters().delta[0],
            2 + 1,
            "a full run starts from the base and the facts"
        );
        assert_eq!((s.base, s.inferred), (2, 1));
        assert_eq!(
            lines(r.triples()),
            set("ex:a ex:anc ex:b\nex:c ex:anc ex:d\nowl:Thing rdf:type owl:Class")
        );

        // Put back: the same closure as before the removal.
        r.insert(parse("ex:b ex:anc ex:c .").iter().map(Triple::as_ref));
        let s = r.reason();
        assert_eq!((s.base, s.inferred), (3, 4));
        assert_eq!(r.counters().delta[0], 1, "incremental again");
    }

    #[test]
    fn an_unchanged_base_costs_nothing() {
        let triples = parse(INPUT);
        let mut r = toy();
        assert!(!r.set_base(triples.iter().map(Triple::as_ref)));
        let first = r.reason();
        assert!(first.rounds > 0);

        // The same base, in another order and with duplicates: no round.
        let again: Vec<&Triple> = triples.iter().rev().chain(&triples).collect();
        assert!(!r.set_base(again.iter().map(|t| t.as_ref())));
        assert_eq!(r.reason(), Summary { rounds: 0, ..first });
        assert_eq!(r.counters().rounds(), 0);

        // A base that only grew runs incrementally.
        let more = parse("ex:e ex:anc ex:f .");
        assert!(!r.set_base(triples.iter().chain(&more).map(Triple::as_ref)));
        let grown = r.reason();
        assert_eq!(r.counters().delta[0], 1);
        assert_eq!(grown.base, first.base + 1);

        // A base that lost a triple runs in full.
        assert!(r.set_base(triples.iter().map(Triple::as_ref)));
        assert_eq!(r.reason(), first);
    }

    #[test]
    fn generalized_triples_feed_rules_but_stay_inside() {
        let mut r = toy();
        load(&mut r, INPUT);
        let s = r.reason();
        let closure = lines(r.triples());
        // `"five" ex:valueOf ex:x` has a literal subject: kept inside, where
        // it gives `ex:x ex:seen "five"`.
        assert!(closure.contains("ex:x ex:seen \"five\""), "{closure:#?}");
        assert!(!closure.iter().any(|t| t.starts_with('"')), "{closure:#?}");
        // `ex:x ex:link ex:x` is RDF; `ex:y "no" ex:y` and `ex:y _:p ex:y` are not.
        assert!(closure.contains("ex:x ex:link ex:x"));
        assert!(
            !closure
                .iter()
                .any(|t| t.starts_with("ex:y \"no\"") || t.starts_with("ex:y _:"))
        );
        let hidden = r.closure.len() - closure.len();
        assert_eq!(hidden, 3, "one literal subject, two predicates that are not IRIs");
        // The summary and the reads agree.
        assert_eq!(s.inferred, r.inferred().count());
        assert_eq!(s.base + s.inferred, closure.len());
        let mut out = Vec::new();
        r.write(&mut out, Format::NTriples, false).unwrap();
        assert_eq!(String::from_utf8(out).unwrap().lines().count(), closure.len());

        // Counted the same way by an incremental run.
        load(&mut r, "ex:z ex:value \"six\" . ex:z ex:predOf \"also no\" .");
        let s = r.reason();
        assert_eq!(s.inferred, r.inferred().count());
        assert_eq!(r.closure.len() - r.triples().count(), 5);
    }

    #[test]
    fn findings_are_deduplicated() {
        let mut r = toy();
        load(&mut r, INPUT);
        let s = r.reason();
        // ex:x, ex:y and _:n are in ex:Bad (through ex:C3): each reported
        // in several rounds, twice a round.
        let reported: Vec<String> = r.diagnostics().iter().map(|d| d.terms[0].to_string()).collect();
        assert_eq!(s.diagnostics, reported.len());
        let mut unique = reported.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(reported.len(), unique.len(), "{reported:?}");
        assert_eq!(s.diagnostics, 3, "{reported:?}");

        // An incremental run reports the old ones again, and one more.
        load(&mut r, "ex:w a ex:C1 .");
        assert_eq!(r.reason().diagnostics, 4);
        // A removal that takes ex:Bad away takes them all.
        r.remove(parse("ex:C3 rdfs:subClassOf ex:Bad .").iter().map(Triple::as_ref));
        assert_eq!(r.reason().diagnostics, 0);
        assert!(r.diagnostics().is_empty());
    }

    #[test]
    fn write_sorts_by_subject_and_can_leave_the_base_out() {
        let mut r = toy();
        load(&mut r, CHAIN);
        r.reason();
        let written = |inferred_only| {
            let mut out = Vec::new();
            r.write(&mut out, Format::NTriples, inferred_only).unwrap();
            String::from_utf8(out).unwrap()
        };
        let all = written(false);
        let read: Vec<Triple> = oxttl::NTriplesParser::new()
            .for_slice(all.as_bytes())
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(lines(read.iter().cloned()), lines(r.triples()));
        // Sorted by subject id: each subject's lines are together.
        let subjects: Vec<&str> = all.lines().map(|l| l.split(' ').next().unwrap()).collect();
        let mut groups = subjects.clone();
        groups.dedup();
        let distinct: BTreeSet<&str> = subjects.iter().copied().collect();
        assert_eq!(groups.len(), distinct.len(), "{all}");

        let inferred: Vec<Triple> = oxttl::NTriplesParser::new()
            .for_slice(written(true).as_bytes())
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(lines(inferred.into_iter()), lines(r.inferred()));
    }

    #[test]
    fn the_base_merges_its_recent_triples() {
        let mut base = Base::default();
        let t = |i: u32| [i, i % 7, i % 5];
        // Small batches stay in the hash set until it passes the bound.
        for i in 0..MERGE_AT_LEAST as u32 {
            let new = base.fresh(vec![t(i), t(i)]);
            assert_eq!(new.len(), 1);
            base.extend(new);
        }
        assert_eq!((base.sorted.len(), base.recent.len()), (0, MERGE_AT_LEAST));
        base.extend(base.fresh(vec![t(MERGE_AT_LEAST as u32)]));
        assert_eq!((base.sorted.len(), base.recent.len()), (MERGE_AT_LEAST + 1, 0));
        assert!(base.sorted.windows(2).all(|w| w[0] < w[1]));
        assert!(base.fresh((0..=MERGE_AT_LEAST as u32).map(t).collect()).is_empty());
        // Removal finds triples in either part.
        base.extend(base.fresh(vec![t(10_000)]));
        assert_eq!(base.remove(vec![t(3), t(10_000), t(20_000), t(3)]), 2);
        assert_eq!(base.len(), MERGE_AT_LEAST);
        assert!(!base.contains(&t(3)) && !base.contains(&t(10_000)) && base.contains(&t(4)));
        assert_eq!(base.iter().count(), base.len());
    }

    #[test]
    fn added_is_the_difference_of_a_superset() {
        let (a, b, c) = ([1, 1, 1], [2, 2, 2], [3, 3, 3]);
        assert_eq!(added(&[a, c], &[a, b, c]), Some(vec![b]));
        assert_eq!(added(&[], &[a]), Some(vec![a]));
        assert_eq!(added(&[a, b], &[a, b]), Some(vec![]));
        assert_eq!(added(&[a, b], &[a, c]), None);
        assert_eq!(added(&[c], &[a, b]), None);
    }
}
