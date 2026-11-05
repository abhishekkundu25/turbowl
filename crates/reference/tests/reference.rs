//! The reference evaluator on one minimal input per rule and per
//! diagnostic, with the expected output written out by hand from the W3C
//! OWL 2 RL rule tables (OWL 2 Profiles, section 4.3) for the version 1
//! rule set (`docs/specs/v1.md`), in the default configuration; and the
//! switches, each of which turns its group's rules off.

mod common;

use common::{lines, parse, term};
use oxrdf::{Term, Triple};
use std::collections::{BTreeSet, HashSet};
use turbowl::{Switch, Switches};
use turbowl_reference::{diagnostic_rules, evaluate, evaluate_counted, evaluate_within, generate, rules};

/// cls-thing and cls-nothing1, the rules with an empty body.
const SEEDS: &str = "owl:Thing a owl:Class . owl:Nothing a owl:Class .";

/// What the rules add to `input` beyond the input and the two seeds, in the
/// default configuration without the `schema` group: these tests are about
/// version 1's rules; the schema rules have their minimal inputs in the
/// oracle crate (`cases/schema.rs`).
fn derived(input: &str) -> BTreeSet<String> {
    let input = parse(input);
    let closure = evaluate_within(&input, Switches::default().off(Switch::Schema), usize::MAX)
        .expect("no limit")
        .closure;
    let known: HashSet<Triple> = input.union(&parse(SEEDS)).cloned().collect();
    lines(closure.difference(&known))
}

fn expect_derived(input: &str, expected: &str) {
    assert_eq!(derived(input), lines(&parse(expected)), "input: {input}");
}

/// A diagnostic as text: its code and its terms as N-Triples writes them.
type Finding = (String, Vec<String>);

fn diagnostics(input: &str) -> BTreeSet<Finding> {
    let (_, diagnostics) = evaluate(&parse(input));
    diagnostics
        .into_iter()
        .map(|(code, terms)| (code, terms.iter().map(ToString::to_string).collect()))
        .collect()
}

/// Findings written with the test parser's terms.
fn findings(items: &[(&str, &[&str])]) -> BTreeSet<Finding> {
    items
        .iter()
        .map(|(code, terms)| (code.to_string(), terms.iter().map(|t| term(t).to_string()).collect()))
        .collect()
}

#[test]
fn empty_input_gives_the_two_seeds() {
    let (closure, diagnostics) = evaluate(&HashSet::new());
    assert_eq!(lines(&closure), lines(&parse(SEEDS)));
    assert!(diagnostics.is_empty());
}

/// No rule of the set types every subject `owl:Thing` (ADR 0004).
#[test]
fn subjects_are_not_typed_thing() {
    expect_derived("ex:a ex:p ex:b .", "");
    expect_derived("ex:C owl:intersectionOf ( ex:D ) . _:r owl:onProperty ex:p .", "");
}

/// A literal subject is never in the closure, but the rules derive it
/// inside and go on from it.
#[test]
fn the_closure_holds_the_input_and_no_literal_subject() {
    let text = "ex:p rdfs:range ex:C . ex:a ex:p \"lit\" . ex:b ex:p ex:c .";
    let input = parse(text);
    let evaluation = evaluate_counted(&input);
    assert!(evaluation.closure.is_superset(&input));
    assert_eq!(derived(text), lines(&parse("ex:c a ex:C .")));
    // `"lit" a ex:C`, inside only.
    assert_eq!(evaluation.not_rdf, 1);
}

/// eq-* apply to literals as to any term; only the triples with a literal
/// subject are left out of the closure.
#[test]
fn literals_made_equal_share_their_triples() {
    // prp-fp gives `"a" sameAs "b"` (inside); eq-rep-o copies `ex:y ex:q "a"`.
    expect_derived(
        "ex:d a owl:FunctionalProperty . ex:x ex:d \"a\" , \"b\" . ex:y ex:q \"a\" .",
        "ex:y ex:q \"b\" .",
    );
    // A literal the same as an IRI: eq-sym shows it from the IRI's side.
    expect_derived(
        "ex:d a owl:FunctionalProperty . ex:x ex:d \"a\" , ex:i .",
        "ex:i owl:sameAs \"a\" , ex:i .",
    );
}

