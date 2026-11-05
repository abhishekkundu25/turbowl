//! The reference evaluator on the parity checklist's minimal inputs (items 3
//! to 15 of `docs/research/reasonable-parity.md`), with the expected output
//! written out by hand. Where `reasonable` is wrong (D1, D2), the expected
//! output is the profiles' one and the test says so.

mod common;

use common::{lines, parse};
use oxrdf::Triple;
use std::collections::{BTreeSet, HashSet};
use turtle_reasoner::Profile;
use turtle_reasoner_reference::{evaluate, evaluate_counted, generate};

/// The seeds of both profiles (cls-thing, cls-nothing1).
const SEEDS: &str = "owl:Thing a owl:Class . owl:Nothing a owl:Class .";

/// What `owl-rl` adds to `input` beyond the input and the two seeds.
fn derived(input: &str) -> BTreeSet<String> {
    let input = parse(input);
    let (closure, _) = evaluate(&input, Profile::OwlRl);
    let known: HashSet<Triple> = input.union(&parse(SEEDS)).cloned().collect();
    lines(closure.difference(&known))
}

fn expect_derived(input: &str, expected: &str) {
    assert_eq!(derived(input), lines(&parse(expected)), "input: {input}");
}

fn diagnostics(input: &str) -> BTreeSet<(String, String)> {
    let (_, diagnostics) = evaluate(&parse(input), Profile::OwlRl);
    let (_, in_reasonable) = evaluate(&parse(input), Profile::Reasonable);
    assert_eq!(
        diagnostics, in_reasonable,
        "the profiles differ only in owl:Thing typing here"
    );
    diagnostics
}

