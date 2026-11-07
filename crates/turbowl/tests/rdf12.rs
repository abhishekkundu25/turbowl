//! RDF 1.2 triple terms (the `rdf-12` feature) through the public
//! interface: a triple term is an opaque term. Rules carry it whole, never
//! look inside it, and the triple inside is not asserted.
#![cfg(feature = "rdf-12")]

use oxrdf::{BlankNode, NamedNode, NamedOrBlankNode, Term, Triple};
use std::collections::HashSet;
use turbowl::{Format, Reasoner};

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";

fn ex(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://example.org/{local}"))
}

fn iri(ns: &str, local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{ns}{local}"))
}

/// `<<( s p o )>>`.
fn quoted(s: impl Into<NamedOrBlankNode>, p: NamedNode, o: impl Into<Term>) -> Term {
    Triple::new(s, p, o).into()
}

fn t(s: impl Into<NamedOrBlankNode>, p: NamedNode, o: impl Into<Term>) -> Triple {
    Triple::new(s, p, o)
}

fn reason(base: &[Triple]) -> Reasoner {
    let mut r = Reasoner::new();
    r.set_base(base.iter().map(Triple::as_ref));
    r.reason();
    r
}

fn inferred(r: &Reasoner) -> HashSet<Triple> {
    r.inferred().collect()
}

#[test]
fn a_triple_term_survives_set_base_reason_and_inferred() {
    let term = quoted(ex("s"), ex("p"), BlankNode::new_unchecked("b"));
    let base = [
        t(ex("a"), iri(RDF, "reifies"), term.clone()),
        t(iri(RDF, "reifies"), iri(RDFS, "subPropertyOf"), ex("about")),
    ];
    let r = reason(&base);
    let closure: HashSet<Triple> = r.triples().collect();
    assert!(closure.contains(&base[0]), "the base triple, its triple term unchanged");
    assert!(
        inferred(&r).contains(&t(ex("a"), ex("about"), term)),
        "prp-spo1 carries the triple term whole"
    );
}

#[test]
fn eq_rep_o_carries_a_triple_term_across_same_as() {
    let term = quoted(ex("s"), ex("p"), ex("o"));
    let base = [
        t(ex("x"), ex("q"), ex("o")),
        t(ex("y"), ex("q"), term.clone()),
        t(ex("o"), iri(OWL, "sameAs"), term.clone()),
    ];
    let mut r = Reasoner::new();
    r.set_base(base.iter().map(Triple::as_ref));
    let summary = r.reason();
    let inferred = inferred(&r);
    assert!(
        inferred.contains(&t(ex("x"), ex("q"), term.clone())),
        "eq-rep-o into the triple term"
    );
    assert!(inferred.contains(&t(ex("y"), ex("q"), ex("o"))), "eq-rep-o out of it");
    // A triple term is never a subject: eq-sym's `<<( … )>> owl:sameAs
    // ex:o` and the rest stay out of the closure and out of the count.
    assert_eq!(summary.inferred, inferred.len());
    assert_eq!(summary.base + summary.inferred, r.triples().count());
}

#[test]
fn a_triple_term_with_a_blank_node_in_two_documents_is_two_terms() {
    let mut r = Reasoner::new();
    // `ex:a ex:says <<( _:b ex:p ex:o )>> . ex:<extra> ex:p ex:o .`
    let document = |extra: &str| {
        format!(
            r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:ex="http://example.org/" rdf:version="1.2">
  <rdf:Description rdf:about="http://example.org/a">
    <ex:says rdf:parseType="Triple">
      <rdf:Description rdf:nodeID="b"><ex:p rdf:resource="http://example.org/o"/></rdf:Description>
    </ex:says>
  </rdf:Description>
  <rdf:Description rdf:about="http://example.org/{extra}"><ex:p rdf:resource="http://example.org/o"/></rdf:Description>
</rdf:RDF>"#
        )
    };
    r.load_str(&document("one"), Format::RdfXml, None)
        .expect("RDF/XML with a triple term");
    r.load_str(&document("two"), Format::RdfXml, None)
        .expect("RDF/XML with a triple term");
    r.reason();
    let said: HashSet<Term> = r
        .triples()
        .filter(|t| t.predicate == ex("says"))
        .map(|t| t.object)
        .collect();
    let expected: HashSet<Term> = ["b_1", "b_2"]
        .into_iter()
        .map(|b| quoted(BlankNode::new_unchecked(b), ex("p"), ex("o")))
        .collect();
    assert_eq!(said, expected);
}

#[test]
fn a_triple_term_never_asserts_the_triple_inside() {
    let base = [
        t(ex("a"), ex("says"), quoted(ex("s"), iri(RDF, "type"), ex("C"))),
        t(ex("C"), iri(RDFS, "subClassOf"), ex("D")),
        t(ex("says"), iri(RDFS, "domain"), ex("Speaker")),
    ];
    let r = reason(&base);
    let closure: Vec<Triple> = r.triples().collect();
    assert!(
        closure
            .iter()
            .all(|t| t.subject != ex("s").into() && t.object != ex("s").into()),
        "nothing about ex:s: {closure:#?}"
    );
    assert!(closure.contains(&t(ex("a"), iri(RDF, "type"), ex("Speaker"))));
}

#[test]
fn a_snapshot_keeps_triple_terms() {
    let nested = quoted(
        ex("s"),
        ex("p"),
        quoted(BlankNode::new_unchecked("b"), ex("p"), ex("o")),
    );
    let base = [
        t(ex("a"), ex("p"), nested.clone()),
        t(ex("p"), iri(RDFS, "subPropertyOf"), ex("q")),
    ];
    let r = reason(&base);
    let mut bytes = Vec::new();
    r.write_snapshot(&mut bytes).expect("a snapshot in memory");
    let mut back = Reasoner::from_snapshot_bytes(&bytes).expect("the snapshot loads");
    let closure = |r: &Reasoner| r.triples().collect::<HashSet<_>>();
    assert_eq!(closure(&back), closure(&r));
    // The loaded dictionary finds the term again: no change, no rounds.
    back.set_base(base.iter().map(Triple::as_ref));
    assert_eq!(back.reason().rounds, 0);
}