#[test]
fn equality_rules() {
    // eq-rep-s (with eq-sym, eq-trans).
    expect_derived(
        "ex:a owl:sameAs ex:b . ex:a ex:p ex:c .",
        "ex:b owl:sameAs ex:a . ex:a owl:sameAs ex:a . ex:b owl:sameAs ex:b . ex:b ex:p ex:c .",
    );
    // eq-rep-p.
    expect_derived(
        "ex:p owl:sameAs ex:q . ex:a ex:p ex:c .",
        "ex:q owl:sameAs ex:p . ex:p owl:sameAs ex:p . ex:q owl:sameAs ex:q . ex:a ex:q ex:c .",
    );
    // eq-rep-o.
    expect_derived(
        "ex:b owl:sameAs ex:d . ex:a ex:p ex:b .",
        "ex:d owl:sameAs ex:b . ex:b owl:sameAs ex:b . ex:d owl:sameAs ex:d . ex:a ex:p ex:d .",
    );
    // eq-trans across a chain of three: the full clique.
    expect_derived(
        "ex:a owl:sameAs ex:b . ex:b owl:sameAs ex:c .",
        "ex:a owl:sameAs ex:a , ex:c . ex:b owl:sameAs ex:a , ex:b . ex:c owl:sameAs ex:a , ex:b , ex:c .",
    );
}

#[test]
fn property_rules() {
    expect_derived("ex:p rdfs:domain ex:C . ex:a ex:p ex:b .", "ex:a a ex:C .");
    expect_derived("ex:p rdfs:range ex:C . ex:a ex:p ex:b .", "ex:b a ex:C .");
    expect_derived(
        "ex:p a owl:FunctionalProperty . ex:a ex:p ex:b , ex:c .",
        "ex:b owl:sameAs ex:b , ex:c . ex:c owl:sameAs ex:b , ex:c .",
    );
    expect_derived(
        "ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:c . ex:b ex:p ex:c .",
        "ex:a owl:sameAs ex:a , ex:b . ex:b owl:sameAs ex:a , ex:b .",
    );
    expect_derived("ex:p a owl:SymmetricProperty . ex:a ex:p ex:b .", "ex:b ex:p ex:a .");
    expect_derived(
        "ex:p a owl:TransitiveProperty . ex:a ex:p ex:b . ex:b ex:p ex:c . ex:c ex:p ex:d .",
        "ex:a ex:p ex:c , ex:d . ex:b ex:p ex:d .",
    );
    expect_derived("ex:p rdfs:subPropertyOf ex:q . ex:a ex:p ex:b .", "ex:a ex:q ex:b .");
    // prp-eqp1, prp-eqp2.
    expect_derived(
        "ex:p owl:equivalentProperty ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
        "ex:a ex:q ex:b . ex:c ex:p ex:d .",
    );
    // prp-inv1, prp-inv2.
    expect_derived(
        "ex:p owl:inverseOf ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
        "ex:b ex:q ex:a . ex:d ex:p ex:c .",
    );
}

/// prp-ifp makes subjects equal only when the objects are equal.
#[test]
fn inverse_functional_needs_a_shared_object() {
    expect_derived(
        "ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:c . ex:b ex:p ex:d .",
        // x1 = x2 is a match of the rule: each subject is the same as itself.
        "ex:a owl:sameAs ex:a . ex:b owl:sameAs ex:b .",
    );
}

