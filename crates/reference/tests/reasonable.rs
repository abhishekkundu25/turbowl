//! The reference evaluator against `reasonable` itself (the pinned crates.io
//! release `0.5.0-a1`, the code `docs/research/reasonable-parity.md` read),
//! in the `reasonable` profile.
//!
//! Closures are compared as sets of triples and diagnostics as sets of
//! (code, message), less the documented differences:
//! - D2: `reasonable` never emits `OWLRL.CLS_COM`; it is dropped from the
//!   reference's diagnostics.
//! - D3, D4, D7 to D10 are about files, I/O, the CLI and Python: nothing
//!   here crosses them (terms are compared in memory, each input is one
//!   document).
//! - D1 (prp-ifp), D5 (lists) and D6 (literal subjects) change the closure
//!   or the diagnostics: each has a test that pins the difference down to
//!   what the documented difference says, and the generated sweep leaves out
//!   the inputs they touch. D6 is wider than the spec's table says:
//!   `reasonable` drops literal-subject triples only from its output, so
//!   their consequences (an `owl:sameAs` between literals copying triples,
//!   diagnostics on a literal) do reach it.

mod common;

use common::{lines, parse};
use oxrdf::Triple;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use turtle_reasoner::Profile;
use turtle_reasoner_reference::{Rng, evaluate, evaluate_within, generate};

/// A closure and its diagnostics, as text, so that a failure reads well.
#[derive(Debug, PartialEq, Eq)]
struct Answer {
    closure: BTreeSet<String>,
    diagnostics: BTreeSet<(String, String)>,
}

/// `reasonable` on `batches`, one `load_triples` and `reason()` per batch.
fn reasonable_on(batches: &[&HashSet<Triple>]) -> Answer {
    let mut reasoner = reasonable::reasoner::Reasoner::new();
    for batch in batches {
        reasoner.load_triples(batch.iter().cloned().collect());
        reasoner.reason();
    }
    let diagnostics = reasoner
        .diagnostics()
        .iter()
        .map(|d| (d.code().to_owned(), d.message().to_owned()))
        .collect();
    Answer {
        closure: lines(&reasoner.get_triples()),
        diagnostics,
    }
}

/// The reference on `input`, in the `reasonable` profile, minus D2.
fn reference_on(input: &HashSet<Triple>) -> Answer {
    let (closure, diagnostics) = evaluate(input, Profile::Reasonable);
    Answer {
        closure: lines(&closure),
        diagnostics: diagnostics
            .into_iter()
            .filter(|(code, _)| code != "OWLRL.CLS_COM")
            .collect(),
    }
}

/// What `reasonable` has that the reference does not, and the other way.
fn differences(input: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let input = parse(input);
    let (ours, theirs) = (reference_on(&input), reasonable_on(&[&input]));
    let theirs_only = theirs.closure.difference(&ours.closure).cloned().collect();
    let ours_only = ours.closure.difference(&theirs.closure).cloned().collect();
    assert_eq!(ours.diagnostics, theirs.diagnostics);
    (theirs_only, ours_only)
}

