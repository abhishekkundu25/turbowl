//! The oracle faults (`CONTEXT.md`): places where an oracle is wrong and the
//! W3C OWL 2 RL rules are not. Each has an id, a test below showing that the
//! other oracles agree with the W3C rule, and a filter that removes exactly
//! the affected triples or findings before a closure is compared with the
//! faulty oracle's. There are no other filters (ADR 0004).
//!
//! All seven are faults of OWL-RL (owlrl 7.6.2 on rdflib 7.6.0). Nemo,
//! HermiT and the W3C tests have none so far.

use crate::{Closure, Finding, Input};
use oxrdf::vocab::{rdf, rdfs};
use oxrdf::{NamedOrBlankNode, Term};
use std::collections::{HashMap, HashSet};

/// A known fault of an oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fault {
    /// OWL-RL's scm-sco and scm-spo never derive `c rdfs:subClassOf c` or
    /// `p rdfs:subPropertyOf p`, so neither scm-eqc2 nor scm-eqp2 gives
    /// `c owl:equivalentClass c` or `p owl:equivalentProperty p` from them
    /// (nor, by eq-rep-p, `c q c` for a `q` made `owl:sameAs` one of the
    /// four).
    F1,
    /// OWL-RL's cls-com names the second class where the individual belongs.
    F2,
    /// OWL-RL's dt-not-type accepts a lexical form with leading or trailing whitespace.
    F3,
    /// OWL-RL fails on a cyclic list.
    F4,
    /// OWL-RL's dt-not-type checks a literal only against its own datatype.
    F5,
    /// OWL-RL's eq-diff2, eq-diff3 and prp-adp miss positions of a list.
    F6,
    /// OWL-RL's dt-type1 leaves out `owl:real` and `owl:rational`.
    F7,
}

impl Fault {
    pub const ALL: [Self; 7] = [Self::F1, Self::F2, Self::F3, Self::F4, Self::F5, Self::F6, Self::F7];

