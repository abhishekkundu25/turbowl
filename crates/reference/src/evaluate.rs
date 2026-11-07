//! The evaluation: the rules that are on, as naive loops over the whole
//! closure, repeated to a fixpoint.
//!
//! Every round rebuilds a few hash indexes from the closure, runs every rule
//! that is on over all of it (not only over what the last round added) and
//! adds what is new. There are no representatives: `owl:sameAs` is closed by
//! the eq-* rules as the W3C tables write them, so every alias gets a copy of
//! every triple. That is the point: an independent formulation of what the
//! core computes with representatives (ADR 0003). The rules themselves are
//! in `crate::rules`, one file per switch group; [`evaluate_within`] runs
//! those the switches leave on.
//!
//! Conventions:
//! - **Generalized triples inside, RDF outside.** The rules run over triples
//!   whose terms may be anything, as the W3C rule tables are written: a
//!   rule that matches with a literal where it would derive a subject (or
//!   anything but an IRI where it would derive a predicate) derives that
//!   triple, and every rule, the eq-* rules included, joins on it like any
//!   other. Only the returned closure leaves such triples out (they are not
//!   RDF). So a literal typed by prp-rng reaches dt-not-type, and two
//!   literals made equal (`ex:d a owl:FunctionalProperty . ex:x ex:d "a" ,
//!   "b"` gives `"a" owl:sameAs "b"` inside) copy each other's visible
//!   triples by eq-rep-o (`ex:y ex:q "a"` gives `ex:y ex:q "b"`); a literal
//!   made equal to an IRI shows as `ex:i owl:sameAs "a"` by eq-sym.
//! - Lists are read as the W3C rules read `LIST[...]` patterns, in Datalog
//!   form (as Nemo's program in `oracles/rules/` does): a list is matched along a
//!   path `z1` (the head), `z2`, ..., `zn` with `zi rdf:rest z(i+1)` and
//!   `zn rdf:rest rdf:nil`, where every cell has at least one `rdf:first`
//!   and each cell contributes the first chosen for it. `rdf:rest` stops at
//!   `rdf:nil`: `rdf:nil` is never a later cell, whatever triples it has,
//!   though it can be a head. So a cell is *well formed* when it has an
//!   `rdf:first` and an `rdf:rest` that is `rdf:nil` or a well-formed cell
//!   other than `rdf:nil`, and the members of a list (cls-int2, cls-uni) are
//!   the firsts of the well-formed cells reached from its head through
//!   well-formed cells. A cyclic list that never reaches `rdf:nil` matches
//!   nothing, a cell without an `rdf:first` breaks every path through it,
//!   shared tails are read once per head, and an empty list (`rdf:nil`
//!   itself) has no members. Every list of a class or property is read, and
//!   list triples derived during the run count from the next round on.

use crate::rules::on;
use crate::vocab::*;
use oxrdf::{NamedNode, NamedOrBlankNode, Term, Triple};
use std::collections::{BTreeMap, HashMap, HashSet};
use turbowl::Switches;

/// Diagnostics as a set of (code, terms). The code is the W3C rule name;
/// the terms come in this order, per code:
/// - `cax-dw`: the individual, then the two classes as `owl:disjointWith`
///   states them.
/// - `prp-pdw`: the two properties as `owl:propertyDisjointWith` states
///   them, then the subject and the object.
/// - `cls-nothing2`: the individual.
/// - `prp-asyp`: the property, the subject, the object.
/// - `prp-irp`: the property, the individual.
/// - `cls-com`: the two classes as `owl:complementOf` states them, then the
///   individual.
/// - `dt-not-type`: the literal, the datatype whose value space it is not
///   in (its own when it is ill-typed).
/// - `eq-diff1`: x, y, as `x owl:differentFrom y` states them.
/// - `eq-diff2`, `eq-diff3`: the `owl:AllDifferent` node, then the two
///   terms, the one at the earlier list position first.
/// - `cax-adc`: the individual, then the two classes, the one at the
///   earlier list position first.
/// - `prp-adp`: the two properties, the one at the earlier list position
///   first, then the subject and the object.
/// - `prp-npa1`, `prp-npa2`: the source individual, the property, the
///   target individual or value.
/// - `cls-maxc1`, `cls-maxqc1`, `cls-maxqc2`: the restriction, the
///   individual, the value.
pub type Diagnostics = HashSet<(String, Vec<Term>)>;