/// The minimal inputs of parity checklist items 3 to 16, the ones
/// `tests/reference.rs` writes the expected output for, less those that show
/// a documented difference (below). Each is one document.
const MINIMAL_INPUTS: &[(&str, &str)] = &[
    ("3 empty input", ""),
    ("4 thing typing", "ex:a ex:p ex:b ."),
    (
        "4 lists and restrictions are subjects",
        "ex:C owl:intersectionOf ( ex:D ) . _:r owl:onProperty ex:p .",
    ),
    (
        "5 no literal subject",
        "ex:p rdfs:range ex:C . ex:a ex:p \"lit\" . ex:b ex:p ex:c .",
    ),
    ("6 eq-rep-s", "ex:a owl:sameAs ex:b . ex:a ex:p ex:c ."),
    ("6 eq-rep-p", "ex:p owl:sameAs ex:q . ex:a ex:p ex:c ."),
    ("6 eq-rep-o", "ex:b owl:sameAs ex:d . ex:a ex:p ex:b ."),
    ("6 eq-trans", "ex:a owl:sameAs ex:b . ex:b owl:sameAs ex:c ."),
    ("6 prp-dom", "ex:p rdfs:domain ex:C . ex:a ex:p ex:b ."),
    ("6 prp-rng", "ex:p rdfs:range ex:C . ex:a ex:p ex:b ."),
    ("6 prp-fp", "ex:p a owl:FunctionalProperty . ex:a ex:p ex:b , ex:c ."),
    (
        "6 prp-ifp, shared object",
        "ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:c . ex:b ex:p ex:c .",
    ),
    ("6 prp-symp", "ex:p a owl:SymmetricProperty . ex:a ex:p ex:b ."),
    (
        "6 prp-trp",
        "ex:p a owl:TransitiveProperty . ex:a ex:p ex:b . ex:b ex:p ex:c . ex:c ex:p ex:d .",
    ),
    ("6 prp-spo1", "ex:p rdfs:subPropertyOf ex:q . ex:a ex:p ex:b ."),
    (
        "6 prp-eqp1, prp-eqp2",
        "ex:p owl:equivalentProperty ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
    ),
    (
        "6 prp-inv1, prp-inv2",
        "ex:p owl:inverseOf ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
    ),
    ("6 cax-sco", "ex:A rdfs:subClassOf ex:B . ex:x a ex:A ."),
    (
        "6 cax-eqc1, cax-eqc2, scm-eqc1",
        "ex:A owl:equivalentClass ex:B . ex:x a ex:A . ex:y a ex:B .",
    ),
    ("6 scm-sco", "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C ."),
    (
        "6 cls-int1, cls-int2",
        "ex:C owl:intersectionOf ( ex:A ex:B ) . ex:x a ex:A , ex:B . ex:y a ex:C . ex:z a ex:A .",
    ),
    (
        "6 cls-uni",
        "ex:C owl:unionOf ( ex:A ex:B ) . ex:x a ex:A . ex:y a ex:B .",
    ),
    (
        "6 cls-svf1",
        "_:r owl:someValuesFrom ex:B ; owl:onProperty ex:p . ex:x ex:p ex:y . ex:y a ex:B . ex:u ex:p ex:v .",
    ),
    (
        "6 cls-svf2",
        "_:r owl:someValuesFrom owl:Thing ; owl:onProperty ex:p . ex:x ex:p \"1\" .",
    ),
    (
        "6 cls-avf",
        "_:r owl:allValuesFrom ex:B ; owl:onProperty ex:p . ex:x a _:r ; ex:p ex:y , \"lit\" .",
    ),
    (
        "6 cls-hv1, cls-hv2",
        "_:r owl:hasValue ex:v ; owl:onProperty ex:p . ex:x a _:r . ex:y ex:p ex:v .",
    ),
    (
        "7 no scm-spo",
        "ex:p rdfs:subPropertyOf ex:q . ex:q rdfs:subPropertyOf ex:r .",
    ),
    (
        "7 no scm-eqc2",
        "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
    ),
    (
        "7 no eq-diff, cls-oo",
        "ex:a owl:differentFrom ex:b . ex:C owl:oneOf ( ex:a ) . ex:p a owl:ObjectProperty .",
    ),
    ("8 sameAs reflexive", "ex:x owl:sameAs ex:y . ex:z ex:p ex:w ."),
    ("8 prp-fp reflexive", "ex:p a owl:FunctionalProperty . ex:a ex:p ex:o ."),
    (
        "9 chain of 1",
        "ex:q owl:propertyChainAxiom ( ex:p1 ) . ex:a ex:p1 ex:b .",
    ),
    (
        "9 chain of 2",
        "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ) . ex:a ex:p1 ex:b . ex:b ex:p2 ex:c . ex:c ex:p1 ex:d .",
    ),
    (
        "9 chain of 3",
        "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ex:p3 ) . ex:a ex:p1 ex:b . ex:b ex:p2 ex:c . ex:c ex:p3 ex:d .
         ex:d ex:p2 ex:e .",
    ),
    (
        "9 chain of 5",
        "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ex:p3 ex:p4 ex:p5 ) .
         ex:a ex:p1 ex:b . ex:b ex:p2 ex:c . ex:c ex:p3 ex:d . ex:d ex:p4 ex:e . ex:e ex:p5 ex:f .",
    ),
    (
        "9 chain order",
        "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ) . ex:a ex:p2 ex:b . ex:b ex:p1 ex:c .",
    ),
    (
        "9 chain through its own property",
        "ex:q owl:propertyChainAxiom ( ex:q ex:p ) . ex:a ex:q ex:b . ex:b ex:p ex:c . ex:c ex:p ex:d .",
    ),
    (
        "9 BJP-004: no transitivity",
        "ex:p owl:propertyChainAxiom ( ex:p ex:p ) . ex:p rdfs:subPropertyOf ex:q .
         ex:a ex:q ex:b . ex:b ex:q ex:c .",
    ),
    (
        "9 unreferenced list",
        "ex:x ex:list ( ex:p1 ex:p2 ) . ex:a ex:p1 ex:b . ex:b ex:p2 ex:c .",
    ),
    (
        "9 cyclic chain list",
        "ex:q owl:propertyChainAxiom ex:l1 . ex:l1 rdf:first ex:p1 ; rdf:rest ex:l2 .
         ex:l2 rdf:first ex:p2 ; rdf:rest ex:l1 . ex:a ex:p1 ex:b . ex:b ex:p2 ex:c .",
    ),
    (
        "9 chains sharing a tail",
        "ex:q owl:propertyChainAxiom ex:h1 . ex:r owl:propertyChainAxiom ex:h2 .
         ex:h1 rdf:first ex:p1 ; rdf:rest ex:t . ex:h2 rdf:first ex:p2 ; rdf:rest ex:t .
         ex:t rdf:first ex:p3 ; rdf:rest rdf:nil .
         ex:a ex:p1 ex:b . ex:b ex:p3 ex:c . ex:d ex:p2 ex:e . ex:e ex:p3 ex:f .",
    ),
    ("10 rdf:_1", "ex:bag rdf:_1 ex:a ."),
    (
        "10 not membership",
        "ex:bag rdf:_0 ex:a . ex:bag rdf:_a ex:b . ex:bag rdf:_ ex:c . ex:x ex:p rdf:_2 .",
    ),
    ("11 cax-dw", "ex:A owl:disjointWith ex:B . ex:i a ex:A , ex:B ."),
    (
        "12 prp-pdw",
        "ex:p owl:propertyDisjointWith ex:q . ex:x ex:p ex:y ; ex:q ex:y .",
    ),
    (
        "12 prp-asyp",
        "ex:p a owl:AsymmetricProperty . ex:x ex:p ex:y . ex:y ex:p ex:x . ex:z ex:p ex:z .",
    ),
    (
        "12 prp-irp",
        "ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:x . ex:y ex:p ex:z .",
    ),
    ("12 cls-nothing2", "ex:x a owl:Nothing ."),
    (
        "12 cls-nothing2, derived",
        "ex:A rdfs:subClassOf owl:Nothing . ex:x a ex:A .",
    ),
    ("13 cls-com (D2)", "ex:A owl:complementOf ex:B . ex:x a ex:A , ex:B ."),
    (
        "14 ill-formed",
        "ex:s ex:p \" 42\"^^xsd:int , \"2147483648\"^^xsd:int , \"abc\"^^xsd:integer , \"<a>t</b>\"^^rdf:XMLLiteral .",
    ),
    (
        "14 well-formed",
        "ex:s ex:p \"+5\"^^xsd:integer , \"-7\"^^xsd:int , \"x\"^^ex:unknown , \"<a>t</a>\"^^rdf:XMLLiteral , \"s\" .",
    ),
    (
        "14 inferred triple checked",
        "ex:p rdfs:subPropertyOf ex:q . ex:s ex:p \"abc\"^^xsd:integer .",
    ),
    (
        "15 range clash",
        "ex:n rdfs:range xsd:integer . ex:t rdfs:range xsd:string .
         ex:a ex:n \"x\" , \"y\"@en , ex:b . ex:a ex:t \"z\"@en .",
    ),
    (
        "15 no range clash",
        "ex:n rdfs:range xsd:integer . ex:a ex:n \"5\"^^xsd:int . ex:v rdfs:range xsd:decimal . ex:a ex:v \"x\" .",
    ),
    (
        "15 class range on an IRI object (not D1-D10)",
        "ex:u rdfs:range ex:Class . ex:a ex:u ex:b , \"lit\" .",
    ),
    (
        "16 diagnostics deduplicated",
        "ex:A owl:disjointWith ex:B . ex:A rdfs:subClassOf ex:B . ex:i a ex:A , ex:B .",
    ),
];