    /// The oracle at fault, as the triangulation table names it.
    pub fn oracle(self) -> &'static str {
        "OWL-RL"
    }

    /// What the oracle does wrong, and what the W3C rule says instead.
    pub fn description(self) -> &'static str {
        match self {
            Self::F1 => {
                "owlrl 7.6.2 skips c rdfs:subClassOf c in scm-sco (OWLRL.py:883-888, `if c1 != c2` and \
                 `if c1 != c3`) and p rdfs:subPropertyOf p in scm-spo (OWLRL.py:908-916, `s != o`, \
                 `if p1 != p2` and `if p1 != p3`), and derives them only through scm-cls and scm-op, which \
                 need `c a owl:Class` or a property type; so neither scm-eqc2 nor scm-eqp2 (OWLRL.py:889, \
                 919) gives `c owl:equivalentClass c` or `p owl:equivalentProperty p` from them. The W3C \
                 rules have no such condition: `A subClassOf B . B subClassOf A` gives A subClassOf A and \
                 A equivalentClass A, `p equivalentProperty q` gives p subPropertyOf p and p \
                 equivalentProperty p, and eq-rep-p copies each to every property made owl:sameAs the \
                 predicate"
            }
            Self::F2 => {
                "owlrl 7.6.2 formats cls-com's message with `x`, which is the second class there \
                 (OWLRL.py:684-691), so its finding is (c1, c2, c2), not (c1, c2, individual)"
            }
            Self::F3 => {
                "owlrl 7.6.2 accepts \" 42\"^^xsd:int (Python's int() strips the spaces); RDF does not \
                 collapse whitespace, and the W3C test rdf-mt xmlsch-02-whitespace-facet-2 expects \
                 \" 3 \"^^xsd:int to be ill-typed"
            }
            Self::F4 => {
                "rdflib's Collection raises \"List contains a recursive rdf:rest reference\" on a cyclic \
                 list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that \
                 never reaches rdf:nil"
            }
            Self::F5 => {
                "owlrl 7.6.2 runs dt-not-type only on literals against their own datatype \
                 (OWLRL.py:217-232), never on `lt rdf:type dt` triples, so a literal typed by prp-rng is \
                 not checked; the W3C dt-not-type matches any such triple, and the W3C test rdf-mt \
                 datatypes-range-clash expects \"25\"^^xsd:integer in an xsd:string range to be \
                 inconsistent"
            }
            Self::F6 => {
                "owlrl 7.6.2 reads the list of eq-diff2, eq-diff3 (OWLRL.py:416-436) and prp-adp \
                 (OWLRL.py:517-531) through rdflib's `items`, one rdf:first per cell, pairs positions \
                 with `for j in range(i + 1, len(list) - 1)`, which never reaches the last member, and \
                 for eq-diff2 and eq-diff3 skips zi = zj; the W3C rules pair every two positions \
                 i < j of the list, with every rdf:first of each cell, so `(a b c)` with \
                 `a owl:sameAs b` gives (a, a), (a, b), (b, a) and (b, b), and `(p q)` gives prp-adp"
            }
            Self::F7 => {
                "owlrl 7.6.2 runs dt-type1 over its list OWL_RL_Datatypes (XsdDatatypes.py:78, \
                 OWLRL.py:268), which has no owl:real and no owl:rational; the OWL 2 RL datatypes \
                 (OWL 2 Profiles §4.2) include both, so dt-type1 types them rdfs:Datatype, and eq-ref \
                 makes each owl:sameAs itself"
            }
        }
    }

    /// What the comparison with the faulty oracle does about it.
    pub fn handling(self) -> &'static str {
        match self {
            Self::F1 => {
                "drops `c p c` triples OWL-RL does not have from the other closure, for p rdfs:subClassOf, \
                 owl:equivalentClass, rdfs:subPropertyOf, owl:equivalentProperty and their aliases"
            }
            Self::F2 => "compares cls-com findings on the two classes only, on both sides",
            Self::F3 => {
                "drops dt-not-type findings OWL-RL does not have whose literal has leading or trailing \
                 whitespace from the other closure"
            }
            Self::F4 => "OWL-RL's failure on an input with a cyclic list is known; nothing is compared with it",
            Self::F5 => {
                "drops dt-not-type findings OWL-RL does not have whose datatype is not the literal's own \
                 from the other closure"
            }
            Self::F6 => "drops eq-diff2, eq-diff3 and prp-adp findings OWL-RL does not have from the other closure",
            Self::F7 => {
                "when OWL-RL does not type owl:real (or owl:rational) rdfs:Datatype, drops the triples \
                 OWL-RL does not have that name it from the other closure"
            }
        }
    }

    /// True when the fault shows between a closure (`ours`, the one judged)
    /// and the faulty oracle's (`theirs`) on this input.
    pub fn shows(self, input: &Input, ours: &Closure, theirs: &Closure) -> bool {
        match self {
            Self::F1 => {
                let aliases = schema_aliases(ours);
                ours.triples
                    .iter()
                    .any(|t| reflexive(t, &aliases) && !theirs.triples.contains(t))
            }
            Self::F2 => [ours, theirs]
                .iter()
                .any(|c| c.findings.iter().any(|f| f.code == "cls-com")),
            Self::F3 => ours
                .findings
                .iter()
                .any(|f| padded_literal(f) && !theirs.findings.contains(f)),
            Self::F4 => has_cyclic_list(input),
            Self::F5 => ours
                .findings
                .iter()
                .any(|f| typed_elsewhere(f) && !theirs.findings.contains(f)),
            Self::F6 => ours
                .findings
                .iter()
                .any(|f| by_list_position(f) && !theirs.findings.contains(f)),
            Self::F7 => {
                let untyped = untyped_datatypes(theirs);
                ours.triples
                    .iter()
                    .any(|t| names_any(t, &untyped) && !theirs.triples.contains(t))
            }
        }
    }

    /// Removes from both closures what this fault makes differ.
    fn filter(self, ours: &mut Closure, theirs: &mut Closure) {
        match self {
            Self::F1 => {
                let aliases = schema_aliases(ours);
                ours.triples
                    .retain(|t| !reflexive(t, &aliases) || theirs.triples.contains(t))
            }
            Self::F2 => {
                for c in [ours, theirs] {
                    c.findings = c
                        .findings
                        .drain()
                        .map(|f| {
                            if f.code == "cls-com" {
                                Finding::new(f.code, f.terms.into_iter().take(2).collect())
                            } else {
                                f
                            }
                        })
                        .collect();
                }
            }
            Self::F3 => ours
                .findings
                .retain(|f| !padded_literal(f) || theirs.findings.contains(f)),
            Self::F4 => {}
            Self::F5 => ours
                .findings
                .retain(|f| !typed_elsewhere(f) || theirs.findings.contains(f)),
            Self::F6 => ours
                .findings
                .retain(|f| !by_list_position(f) || theirs.findings.contains(f)),
            Self::F7 => {
                let untyped = untyped_datatypes(theirs);
                ours.triples
                    .retain(|t| !names_any(t, &untyped) || theirs.triples.contains(t))
            }
        }
    }
}