#[test]
fn class_rules() {
    expect_derived("ex:A rdfs:subClassOf ex:B . ex:x a ex:A .", "ex:x a ex:B .");
    // cax-eqc1, cax-eqc2, scm-eqc1 (no `B equivalentClass A`).
    expect_derived(
        "ex:A owl:equivalentClass ex:B . ex:x a ex:A . ex:y a ex:B .",
        "ex:x a ex:B . ex:y a ex:A . ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .
         ex:A rdfs:subClassOf ex:A . ex:B rdfs:subClassOf ex:B .",
    );
    expect_derived(
        "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C .",
        "ex:A rdfs:subClassOf ex:C .",
    );
    // cls-int1 and cls-int2.
    expect_derived(
        "ex:C owl:intersectionOf ( ex:A ex:B ) . ex:x a ex:A , ex:B . ex:y a ex:C . ex:z a ex:A .",
        "ex:x a ex:C . ex:y a ex:A , ex:B .",
    );
    // Two intersection lists on one class: both are read.
    expect_derived(
        "ex:C owl:intersectionOf ( ex:A ex:B ) , ( ex:D ex:E ) . ex:x a ex:D , ex:E .",
        "ex:x a ex:C , ex:A , ex:B .",
    );
    expect_derived(
        "ex:C owl:unionOf ( ex:A ex:B ) . ex:x a ex:A . ex:y a ex:B .",
        "ex:x a ex:C . ex:y a ex:C .",
    );
    // Two union lists on one class: both are read.
    expect_derived(
        "ex:C owl:unionOf ( ex:A ex:B ) , ( ex:D ) . ex:x a ex:D .",
        "ex:x a ex:C .",
    );
    // A list triple derived during the run (prp-spo1 gives the last
    // `rdf:rest`) counts.
    expect_derived(
        "ex:C owl:unionOf ex:h . ex:h rdf:first ex:A ; ex:next rdf:nil . ex:next rdfs:subPropertyOf rdf:rest .
         ex:x a ex:A .",
        "ex:h rdf:rest rdf:nil . ex:x a ex:C .",
    );
    // cls-svf1.
    expect_derived(
        "_:r owl:someValuesFrom ex:B ; owl:onProperty ex:p . ex:x ex:p ex:y . ex:y a ex:B . ex:u ex:p ex:v .",
        "ex:x a _:r .",
    );
    // cls-svf2: the filler is owl:Thing, and the value may be a literal.
    expect_derived(
        "_:r owl:someValuesFrom owl:Thing ; owl:onProperty ex:p . ex:x ex:p \"1\" .",
        "ex:x a _:r .",
    );
    // cls-avf; `"lit" a ex:B` is derived inside only.
    expect_derived(
        "_:r owl:allValuesFrom ex:B ; owl:onProperty ex:p . ex:x a _:r ; ex:p ex:y , \"lit\" .",
        "ex:y a ex:B .",
    );
    // cls-hv1 and cls-hv2.
    expect_derived(
        "_:r owl:hasValue ex:v ; owl:onProperty ex:p . ex:x a _:r . ex:y ex:p ex:v .",
        "ex:x ex:p ex:v . ex:y a _:r .",
    );
}

/// Lists are read as the W3C `LIST[...]` patterns: every cell on the path
/// from the head to `rdf:nil` needs an `rdf:first`, and `rdf:rest` stops at
/// `rdf:nil`.
#[test]
fn lists_need_a_first_in_every_cell_and_stop_at_nil() {
    // (a) A cell without an rdf:first on the only path: no member counts,
    // for a union (cls-uni) and an intersection (cls-int1, cls-int2).
    expect_derived(
        "ex:U owl:unionOf ex:u1 . ex:u1 rdf:first ex:A ; rdf:rest ex:u2 . ex:u2 rdf:rest rdf:nil .
         ex:I owl:intersectionOf ex:u1 . ex:x a ex:A . ex:y a ex:I .",
        "",
    );
    // (a) A first-less cell on one branch, a well-formed branch beside it:
    // the well-formed path's members count, the other branch's do not.
    expect_derived(
        "ex:U owl:unionOf ex:b1 . ex:b1 rdf:first ex:A ; rdf:rest ex:b2 , ex:b3 .
         ex:b2 rdf:rest ex:b4 . ex:b4 rdf:first ex:B ; rdf:rest rdf:nil .
         ex:b3 rdf:first ex:D ; rdf:rest rdf:nil .
         ex:x a ex:A . ex:y a ex:B . ex:z a ex:D .",
        "ex:x a ex:U . ex:z a ex:U .",
    );
    // (b) rdf:nil reached by rdf:rest contributes nothing, even with an
    // rdf:first and an rdf:rest of its own.
    expect_derived(
        "ex:U owl:unionOf ex:n1 . ex:n1 rdf:first ex:A ; rdf:rest rdf:nil .
         rdf:nil rdf:first ex:B ; rdf:rest rdf:nil .
         ex:I owl:intersectionOf ex:n1 . ex:x a ex:B . ex:y a ex:I .",
        "ex:y a ex:A . ex:y a ex:U .",
    );
    // rdf:nil as a head, with a first and a rest of its own, is a list.
    expect_derived(
        "ex:U owl:unionOf rdf:nil . rdf:nil rdf:first ex:B ; rdf:rest rdf:nil . ex:x a ex:B .",
        "ex:x a ex:U .",
    );
}