fn messages(input: &str, code: &str) -> BTreeSet<String> {
    diagnostics(input)
        .into_iter()
        .filter(|(c, _)| c == code)
        .map(|(_, m)| m)
        .collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// Item 3.
#[test]
fn empty_input_gives_the_seeds_and_their_thing_typing() {
    let (closure, diagnostics) = evaluate(&HashSet::new(), Profile::Reasonable);
    let expected = parse(&format!("{SEEDS} owl:Thing a owl:Thing . owl:Nothing a owl:Thing ."));
    assert_eq!(lines(&closure), lines(&expected));
    assert!(diagnostics.is_empty());

    let (closure, _) = evaluate(&HashSet::new(), Profile::OwlRl);
    assert_eq!(lines(&closure), lines(&parse(SEEDS)));
}

// Item 4.
#[test]
fn every_subject_is_a_thing_and_an_object_alone_is_not() {
    let input = parse("ex:a ex:p ex:b .");
    let (closure, _) = evaluate(&input, Profile::Reasonable);
    let expected = parse(&format!(
        "ex:a ex:p ex:b . ex:a a owl:Thing . {SEEDS} owl:Thing a owl:Thing . owl:Nothing a owl:Thing ."
    ));
    assert_eq!(lines(&closure), lines(&expected));

    // A list cell and a restriction are subjects too.
    let input = parse("ex:C owl:intersectionOf ( ex:D ) . _:r owl:onProperty ex:p .");
    let (closure, _) = evaluate(&input, Profile::Reasonable);
    let things: usize = closure
        .iter()
        .filter(|t| t.predicate.as_str().ends_with("#type") && t.object.to_string().ends_with("#Thing>"))
        .count();
    // ex:C, the list cell, _:r, owl:Thing, owl:Nothing.
    assert_eq!(things, 5);
}

// Item 5.
#[test]
fn the_closure_holds_the_input_and_no_literal_subject() {
    let input = parse("ex:p rdfs:range ex:C . ex:a ex:p \"lit\" . ex:b ex:p ex:c .");
    let (closure, _) = evaluate(&input, Profile::OwlRl);
    assert!(closure.is_superset(&input));
    assert_eq!(
        derived("ex:p rdfs:range ex:C . ex:a ex:p \"lit\" . ex:b ex:p ex:c ."),
        lines(&parse("ex:c a ex:C ."))
    );

    // prp-fp on two literal values: `"a" sameAs "b"` is not derived (D6).
    expect_derived("ex:d a owl:FunctionalProperty . ex:x ex:d \"a\" , \"b\" .", "");
}

// Item 6: each implemented rule on its minimal input.
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

// D1: prp-ifp makes subjects equal only when the objects are equal.
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

// Item 7: rules `reasonable` does not implement add nothing.
#[test]
fn unimplemented_rules_add_nothing() {
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

// Item 8.
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

// Item 9.
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

// Item 10.
#[test]
fn container_membership() {
    expect_derived(
        "ex:bag rdf:_1 ex:a .",
        "rdf:_1 a rdf:Property , rdfs:ContainerMembershipProperty ; rdfs:subPropertyOf rdfs:member .
         ex:bag rdfs:member ex:a .",
    );
    expect_derived("ex:bag rdf:_0 ex:a . ex:bag rdf:_a ex:b . ex:bag rdf:_ ex:c .", "");
    // An IRI used only as an object does not count.
    expect_derived("ex:x ex:p rdf:_2 .", "");
}

// Item 11.
#[test]
fn disjoint_classes_fire_once_in_the_stated_direction() {
    let input = "ex:A owl:disjointWith ex:B . ex:i a ex:A , ex:B .";
    assert_eq!(
        diagnostics(input),
        BTreeSet::from([(
            "OWLRL.CAX_DW".to_owned(),
            "inst <http://example.org/i> is both <http://example.org/A> and <http://example.org/B> (disjoint classes)"
                .to_owned()
        )])
    );
}

// Item 12.
#[test]
fn inconsistency_messages() {
    assert_eq!(
        messages(
            "ex:p owl:propertyDisjointWith ex:q . ex:x ex:p ex:y ; ex:q ex:y .",
            "OWLRL.PRP_PDW"
        ),
        set(&[
            "property <http://example.org/p> is disjoint with <http://example.org/q> for pair \
               (<http://example.org/x> <http://example.org/p> <http://example.org/y>)"
        ])
    );
    assert_eq!(
        messages(
            "ex:p a owl:AsymmetricProperty . ex:x ex:p ex:y . ex:y ex:p ex:x .",
            "OWLRL.PRP_ASYP"
        ),
        set(&[
            "property <http://example.org/p> of <http://example.org/x> and <http://example.org/y> is asymmetric",
            "property <http://example.org/p> of <http://example.org/y> and <http://example.org/x> is asymmetric",
        ])
    );
    assert_eq!(
        messages("ex:p a owl:AsymmetricProperty . ex:x ex:p ex:x .", "OWLRL.PRP_ASYP"),
        set(&["property <http://example.org/p> of <http://example.org/x> and <http://example.org/x> is asymmetric"])
    );
    assert_eq!(
        messages(
            "ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:x . ex:y ex:p ex:z .",
            "OWLRL.PRP_IRP"
        ),
        set(&["property <http://example.org/p> of <http://example.org/x> is irreflexive"])
    );
    assert_eq!(
        messages("ex:x a owl:Nothing .", "OWLRL.CLS_NOTHING"),
        set(&["instance <http://example.org/x> is owl:Nothing (unsatisfiable)"])
    );
    // Through a derivation: the subclass of owl:Nothing.
    assert_eq!(
        messages("ex:A rdfs:subClassOf owl:Nothing . ex:x a ex:A .", "OWLRL.CLS_NOTHING"),
        set(&["instance <http://example.org/x> is owl:Nothing (unsatisfiable)"])
    );
}

// Item 13 (D2: `reasonable` never fires it; the profiles do).
#[test]
fn complement_fires() {
    assert_eq!(
        messages("ex:A owl:complementOf ex:B . ex:x a ex:A , ex:B .", "OWLRL.CLS_COM"),
        set(&[
            "Individual <http://example.org/x> has type <http://example.org/B> and <http://example.org/A> which are \
             complements"
        ])
    );
}

// Item 14.
#[test]
fn ill_formed_literals() {
    let fired = messages(
        "ex:s ex:p \" 42\"^^xsd:int , \"2147483648\"^^xsd:int , \"abc\"^^xsd:integer , \"<a>t</b>\"^^rdf:XMLLiteral .",
        "RDFS.DATATYPE",
    );
    assert_eq!(
        fired,
        set(&[
            "ill-formed literal \" 42\"^^<http://www.w3.org/2001/XMLSchema#int> used in triple \
             <http://example.org/s> <http://example.org/p> \" 42\"^^<http://www.w3.org/2001/XMLSchema#int>",
            "ill-formed literal \"2147483648\"^^<http://www.w3.org/2001/XMLSchema#int> used in triple \
             <http://example.org/s> <http://example.org/p> \"2147483648\"^^<http://www.w3.org/2001/XMLSchema#int>",
            "ill-formed literal \"abc\"^^<http://www.w3.org/2001/XMLSchema#integer> used in triple \
             <http://example.org/s> <http://example.org/p> \"abc\"^^<http://www.w3.org/2001/XMLSchema#integer>",
            "ill-formed literal \"<a>t</b>\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral> used in \
             triple <http://example.org/s> <http://example.org/p> \
             \"<a>t</b>\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral>",
        ])
    );
    let quiet = diagnostics(
        "ex:s ex:p \"+5\"^^xsd:integer , \"-7\"^^xsd:int , \"x\"^^ex:unknown , \"<a>t</a>\"^^rdf:XMLLiteral , \"s\" .",
    );
    assert!(quiet.is_empty(), "{quiet:?}");
    // An inferred triple is checked too.
    assert_eq!(
        messages(
            "ex:p rdfs:subPropertyOf ex:q . ex:s ex:p \"abc\"^^xsd:integer .",
            "RDFS.DATATYPE"
        )
        .len(),
        2
    );
}

// Item 15.
#[test]
fn range_clashes() {
    let fired = messages(
        "ex:n rdfs:range xsd:integer . ex:t rdfs:range xsd:string .
         ex:a ex:n \"x\" , \"y\"@en , ex:b . ex:a ex:t \"z\"@en .",
        "RDFS.DATATYPE_RANGE",
    );
    assert_eq!(
        fired,
        set(&[
            "range clash: object \"x\" is not in datatype <http://www.w3.org/2001/XMLSchema#integer> for predicate \
             <http://example.org/n>",
            "range clash: object \"y\"@en is not in datatype <http://www.w3.org/2001/XMLSchema#integer> for \
             predicate <http://example.org/n>",
            "range clash: object <http://example.org/b> is not in datatype \
             <http://www.w3.org/2001/XMLSchema#integer> for predicate <http://example.org/n>",
            "range clash: object \"z\"@en is not in datatype <http://www.w3.org/2001/XMLSchema#string> for \
             predicate <http://example.org/t>",
        ])
    );
    let quiet = diagnostics(
        "ex:n rdfs:range xsd:integer . ex:a ex:n \"5\"^^xsd:int .
         ex:u rdfs:range ex:Class . ex:a ex:u \"lit\" .
         ex:v rdfs:range xsd:decimal . ex:a ex:v \"x\" .",
    );
    assert!(quiet.is_empty(), "{quiet:?}");
    // As `reasonable` (no documented difference covers it): an IRI object
    // clashes with any range, a class too.
    assert_eq!(
        messages("ex:u rdfs:range ex:Class . ex:a ex:u ex:b .", "RDFS.DATATYPE_RANGE"),
        set(&[
            "range clash: object <http://example.org/b> is not in datatype <http://example.org/Class> for \
             predicate <http://example.org/u>"
        ])
    );
}

#[test]
fn generator_is_deterministic_and_reaches_a_fixpoint() {
    for seed in 0..50 {
        let input = generate(seed, 40);
        assert_eq!(input, generate(seed, 40));
        let set: HashSet<Triple> = input.iter().cloned().collect();
        assert_eq!(set.len(), input.len(), "no duplicates");
        for profile in [Profile::OwlRl, Profile::Reasonable] {
            let first = evaluate_counted(&set, profile);
            // The closure is a fixpoint: evaluating it again adds nothing.
            let again = evaluate(&first.closure, profile);
            assert_eq!(again.0, first.closure, "seed {seed}");
            assert_eq!(again.1, first.diagnostics, "seed {seed}");
        }
    }
    assert_ne!(generate(1, 40), generate(2, 40));
}

/// The generator exercises every rule: over 300 seeds (COVERAGE_SEEDS
/// overrides), each deriving and diagnostic rule fires for some seed in each
/// profile that runs it. Prints, per rule, the number of seeds it fired for.
#[test]
fn generator_exercises_every_rule() {
    let seeds: u64 = std::env::var("COVERAGE_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    for profile in [Profile::OwlRl, Profile::Reasonable] {
        let mut seeds_fired: std::collections::BTreeMap<&str, u64> = Default::default();
        let mut too_large = 0;
        for seed in 0..seeds {
            let mut rng = turtle_reasoner_reference::Rng::at(0x5EED_0005, seed as usize);
            let size = 1 + rng.below(48);
            let input: HashSet<Triple> = generate(rng.next_u64(), size).into_iter().collect();
            let Some(evaluation) = turtle_reasoner_reference::evaluate_within(&input, profile, 20_000) else {
                too_large += 1;
                continue;
            };
            for rule in evaluation.fired.keys() {
                *seeds_fired.entry(rule).or_default() += 1;
            }
        }
        let report: Vec<String> = seeds_fired.iter().map(|(rule, n)| format!("{rule} {n}")).collect();
        eprintln!(
            "{profile:?}, {seeds} seeds ({too_large} too large): {}",
            report.join(", ")
        );
        for rule in
            turtle_reasoner_reference::rules(profile).chain(turtle_reasoner_reference::DIAGNOSTIC_RULES.iter().copied())
        {
            assert!(seeds_fired.contains_key(rule), "{profile:?}: {rule} never fires");
        }
    }
}

/// The parser these tests use reads what they write.
#[test]
fn the_test_parser_reads_lists_literals_and_shorthand() {
    let triples = parse("ex:a ex:p \"1\"^^xsd:int , \"x\"@en ; ex:q ( ex:b ex:c ) . _:r a ex:C .");
    assert_eq!(triples.len(), 8);
    assert!(lines(&triples).contains("<http://example.org/a> <http://example.org/p> \"x\"@en ."));
}