/// Applies every fault of `oracle` that shows on this input to the two
/// closures (`ours` is compared with `theirs`, the oracle's), and returns them.
pub fn apply(oracle: &str, input: &Input, ours: &mut Closure, theirs: &mut Closure) -> Vec<Fault> {
    let shown: Vec<Fault> = Fault::ALL
        .into_iter()
        .filter(|f| f.oracle() == oracle && f.shows(input, ours, theirs))
        .collect();
    for f in &shown {
        f.filter(ours, theirs);
    }
    shown
}

/// The fault that explains why `oracle` failed on this input, if one does.
pub fn explains_failure(oracle: &str, input: &Input) -> Option<Fault> {
    (oracle == Fault::F4.oracle() && has_cyclic_list(input)).then_some(Fault::F4)
}

/// The predicates of F1 (`rdfs:subClassOf`, `owl:equivalentClass`,
/// `rdfs:subPropertyOf`, `owl:equivalentProperty`), and every IRI the
/// closure makes `owl:sameAs` one of them.
fn schema_aliases(closure: &Closure) -> HashSet<oxrdf::NamedNode> {
    let predicates: [Term; 4] = [
        rdfs::SUB_CLASS_OF.into_owned().into(),
        oxrdf::NamedNode::new_unchecked("http://www.w3.org/2002/07/owl#equivalentClass").into(),
        rdfs::SUB_PROPERTY_OF.into_owned().into(),
        oxrdf::NamedNode::new_unchecked("http://www.w3.org/2002/07/owl#equivalentProperty").into(),
    ];
    let mut aliases: HashSet<oxrdf::NamedNode> = closure
        .triples
        .iter()
        .filter(|t| t.predicate.as_str() == "http://www.w3.org/2002/07/owl#sameAs" && predicates.contains(&t.object))
        .filter_map(|t| match &t.subject {
            NamedOrBlankNode::NamedNode(n) => Some(n.clone()),
            NamedOrBlankNode::BlankNode(_) => None,
        })
        .collect();
    aliases.extend(predicates.into_iter().filter_map(|p| match p {
        Term::NamedNode(n) => Some(n),
        _ => None,
    }));
    aliases
}

/// `c p c` with `p` one of `aliases` ([`schema_aliases`]).
fn reflexive(t: &oxrdf::Triple, aliases: &HashSet<oxrdf::NamedNode>) -> bool {
    aliases.contains(&t.predicate) && Term::from(t.subject.clone()) == t.object
}

/// A dt-not-type finding about a literal with leading or trailing whitespace.
fn padded_literal(f: &Finding) -> bool {
    f.code == "dt-not-type"
        && matches!(f.terms.first(), Some(Term::Literal(l))
            if l.value() != l.value().trim_matches([' ', '\t', '\n', '\r']))
}

/// A dt-not-type finding whose datatype is not the literal's own.
fn typed_elsewhere(f: &Finding) -> bool {
    match (f.code.as_str(), f.terms.as_slice()) {
        ("dt-not-type", [Term::Literal(l), Term::NamedNode(dt)]) => l.datatype() != dt.as_ref(),
        _ => false,
    }
}

/// A finding of a rule that pairs list positions as F6 says OWL-RL does not.
fn by_list_position(f: &Finding) -> bool {
    matches!(f.code.as_str(), "eq-diff2" | "eq-diff3" | "prp-adp")
}