/// Rules outside the version 1 rule set add nothing with the `schema`
/// group off: scm-spo, scm-sco's equivalence (scm-eqc2), eq-diff*, cls-oo.
#[test]
fn rules_outside_the_set_add_nothing() {
    expect_derived("ex:p rdfs:subPropertyOf ex:q . ex:q rdfs:subPropertyOf ex:r .", "");
    // scm-sco makes each class a subclass of itself; no equivalentClass.
    expect_derived(
        "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
        "ex:A rdfs:subClassOf ex:A . ex:B rdfs:subClassOf ex:B .",
    );
    expect_derived(
        "ex:a owl:differentFrom ex:b . ex:C owl:oneOf ( ex:a ) . ex:p a owl:ObjectProperty .",
        "",
    );
}

/// No eq-ref: only terms in an `owl:sameAs` triple are the same as
/// themselves.
#[test]
fn same_as_is_reflexive_on_its_nodes_only() {
    expect_derived(
        "ex:x owl:sameAs ex:y . ex:z ex:p ex:w .",
        "ex:x owl:sameAs ex:x . ex:y owl:sameAs ex:y . ex:y owl:sameAs ex:x .",
    );
    expect_derived(
        "ex:p a owl:FunctionalProperty . ex:a ex:p ex:o .",
        "ex:o owl:sameAs ex:o .",
    );
}

#[test]
fn property_chains_compose_in_order() {
    // Length 1, 2 and 3.
    expect_derived(
        "ex:q owl:propertyChainAxiom ( ex:p1 ) . ex:a ex:p1 ex:b .",
        "ex:a ex:q ex:b .",
    );
    expect_derived(
        "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ) . ex:a ex:p1 ex:b . ex:b ex:p2 ex:c . ex:c ex:p1 ex:d .",
        "ex:a ex:q ex:c .",
    );
    expect_derived(
        "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ex:p3 ) . ex:a ex:p1 ex:b . ex:b ex:p2 ex:c . ex:c ex:p3 ex:d .
         ex:d ex:p2 ex:e .",
        "ex:a ex:q ex:d .",
    );
    // Order matters: p2 then p1 does not match (p1 p2).
    expect_derived(
        "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ) . ex:a ex:p2 ex:b . ex:b ex:p1 ex:c .",
        "",
    );
    // A chain whose head is its own property composes with what it derives.
    expect_derived(
        "ex:q owl:propertyChainAxiom ( ex:q ex:p ) . ex:a ex:q ex:b . ex:b ex:p ex:c . ex:c ex:p ex:d .",
        "ex:a ex:q ex:c , ex:d .",
    );
    // A list no axiom references composes nothing.
    expect_derived("ex:x ex:list ( ex:p1 ex:p2 ) . ex:a ex:p1 ex:b . ex:b ex:p2 ex:c .", "");
    // A cyclic list never reaches rdf:nil: no match, and it terminates.
    expect_derived(
        "ex:q owl:propertyChainAxiom ex:l1 . ex:l1 rdf:first ex:p1 ; rdf:rest ex:l2 .
         ex:l2 rdf:first ex:p2 ; rdf:rest ex:l1 . ex:a ex:p1 ex:b . ex:b ex:p2 ex:c .",
        "",
    );
    // Two chains sharing a tail are read separately.
    expect_derived(
        "ex:q owl:propertyChainAxiom ex:h1 . ex:r owl:propertyChainAxiom ex:h2 .
         ex:h1 rdf:first ex:p1 ; rdf:rest ex:t . ex:h2 rdf:first ex:p2 ; rdf:rest ex:t .
         ex:t rdf:first ex:p3 ; rdf:rest rdf:nil .
         ex:a ex:p1 ex:b . ex:b ex:p3 ex:c . ex:d ex:p2 ex:e . ex:e ex:p3 ex:f .",
        "ex:a ex:q ex:c . ex:d ex:r ex:f .",
    );
}

