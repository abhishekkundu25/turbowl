//! The `Reasoner` through its public interface alone: loading, the base,
//! blank nodes, formats and the cost of an unchanged base. The tests that
//! need rules to infer something run over a toy rule table inside the crate
//! (`src/reasoner.rs`).

use oxrdf::{BlankNode, Literal, NamedNode, Term, Triple};
use std::collections::BTreeSet;
use turbowl::{Error, Format, Reasoner, Summary};

// The interface promises `Send`.
const _: () = {
    const fn send<T: Send>() {}
    send::<Reasoner>()
};

const DOCUMENT: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    ex:Dog rdfs:subClassOf ex:Animal ; rdfs:label "chien"@fr, "dog"@en-GB .
    ex:rex a ex:Dog ; ex:age 3 ; ex:owner [ ex:name "Ann" ] ; ex:toys ( ex:ball ex:bone ) .
    _:b ex:p ex:o .
"#;

fn ex(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://example.org/{local}"))
}

fn closure(r: &Reasoner) -> BTreeSet<String> {
    r.triples().map(|t| t.to_string()).collect()
}

fn loaded(text: &str, format: Format) -> Reasoner {
    let mut r = Reasoner::new();
    r.load_str(text, format, None).unwrap_or_else(|e| panic!("{e}"));
    r.reason();
    r
}

#[test]
fn the_summary_counts_what_the_reads_give() {
    let mut r = Reasoner::new();
    r.load_str(DOCUMENT, Format::Turtle, None).unwrap();
    let s = r.reason();
    assert_eq!(s.base, 13);
    assert_eq!(s.base + s.inferred, r.triples().count());
    assert_eq!(s.inferred, r.inferred().count());
    assert_eq!(s.diagnostics, r.diagnostics().len());
}