/// The datatypes F7 says OWL-RL's dt-type1 leaves out, those its closure
/// does not type `rdfs:Datatype`.
fn untyped_datatypes(theirs: &Closure) -> Vec<Term> {
    ["real", "rational"]
        .map(|local| {
            Term::from(oxrdf::NamedNode::new_unchecked(format!(
                "http://www.w3.org/2002/07/owl#{local}"
            )))
        })
        .into_iter()
        .filter(|dt| {
            !theirs.triples.iter().any(|t| {
                Term::from(t.subject.clone()) == *dt
                    && t.predicate == rdf::TYPE
                    && t.object == Term::from(rdfs::DATATYPE.into_owned())
            })
        })
        .collect()
}

/// True when `t` names one of `terms`, in any position.
fn names_any(t: &oxrdf::Triple, terms: &[Term]) -> bool {
    [
        Term::from(t.subject.clone()),
        Term::from(t.predicate.clone()),
        t.object.clone(),
    ]
    .iter()
    .any(|x| terms.contains(x))
}

/// True when following `rdf:rest` from some list cell comes back to it.
fn has_cyclic_list(input: &Input) -> bool {
    let rest: HashMap<NamedOrBlankNode, &Term> = input
        .triples()
        .iter()
        .filter(|t| t.predicate == rdf::REST)
        .map(|t| (t.subject.clone(), &t.object))
        .collect();
    rest.keys().any(|start| {
        let mut seen = HashSet::new();
        let mut cell = start.clone();
        while seen.insert(cell.clone()) {
            let next = match rest.get(&cell) {
                Some(Term::NamedNode(n)) => NamedOrBlankNode::from(n.clone()),
                Some(Term::BlankNode(b)) => NamedOrBlankNode::from(b.clone()),
                _ => return false,
            };
            if next == *start {
                return true;
            }
            cell = next;
        }
        false
    })
}