#[test]
fn chains_of_length_five() {
    expect_derived(
        "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ex:p3 ex:p4 ex:p5 ) .
         ex:a ex:p1 ex:b . ex:b ex:p2 ex:c . ex:c ex:p3 ex:d . ex:d ex:p4 ex:e . ex:e ex:p5 ex:f .",
        "ex:a ex:q ex:f .",
    );
    // One hop short: nothing.
    expect_derived(
        "ex:q owl:propertyChainAxiom ( ex:p1 ex:p2 ex:p3 ex:p4 ex:p5 ) .
         ex:a ex:p1 ex:b . ex:b ex:p2 ex:c . ex:c ex:p3 ex:d . ex:d ex:p4 ex:e .",
        "",
    );
}

/// No rule of the set reads container membership (ADR 0004).
#[test]
fn container_membership_derives_nothing() {
    expect_derived("ex:bag rdf:_1 ex:a . ex:bag rdf:_2 ex:b .", "");
}

#[test]
fn disjoint_classes_fire_once_in_the_stated_direction() {
    assert_eq!(
        diagnostics("ex:A owl:disjointWith ex:B . ex:i a ex:A , ex:B ."),
        findings(&[("cax-dw", &["ex:i", "ex:A", "ex:B"])])
    );
}

#[test]
fn inconsistencies() {
    assert_eq!(
        diagnostics("ex:p owl:propertyDisjointWith ex:q . ex:x ex:p ex:y ; ex:q ex:y ."),
        findings(&[("prp-pdw", &["ex:p", "ex:q", "ex:x", "ex:y"])])
    );
    assert_eq!(
        diagnostics("ex:p a owl:AsymmetricProperty . ex:x ex:p ex:y . ex:y ex:p ex:x ."),
        findings(&[
            ("prp-asyp", &["ex:p", "ex:x", "ex:y"]),
            ("prp-asyp", &["ex:p", "ex:y", "ex:x"]),
        ])
    );
    assert_eq!(
        diagnostics("ex:p a owl:AsymmetricProperty . ex:x ex:p ex:x ."),
        findings(&[("prp-asyp", &["ex:p", "ex:x", "ex:x"])])
    );
    assert_eq!(
        diagnostics("ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:x . ex:y ex:p ex:z ."),
        findings(&[("prp-irp", &["ex:p", "ex:x"])])
    );
    assert_eq!(
        diagnostics("ex:x a owl:Nothing ."),
        findings(&[("cls-nothing2", &["ex:x"])])
    );
    // Through a derivation: the subclass of owl:Nothing.
    assert_eq!(
        diagnostics("ex:A rdfs:subClassOf owl:Nothing . ex:x a ex:A ."),
        findings(&[("cls-nothing2", &["ex:x"])])
    );
    // On a literal: `"v" a owl:Nothing` is derived inside, and found.
    assert_eq!(
        diagnostics("ex:d rdfs:range owl:Nothing . ex:x ex:d \"v\" ."),
        findings(&[("cls-nothing2", &["\"v\""])])
    );
}

#[test]
fn complement_fires() {
    assert_eq!(
        diagnostics("ex:A owl:complementOf ex:B . ex:x a ex:A , ex:B ."),
        findings(&[("cls-com", &["ex:A", "ex:B", "ex:x"])])
    );
}

