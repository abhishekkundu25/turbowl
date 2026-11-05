//! The oracle faults (`CONTEXT.md`): places where an oracle is wrong and the
//! W3C OWL 2 RL rules are not. Each has an id, a test below showing that the
//! other oracles agree with the W3C rule, and a filter that removes exactly
//! the affected triples or findings before a closure is compared with the
//! faulty oracle's. There are no other filters (ADR 0004).
//!
//! All five are faults of OWL-RL (owlrl 7.6.2 on rdflib 7.6.0). Nemo, HermiT
//! and the W3C tests have none so far.

use crate::{Closure, Finding, Input};
use oxrdf::vocab::{rdf, rdfs};
use oxrdf::{NamedOrBlankNode, Term};
use std::collections::{HashMap, HashSet};

/// A known fault of an oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fault {
    /// OWL-RL's scm-sco never derives `c rdfs:subClassOf c`.
    F1,
    /// OWL-RL's cls-com names the second class where the individual belongs.
    F2,
    /// OWL-RL's dt-not-type accepts a lexical form with leading or trailing whitespace.
    F3,
    /// OWL-RL fails on a cyclic list.
    F4,
    /// OWL-RL's dt-not-type checks a literal only against its own datatype.
    F5,
}

impl Fault {
    pub const ALL: [Self; 5] = [Self::F1, Self::F2, Self::F3, Self::F4, Self::F5];

    /// The oracle at fault, as the triangulation table names it.
    pub fn oracle(self) -> &'static str {
        "OWL-RL"
    }

    /// What the oracle does wrong, and what the W3C rule says instead.
    pub fn description(self) -> &'static str {
        match self {
            Self::F1 => {
                "owlrl 7.6.2 skips c rdfs:subClassOf c in scm-sco (OWLRL.py:883-888, `if c1 != c2` and \
                 `if c1 != c3`) and derives it only through scm-cls, which needs `c a owl:Class`; the W3C \
                 scm-sco has no such condition, so `A subClassOf B . B subClassOf A` gives A subClassOf A"
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
        }
    }

    /// What the comparison with the faulty oracle does about it.
    pub fn handling(self) -> &'static str {
        match self {
            Self::F1 => "drops `c rdfs:subClassOf c` triples OWL-RL does not have from the other closure",
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
        }
    }

    /// True when the fault shows between a closure (`ours`, the one judged)
    /// and the faulty oracle's (`theirs`) on this input.
    pub fn shows(self, input: &Input, ours: &Closure, theirs: &Closure) -> bool {
        match self {
            Self::F1 => ours
                .triples
                .iter()
                .any(|t| reflexive_sub_class(t) && !theirs.triples.contains(t)),
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
        }
    }

    /// Removes from both closures what this fault makes differ.
    fn filter(self, ours: &mut Closure, theirs: &mut Closure) {
        match self {
            Self::F1 => ours
                .triples
                .retain(|t| !reflexive_sub_class(t) || theirs.triples.contains(t)),
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

fn reflexive_sub_class(t: &oxrdf::Triple) -> bool {
    t.predicate == rdfs::SUB_CLASS_OF && Term::from(t.subject.clone()) == t.object
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
