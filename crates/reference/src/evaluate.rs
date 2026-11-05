//! The rules, as naive loops over the whole closure, repeated to a fixpoint.
//!
//! Every round rebuilds a few hash indexes from the closure, runs every rule
//! over all of it (not only over what the last round added) and adds what is
//! new. There are no representatives: `owl:sameAs` is closed by the eq-*
//! rules as the W3C tables write them, so every alias gets a copy of every
//! triple. That is the point: an independent formulation of what the core
//! computes with representatives (ADR 0003).
//!
//! Conventions both profiles share:
//! - A triple whose subject would be a literal, or whose predicate would not
//!   be an IRI, is never derived (D6), so no rule ever joins on one: a
//!   functional property with the values `"a"` and `"b"` derives neither
//!   `"a" sameAs "b"` nor anything that would follow from it.
//! - Lists are read as the W3C rules read `LIST[...]` patterns, in Datalog
//!   form: a list is any path of `rdf:rest` from its head to `rdf:nil`. A
//!   cyclic list that never reaches `rdf:nil` matches nothing, shared tails
//!   are read once per head, and an empty list (`rdf:nil` itself) has no
//!   members.
//! - Container membership: every predicate `rdf:_N` (ASCII digits, N > 0 as
//!   a `u64`) in the closure gets `a rdf:Property`,
//!   `a rdfs:ContainerMembershipProperty` and `rdfs:subPropertyOf
//!   rdfs:member`. `reasonable` looks only at loaded triples; here a
//!   derived `rdf:_N` predicate counts too.

use crate::datatype;
use crate::vocab::*;
use oxrdf::{NamedNode, NamedOrBlankNode, Term, Triple};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use turtle_reasoner::Profile;

/// Diagnostics as `(code, message)` pairs, deduplicated as `reasonable` does.
pub type Diagnostics = BTreeSet<(String, String)>;

/// The deriving rules, in the order a round runs them. A triple two rules
/// derive in the same round is counted for the first.
pub const RULES: &[&str] = &[
    "cls-thing",
    "cls-nothing1",
    "thing-typing",
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
    "rdfs-container",
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

/// The diagnostic rules, run once over the closure at the fixpoint.
pub const DIAGNOSTIC_RULES: &[&str] = &[
    "cax-dw",
    "prp-pdw",
    "cls-nothing2",
    "prp-asyp",
    "prp-irp",
    "cls-com",
    "rdfs-datatype",
    "rdfs-datatype-range",
];

/// The rules a profile runs: both profiles run every rule in [`RULES`] but
/// `thing-typing`, which only `reasonable` runs.
pub fn rules(profile: Profile) -> impl Iterator<Item = &'static str> {
    RULES
        .iter()
        .copied()
        .filter(move |rule| *rule != "thing-typing" || profile == Profile::Reasonable)
}

/// One evaluation: the closure, the diagnostics, and what each rule added.
#[derive(Debug, Clone, Default)]
pub struct Evaluation {
    pub closure: HashSet<Triple>,
    pub diagnostics: Diagnostics,
    /// For each rule that added something: the triples it added (deriving
    /// rules) or the diagnostics it gave (diagnostic rules).
    pub fired: BTreeMap<&'static str, usize>,
    /// Rounds until nothing new was derived (the last round included).
    pub rounds: usize,
    /// Whether some rule matched with a literal in the subject position of
    /// what it would derive, and so derived nothing there (D6). `reasonable`
    /// derives such triples and drops them only from its output, so on
    /// these inputs its closure and diagnostics may hold their consequences.
    pub literal_subject: bool,
}

/// The closure and the diagnostics of `triples` under `profile`.
pub fn evaluate(triples: &HashSet<Triple>, profile: Profile) -> (HashSet<Triple>, Diagnostics) {
    let evaluation = evaluate_counted(triples, profile);
    (evaluation.closure, evaluation.diagnostics)
}

/// [`evaluate`], also counting what each rule added.
pub fn evaluate_counted(triples: &HashSet<Triple>, profile: Profile) -> Evaluation {
    evaluate_within(triples, profile, usize::MAX).expect("no limit")
}