/// dt-not-type for a literal not in its own datatype's value space.
#[test]
fn ill_typed_literals() {
    assert_eq!(
        diagnostics(
            "ex:s ex:p \" 42\"^^xsd:int , \"2147483648\"^^xsd:int , \"abc\"^^xsd:integer ,
             \"<a>t</b>\"^^rdf:XMLLiteral , \"<a>\"^^rdf:XMLLiteral , \"3.x\"^^xsd:decimal ."
        ),
        findings(&[
            ("dt-not-type", &["\" 42\"^^xsd:int", "xsd:int"]),
            ("dt-not-type", &["\"3.x\"^^xsd:decimal", "xsd:decimal"]),
            ("dt-not-type", &["\"2147483648\"^^xsd:int", "xsd:int"]),
            ("dt-not-type", &["\"abc\"^^xsd:integer", "xsd:integer"]),
            ("dt-not-type", &["\"<a>t</b>\"^^rdf:XMLLiteral", "rdf:XMLLiteral"]),
            ("dt-not-type", &["\"<a>\"^^rdf:XMLLiteral", "rdf:XMLLiteral"]),
        ])
    );
    let quiet = diagnostics(
        "ex:s ex:p \"+5\"^^xsd:integer , \"-7\"^^xsd:int , \"x\"^^ex:unknown , \"<a>t</a>\"^^rdf:XMLLiteral , \"s\" ,
         \"t\"@en .",
    );
    assert!(quiet.is_empty(), "{quiet:?}");
    // A literal is found once, however many triples hold it.
    assert_eq!(
        diagnostics("ex:p rdfs:subPropertyOf ex:q . ex:s ex:p \"abc\"^^xsd:integer ."),
        findings(&[("dt-not-type", &["\"abc\"^^xsd:integer", "xsd:integer"])])
    );
}

/// dt-not-type for a literal a rule types with a datatype whose value space
/// it is not in: prp-rng, or prp-rng then cax-eqc1.
#[test]
fn literals_outside_a_datatype_range() {
    assert_eq!(
        diagnostics(
            "ex:n rdfs:range xsd:integer . ex:t rdfs:range xsd:string .
             ex:a ex:n \"x\" , \"y\"@en , ex:b . ex:a ex:t \"z\"@en ."
        ),
        findings(&[
            ("dt-not-type", &["\"x\"", "xsd:integer"]),
            ("dt-not-type", &["\"y\"@en", "xsd:integer"]),
            ("dt-not-type", &["\"z\"@en", "xsd:string"]),
        ])
    );
    assert_eq!(
        diagnostics(
            "ex:n rdfs:range ex:Num . ex:Num owl:equivalentClass xsd:int . ex:a ex:n \"2147483648\"^^xsd:integer ."
        ),
        findings(&[("dt-not-type", &["\"2147483648\"^^xsd:integer", "xsd:int"])])
    );
    // Ill-typed, in the range of its own datatype: one finding.
    assert_eq!(
        diagnostics("ex:n rdfs:range xsd:integer . ex:a ex:n \"abc\"^^xsd:integer ."),
        findings(&[("dt-not-type", &["\"abc\"^^xsd:integer", "xsd:integer"])])
    );
    let quiet = diagnostics(
        "ex:n rdfs:range xsd:integer . ex:a ex:n \"5\"^^xsd:int .
         ex:i rdfs:range xsd:int . ex:a ex:i \"5\"^^xsd:integer .
         ex:u rdfs:range ex:Class . ex:a ex:u \"lit\" .",
    );
    assert!(quiet.is_empty(), "{quiet:?}");
    // Every OWL 2 RL datatype is checked, xsd:decimal among them.
    assert_eq!(
        diagnostics("ex:v rdfs:range xsd:decimal . ex:a ex:v \"x\" ."),
        findings(&[("dt-not-type", &["\"x\"", "xsd:decimal"])])
    );
    // An IRI or blank-node object of a property with a class range is an
    // instance of the class, and nothing is found.
    let input = "ex:u rdfs:range ex:Class . ex:a ex:u ex:b , _:c .";
    assert!(diagnostics(input).is_empty());
    expect_derived(input, "ex:b a ex:Class . _:c a ex:Class .");
}

