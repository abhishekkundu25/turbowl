//! The rules, as naive loops over the whole closure, repeated to a fixpoint.
//!
//! Every round rebuilds a few hash indexes from the closure, runs every rule
//! over all of it (not only over what the last round added) and adds what is
//! new. There are no representatives: `owl:sameAs` is closed by the eq-*
//! rules as the W3C tables write them, so every alias gets a copy of every
//! triple. That is the point: an independent formulation of what the core
//! computes with representatives (ADR 0003).
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
//!   form (as `oracles/nemo/owl2rl.rls` does): a list is matched along a
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

use crate::datatype;
use crate::vocab::*;
use oxrdf::{NamedNode, NamedOrBlankNode, Term, Triple};
use std::collections::{BTreeMap, HashMap, HashSet};

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
pub type Diagnostics = HashSet<(String, Vec<Term>)>;

/// The deriving rules, in the order a round runs them. A triple two rules
/// derive in the same round is counted for the first.
pub const RULES: &[&str] = &[
    "cls-thing",
    "cls-nothing1",
    "eq-sym",
    "eq-trans",
    "eq-rep-s",
    "eq-rep-p",
    "eq-rep-o",
    "prp-dom",
    "prp-rng",
    "prp-fp",
    "prp-ifp",
    "prp-symp",
    "prp-trp",
    "prp-spo1",
    "prp-spo2",
    "prp-eqp1",
    "prp-eqp2",
    "prp-inv1",
    "prp-inv2",
    "cax-sco",
    "cax-eqc1",
    "cax-eqc2",
    "scm-sco",
    "scm-eqc1",
    "cls-int1",
    "cls-int2",
    "cls-uni",
    "cls-svf1",
    "cls-svf2",
    "cls-avf",
    "cls-hv1",
    "cls-hv2",
];

/// The diagnostic rules, run once over the closure at the fixpoint. Each
/// name is also the code of what it finds.
pub const DIAGNOSTIC_RULES: &[&str] = &[
    "cax-dw",
    "prp-pdw",
    "cls-nothing2",
    "prp-asyp",
    "prp-irp",
    "cls-com",
    "dt-not-type",
];

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

/// The closure and the diagnostics of `triples`.
pub fn evaluate(triples: &HashSet<Triple>) -> (HashSet<Triple>, Diagnostics) {
    let evaluation = evaluate_counted(triples);
    (evaluation.closure, evaluation.diagnostics)
}

/// [`evaluate`], also counting what each rule added.
pub fn evaluate_counted(triples: &HashSet<Triple>) -> Evaluation {
    evaluate_within(triples, usize::MAX).expect("no limit")
}

