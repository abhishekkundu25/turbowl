use oxrdf::{NamedNode, Triple};
use std::collections::BTreeSet;
use turbowl::{Cancel, Error, Format, Reasoner, Switch, Switches, Threads, profile};

const PREFIXES: &str = "@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .\n";

fn ex(name: &str) -> NamedNode {
    NamedNode::new(format!("http://example.org/{name}")).unwrap()
}

fn typed(subject: &str, class: &str) -> Triple {
    Triple::new(ex(subject), oxrdf::vocab::rdf::TYPE, ex(class))
}

fn loaded(text: &str, switches: Switches, threads: Threads) -> Reasoner {
    let mut r = Reasoner::with_switches(switches);
    r.set_threads(threads);
    r.load_str(&format!("{PREFIXES}{text}"), Format::Turtle, None).unwrap();
    r.reason();
    r
}

fn closure(r: &Reasoner) -> BTreeSet<String> {
    r.triples().map(|t| t.to_string()).collect()
}

#[test]
fn a_small_deletion_starts_from_the_affected_facts() {
    let mut text = "ex:A rdfs:subClassOf ex:B . ex:x a ex:A .\n".to_owned();
    for i in 0..512 {
        text.push_str(&format!("ex:u{i} ex:p ex:v{i} .\n"));
    }
    let mut r = loaded(&text, Switches::default(), Threads::ONE);
    let removed = typed("x", "A");
    r.remove([removed.as_ref()]);
    let summary = r.reason();
    assert!(!closure(&r).contains(&typed("x", "B").to_string()));
    assert_eq!(summary.base + summary.inferred, r.triples().count());
    assert_eq!(r.profile().rounds.first().unwrap().delta, 2);
    assert!(r.profile().rounds.iter().all(|round| round.delta < 16));
    assert_eq!(r.profile().deletion.fallback, None);
}

#[test]
fn removing_an_assertion_with_another_grounded_proof_needs_no_retraction() {
    let mut r = loaded(
        "ex:A rdfs:subClassOf ex:B . ex:x a ex:A, ex:B .",
        Switches::default(),
        Threads::ONE,
    );
    let before = closure(&r);
    let fact = typed("x", "B");
    r.remove([fact.as_ref()]);
    assert_eq!(r.reason().rounds, 0);
    assert_eq!(closure(&r), before);
}

#[test]
fn alternate_derivations_survive_and_unsupported_cycles_disappear() {
    for threads in [Threads::ONE, Threads::ALL.every_round()] {
        for switches in [Switches::default(), Switches::complete()] {
            let schema = "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .
                          ex:C rdfs:subClassOf ex:B .";
            let mut r = loaded(&format!("{schema} ex:x a ex:A, ex:C ."), switches, threads);
            let a = typed("x", "A");
            let c = typed("x", "C");
            r.remove([a.as_ref(), a.as_ref()]);
            r.reason();
            assert_eq!(
                closure(&r),
                closure(&loaded(&format!("{schema} ex:x a ex:C ."), switches, threads))
            );
            r.remove([c.as_ref()]);
            r.reason();
            assert_eq!(closure(&r), closure(&loaded(schema, switches, threads)));
        }
    }
}

#[test]
fn deleting_an_absent_or_inferred_only_fact_changes_nothing() {
    let mut r = loaded(
        "ex:A rdfs:subClassOf ex:B . ex:x a ex:A .",
        Switches::default(),
        Threads::ONE,
    );
    let before = closure(&r);
    for fact in [typed("x", "B"), typed("absent", "A")] {
        r.remove([fact.as_ref()]);
        assert_eq!(r.reason().rounds, 0);
        assert_eq!(closure(&r), before);
    }
}