/// One evaluation: the closure, the diagnostics, and what each rule added.
#[derive(Debug, Clone, Default)]
pub struct Evaluation {
    /// The closure's RDF triples: every derived triple but those with a
    /// literal subject or a predicate that is not an IRI.
    pub closure: HashSet<Triple>,
    pub diagnostics: Diagnostics,
    /// For each rule that added something: the triples it added (deriving
    /// rules, counting the ones left out of `closure`) or the diagnostics
    /// it gave (diagnostic rules).
    pub fired: BTreeMap<&'static str, usize>,
    /// Rounds until nothing new was derived (the last round included).
    pub rounds: usize,
    /// How many derived triples are not RDF (a literal subject, or a
    /// predicate that is not an IRI), and so are not in `closure`.
    pub not_rdf: usize,
}

/// The closure and the diagnostics of `triples` in the default
/// configuration ([`Switches::default`]).
pub fn evaluate(triples: &HashSet<Triple>) -> (HashSet<Triple>, Diagnostics) {
    let evaluation = evaluate_counted(triples);
    (evaluation.closure, evaluation.diagnostics)
}

/// [`evaluate`], also counting what each rule added.
pub fn evaluate_counted(triples: &HashSet<Triple>) -> Evaluation {
    evaluate_within(triples, Switches::default(), usize::MAX).expect("no limit")
}

/// The evaluation of `triples` with the rules `switches` leave on, giving up
/// (`None`) once the closure, with the triples that are not RDF, would hold
/// more than `max_closure` triples (`usize::MAX`: no limit). Equality makes
/// some small inputs explode (a universal `owl:sameAs` clique copies every
/// triple for every pair of aliases), and this evaluator has no
/// representatives to keep that small.
pub fn evaluate_within(triples: &HashSet<Triple>, switches: Switches, max_closure: usize) -> Option<Evaluation> {
    let mut closure: HashSet<Fact> = triples.iter().map(fact).collect();
    let mut fired = BTreeMap::new();
    let mut rounds = 0;
    loop {
        rounds += 1;
        let view = View::new(&closure);
        let room = max_closure.saturating_sub(closure.len());
        let mut round = Round {
            closure: &closure,
            fresh: HashMap::new(),
            room,
            overflow: false,
            log: None,
        };
        for family in on(switches) {
            (family.derive)(&view, &mut round);
            if round.overflow {
                return None;
            }
        }
        let fresh = round.fresh;
        if fresh.is_empty() {
            break;
        }
        for (triple, rule) in fresh {
            *fired.entry(rule).or_insert(0) += 1;
            closure.insert(triple);
        }
    }
    let view = View::new(&closure);
    let mut found = Vec::new();
    for family in on(switches) {
        (family.diagnose)(&view, &mut found);
    }
    let mut diagnostics = Diagnostics::new();
    for (code, terms) in found {
        if diagnostics.insert((code.to_owned(), terms)) {
            *fired.entry(code).or_insert(0) += 1;
        }
    }
    let total = closure.len();
    let closure: HashSet<Triple> = closure.into_iter().filter_map(rdf).collect();
    Some(Evaluation {
        not_rdf: total - closure.len(),
        closure,
        diagnostics,
        fired,
        rounds,
    })
}

/// A generalized triple: any term in any position.
pub(crate) type Fact = (Term, Term, Term);

fn fact(triple: &Triple) -> Fact {
    (
        Term::from(triple.subject.clone()),
        Term::from(triple.predicate.clone()),
        triple.object.clone(),
    )
}

/// The RDF triple `fact` is, if it is one.
fn rdf((s, p, o): Fact) -> Option<Triple> {
    let subject = NamedOrBlankNode::try_from(s).ok()?;
    let predicate = NamedNode::try_from(p).ok()?;
    Some(Triple::new(subject, predicate, o))
}