/// [`evaluate_counted`], giving up (`None`) once the closure would hold more
/// than `max_closure` triples. Equality makes some small inputs explode
/// (a universal `owl:sameAs` clique copies every triple for every pair of
/// aliases), and this evaluator has no representatives to keep that small.
pub fn evaluate_within(triples: &HashSet<Triple>, profile: Profile, max_closure: usize) -> Option<Evaluation> {
    let mut closure = triples.clone();
    let mut fired = BTreeMap::new();
    let mut rounds = 0;
    let mut literal_subject = false;
    loop {
        rounds += 1;
        let view = View::new(&closure);
        let room = max_closure.saturating_sub(closure.len());
        let mut round = Round {
            closure: &closure,
            fresh: HashMap::new(),
            room,
            overflow: false,
            literal_subject: false,
        };
        derive_all(&view, profile, &mut round);
        if round.overflow {
            return None;
        }
        literal_subject |= round.literal_subject;
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
    for (rule, code, message) in diagnose(&View::new(&closure)) {
        if diagnostics.insert((code.to_owned(), message)) {
            *fired.entry(rule).or_insert(0) += 1;
        }
    }
    Some(Evaluation {
        closure,
        diagnostics,
        fired,
        rounds,
        literal_subject,
    })
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
    subjects: Vec<Term>,
    /// List cells from which a path of `rdf:rest` reaches `rdf:nil`.
    reaches_nil: HashSet<Term>,
}

const NONE: &[Term] = &[];
const NO_PAIRS: &[(Term, Term)] = &[];

impl View {
    fn new(closure: &HashSet<Triple>) -> View {
        let mut view = View {
            by_p: HashMap::new(),
            by_s: HashMap::new(),
            by_o: HashMap::new(),
            sp: HashMap::new(),
            po: HashMap::new(),
            all: HashSet::new(),
            subjects: Vec::new(),
            reaches_nil: HashSet::new(),
        };
        let mut subjects = HashSet::new();
        for triple in closure {
            let s = Term::from(triple.subject.clone());
            let p = Term::from(triple.predicate.clone());
            let o = triple.object.clone();
            view.by_p.entry(p.clone()).or_default().push((s.clone(), o.clone()));
            view.by_s.entry(s.clone()).or_default().push((p.clone(), o.clone()));
            view.by_o.entry(o.clone()).or_default().push((s.clone(), p.clone()));
            view.sp.entry((s.clone(), p.clone())).or_default().push(o.clone());
            view.po.entry((p.clone(), o.clone())).or_default().push(s.clone());
            if subjects.insert(s.clone()) {
                view.subjects.push(s.clone());
            }
            view.all.insert((s, p, o));
        }
        // A cell reaches nil when its rest is nil or a cell that reaches nil.
        let nil = iri(RDF_NIL);
        let rest = iri(RDF_REST);
        let mut reaches_nil = HashSet::new();
        let mut changed = true;
        while changed {
            changed = false;
            for (cell, tail) in view.pairs(&rest) {
                if !reaches_nil.contains(cell) && (*tail == nil || reaches_nil.contains(tail)) {
                    reaches_nil.insert(cell.clone());
                    changed = true;
                }
            }
        }
        view.reaches_nil = reaches_nil;
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

    /// The members of the list at `head`: the `rdf:first` of every cell on a
    /// path from `head` to `rdf:nil`.
    fn members(&self, head: &Term) -> Vec<Term> {
        let first = iri(RDF_FIRST);
        let mut members = Vec::new();
        for cell in self.cells(head) {
            if self.reaches_nil.contains(&cell) {
                members.extend(self.objects(&cell, &first).iter().cloned());
            }
        }
        members
    }

    /// Whether some path from `head` to `rdf:nil` has, at every cell, a
    /// member that satisfies `holds` (cls-int1's "every member").
    fn every_member(&self, head: &Term, holds: impl Fn(&Term) -> bool) -> bool {
        let (first, rest, nil) = (iri(RDF_FIRST), iri(RDF_REST), iri(RDF_NIL));
        let cells = self.cells(head);
        let mut good: HashSet<Term> = HashSet::new();
        let mut changed = true;
        while changed {
            changed = false;
            for cell in &cells {
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
    fn chain(&self, head: &Term, cap: usize) -> Option<Vec<(Term, Term)>> {
        let (first, rest, nil) = (iri(RDF_FIRST), iri(RDF_REST), iri(RDF_NIL));
        let cells = self.cells(head);
        // For each cell: u -> every v the chain from that cell to nil joins.
        let mut joined: HashMap<Term, HashMap<Term, HashSet<Term>>> = HashMap::new();
        let mut total = 0usize;
        let mut changed = true;
        while changed {
            changed = false;
            for cell in &cells {
                let mut found = Vec::new();
                for p in self.objects(cell, &first) {
                    for tail in self.objects(cell, &rest) {
                        if *tail == nil {
                            found.extend(self.pairs(p).iter().cloned());
                        } else if let Some(after) = joined.get(tail) {
                            for (u, x) in self.pairs(p) {
                                for v in after.get(x).into_iter().flatten() {
                                    found.push((u.clone(), v.clone()));
                                }
                            }
                        }
                    }
                }
                let map = joined.entry(cell.clone()).or_default();
                for (u, v) in found {
                    if map.entry(u).or_default().insert(v) {
                        changed = true;
                        total += 1;
                        if total > cap {
                            return None;
                        }
                    }
                }
            }
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
    closure: &'a HashSet<Triple>,
    fresh: HashMap<Triple, &'static str>,
    /// How many fresh triples fit under the limit.
    room: usize,
    overflow: bool,
    literal_subject: bool,
}

impl Round<'_> {
    /// Adds `s p o` unless it is already known, or is not an RDF triple (a
    /// literal subject, a predicate that is not an IRI).
    fn derive(&mut self, rule: &'static str, s: &Term, p: &Term, o: &Term) {
        let Ok(subject) = NamedOrBlankNode::try_from(s.clone()) else {
            self.literal_subject = true;
            return;
        };
        let Ok(predicate) = NamedNode::try_from(p.clone()) else {
            return;
        };
        let triple = Triple::new(subject, predicate, o.clone());
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

fn derive_all(v: &View, profile: Profile, r: &mut Round) {
    let ty = iri(RDF_TYPE);
    let thing = iri(OWL_THING);

    // cls-thing, cls-nothing1: rules with an empty body.
    r.derive("cls-thing", &thing, &ty, &iri(OWL_CLASS));
    r.derive("cls-nothing1", &iri(OWL_NOTHING), &ty, &iri(OWL_CLASS));

    // `reasonable`'s convention: every subject is an owl:Thing.
    if profile == Profile::Reasonable {
        for s in &v.subjects {
            r.derive("thing-typing", s, &ty, &thing);
        }
    }

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
    for p in v.by_p.keys() {
        if let Term::NamedNode(node) = p
            && is_container_membership(node.as_str())
        {
            r.derive("rdfs-container", p, &ty, &iri(RDF_PROPERTY));
            r.derive("rdfs-container", p, &ty, &iri(RDFS_CONTAINER_MEMBERSHIP_PROPERTY));
            r.derive("rdfs-container", p, &iri(RDFS_SUB_PROPERTY_OF), &iri(RDFS_MEMBER));
        }
    }
}

/// `rdf:_N` with N one or more ASCII digits whose value, as a `u64`, is
/// above 0 (`reasonable`'s test, so `rdf:_01` counts and `rdf:_0` does not).
pub(crate) fn is_container_membership(iri: &str) -> bool {
    let Some(suffix) = iri.strip_prefix(RDF).and_then(|rest| rest.strip_prefix('_')) else {
        return false;
    };
    !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) && suffix.parse::<u64>().is_ok_and(|n| n > 0)
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
        for y in candidates {
            if v.every_member(head, |k| v.has(y, &ty, k)) {
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

/// The diagnostic rules over the closure: (rule, code, message), with
/// `reasonable`'s messages (terms written as oxrdf writes them).
fn diagnose(v: &View) -> Vec<(&'static str, &'static str, String)> {
    let ty = iri(RDF_TYPE);
    let mut found = Vec::new();

    for (c1, c2) in v.pairs(&iri(OWL_DISJOINT_WITH)) {
        for x in v.subjects(&ty, c1) {
            if v.has(x, &ty, c2) {
                let message = format!("inst {x} is both {c1} and {c2} (disjoint classes)");
                found.push(("cax-dw", "OWLRL.CAX_DW", message));
            }
        }
    }
    for (p1, p2) in v.pairs(&iri(OWL_PROPERTY_DISJOINT_WITH)) {
        for (x, y) in v.pairs(p1) {
            if v.has(x, p2, y) {
                let message = format!("property {p1} is disjoint with {p2} for pair ({x} {p1} {y})");
                found.push(("prp-pdw", "OWLRL.PRP_PDW", message));
            }
        }
    }
    for x in instances(v, OWL_NOTHING) {
        let message = format!("instance {x} is owl:Nothing (unsatisfiable)");
        found.push(("cls-nothing2", "OWLRL.CLS_NOTHING", message));
    }
    for p in instances(v, OWL_ASYMMETRIC_PROPERTY) {
        for (x, y) in v.pairs(p) {
            if v.has(y, p, x) {
                let message = format!("property {p} of {x} and {y} is asymmetric");
                found.push(("prp-asyp", "OWLRL.PRP_ASYP", message));
            }
        }
    }
    for p in instances(v, OWL_IRREFLEXIVE_PROPERTY) {
        for (x, y) in v.pairs(p) {
            if x == y {
                let message = format!("property {p} of {x} is irreflexive");
                found.push(("prp-irp", "OWLRL.PRP_IRP", message));
            }
        }
    }
    // D2: `reasonable` never emits this. The message is the one its
    // commented-out call would write: the complementOf object first.
    for (c1, c2) in v.pairs(&iri(OWL_COMPLEMENT_OF)) {
        for x in v.subjects(&ty, c1) {
            if v.has(x, &ty, c2) {
                let message = format!("Individual {x} has type {c2} and {c1} which are complements");
                found.push(("cls-com", "OWLRL.CLS_COM", message));
            }
        }
    }
    for (p, pairs) in &v.by_p {
        for (s, o) in pairs {
            if let Term::Literal(literal) = o
                && datatype::well_formed(literal) == Some(false)
            {
                let message = format!("ill-formed literal {literal} used in triple {s} {p} {o}");
                found.push(("rdfs-datatype", "RDFS.DATATYPE", message));
            }
        }
    }
    for (p, range) in v.pairs(&iri(RDFS_RANGE)) {
        let Term::NamedNode(range) = range else { continue };
        for (_, o) in v.pairs(p) {
            if datatype::in_value_space(o, range.as_str()) == Some(false) {
                let message = format!("range clash: object {o} is not in datatype {range} for predicate {p}");
                found.push(("rdfs-datatype-range", "RDFS.DATATYPE_RANGE", message));
            }
        }
    }
    found
}