#[test]
fn pending_deletions_keep_reads_and_repeated_changes_match_a_fresh_run() {
    let schema = "ex:A rdfs:subClassOf ex:B .";
    let mut r = loaded(
        &format!("{schema} ex:x a ex:A . ex:y a ex:A ."),
        Switches::default(),
        Threads::ONE,
    );
    let before = closure(&r);
    let x = typed("x", "A");
    let y = typed("y", "A");
    r.remove([x.as_ref()]);
    r.remove([y.as_ref()]);
    assert_eq!(closure(&r), before);
    r.insert([x.as_ref()]);
    r.reason();
    assert_eq!(
        closure(&r),
        closure(&loaded(
            &format!("{schema} ex:x a ex:A ."),
            Switches::default(),
            Threads::ONE
        ))
    );
}

#[test]
fn deleting_the_support_of_a_diagnostic_removes_it() {
    let schema = "ex:A owl:disjointWith ex:B .";
    let mut r = loaded(
        &format!("{schema} ex:x a ex:A, ex:B ."),
        Switches::default(),
        Threads::ONE,
    );
    assert!(!r.diagnostics().is_empty());
    let fact = typed("x", "B");
    r.remove([fact.as_ref()]);
    r.reason();
    assert!(r.diagnostics().is_empty());
    assert_eq!(
        closure(&r),
        closure(&loaded(
            &format!("{schema} ex:x a ex:A ."),
            Switches::default(),
            Threads::ONE
        ))
    );
}

#[test]
fn property_chains_lists_and_schema_retractions_match_full_reasoning() {
    let cases = [
        (
            "ex:p a owl:TransitiveProperty . ex:a ex:p ex:b . ex:b ex:p ex:c . ex:c ex:p ex:a .",
            Triple::new(ex("a"), ex("p"), ex("b")),
        ),
        (
            "ex:p a owl:SymmetricProperty ; rdfs:domain ex:A ; rdfs:range ex:B ; owl:inverseOf ex:q . ex:a ex:p ex:b .",
            Triple::new(ex("a"), ex("p"), ex("b")),
        ),
        (
            "ex:r owl:propertyChainAxiom ex:l . ex:l rdf:first ex:p ; rdf:rest ex:m . ex:m rdf:first ex:q ; rdf:rest rdf:nil . ex:a ex:p ex:b . ex:b ex:q ex:c .",
            Triple::new(ex("a"), ex("p"), ex("b")),
        ),
        (
            "ex:C owl:intersectionOf ex:l . ex:l rdf:first ex:A ; rdf:rest ex:m . ex:m rdf:first ex:B ; rdf:rest rdf:nil . ex:x a ex:A, ex:B .",
            typed("x", "A"),
        ),
        (
            "ex:C owl:oneOf ex:l . ex:l rdf:first ex:x ; rdf:rest rdf:nil .",
            Triple::new(ex("l"), oxrdf::vocab::rdf::FIRST, ex("x")),
        ),
        (
            "ex:p rdfs:domain ex:A . ex:A rdfs:subClassOf ex:B . ex:x ex:p ex:y .",
            Triple::new(ex("p"), oxrdf::vocab::rdfs::DOMAIN, ex("A")),
        ),
    ];
    for (text, gone) in cases {
        for threads in [Threads::ONE, Threads::ALL.every_round()] {
            let switches = Switches::default().on(turbowl::Switch::Schema);
            let mut input = oxttl::TurtleParser::new()
                .for_reader(format!("{PREFIXES}{text}").as_bytes())
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let mut r = Reasoner::with_switches(switches);
            r.set_threads(threads);
            r.insert(input.iter().map(Triple::as_ref));
            r.reason();
            r.remove([gone.as_ref()]);
            r.reason();
            if text.contains("owl:propertyChainAxiom") {
                assert_eq!(
                    r.profile().deletion.fallback,
                    None,
                    "a short acyclic chain uses maintenance"
                );
            }
            input.retain(|t| *t != gone);
            let mut fresh = Reasoner::with_switches(switches);
            fresh.insert(input.iter().map(Triple::as_ref));
            fresh.reason();
            assert_eq!(closure(&r), closure(&fresh), "{text}");
            assert_eq!(r.diagnostics(), fresh.diagnostics(), "{text}");
        }
    }
}