/// [`evaluate_counted`], giving up (`None`) once the closure, with the
/// triples that are not RDF, would hold more than `max_closure` triples.
/// Equality makes some small inputs explode (a universal `owl:sameAs`
/// clique copies every triple for every pair of aliases), and this
/// evaluator has no representatives to keep that small.
pub fn evaluate_within(triples: &HashSet<Triple>, max_closure: usize) -> Option<Evaluation> {
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
        };
        derive_all(&view, &mut round);
        if round.overflow {
            return None;
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
    let mut diagnostics = Diagnostics::new();
    for (code, terms) in diagnose(&View::new(&closure)) {
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
type Fact = (Term, Term, Term);

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
struct View {
    by_p: HashMap<Term, Vec<(Term, Term)>>,
    by_s: HashMap<Term, Vec<(Term, Term)>>,
    by_o: HashMap<Term, Vec<(Term, Term)>>,
    sp: HashMap<(Term, Term), Vec<Term>>,
    po: HashMap<(Term, Term), Vec<Term>>,
    all: HashSet<(Term, Term, Term)>,
    /// Well-formed list cells: an `rdf:first`, and an `rdf:rest` that is
    /// `rdf:nil` or a well-formed cell other than `rdf:nil`.
    well_formed: HashSet<Term>,
}

const NONE: &[Term] = &[];
const NO_PAIRS: &[(Term, Term)] = &[];

impl View {
    fn new(closure: &HashSet<Fact>) -> View {
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
    fn pairs(&self, p: &Term) -> &[(Term, Term)] {
        self.by_p.get(p).map_or(NO_PAIRS, Vec::as_slice)
    }

    /// The (predicate, object) pairs of subject `s`.
    fn about(&self, s: &Term) -> &[(Term, Term)] {
        self.by_s.get(s).map_or(NO_PAIRS, Vec::as_slice)
    }

    /// The (subject, predicate) pairs of object `o`.
    fn pointing_at(&self, o: &Term) -> &[(Term, Term)] {
        self.by_o.get(o).map_or(NO_PAIRS, Vec::as_slice)
    }

    fn objects(&self, s: &Term, p: &Term) -> &[Term] {
        self.sp.get(&(s.clone(), p.clone())).map_or(NONE, Vec::as_slice)
    }

    fn subjects(&self, p: &Term, o: &Term) -> &[Term] {
        self.po.get(&(p.clone(), o.clone())).map_or(NONE, Vec::as_slice)
    }

    fn has(&self, s: &Term, p: &Term, o: &Term) -> bool {
        self.all.contains(&(s.clone(), p.clone(), o.clone()))
    }

    /// The cells of the list at `head`: `head` and every cell its
    /// `rdf:rest` path reaches, in no particular order.
    fn cells(&self, head: &Term) -> Vec<Term> {
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
    fn members(&self, head: &Term) -> Vec<Term> {
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
    fn every_member(&self, head: &Term, cells: &[Term], holds: impl Fn(&Term) -> bool) -> bool {
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
    fn chain(&self, head: &Term, cap: usize) -> Option<Vec<(Term, Term)>> {
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
struct Round<'a> {
    closure: &'a HashSet<Fact>,
    fresh: HashMap<Fact, &'static str>,
    /// How many fresh triples fit under the limit.
    room: usize,
    overflow: bool,
}

impl Round<'_> {
    /// Adds `s p o` unless it is already known. It need not be RDF.
    fn derive(&mut self, rule: &'static str, s: &Term, p: &Term, o: &Term) {
        let triple = (s.clone(), p.clone(), o.clone());
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

fn derive_all(v: &View, r: &mut Round) {
    let ty = iri(RDF_TYPE);

    // cls-thing, cls-nothing1: rules with an empty body.
    r.derive("cls-thing", &iri(OWL_THING), &ty, &iri(OWL_CLASS));
    r.derive("cls-nothing1", &iri(OWL_NOTHING), &ty, &iri(OWL_CLASS));

    equality(v, r);
    properties(v, r);
    if !r.overflow {
        classes(v, r);
    }
}

fn equality(v: &View, r: &mut Round) {
    let same = iri(OWL_SAME_AS);
    for (x, y) in v.pairs(&same) {
        r.derive("eq-sym", y, &same, x);
        for z in v.objects(y, &same) {
            r.derive("eq-trans", x, &same, z);
        }
        for (p, o) in v.about(x) {
            r.derive("eq-rep-s", y, p, o);
        }
        for (s, o) in v.pairs(x) {
            r.derive("eq-rep-p", s, y, o);
        }
        for (s, p) in v.pointing_at(x) {
            r.derive("eq-rep-o", s, p, y);
        }
    }
}

/// The subjects typed `class`.
fn instances<'v>(v: &'v View, class: &str) -> &'v [Term] {
    v.subjects(&iri(RDF_TYPE), &iri(class))
}

fn properties(v: &View, r: &mut Round) {
    let ty = iri(RDF_TYPE);
    let same = iri(OWL_SAME_AS);

    for (p, c) in v.pairs(&iri(RDFS_DOMAIN)) {
        for (x, _) in v.pairs(p) {
            r.derive("prp-dom", x, &ty, c);
        }
    }
    for (p, c) in v.pairs(&iri(RDFS_RANGE)) {
        for (_, y) in v.pairs(p) {
            r.derive("prp-rng", y, &ty, c);
        }
    }
    for p in instances(v, OWL_FUNCTIONAL_PROPERTY) {
        for (x, y1) in v.pairs(p) {
            for y2 in v.objects(x, p) {
                r.derive("prp-fp", y1, &same, y2);
            }
        }
    }
    for p in instances(v, OWL_INVERSE_FUNCTIONAL_PROPERTY) {
        for (x1, y) in v.pairs(p) {
            for x2 in v.subjects(p, y) {
                r.derive("prp-ifp", x1, &same, x2);
            }
        }
    }
    for p in instances(v, OWL_SYMMETRIC_PROPERTY) {
        for (x, y) in v.pairs(p) {
            r.derive("prp-symp", y, p, x);
        }
    }
    for p in instances(v, OWL_TRANSITIVE_PROPERTY) {
        for (x, y) in v.pairs(p) {
            for z in v.objects(y, p) {
                r.derive("prp-trp", x, p, z);
            }
        }
    }
    for (p1, p2) in v.pairs(&iri(RDFS_SUB_PROPERTY_OF)) {
        for (x, y) in v.pairs(p1) {
            r.derive("prp-spo1", x, p2, y);
        }
    }
    for (p, head) in v.pairs(&iri(OWL_PROPERTY_CHAIN_AXIOM)) {
        let Some(pairs) = v.chain(head, r.room) else {
            r.overflow = true;
            return;
        };
        for (u, w) in pairs {
            r.derive("prp-spo2", &u, p, &w);
        }
    }
    for (p1, p2) in v.pairs(&iri(OWL_EQUIVALENT_PROPERTY)) {
        for (x, y) in v.pairs(p1) {
            r.derive("prp-eqp1", x, p2, y);
        }
        for (x, y) in v.pairs(p2) {
            r.derive("prp-eqp2", x, p1, y);
        }
    }
    for (p1, p2) in v.pairs(&iri(OWL_INVERSE_OF)) {
        for (x, y) in v.pairs(p1) {
            r.derive("prp-inv1", y, p2, x);
        }
        for (x, y) in v.pairs(p2) {
            r.derive("prp-inv2", y, p1, x);
        }
    }
}

fn classes(v: &View, r: &mut Round) {
    let ty = iri(RDF_TYPE);
    let sco = iri(RDFS_SUB_CLASS_OF);
    let on_property = iri(OWL_ON_PROPERTY);

    for (c1, c2) in v.pairs(&sco) {
        for x in v.subjects(&ty, c1) {
            r.derive("cax-sco", x, &ty, c2);
        }
        for c3 in v.objects(c2, &sco) {
            r.derive("scm-sco", c1, &sco, c3);
        }
    }
    for (c1, c2) in v.pairs(&iri(OWL_EQUIVALENT_CLASS)) {
        for x in v.subjects(&ty, c1) {
            r.derive("cax-eqc1", x, &ty, c2);
        }
        for x in v.subjects(&ty, c2) {
            r.derive("cax-eqc2", x, &ty, c1);
        }
        r.derive("scm-eqc1", c1, &sco, c2);
        r.derive("scm-eqc1", c2, &sco, c1);
    }
    for (c, head) in v.pairs(&iri(OWL_INTERSECTION_OF)) {
        let members = v.members(head);
        let candidates: HashSet<&Term> = members.iter().flat_map(|k| v.subjects(&ty, k)).collect();
        let cells = v.cells(head);
        for y in candidates {
            if v.every_member(head, &cells, |k| v.has(y, &ty, k)) {
                r.derive("cls-int1", y, &ty, c);
            }
        }
        for y in v.subjects(&ty, c) {
            for k in &members {
                r.derive("cls-int2", y, &ty, k);
            }
        }
    }
    for (c, head) in v.pairs(&iri(OWL_UNION_OF)) {
        for k in v.members(head) {
            for y in v.subjects(&ty, &k) {
                r.derive("cls-uni", y, &ty, c);
            }
        }
    }
    let thing = iri(OWL_THING);
    for (x, y) in v.pairs(&iri(OWL_SOME_VALUES_FROM)) {
        for p in v.objects(x, &on_property) {
            for (u, value) in v.pairs(p) {
                if v.has(value, &ty, y) {
                    r.derive("cls-svf1", u, &ty, x);
                }
                if *y == thing {
                    r.derive("cls-svf2", u, &ty, x);
                }
            }
        }
    }
    for (x, y) in v.pairs(&iri(OWL_ALL_VALUES_FROM)) {
        for p in v.objects(x, &on_property) {
            for u in v.subjects(&ty, x) {
                for value in v.objects(u, p) {
                    r.derive("cls-avf", value, &ty, y);
                }
            }
        }
    }
    for (x, y) in v.pairs(&iri(OWL_HAS_VALUE)) {
        for p in v.objects(x, &on_property) {
            for u in v.subjects(&ty, x) {
                r.derive("cls-hv1", u, p, y);
            }
            for u in v.subjects(p, y) {
                r.derive("cls-hv2", u, &ty, x);
            }
        }
    }
}

/// The diagnostic rules over the closure: (code, terms), in the orders
/// [`Diagnostics`] documents.
fn diagnose(v: &View) -> Vec<(&'static str, Vec<Term>)> {
    let ty = iri(RDF_TYPE);
    let mut found = Vec::new();

    for (c1, c2) in v.pairs(&iri(OWL_DISJOINT_WITH)) {
        for x in v.subjects(&ty, c1) {
            if v.has(x, &ty, c2) {
                found.push(("cax-dw", vec![x.clone(), c1.clone(), c2.clone()]));
            }
        }
    }
    for (p1, p2) in v.pairs(&iri(OWL_PROPERTY_DISJOINT_WITH)) {
        for (x, y) in v.pairs(p1) {
            if v.has(x, p2, y) {
                found.push(("prp-pdw", vec![p1.clone(), p2.clone(), x.clone(), y.clone()]));
            }
        }
    }
    for x in instances(v, OWL_NOTHING) {
        found.push(("cls-nothing2", vec![x.clone()]));
    }
    for p in instances(v, OWL_ASYMMETRIC_PROPERTY) {
        for (x, y) in v.pairs(p) {
            if v.has(y, p, x) {
                found.push(("prp-asyp", vec![p.clone(), x.clone(), y.clone()]));
            }
        }
    }
    for p in instances(v, OWL_IRREFLEXIVE_PROPERTY) {
        for (x, y) in v.pairs(p) {
            if x == y {
                found.push(("prp-irp", vec![p.clone(), x.clone()]));
            }
        }
    }
    for (c1, c2) in v.pairs(&iri(OWL_COMPLEMENT_OF)) {
        for x in v.subjects(&ty, c1) {
            if v.has(x, &ty, c2) {
                found.push(("cls-com", vec![c1.clone(), c2.clone(), x.clone()]));
            }
        }
    }
    // dt-not-type, for an ill-typed literal anywhere in the closure: the
    // literal is not in its own datatype's value space.
    for (s, p, o) in &v.all {
        for term in [s, p, o] {
            if let Term::Literal(literal) = term
                && datatype::well_formed(literal) == Some(false)
            {
                let own = Term::from(literal.datatype().into_owned());
                found.push(("dt-not-type", vec![term.clone(), own]));
            }
        }
    }
    // dt-not-type, for a literal the closure types with a datatype (by
    // prp-rng or any other rule) whose value space it is not in.
    for (x, class) in v.pairs(&ty) {
        if let (Term::Literal(literal), Term::NamedNode(class)) = (x, class)
            && datatype::in_value_space(literal, class.as_str()) == Some(false)
        {
            found.push(("dt-not-type", vec![x.clone(), Term::from(class.clone())]));
        }
    }
    found
}
