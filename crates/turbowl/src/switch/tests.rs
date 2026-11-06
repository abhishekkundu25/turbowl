//! The switches: the defaults of the spec's table, the families registered
//! with them, and the reasoner's runs under each configuration. The groups
//! that are still empty have their own test in their family's file.

use super::*;
use crate::rules::FAMILIES;
use crate::rules::testing::expand;
use crate::{Diagnostic, Format, Reasoner, Summary};
use std::collections::BTreeSet;

const PREFIXES: &str = "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex: <http://example.org/> .\n";

fn load(r: &mut Reasoner, turtle: &str) {
    r.load_str(&format!("{PREFIXES}{turtle}"), Format::Turtle, None)
        .unwrap_or_else(|e| panic!("{e}"));
}

/// A reasoner with `switches` over a document, run.
fn reasoned(switches: Switches, turtle: &str) -> Reasoner {
    let mut r = Reasoner::with_switches(switches);
    load(&mut r, turtle);
    r.reason();
    r
}

fn lines(triples: impl Iterator<Item = oxrdf::Triple>) -> BTreeSet<String> {
    triples.map(|t| t.to_string()).collect()
}

/// What a caller reads after a run.
#[derive(Debug, PartialEq, Eq)]
struct View {
    closure: BTreeSet<String>,
    inferred: BTreeSet<String>,
    diagnostics: Vec<Diagnostic>,
}

fn view(r: &Reasoner) -> View {
    View {
        closure: lines(r.triples()),
        inferred: lines(r.inferred()),
        diagnostics: r.diagnostics().to_vec(),
    }
}

fn codes(r: &Reasoner) -> BTreeSet<&'static str> {
    r.diagnostics().iter().map(|d| d.code).collect()
}

/// The spec's table ("The switches"): each switch's name and default.
const TABLE: [(Switch, &str, bool); 7] = [
    (Switch::Equality, "equality", true),
    (Switch::PropertyChains, "property-chains", true),
    (Switch::Checks, "checks", true),
    (Switch::Identity, "identity", false),
    (Switch::Schema, "schema", false),
    (Switch::Trivial, "trivial", false),
    (Switch::LiteralValues, "literal-values", false),
];