#[test]
fn snapshots_preserve_pending_deletions_and_set_base_uses_maintenance() {
    let mut input = vec![typed("x", "A"), typed("y", "A")];
    let mut r = Reasoner::new();
    r.load_str(&format!("{PREFIXES}ex:A rdfs:subClassOf ex:B ."), Format::Turtle, None)
        .unwrap();
    r.insert(input.iter().map(Triple::as_ref));
    r.reason();
    r.remove([input[0].as_ref()]);
    let mut bytes = Vec::new();
    r.write_snapshot(&mut bytes).unwrap();
    let mut restored = Reasoner::from_snapshot_bytes(&bytes).unwrap();
    assert_eq!(closure(&r), closure(&restored));
    let mut again = Vec::new();
    restored.write_snapshot(&mut again).unwrap();
    assert_eq!(again, bytes);
    assert_eq!(r.reason(), restored.reason());
    assert_eq!(closure(&r), closure(&restored));
    input.remove(0);
    let mut r = Reasoner::new();
    r.insert(input.iter().map(Triple::as_ref));
    r.reason();
    assert!(r.set_base(std::iter::empty()));
    r.reason();
    assert!(!closure(&r).contains(&input[0].to_string()));
    assert_eq!(r.reason().rounds, 0);
}

#[test]
fn removing_equality_support_splits_a_class_despite_a_stated_self_equality() {
    let schema = "ex:x ex:p ex:a, ex:b . ex:a owl:sameAs ex:a .";
    for switches in [Switches::default(), Switches::default().off(Switch::Equality)] {
        let mut r = loaded(
            &format!("{schema} ex:p a owl:FunctionalProperty ."),
            switches,
            Threads::ONE,
        );
        let fact = Triple::new(
            ex("p"),
            oxrdf::vocab::rdf::TYPE,
            NamedNode::new("http://www.w3.org/2002/07/owl#FunctionalProperty").unwrap(),
        );
        r.remove([fact.as_ref()]);
        r.reason();
        assert_eq!(closure(&r), closure(&loaded(schema, switches, Threads::ONE)));
    }
}

#[test]
fn an_unrelated_deletion_keeps_findings_for_literals_hidden_by_equality() {
    let text = "ex:a owl:sameAs \"bad\"^^<http://www.w3.org/2001/XMLSchema#integer> .";
    let mut r = loaded(&format!("{text} ex:x ex:p ex:y ."), Switches::default(), Threads::ONE);
    let expected = r.diagnostics().to_vec();
    assert!(!expected.is_empty());
    let gone = Triple::new(ex("x"), ex("p"), ex("y"));
    r.remove([gone.as_ref()]);
    r.reason();
    assert_eq!(r.diagnostics(), expected);
    assert_eq!(closure(&r), closure(&loaded(text, Switches::default(), Threads::ONE)));
}

#[test]
fn cancellation_during_deletion_leaves_a_recoverable_base() {
    let schema = "ex:A rdfs:subClassOf ex:B .";
    let mut r = loaded(&format!("{schema} ex:x a ex:A ."), Switches::default(), Threads::ONE);
    let fact = typed("x", "A");
    r.remove([fact.as_ref()]);
    let before = closure(&r);
    let pretripped = Cancel::new();
    pretripped.cancel();
    assert!(matches!(r.reason_cancellable(&pretripped), Err(Error::Cancelled)));
    assert_eq!(closure(&r), before);
    assert!(matches!(
        r.reason_cancellable(&profile::cancel_after(1)),
        Err(Error::Cancelled)
    ));
    assert_eq!(r.triples().count(), 0);
    r.reason();
    assert_eq!(closure(&r), closure(&loaded(schema, Switches::default(), Threads::ONE)));
}