/// The closure as it stands at the start of a round, indexed every way the
/// rules look it up. Subjects and predicates are kept as terms, so a lookup
/// never needs to know which position a term came from.
pub(crate) struct View {
    by_p: HashMap<Term, Vec<(Term, Term)>>,
    by_s: HashMap<Term, Vec<(Term, Term)>>,
    by_o: HashMap<Term, Vec<(Term, Term)>>,
    sp: HashMap<(Term, Term), Vec<Term>>,
    po: HashMap<(Term, Term), Vec<Term>>,
    pub(crate) all: HashSet<(Term, Term, Term)>,
    /// Well-formed list cells: an `rdf:first`, and an `rdf:rest` that is
    /// `rdf:nil` or a well-formed cell other than `rdf:nil`.
    well_formed: HashSet<Term>,
}

const NONE: &[Term] = &[];
const NO_PAIRS: &[(Term, Term)] = &[];

impl View {
    pub(crate) fn new(closure: &HashSet<Fact>) -> View {
        let mut view = View {
            by_p: HashMap::new(),
            by_s: HashMap::new(),
            by_o: HashMap::new(),
            sp: HashMap::new(),
            po: HashMap::new(),
            all: HashSet::new(),
            well_formed: HashSet::new(),
        };
        for (s, p, o) in closure {
            view.by_p.entry(p.clone()).or_default().push((s.clone(), o.clone()));
            view.by_s.entry(s.clone()).or_default().push((p.clone(), o.clone()));
            view.by_o.entry(o.clone()).or_default().push((s.clone(), p.clone()));
            view.sp.entry((s.clone(), p.clone())).or_default().push(o.clone());
            view.po.entry((p.clone(), o.clone())).or_default().push(s.clone());
            view.all.insert((s.clone(), p.clone(), o.clone()));
        }
        // A cell is well formed when it has a first, and its rest is nil or
        // a well-formed cell other than nil.
        let (first, rest, nil) = (iri(RDF_FIRST), iri(RDF_REST), iri(RDF_NIL));
        let mut well_formed = HashSet::new();
        let mut changed = true;
        while changed {
            changed = false;
            for (cell, tail) in view.pairs(&rest) {
                if !well_formed.contains(cell)
                    && !view.objects(cell, &first).is_empty()
                    && (*tail == nil || well_formed.contains(tail))
                {
                    well_formed.insert(cell.clone());
                    changed = true;
                }
            }
        }
        view.well_formed = well_formed;
        view
    }

    /// The (subject, object) pairs of predicate `p`.
    pub(crate) fn pairs(&self, p: &Term) -> &[(Term, Term)] {
        self.by_p.get(p).map_or(NO_PAIRS, Vec::as_slice)
    }

    /// The (predicate, object) pairs of subject `s`.
    pub(crate) fn about(&self, s: &Term) -> &[(Term, Term)] {
        self.by_s.get(s).map_or(NO_PAIRS, Vec::as_slice)
    }

    /// The (subject, predicate) pairs of object `o`.
    pub(crate) fn pointing_at(&self, o: &Term) -> &[(Term, Term)] {
        self.by_o.get(o).map_or(NO_PAIRS, Vec::as_slice)
    }

    pub(crate) fn objects(&self, s: &Term, p: &Term) -> &[Term] {
        self.sp.get(&(s.clone(), p.clone())).map_or(NONE, Vec::as_slice)
    }

    pub(crate) fn subjects(&self, p: &Term, o: &Term) -> &[Term] {
        self.po.get(&(p.clone(), o.clone())).map_or(NONE, Vec::as_slice)
    }

    pub(crate) fn has(&self, s: &Term, p: &Term, o: &Term) -> bool {
        self.all.contains(&(s.clone(), p.clone(), o.clone()))
    }

    /// The subjects typed `class`.
    pub(crate) fn instances(&self, class: &str) -> &[Term] {
        self.subjects(&iri(RDF_TYPE), &iri(class))
    }