#[test]
fn the_defaults_follow_the_spec_table() {
    assert_eq!(Switch::ALL, TABLE.map(|(s, _, _)| s));
    let (default, complete) = (Switches::default(), Switches::complete());
    for (switch, name, on) in TABLE {
        assert_eq!(switch.name(), name);
        assert_eq!(Switch::from_name(name), Some(switch));
        assert_eq!(default.is_on(switch), on, "{name}");
        assert!(complete.is_on(switch), "{name}");
    }
    assert_eq!(Reasoner::new().switches(), default);
    assert_eq!(Reasoner::default().switches(), default);
    assert_eq!(format!("{default:?}"), r#"{"equality", "property-chains", "checks"}"#);
}

#[test]
fn names_are_parsed_exactly() {
    for name in [
        "",
        "Equality",
        "property_chains",
        "prp-spo2",
        "literal values",
        " checks",
    ] {
        assert_eq!(Switch::from_name(name), None, "{name:?}");
    }
}

#[test]
fn on_and_off_change_one_switch() {
    let none = Switch::ALL.into_iter().fold(Switches::complete(), Switches::off);
    assert!(Switch::ALL.into_iter().all(|s| !none.is_on(s)));
    assert_eq!(Switch::ALL.into_iter().fold(none, Switches::on), Switches::complete());
    for s in Switch::ALL {
        let one = none.on(s);
        assert!(Switch::ALL.into_iter().all(|t| one.is_on(t) == (t == s)));
        assert_eq!(one.on(s), one, "on is idempotent");
        assert_eq!(one.off(s), none);
        assert_eq!(Switches::complete().off(s).on(s), Switches::complete());
    }
    assert_eq!(
        [Switch::Identity, Switch::Schema, Switch::Trivial, Switch::LiteralValues]
            .into_iter()
            .fold(Switches::default(), Switches::on),
        Switches::complete()
    );
}

#[test]
fn each_family_is_registered_with_its_switch() {
    let registered: Vec<(&str, Option<Switch>)> = FAMILIES.iter().map(|f| (f.name, f.switch)).collect();
    assert_eq!(
        registered,
        [
            ("class", None),
            ("property", None),
            ("property-chains", Some(Switch::PropertyChains)),
            ("checks", Some(Switch::Checks)),
            ("datatype", Some(Switch::Checks)),
            ("equality", Some(Switch::Equality)),
            ("identity", Some(Switch::Identity)),
            ("schema", Some(Switch::Schema)),
            ("trivial", Some(Switch::Trivial)),
            ("literal-values", Some(Switch::LiteralValues)),
        ]
    );
    for s in Switch::ALL {
        assert!(FAMILIES.iter().any(|f| f.switch == Some(s)), "{s:?} has no family");
    }
}

/// An input for every switch version 1 had rules for: the core, equality,
/// property chains and checks, each feeding the others. Its list is
/// written out with labelled blank nodes, which load with the same label in
/// every reasoner (Turtle's `( ... )` gets random ones).
const INPUT: &str = "
    ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .
    ex:uncle owl:propertyChainAxiom _:l1 .
    _:l1 rdf:first ex:parent ; rdf:rest _:l2 . _:l2 rdf:first ex:brother ; rdf:rest rdf:nil .
    ex:kid ex:parent ex:dad . ex:dad ex:brother ex:bob . ex:dad ex:brother ex:bert .
    ex:uncle a owl:FunctionalProperty ; rdfs:domain ex:Nephew .
    ex:bob ex:owns ex:boat . ex:bert a ex:Plant .
    ex:Animal owl:disjointWith ex:Plant . ex:bob a ex:Dog .
    ex:rex owl:sameAs ex:rexy . ex:rexy ex:age \"abc\"^^xsd:integer .
    ex:knows a owl:IrreflexiveProperty . ex:rexy ex:knows ex:rex .
";

/// Added after a run, to check the incremental runs too.
const MORE: &str = "ex:kid2 ex:parent ex:dad . ex:rexy a ex:Plant .";

/// Every configuration of the seven switches.
fn configurations() -> impl Iterator<Item = Switches> {
    (0..1u32 << Switch::ALL.len()).map(|bits| {
        Switch::ALL
            .into_iter()
            .enumerate()
            .fold(Switches::default(), |c, (i, s)| {
                if (bits >> i) & 1 == 1 { c.on(s) } else { c.off(s) }
            })
    })
}

#[test]
fn a_changed_configuration_gives_the_closure_of_a_fresh_reasoner() {
    let default = reasoned(Switches::default(), INPUT);
    let mut grown = reasoned(Switches::default(), INPUT);
    load(&mut grown, MORE);
    grown.reason();
    for switches in configurations() {
        let mut fresh = reasoned(switches, INPUT);
        let mut changed = reasoned(Switches::default(), INPUT);
        assert_eq!(view(&changed), view(&default));
        changed.set_switches(switches);
        assert_eq!(changed.switches(), switches);
        // The reads show the last run until the next one.
        assert_eq!(view(&changed), view(&default));
        let summary = changed.reason();
        let expected = Summary {
            rounds: summary.rounds,
            ..fresh.reason()
        };
        assert_eq!(summary, expected, "{switches:?}");
        assert_eq!(view(&changed), view(&fresh), "{switches:?}");
        if switches != Switches::default() {
            assert!(summary.rounds > 0, "a change runs in full: {switches:?}");
        }

        // Then an incremental run, in both.
        for r in [&mut changed, &mut fresh] {
            load(r, MORE);
            r.reason();
        }
        assert_eq!(view(&changed), view(&fresh), "{switches:?}, then more");

        // And back to the defaults: the closure of a reasoner that never changed.
        changed.set_switches(Switches::default());
        changed.reason();
        assert_eq!(view(&changed), view(&grown), "{switches:?}, then back");
    }
}

#[test]
fn the_same_switches_again_change_nothing() {
    let mut r = reasoned(Switches::default(), INPUT);
    r.set_switches(Switches::default());
    assert_eq!(r.reason().rounds, 0);
    let fast = Switches::default().off(Switch::Equality);
    r.set_switches(fast);
    assert!(r.reason().rounds > 0);
    r.set_switches(fast);
    assert_eq!(r.reason().rounds, 0);
    // `clear` keeps the switches.
    r.clear();
    assert_eq!(r.switches(), fast);
}

/// The inferred triples of `turtle` with the default switches and with
/// `switch` off: the first holds the second, and the difference is exactly
/// `removed` (in `expand`'s notation).
fn removes_exactly(switch: Switch, turtle: &str, removed: &str) -> (Reasoner, Reasoner) {
    let on = reasoned(Switches::default(), turtle);
    let off = reasoned(Switches::default().off(switch), turtle);
    let (with, without) = (lines(on.inferred()), lines(off.inferred()));
    assert!(
        with.is_superset(&without),
        "{switch:?} off adds {:#?}",
        &without - &with
    );
    assert_eq!(&with - &without, expand(removed), "{switch:?}");
    // The switch's families ran with it on, and not with it off.
    for family in FAMILIES.iter().filter(|f| f.switch == Some(switch)) {
        assert!(on.counters().families.contains(&family.name));
        assert!(!off.counters().families.contains(&family.name));
    }
    (on, off)
}

#[test]
fn equality_off_removes_exactly_the_eq_rules_consequences() {
    let (on, off) = removes_exactly(
        Switch::Equality,
        "ex:a owl:sameAs ex:b . ex:a ex:p ex:c . ex:b a ex:C . ex:C rdfs:subClassOf ex:D .
         ex:p a owl:FunctionalProperty . ex:a ex:p ex:d .",
        // eq-sym, eq-trans and eq-rep-s over a = b, and the class rules
        // over what they give. prp-fp's `owl:sameAs` among c and d is
        // derived either way (the core).
        "ex:a owl:sameAs ex:a
         ex:b owl:sameAs ex:a
         ex:b owl:sameAs ex:b
         ex:b ex:p ex:c
         ex:b ex:p ex:d
         ex:a a ex:C
         ex:a a ex:D",
    );
    assert!(on.store().canon().merges() > 0);
    assert_eq!(off.store().canon().merges(), 0);
}

#[test]
fn property_chains_off_removes_exactly_prp_spo2s_consequences() {
    removes_exactly(
        Switch::PropertyChains,
        "ex:uncle owl:propertyChainAxiom ( ex:parent ex:brother ) .
         ex:kid ex:parent ex:dad . ex:dad ex:brother ex:bob .
         ex:uncle rdfs:domain ex:Nephew ; rdfs:subPropertyOf ex:relative .
         ex:dad ex:parent ex:gran . ex:parent rdfs:subPropertyOf ex:relative .",
        "ex:kid ex:uncle ex:bob
         ex:kid ex:relative ex:bob
         ex:kid a ex:Nephew",
    );
}

#[test]
fn checks_off_removes_exactly_the_findings() {
    let input = "
        ex:A owl:disjointWith ex:B . ex:x a ex:A , ex:B .
        ex:C owl:complementOf ex:D . ex:x a ex:C , ex:D .
        ex:y a owl:Nothing .
        ex:p owl:propertyDisjointWith ex:q . ex:x ex:p ex:y ; ex:q ex:y .
        ex:asym a owl:AsymmetricProperty . ex:x ex:asym ex:y . ex:y ex:asym ex:x .
        ex:irr a owl:IrreflexiveProperty . ex:x ex:irr ex:x .
        ex:x ex:age \"abc\"^^xsd:integer .
    ";
    let (on, off) = removes_exactly(Switch::Checks, input, "");
    assert_eq!(
        codes(&on),
        BTreeSet::from([
            "cax-dw",
            "cls-com",
            "cls-nothing2",
            "prp-pdw",
            "prp-asyp",
            "prp-irp",
            "dt-not-type"
        ])
    );
    assert!(off.diagnostics().is_empty());
    assert_eq!(lines(on.triples()), lines(off.triples()));
}

/// [`INPUT`] makes the switches feed each other, so that the
/// configurations differ: chains make ex:bob and ex:bert the same (through
/// prp-fp), which makes a cax-dw finding; ex:rex = ex:rexy makes a prp-irp one.
#[test]
fn the_input_reaches_across_the_switches() {
    let found = |switches| codes(&reasoned(switches, INPUT));
    let all = BTreeSet::from(["cax-dw", "dt-not-type", "prp-irp"]);
    assert_eq!(found(Switches::default()), all);
    assert_eq!(
        found(Switches::default().off(Switch::PropertyChains)),
        BTreeSet::from(["dt-not-type", "prp-irp"])
    );
    assert_eq!(
        found(Switches::default().off(Switch::Equality)),
        BTreeSet::from(["dt-not-type"])
    );
    assert!(found(Switches::default().off(Switch::Checks)).is_empty());
}