/// Parity checklist items 3 to 16: the reference equals `reasonable`.
#[test]
fn minimal_inputs_match_reasonable() {
    let mut failures = Vec::new();
    for (name, turtle) in MINIMAL_INPUTS {
        let input = parse(turtle);
        let (ours, theirs) = (reference_on(&input), reasonable_on(&[&input]));
        if ours != theirs {
            failures.push(format!(
                "{name}:\n  reference only: {:?} {:?}\n  reasonable only: {:?} {:?}",
                ours.closure.difference(&theirs.closure).collect::<Vec<_>>(),
                ours.diagnostics.difference(&theirs.diagnostics).collect::<Vec<_>>(),
                theirs.closure.difference(&ours.closure).collect::<Vec<_>>(),
                theirs.diagnostics.difference(&ours.diagnostics).collect::<Vec<_>>(),
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Items 15 (a range added in a later batch clashes with an older triple)
/// and 19 (batches equal one run): `reasonable` incremental, the reference
/// on the union.
#[test]
fn batches_in_reasonable_match_the_reference_on_their_union() {
    for (first, second) in [
        ("ex:a ex:n \"x\" .", "ex:n rdfs:range xsd:integer ."),
        (
            "ex:A rdfs:subClassOf ex:B . ex:x a ex:A .",
            "ex:B rdfs:subClassOf ex:C .",
        ),
        (
            "ex:a owl:sameAs ex:b .",
            "ex:b ex:p ex:c . ex:p a owl:SymmetricProperty .",
        ),
        (
            "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ) . ex:a ex:p1 ex:b .",
            "ex:b ex:p2 ex:c .",
        ),
        ("ex:C owl:intersectionOf ( ex:A ex:B ) . ex:x a ex:A .", "ex:x a ex:B ."),
    ] {
        let (first, second) = (parse(first), parse(second));
        let union: HashSet<Triple> = first.union(&second).cloned().collect();
        assert_eq!(reference_on(&union), reasonable_on(&[&first, &second]));
    }
}

/// D1: `reasonable` makes two subjects of an inverse-functional property the
/// same even when their objects differ. That, and what it copies, is the
/// whole difference.
#[test]
fn d1_inverse_functional() {
    let (theirs_only, ours_only) =
        differences("ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:c . ex:b ex:p ex:d .");
    assert!(ours_only.is_empty(), "{ours_only:?}");
    assert_eq!(
        theirs_only,
        lines(&parse(
            "ex:a owl:sameAs ex:b . ex:b owl:sameAs ex:a . ex:a ex:p ex:d . ex:b ex:p ex:c ."
        ))
    );
}

/// D5: `reasonable` reads lists as sets of cells joined by `rdf:rest`, so
/// two lists that share a tail are one set, and it keeps one
/// `owl:intersectionOf` or `owl:unionOf` list per class for cls-int1 and
/// cls-uni. The reference reads each list from its head.
#[test]
fn d5_lists() {
    // Shared tail: `reasonable` reads both lists as the one set { A B D }, so
    // cls-int2 types the instance of C as D, and then cls-int1 as E.
    let (theirs_only, ours_only) = differences(
        "ex:C owl:intersectionOf ex:h1 . ex:h1 rdf:first ex:A ; rdf:rest ex:t . ex:t rdf:first ex:B ; rdf:rest rdf:nil .
         ex:E owl:intersectionOf ex:h2 . ex:h2 rdf:first ex:D ; rdf:rest ex:t .
         ex:x a ex:C .",
    );
    assert!(ours_only.is_empty(), "{ours_only:?}");
    assert_eq!(theirs_only, lines(&parse("ex:x a ex:D , ex:E .")));

    // Two union lists on one class: `reasonable` uses one of them.
    let (theirs_only, ours_only) = differences("ex:C owl:unionOf ( ex:A ) , ( ex:B ) . ex:x a ex:A . ex:y a ex:B .");
    assert!(theirs_only.is_empty(), "{theirs_only:?}");
    assert_eq!(ours_only.len(), 1, "{ours_only:?}");
}

/// D6: `reasonable` derives `"a" owl:sameAs "b"` from a functional property
/// with two literal values and drops it from its output, but eq-rep-o has
/// already copied every triple with `"a"` to `"b"`, and its datatype scan
/// reports the dropped triples. The reference derives none of it.
#[test]
fn d6_literal_same_as() {
    let input = parse("ex:d a owl:FunctionalProperty . ex:x ex:d \"a\" , \"b\" . ex:y ex:d \"a\" .");
    let ours = reference_on(&input);
    let theirs = reasonable_on(&[&input]);
    let theirs_only: BTreeSet<String> = theirs.closure.difference(&ours.closure).cloned().collect();
    assert!(ours.closure.is_subset(&theirs.closure));
    // `ex:y ex:d "a"` and `"a" owl:sameAs "b"` give `ex:y ex:d "b"`.
    assert_eq!(theirs_only, lines(&parse("ex:y ex:d \"b\" .")));

    // One ill-formed value: prp-fp gives `"abc" owl:sameAs "abc"`, and the
    // datatype scan reports that triple as well as the loaded one.
    let input = parse("ex:d a owl:FunctionalProperty . ex:x ex:d \"abc\"^^xsd:integer .");
    let ours = reference_on(&input);
    let theirs = reasonable_on(&[&input]);
    assert_eq!(ours.closure, theirs.closure);
    let lit = "\"abc\"^^<http://www.w3.org/2001/XMLSchema#integer>";
    let theirs_only: Vec<_> = theirs.diagnostics.difference(&ours.diagnostics).collect();
    assert_eq!(
        theirs_only,
        [&(
            "RDFS.DATATYPE".to_owned(),
            format!("ill-formed literal {lit} used in triple {lit} <http://www.w3.org/2002/07/owl#sameAs> {lit}")
        )]
    );
    assert_eq!(ours.diagnostics.len(), 1);
}

/// Generated inputs: the reference equals `reasonable` on every input no
/// documented difference touches (REASONABLE_SEEDS overrides the count).
/// Inputs are left out, and counted, when they have an
/// `owl:InverseFunctionalProperty` (D1), a list that is not a simple chain
/// from one head (a shared tail, or a second `owl:intersectionOf` or
/// `owl:unionOf` on one class) or list triples the rules derive (D5), or a
/// reference closure past 20,000 triples; and, when they differ, when a rule
/// matched with a literal where the subject of what it derives goes (D6).
#[test]
fn generated_inputs_match_reasonable() {
    let seeds: usize = std::env::var("REASONABLE_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    let mut left_out: BTreeMap<&str, usize> = BTreeMap::new();
    let mut compared = 0;
    let mut failures = Vec::new();
    for i in 0..seeds {
        let mut rng = Rng::at(0x5EED_0005, i);
        let size = 1 + rng.below(48);
        let input: HashSet<Triple> = generate(rng.next_u64(), size).into_iter().collect();
        let Some(evaluation) = evaluate_within(&input, Profile::Reasonable, 20_000) else {
            *left_out.entry("too large").or_default() += 1;
            continue;
        };
        if let Some(reason) = touched_by_a_difference(&input, &evaluation.closure) {
            *left_out.entry(reason).or_default() += 1;
            continue;
        }
        let ours = reference_on(&input);
        let theirs = reasonable_on(&[&input]);
        if ours != theirs && evaluation.literal_subject {
            *left_out.entry("D6").or_default() += 1;
            continue;
        }
        compared += 1;
        if ours != theirs {
            failures.push(format!(
                "seed index {i}:\n  reference only: {:?} {:?}\n  reasonable only: {:?} {:?}\n{}",
                ours.closure.difference(&theirs.closure).take(5).collect::<Vec<_>>(),
                ours.diagnostics
                    .difference(&theirs.diagnostics)
                    .take(5)
                    .collect::<Vec<_>>(),
                theirs.closure.difference(&ours.closure).take(5).collect::<Vec<_>>(),
                theirs
                    .diagnostics
                    .difference(&ours.diagnostics)
                    .take(5)
                    .collect::<Vec<_>>(),
                turtle_reasoner_reference::to_ntriples(&input),
            ));
        }
    }
    eprintln!("{seeds} generated inputs: {compared} compared, left out {left_out:?}");
    assert!(
        failures.is_empty(),
        "{} differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(compared * 3 > seeds, "too few compared: {compared} of {seeds}");
}

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";

/// The documented difference that changes the closure of `input`, if any.
fn touched_by_a_difference(input: &HashSet<Triple>, closure: &HashSet<Triple>) -> Option<&'static str> {
    let is = |t: &Triple, ns: &str, local: &str| t.predicate.as_str().strip_prefix(ns) == Some(local);
    let object = |t: &Triple| t.object.to_string();
    if input
        .iter()
        .any(|t| object(t) == format!("<{OWL}InverseFunctionalProperty>"))
    {
        return Some("D1");
    }
    // D5: a list cell reached twice (by rdf:rest or as a head), or a class
    // with two intersection or union lists, or list triples derived.
    let mut reached: HashSet<String> = HashSet::new();
    let mut heads: HashSet<(String, &str)> = HashSet::new();
    for t in input {
        let list_axiom = ["intersectionOf", "unionOf", "propertyChainAxiom"]
            .into_iter()
            .find(|local| is(t, OWL, local));
        if (is(t, RDF, "rest") && object(t) != format!("<{RDF}nil>") || list_axiom.is_some())
            && !reached.insert(object(t))
        {
            return Some("D5");
        }
        if let Some(local) = list_axiom
            && local != "propertyChainAxiom"
            && !heads.insert((t.subject.to_string(), local))
        {
            return Some("D5");
        }
    }
    if closure
        .iter()
        .any(|t| (is(t, RDF, "first") || is(t, RDF, "rest")) && !input.contains(t))
    {
        return Some("D5");
    }
    None
}
