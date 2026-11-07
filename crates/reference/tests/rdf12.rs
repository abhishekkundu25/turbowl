//! A proof that passes through an RDF 1.2 triple term (the `rdf-12`
//! feature): the core's explainer shows the triple term as the term it is,
//! and the reference checker accepts the proof.
#![cfg(feature = "rdf-12")]

use oxrdf::{BlankNode, NamedNode, NamedOrBlankNode, Term, Triple};
use std::collections::HashSet;
use turbowl::{Reasoner, Switches};
use turbowl_reference::check_proof;

const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";

fn ex(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://example.org/{local}"))
}

fn iri(ns: &str, local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{ns}{local}"))
}

fn t(s: impl Into<NamedOrBlankNode>, p: NamedNode, o: impl Into<Term>) -> Triple {
    Triple::new(s, p, o)
}

#[test]
fn a_proof_through_a_triple_term_passes_the_reference_checker() {
    let term: Term = t(BlankNode::new_unchecked("b"), ex("p"), ex("o")).into();
    let base: HashSet<Triple> = [
        t(ex("a"), ex("says"), ex("claim")),
        t(ex("claim"), iri(OWL, "sameAs"), term.clone()),
        t(ex("says"), iri(RDFS, "subPropertyOf"), ex("said")),
    ]
    .into_iter()
    .collect();
    let mut r = Reasoner::with_switches(Switches::complete());
    r.insert(base.iter().map(Triple::as_ref));
    r.reason();
    let goal = t(ex("a"), ex("said"), term.clone());
    assert!(r.inferred().any(|i| i == goal), "prp-spo1 and eq-rep-o");
    let proof = r.explain(goal.as_ref()).unwrap().expect("a proof");
    assert!(
        proof.steps().iter().any(|s| s.statement.object == term),
        "the proof shows the triple term as it is:\n{proof}"
    );
    assert_eq!(check_proof(&proof, &base, r.switches()), Ok(()));
}