#[test]
fn loading_a_document_twice_changes_nothing() {
    let mut r = loaded(DOCUMENT, Format::Turtle);
    let once = closure(&r);
    // Its anonymous nodes and lists would get new labels from a second parse.
    r.load_str(DOCUMENT, Format::Turtle, None).unwrap();
    assert_eq!(r.reason().rounds, 0);
    assert_eq!(closure(&r), once);

    // A file too, and the same text read from a file is the same document.
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("reasoner-twice.ttl");
    std::fs::write(&path, DOCUMENT).unwrap();
    r.load_file(&path).unwrap();
    r.load_file(&path).unwrap();
    assert_eq!(r.reason().rounds, 0);
    assert_eq!(closure(&r), once);

    // Another base IRI makes another document.
    r.load_str(DOCUMENT, Format::Turtle, Some("http://example.org/other/"))
        .unwrap();
    assert_eq!(r.reason().base, 13 + 8, "a new copy of the 8 triples with blank nodes");

    // After a removal, loading it again adds it again.
    let mut r = loaded(DOCUMENT, Format::Turtle);
    let dog = Triple::new(ex("rex"), oxrdf::vocab::rdf::TYPE, ex("Dog"));
    r.remove([dog.as_ref()]);
    r.reason();
    assert!(!closure(&r).contains(&dog.to_string()));
    r.load_str(DOCUMENT, Format::Turtle, None).unwrap();
    r.reason();
    assert!(closure(&r).contains(&dog.to_string()));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_failed_load_changes_nothing() {
    let mut r = loaded(DOCUMENT, Format::Turtle);
    let before = closure(&r);
    let bad = "@prefix ex: <http://example.org/> .\nex:new ex:p ex:o .\nex:broken ex:p .\n";
    match r.load_str(bad, Format::Turtle, None) {
        Err(Error::Parse { line: 3, .. }) => {}
        other => panic!("expected a parse error on line 3, got {other:?}"),
    }
    assert_eq!(r.reason().rounds, 0);
    assert_eq!(closure(&r), before, "the triple before the error is not added");

    let missing = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-such-file.ttl");
    assert!(matches!(r.load_file(&missing), Err(Error::Io { .. })));
    assert!(matches!(
        r.load_file(std::path::Path::new("model.docx")),
        Err(Error::UnknownFormat { .. })
    ));
    assert_eq!(r.reason().rounds, 0);
}

#[test]
fn blank_nodes_of_two_documents_stay_apart() {
    let doc = "_:b <http://example.org/p> <http://example.org/o> .";
    let mut r = Reasoner::new();
    r.load_str(doc, Format::NTriples, None).unwrap();
    r.load_str(&format!("{doc}\n"), Format::NTriples, None).unwrap();
    assert_eq!(r.reason().base, 2);
    let blank: BTreeSet<String> = r
        .triples()
        .filter(|t| t.subject.is_blank_node())
        .map(|t| t.subject.to_string())
        .collect();
    assert_eq!(blank, BTreeSet::from(["_:b_1".to_owned(), "_:b_2".to_owned()]));
}

#[test]
fn inserted_blank_nodes_keep_their_labels() {
    let (x, y) = (BlankNode::new_unchecked("x"), BlankNode::new_unchecked("y"));
    let shaped = BlankNode::new_unchecked("x_1");
    let input = [
        Triple::new(x.clone(), ex("p"), y.clone()),
        Triple::new(
            y.clone(),
            ex("p"),
            Literal::new_language_tagged_literal_unchecked("hé", "fr"),
        ),
        Triple::new(shaped.clone(), ex("p"), x.clone()),
    ];
    let mut r = Reasoner::new();
    r.insert(input.iter().map(Triple::as_ref));
    // A document's `_:x` is another node.
    r.load_str("_:x <http://example.org/p> _:y .", Format::Turtle, None)
        .unwrap();
    assert_eq!(r.reason().base, 4);
    let out = closure(&r);
    for t in &input[..2] {
        assert!(out.contains(&t.to_string()), "{t} round-trips: {out:#?}");
    }
    // A label shaped like a document's (`x_1`) gets `_0`, so it never takes
    // the label of the document's `_:x`, which is `x_1`.
    assert!(out.contains("_:x_1_0 <http://example.org/p> _:x"), "{out:#?}");
    assert!(out.contains("_:x_1 <http://example.org/p> _:y_1"), "{out:#?}");

    // `remove` and `set_base` name blank nodes as `insert` does.
    r.remove([input[0].as_ref()]);
    assert_eq!(r.reason().base, 3);
    assert!(!closure(&r).contains(&input[0].to_string()));
}

#[test]
fn every_format_round_trips_with_language_tags() {
    let text = r#"
        @prefix ex: <http://example.org/> .
        ex:Dog ex:label "chien"@fr, "dog"@en-GB, "plain" ; ex:legs 4 ; ex:weight "12.5"^^<http://www.w3.org/2001/XMLSchema#decimal> .
        ex:rex a ex:Dog ; ex:knows ex:Dog .
    "#;
    let source = loaded(text, Format::Turtle);
    let expected = closure(&source);
    // Seven base triples, and the heads of cls-thing and cls-nothing1.
    assert_eq!(expected.len(), 9);
    let french = Triple::new(
        ex("Dog"),
        ex("label"),
        Term::from(Literal::new_language_tagged_literal_unchecked("chien", "fr")),
    );
    assert!(expected.contains(&french.to_string()));
    for format in [Format::Turtle, Format::NTriples, Format::N3, Format::RdfXml] {
        let mut out = Vec::new();
        source.write(&mut out, format, false).unwrap();
        let written = String::from_utf8(out).unwrap();
        // N3 is written as Turtle, which N3 reads.
        let back = loaded(&written, format);
        assert_eq!(closure(&back), expected, "{format:?}:\n{written}");
    }
    let mut out = Vec::new();
    source.write(&mut out, Format::Turtle, true).unwrap();
    let inferred = String::from_utf8(out).unwrap();
    assert_eq!(
        loaded(&inferred, Format::Turtle).triples().count(),
        source.inferred().count()
    );
}

#[test]
fn an_unchanged_base_costs_nothing() {
    let triples: Vec<Triple> = loaded(DOCUMENT, Format::Turtle).triples().collect();
    let mut r = Reasoner::new();
    assert!(!r.set_base(triples.iter().map(Triple::as_ref)));
    let first = r.reason();
    assert!(first.rounds > 0, "the first run");
    assert!(!r.set_base(triples.iter().rev().map(Triple::as_ref)));
    assert_eq!(r.reason(), Summary { rounds: 0, ..first });
    // Running again, or removing what is not there, costs nothing either.
    assert_eq!(r.reason().rounds, 0);
    r.remove([Triple::new(ex("nobody"), ex("p"), ex("o")).as_ref()]);
    assert_eq!(r.reason().rounds, 0);
    // A smaller base is a removal.
    assert!(r.set_base(triples[1..].iter().map(Triple::as_ref)));
    assert_eq!(r.reason().base, first.base - 1);
}

#[test]
fn clear_empties_everything() {
    let mut r = loaded(DOCUMENT, Format::Turtle);
    r.clear();
    assert_eq!(r.triples().count(), 0);
    assert!(r.diagnostics().is_empty());
    assert_eq!(r.reason().base, 0);
    // A document loaded before `clear` loads again.
    r.load_str(DOCUMENT, Format::Turtle, None).unwrap();
    assert_eq!(r.reason().base, 13);
}

#[test]
fn an_untagged_lang_string_is_reported_ill_typed() {
    let lang_string = NamedNode::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#langString");
    let untagged = Literal::new_typed_literal("x", lang_string.clone());
    let mut r = Reasoner::new();
    r.insert([Triple::new(ex("s"), ex("p"), untagged.clone()).as_ref()]);
    r.reason();
    let found: Vec<(&str, Vec<Term>)> = r.diagnostics().iter().map(|d| (d.code, d.terms.clone())).collect();
    assert_eq!(
        found,
        [("dt-not-type", vec![Term::from(untagged), Term::from(lang_string)])]
    );
}