/// Each fault's test: the faulty oracle disagrees with the W3C rule, and the
/// other oracles (Nemo, HermiT, the W3C tests) agree with it. An oracle that
/// is not installed is skipped (with `CI=true`, the test fails).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::closure::triple;
    use crate::compare::{Relation, compare};
    use crate::w3c::{self, Verdict};
    use crate::{Outcome, hermit, nemo, owlrl};
    use oxrdf::vocab::xsd;
    use oxrdf::{Literal, NamedNode};

    const PREFIXES: &str = "@prefix ex: <http://example.org/> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n";

    fn input(name: &str, turtle: &str) -> Input {
        Input::turtle(name, &format!("{PREFIXES}{turtle}"))
    }

    fn ex(local: &str) -> Term {
        NamedNode::new_unchecked(format!("http://example.org/{local}")).into()
    }

    /// Nemo's closure and OWL-RL's outcome, or `None` when one is not installed.
    fn both(input: &Input) -> Option<(Closure, Outcome)> {
        let nemo = match nemo::run(input) {
            Outcome::Ran(c) => c,
            Outcome::Skipped(why) => {
                println!("skipped: {why}");
                return None;
            }
            other => panic!("Nemo: {other:?}"),
        };
        let owlrl = owlrl::run(input);
        if let Outcome::Skipped(why) = &owlrl {
            println!("skipped: {why}");
            return None;
        }
        Some((nemo, owlrl))
    }

    fn ran(o: Outcome) -> Closure {
        match o {
            Outcome::Ran(c) => c,
            other => panic!("OWL-RL: {other:?}"),
        }
    }

    /// Nemo ⊆ OWL-RL fails without the fault's filter and holds with it.
    fn filter_is_needed_and_enough(fault: Fault, input: &Input, nemo: &Closure, owlrl: &Closure) {
        let raw = compare(("Nemo", nemo), ("OWL-RL", owlrl), Relation::Subset);
        assert!(!raw.holds(), "{fault:?} no longer shows:\n{raw}");
        let (mut ours, mut theirs) = (nemo.clone(), owlrl.clone());
        let applied = apply("OWL-RL", input, &mut ours, &mut theirs);
        assert!(applied.contains(&fault), "{fault:?} not detected: {applied:?}");
        let filtered = compare(("Nemo", &ours), ("OWL-RL", &theirs), Relation::Subset);
        assert!(filtered.holds(), "{fault:?}: the filter is not enough:\n{filtered}");
    }

    /// The verdict on the RDF-MT case `id` for a closure of its premise;
    /// `None` when the suite is not fetched.
    fn rdf_mt_verdict(id: &str, closure: impl Fn(&w3c::Case) -> Closure) -> Option<Verdict> {
        let cases = match w3c::rdf_mt() {
            Ok(c) => c,
            Err(o) => {
                println!("skipped: {o:?}");
                return None;
            }
        };
        let case = cases
            .cases
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("no W3C case {id}"));
        Some(w3c::judge(case, &closure(case)))
    }

    #[test]
    fn f1_owlrl_scm_sco_skips_reflexive_subclass() {
        let input = input("f1", "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .");
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        let a_sub_a =
            triple("<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/A>");
        assert!(
            nemo.triples.contains(&a_sub_a),
            "the W3C scm-sco (Nemo) derives A subClassOf A"
        );
        assert!(
            !owlrl.triples.contains(&a_sub_a),
            "OWL-RL now derives A subClassOf A: F1 is gone"
        );
        match hermit::check(&input, &nemo) {
            Ok(s) => assert!(s.not_entailed.is_empty(), "HermiT: {:?}", s.not_entailed),
            Err(o) => println!("HermiT not run: {o:?}"),
        }
        filter_is_needed_and_enough(Fault::F1, &input, &nemo, &owlrl);
    }

    /// F1 through equality: with `rdfs:subClassOf owl:sameAs
    /// owl:equivalentClass`, eq-rep-p copies `A subClassOf A` to
    /// `A equivalentClass A`, which OWL-RL misses with the first.
    #[test]
    fn f1_reaches_the_aliases_of_sub_class_of() {
        let input = input(
            "f1-alias",
            "rdfs:subClassOf owl:sameAs owl:equivalentClass . ex:A rdfs:subClassOf ex:B .",
        );
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        let a_eqc_a =
            triple("<http://example.org/A> <http://www.w3.org/2002/07/owl#equivalentClass> <http://example.org/A>");
        assert!(
            nemo.triples.contains(&a_eqc_a),
            "the W3C rules (Nemo) derive A equivalentClass A"
        );
        assert!(
            !owlrl.triples.contains(&a_eqc_a),
            "OWL-RL now derives it: F1 is gone here"
        );
        filter_is_needed_and_enough(Fault::F1, &input, &nemo, &owlrl);
    }

    /// F1 on properties: scm-spo closes the cycle scm-eqp1 gives
    /// reflexively, and scm-eqp2 makes `p owl:equivalentProperty p`; OWL-RL
    /// has neither.
    #[test]
    fn f1_owlrl_scm_spo_skips_reflexive_subproperty() {
        let input = input("f1-property", "ex:p owl:equivalentProperty ex:q .");
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        for t in [
            "<http://example.org/p> <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> <http://example.org/p>",
            "<http://example.org/p> <http://www.w3.org/2002/07/owl#equivalentProperty> <http://example.org/p>",
        ] {
            assert!(nemo.triples.contains(&triple(t)), "the W3C rules (Nemo) derive {t}");
            assert!(
                !owlrl.triples.contains(&triple(t)),
                "OWL-RL now derives {t}: F1 is gone here"
            );
        }
        filter_is_needed_and_enough(Fault::F1, &input, &nemo, &owlrl);
    }

    #[test]
    fn f2_owlrl_cls_com_names_the_class() {
        let input = input("f2", "ex:C1 owl:complementOf ex:C2 . ex:x a ex:C1, ex:C2 .");
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        let right = Finding::new("cls-com", vec![ex("C1"), ex("C2"), ex("x")]);
        let wrong = Finding::new("cls-com", vec![ex("C1"), ex("C2"), ex("C2")]);
        assert!(
            nemo.findings.contains(&right),
            "the W3C cls-com (Nemo) names the individual"
        );
        assert!(
            owlrl.findings.contains(&wrong),
            "OWL-RL now names the individual: F2 is gone"
        );
        filter_is_needed_and_enough(Fault::F2, &input, &nemo, &owlrl);
    }

    #[test]
    fn f3_owlrl_accepts_padded_integers() {
        let input = input("f3", "ex:s ex:p \" 42\"^^xsd:int .");
        let owlrl = match owlrl::run(&input) {
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            o => ran(o),
        };
        assert!(
            owlrl.findings.is_empty(),
            "OWL-RL now rejects \" 42\"^^xsd:int: F3 is gone"
        );
        // The W3C test says " 3 "^^xsd:int is ill-typed; OWL-RL disagrees.
        if let Some(v) = rdf_mt_verdict("xmlsch-02-whitespace-facet-2", |c| ran(owlrl::run(&c.premise))) {
            assert!(
                matches!(v, Verdict::Fail(_)),
                "OWL-RL now passes the W3C test: F3 is gone"
            );
        }
        let lit = Literal::new_typed_literal(" 42", xsd::INT);
        let mut ours = Closure::new([], [Finding::new("dt-not-type", vec![lit.into(), xsd::INT.into()])]);
        let mut theirs = owlrl;
        assert_eq!(apply("OWL-RL", &input, &mut ours, &mut theirs), [Fault::F3]);
        assert!(ours.findings.is_empty());
    }

    #[test]
    fn f4_owlrl_fails_on_a_cyclic_list() {
        let input = input(
            "f4",
            "ex:p owl:propertyChainAxiom ex:l1 . ex:l1 rdf:first ex:q ; rdf:rest ex:l2 . ex:l2 rdf:first ex:r ; rdf:rest ex:l1 .
             ex:a ex:q ex:b . ex:b ex:r ex:c .",
        );
        let Some((nemo, owlrl)) = both(&input) else { return };
        assert!(
            matches!(owlrl, Outcome::Failed(ref why) if why.contains("recursive")),
            "OWL-RL now runs on a cyclic list: F4 is gone ({owlrl:?})"
        );
        assert!(
            !nemo.has("<http://example.org/a> <http://example.org/p> <http://example.org/c>"),
            "the W3C prp-spo2 (Nemo) derives nothing from a list without rdf:nil"
        );
        assert_eq!(explains_failure("OWL-RL", &input), Some(Fault::F4));
    }

    #[test]
    fn f5_owlrl_ignores_range_typed_literals() {
        let input = input("f5", "ex:p rdfs:range xsd:string . ex:s ex:p \"25\"^^xsd:integer .");
        let owlrl = match owlrl::run(&input) {
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            o => ran(o),
        };
        assert!(
            owlrl.findings.is_empty(),
            "OWL-RL now checks range-typed literals: F5 is gone"
        );
        // The same input is the W3C test datatypes-range-clash, which expects an inconsistency.
        if let Some(v) = rdf_mt_verdict("datatypes-range-clash", |c| ran(owlrl::run(&c.premise))) {
            assert!(
                matches!(v, Verdict::Fail(_)),
                "OWL-RL now passes the W3C test: F5 is gone"
            );
        }
        let lit = Literal::new_typed_literal("25", xsd::INTEGER);
        let finding = Finding::new("dt-not-type", vec![lit.into(), xsd::STRING.into()]);
        if let Some(v) = rdf_mt_verdict("datatypes-range-clash", |_| Closure::new([], [finding.clone()])) {
            assert_eq!(v, Verdict::Pass, "the W3C test accepts the finding the W3C rule gives");
        }
        let mut ours = Closure::new([], [finding]);
        let mut theirs = owlrl;
        assert_eq!(apply("OWL-RL", &input, &mut ours, &mut theirs), [Fault::F5]);
        assert!(ours.findings.is_empty());
    }

    /// Nemo, the reference evaluator and TurbOWL agree on the W3C findings;
    /// OWL-RL reports fewer, and only those the others report.
    #[test]
    fn f6_owlrl_misses_list_positions() {
        let input = input(
            "f6",
            "ex:d a owl:AllDifferent ; owl:members ( ex:a ex:b ex:c ) ; owl:distinctMembers ( ex:c ex:c ) .
             ex:a owl:sameAs ex:b . ex:c owl:sameAs ex:c .
             ex:x a owl:AllDisjointProperties ; owl:members ( ex:p ex:q ) . ex:u ex:p ex:v ; ex:q ex:v .",
        );
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        let switches = turbowl::Switches::default();
        let ours = match crate::ours::run(&input, switches) {
            Outcome::Ran(c) => c,
            other => panic!("TurbOWL: {other:?}"),
        };
        let base: HashSet<_> = input.triples().iter().cloned().collect();
        let evaluation = turbowl_reference::evaluate_within(&base, switches, 10_000).expect("a small closure");
        let listed = |findings: &mut dyn Iterator<Item = Finding>| -> HashSet<Finding> {
            findings.filter(by_list_position).collect()
        };
        let reference = listed(
            &mut evaluation
                .diagnostics
                .iter()
                .map(|(c, t)| Finding::new(c.clone(), t.clone())),
        );
        let (nemo_found, ours_found) = (
            listed(&mut nemo.findings.iter().cloned()),
            listed(&mut ours.findings.iter().cloned()),
        );
        let [a, b, c, d, p, q, u, v] = ["a", "b", "c", "d", "p", "q", "u", "v"].map(ex);
        let mut expected: HashSet<Finding> = [
            ("eq-diff3", vec![d.clone(), c.clone(), c.clone()]),
            ("prp-adp", vec![p, q, u, v]),
        ]
        .into_iter()
        .map(|(code, terms)| Finding::new(code, terms))
        .collect();
        for (zi, zj) in [(&a, &a), (&a, &b), (&b, &a), (&b, &b)] {
            expected.insert(Finding::new("eq-diff2", vec![d.clone(), zi.clone(), zj.clone()]));
        }
        assert_eq!(nemo_found, expected, "the W3C rules (Nemo)");
        assert_eq!(reference, expected, "the reference evaluator");
        assert_eq!(ours_found, expected, "TurbOWL");
        let theirs = listed(&mut owlrl.findings.iter().cloned());
        assert!(theirs.is_subset(&expected), "OWL-RL reports {theirs:?}");
        assert!(
            theirs.len() < expected.len(),
            "OWL-RL now reports every position: F6 is gone"
        );
        filter_is_needed_and_enough(Fault::F6, &input, &nemo, &owlrl);
    }

    /// In the complete configuration, Nemo (the W3C dt-type1 over the OWL 2
    /// RL datatypes) types owl:real and owl:rational rdfs:Datatype, and
    /// eq-ref makes each owl:sameAs itself; OWL-RL has neither, and
    /// nothing else that names them.
    #[test]
    fn f7_owlrl_dt_type1_leaves_out_real_and_rational() {
        let input = input("f7", "ex:a ex:p ex:b .");
        let nemo = match nemo::run_with(&input, turbowl::Switches::complete()) {
            Outcome::Ran(c) => c,
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            other => panic!("Nemo: {other:?}"),
        };
        let owlrl = match owlrl::run(&input) {
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            o => ran(o),
        };
        let owl = |local: &str| format!("<http://www.w3.org/2002/07/owl#{local}>");
        for dt in ["real", "rational"] {
            let typed = triple(&format!(
                "{} <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2000/01/rdf-schema#Datatype>",
                owl(dt)
            ));
            assert!(nemo.triples.contains(&typed), "the W3C dt-type1 (Nemo) types owl:{dt}");
            assert!(!owlrl.triples.contains(&typed), "OWL-RL now types owl:{dt}: F7 is gone");
        }
        let named = untyped_datatypes(&owlrl);
        assert_eq!(named.len(), 2);
        assert!(!owlrl.triples.iter().any(|t| names_any(t, &named)));
        filter_is_needed_and_enough(Fault::F7, &input, &nemo, &owlrl);
        // Where OWL-RL types them, nothing is filtered.
        let mut theirs = owlrl.clone();
        for dt in ["real", "rational"] {
            theirs.triples.insert(triple(&format!(
                "{} <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2000/01/rdf-schema#Datatype>",
                owl(dt)
            )));
        }
        assert!(untyped_datatypes(&theirs).is_empty());
    }

    #[test]
    fn finds_cyclic_lists() {
        let cyclic = input("c", "ex:l1 rdf:rest ex:l2 . ex:l2 rdf:rest ex:l1 .");
        assert!(has_cyclic_list(&cyclic));
        let shared = input(
            "s",
            "ex:I owl:intersectionOf _:l1 . _:l1 rdf:first ex:A ; rdf:rest _:l2 . _:l2 rdf:first ex:B ; rdf:rest rdf:nil .
             ex:J owl:intersectionOf _:l2 .",
        );
        assert!(!has_cyclic_list(&shared));
        assert_eq!(explains_failure("OWL-RL", &shared), None);
    }
}