#[test]
fn generator_is_deterministic_and_reaches_a_fixpoint() {
    for seed in 0..50 {
        let input = generate(seed, 40);
        assert_eq!(input, generate(seed, 40));
        let set: HashSet<Triple> = input.iter().cloned().collect();
        assert_eq!(set.len(), input.len(), "no duplicates");
        let first = evaluate_counted(&set);
        // The closure is a fixpoint: evaluating it again adds nothing.
        let again = evaluate(&first.closure);
        assert_eq!(again.0, first.closure, "seed {seed}");
        assert_eq!(again.1, first.diagnostics, "seed {seed}");
    }
    assert_ne!(generate(1, 40), generate(2, 40));
}

/// The generator exercises every rule: over 300 seeds (COVERAGE_SEEDS
/// overrides), in the complete configuration, each deriving and diagnostic
/// rule fires for some seed, and some seed derives a triple that is not
/// RDF. Prints, per rule, the number of seeds it fired for.
#[test]
fn generator_exercises_every_rule() {
    let seeds: u64 = std::env::var("COVERAGE_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    let mut seeds_fired: std::collections::BTreeMap<&str, u64> = Default::default();
    let (mut too_large, mut not_rdf) = (0, 0);
    for seed in 0..seeds {
        let mut rng = turbowl_reference::Rng::at(0x5EED_0005, seed as usize);
        let size = 1 + rng.below(48);
        let input: HashSet<Triple> = generate(rng.next_u64(), size).into_iter().collect();
        let Some(evaluation) = evaluate_within(&input, Switches::complete(), 20_000) else {
            too_large += 1;
            continue;
        };
        for rule in evaluation.fired.keys() {
            *seeds_fired.entry(rule).or_default() += 1;
        }
        not_rdf += u64::from(evaluation.not_rdf > 0);
    }
    let report: Vec<String> = seeds_fired.iter().map(|(rule, n)| format!("{rule} {n}")).collect();
    eprintln!(
        "{seeds} seeds ({too_large} too large, {not_rdf} with triples that are not RDF): {}",
        report.join(", ")
    );
    let complete = Switches::complete();
    for rule in rules(complete).iter().chain(&diagnostic_rules(complete)) {
        assert!(seeds_fired.contains_key(rule), "{rule} never fires");
    }
    assert!(not_rdf > 0, "no seed derives a literal subject");
}

/// The generator makes vocabulary terms equal (ADR 0003's amendment): over
/// the chaos test's first 300 seeds, some inputs make a term `owl:sameAs` a
/// rule constant, some make two rule constants equal, and some make a term
/// equal to `rdf:nil`, the constant the list rules test for identity.
#[test]
fn generator_makes_vocabulary_terms_equal() {
    const VOCABULARY: [&str; 4] = [
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
        "http://www.w3.org/2000/01/rdf-schema#",
        "http://www.w3.org/2002/07/owl#",
        "http://www.w3.org/2001/XMLSchema#",
    ];
    let vocabulary =
        |t: &Term| matches!(t, Term::NamedNode(n) if VOCABULARY.iter().any(|ns| n.as_str().starts_with(ns)));
    let nil = Term::from(oxrdf::NamedNode::new_unchecked(
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil",
    ));
    let (mut one, mut two, mut to_nil) = (0, 0, 0);
    for seed in 0..300 {
        let mut rng = turbowl_reference::Rng::at(0x5EED_0005, seed);
        let size = 1 + rng.below(48);
        let input = generate(rng.next_u64(), size);
        let same: Vec<(Term, Term)> = input
            .iter()
            .filter(|t| t.predicate.as_str() == "http://www.w3.org/2002/07/owl#sameAs")
            .map(|t| (Term::from(t.subject.clone()), t.object.clone()))
            .collect();
        one += usize::from(same.iter().any(|(s, o)| vocabulary(s) != vocabulary(o)));
        two += usize::from(same.iter().any(|(s, o)| vocabulary(s) && vocabulary(o)));
        to_nil += usize::from(same.iter().any(|(s, o)| (*s == nil) != (*o == nil)));
    }
    eprintln!("of 300 inputs: {one} make a term equal to a constant, {two} two constants, {to_nil} a term rdf:nil");
    assert!(one > 0 && two > 0 && to_nil > 0, "{one} {two} {to_nil}");
}

/// The rules of each switch: a switch off takes exactly its rules out of
/// the lists, and over 100 generated inputs none of them fires; the other
/// rules fire as they do in the complete configuration, apart from what
/// the group's triples fed them.
#[test]
fn a_switch_off_turns_its_rules_off() {
    let complete = Switches::complete();
    let all: BTreeSet<&str> = rules(complete).into_iter().chain(diagnostic_rules(complete)).collect();
    // The rules of every group with rules, by switch.
    let checks = [
        "cax-dw",
        "prp-pdw",
        "cls-nothing2",
        "prp-asyp",
        "prp-irp",
        "cls-com",
        "dt-not-type",
        "eq-diff1",
        "eq-diff2",
        "eq-diff3",
        "cax-adc",
        "prp-adp",
        "prp-npa1",
        "prp-npa2",
        "cls-maxc1",
        "cls-maxqc1",
        "cls-maxqc2",
    ];
    let groups: &[(Switch, &[&str])] = &[
        (
            Switch::Equality,
            &["eq-sym", "eq-trans", "eq-rep-s", "eq-rep-p", "eq-rep-o"],
        ),
        (Switch::PropertyChains, &["prp-spo2"]),
        (Switch::Checks, &checks),
        (
            Switch::Schema,
            &[
                "scm-eqc2", "scm-spo", "scm-eqp1", "scm-eqp2", "scm-dom1", "scm-dom2", "scm-rng1", "scm-rng2",
                "scm-hv", "scm-svf1", "scm-svf2", "scm-avf1", "scm-avf2", "scm-int", "scm-uni",
            ],
        ),
    ];
    for &(switch, group) in groups {
        let off = complete.off(switch);
        let left: BTreeSet<&str> = rules(off).into_iter().chain(diagnostic_rules(off)).collect();
        let expected: BTreeSet<&str> = all.iter().copied().filter(|r| !group.contains(r)).collect();
        assert_eq!(left, expected, "{switch:?} off");
        for seed in 0..100 {
            let input: HashSet<Triple> = generate(seed, 30).into_iter().collect();
            let Some(e) = evaluate_within(&input, off, 20_000) else {
                continue;
            };
            let fired: Vec<&&str> = e.fired.keys().filter(|r| group.contains(r)).collect();
            assert!(fired.is_empty(), "{switch:?} off, seed {seed}: {fired:?} fired");
            assert!(e.closure.is_superset(&input));
        }
    }
    // With `checks` off, nothing is reported.
    let (closure, _) = evaluate(&parse("ex:x a owl:Nothing ."));
    let e = evaluate_within(
        &parse("ex:x a owl:Nothing ."),
        Switches::default().off(Switch::Checks),
        usize::MAX,
    )
    .expect("no limit");
    assert!(e.diagnostics.is_empty());
    assert_eq!(e.closure, closure);
    // The default configuration is `evaluate`'s.
    for seed in 0..20 {
        let input: HashSet<Triple> = generate(seed, 30).into_iter().collect();
        let a = evaluate_counted(&input);
        let b = evaluate_within(&input, Switches::default(), usize::MAX).expect("no limit");
        assert!(a.closure == b.closure && a.diagnostics == b.diagnostics, "seed {seed}");
    }
}

/// The parser these tests use reads what they write.
#[test]
fn the_test_parser_reads_lists_literals_and_shorthand() {
    let triples = parse("ex:a ex:p \"1\"^^xsd:int , \"x\"@en ; ex:q ( ex:b ex:c ) . _:r a ex:C .");
    assert_eq!(triples.len(), 8);
    assert!(lines(&triples).contains("<http://example.org/a> <http://example.org/p> \"x\"@en ."));
    assert_eq!(
        term("\"5\"^^xsd:int").to_string(),
        "\"5\"^^<http://www.w3.org/2001/XMLSchema#int>"
    );
}