    /// The cells of the list at `head`: `head` and every cell its
    /// `rdf:rest` path reaches, in no particular order.
    pub(crate) fn cells(&self, head: &Term) -> Vec<Term> {
        let rest = iri(RDF_REST);
        let mut seen = HashSet::from([head.clone()]);
        let mut cells = vec![head.clone()];
        let mut i = 0;
        while i < cells.len() {
            for tail in self.objects(&cells[i], &rest) {
                if seen.insert(tail.clone()) {
                    cells.push(tail.clone());
                }
            }
            i += 1;
        }
        cells
    }

    /// The members of the list at `head`: the `rdf:first` of every
    /// well-formed cell reached from `head` (itself well formed) through
    /// well-formed cells, never through `rdf:nil`. These are the cells on
    /// some path from `head` to `rdf:nil` whose every cell has a first.
    pub(crate) fn members(&self, head: &Term) -> Vec<Term> {
        let (first, rest, nil) = (iri(RDF_FIRST), iri(RDF_REST), iri(RDF_NIL));
        if !self.well_formed.contains(head) {
            return Vec::new();
        }
        let mut seen = HashSet::from([head.clone()]);
        let mut cells = vec![head.clone()];
        let mut i = 0;
        while i < cells.len() {
            for tail in self.objects(&cells[i], &rest) {
                if *tail != nil && self.well_formed.contains(tail) && seen.insert(tail.clone()) {
                    cells.push(tail.clone());
                }
            }
            i += 1;
        }
        cells
            .iter()
            .flat_map(|cell| self.objects(cell, &first).iter().cloned())
            .collect()
    }

    /// Whether some path from `head` to `rdf:nil` has, at every cell, a
    /// member that satisfies `holds` (cls-int1's "every member"). `cells`
    /// are the list's cells ([`View::cells`]); they are visited from the
    /// last found, so a simple list settles in one sweep.
    pub(crate) fn every_member(&self, head: &Term, cells: &[Term], holds: impl Fn(&Term) -> bool) -> bool {
        let (first, rest, nil) = (iri(RDF_FIRST), iri(RDF_REST), iri(RDF_NIL));
        let mut good: HashSet<Term> = HashSet::new();
        let mut changed = true;
        while changed {
            changed = false;
            for cell in cells.iter().rev() {
                if good.contains(cell) {
                    continue;
                }
                let member_holds = self.objects(cell, &first).iter().any(&holds);
                let tail_good = self.objects(cell, &rest).iter().any(|t| *t == nil || good.contains(t));
                if member_holds && tail_good {
                    good.insert(cell.clone());
                    changed = true;
                }
            }
        }
        good.contains(head)
    }

    /// The (u, v) pairs a property chain whose list starts at `head` joins:
    /// `u p1 x1 ... x(n-1) pn v` along some path from `head` to `rdf:nil`.
    /// `None` once the pairs joined for all the list's suffixes pass `cap`.
    ///
    /// For each cell, the pairs the list from that cell to `rdf:nil` joins,
    /// to a fixpoint: a cell's are its firsts' pairs when a rest is
    /// `rdf:nil`, and its firsts' pairs joined with its other rests' pairs.
    /// The join distributes over unions, so each cell joins the union of its
    /// firsts' pairs with the union of its rests' once (a cell with many
    /// firsts and rests would otherwise find each pair many times), and
    /// each sweep joins only what the rests gained in the sweep before (the
    /// first sweep: the pairs of the cells that end at `rdf:nil`).
    pub(crate) fn chain(&self, head: &Term, cap: usize) -> Option<Vec<(Term, Term)>> {
        let (first, rest, nil) = (iri(RDF_FIRST), iri(RDF_REST), iri(RDF_NIL));
        let cells = self.cells(head);
        // For each cell: u -> every v the chain from that cell to nil joins.
        let mut joined: HashMap<Term, HashMap<Term, HashSet<Term>>> = HashMap::new();
        // What each cell gained in the last sweep: x -> the v's, as above.
        let mut gained: HashMap<Term, HashMap<Term, HashSet<Term>>> = HashMap::new();
        // Each cell's firsts' pairs, by their object x: x -> the u's.
        let firsts: Vec<HashMap<&Term, HashSet<&Term>>> = cells
            .iter()
            .map(|cell| {
                let mut by_x: HashMap<&Term, HashSet<&Term>> = HashMap::new();
                for p in self.objects(cell, &first) {
                    for (u, x) in self.pairs(p) {
                        by_x.entry(x).or_default().insert(u);
                    }
                }
                by_x
            })
            .collect();
        let mut total = 0usize;
        let mut sweep = 0;
        loop {
            let mut next: HashMap<Term, HashMap<Term, HashSet<Term>>> = HashMap::new();
            for (cell, firsts) in cells.iter().zip(&firsts) {
                // The rests' new pairs, by their subject x: x -> the v's.
                let mut tails: HashMap<&Term, HashSet<&Term>> = HashMap::new();
                let mut to_nil = false;
                for tail in self.objects(cell, &rest) {
                    if *tail == nil {
                        to_nil = true;
                    } else if let Some(after) = gained.get(tail) {
                        for (x, vs) in after {
                            tails.entry(x).or_default().extend(vs);
                        }
                    }
                }
                let mut found: Vec<(&Term, &Term)> = Vec::new();
                if to_nil && sweep == 0 {
                    found.extend(firsts.iter().flat_map(|(x, us)| us.iter().map(move |u| (*u, *x))));
                }
                for (x, us) in firsts {
                    if let Some(vs) = tails.get(x) {
                        found.extend(us.iter().flat_map(|u| vs.iter().map(move |v| (*u, *v))));
                    }
                }
                let map = joined.entry(cell.clone()).or_default();
                for (u, v) in found {
                    if map.entry(u.clone()).or_default().insert(v.clone()) {
                        total += 1;
                        if total > cap {
                            return None;
                        }
                        next.entry(cell.clone())
                            .or_default()
                            .entry(u.clone())
                            .or_default()
                            .insert(v.clone());
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            gained = next;
            sweep += 1;
        }
        let from_head = joined.remove(head).unwrap_or_default();
        Some(
            from_head
                .into_iter()
                .flat_map(|(u, vs)| vs.into_iter().map(move |v| (u.clone(), v)))
                .collect(),
        )
    }
}

/// What one round derives, with the first rule that derived each triple.
pub(crate) struct Round<'a> {
    closure: &'a HashSet<Fact>,
    fresh: HashMap<Fact, &'static str>,
    /// How many fresh triples fit under the limit.
    pub(crate) room: usize,
    pub(crate) overflow: bool,
    /// Every (triple, rule) a rule derives, known or not, when kept (the
    /// proof checker's round, `crate::proof`).
    log: Option<HashSet<(Fact, &'static str)>>,
}

impl<'a> Round<'a> {
    /// A round over `closure` that keeps every (triple, rule) its rules
    /// derive, not only the first rule of each new triple.
    pub(crate) fn logged(closure: &'a HashSet<Fact>) -> Self {
        Round {
            closure,
            fresh: HashMap::new(),
            room: usize::MAX,
            overflow: false,
            log: Some(HashSet::new()),
        }
    }

    /// The (triple, rule) pairs a [`Round::logged`] round derived.
    pub(crate) fn into_log(self) -> HashSet<(Fact, &'static str)> {
        self.log.unwrap_or_default()
    }

    /// Adds `s p o` unless it is already known. It need not be RDF.
    pub(crate) fn derive(&mut self, rule: &'static str, s: &Term, p: &Term, o: &Term) {
        let triple = (s.clone(), p.clone(), o.clone());
        if let Some(log) = &mut self.log {
            log.insert((triple.clone(), rule));
        }
        if self.overflow || self.closure.contains(&triple) || self.fresh.contains_key(&triple) {
            return;
        }
        if self.fresh.len() == self.room {
            self.overflow = true;
            return;
        }
        self.fresh.insert(triple, rule);
    }
}
