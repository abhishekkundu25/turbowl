//! The equality family through the reasoner: every read must give the
//! closure of the W3C eq-* rules (as the reference evaluator and Nemo
//! compute it), with each class stored once.

use crate::rules::testing::{self, expand};
use crate::term::vocab;
use crate::{Format, Reasoner, Summary};
use oxrdf::Triple;
use std::collections::BTreeSet;

const PREFIXES: &str = "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex: <http://example.org/> .\n";

fn reasoner(turtle: &str) -> Reasoner {
    let mut r = Reasoner::new();
    r.load_str(&format!("{PREFIXES}{turtle}"), Format::Turtle, None)
        .unwrap_or_else(|e| panic!("{e}"));
    r
}

fn parse(turtle: &str) -> Vec<Triple> {
    oxttl::TurtleParser::new()
        .for_slice(format!("{PREFIXES}{turtle}").as_bytes())
        .collect::<Result<_, _>>()
        .unwrap()
}

fn lines(triples: impl Iterator<Item = Triple>) -> BTreeSet<String> {
    triples.map(|t| t.to_string()).collect()
}

fn one(short: &str) -> String {
    expand(short).pop_first().expect("one line")
}

/// The diagnostics as `code t1 t2 ...`, the harness's text form.
fn findings(r: &Reasoner) -> BTreeSet<String> {
    r.diagnostics()
        .iter()
        .map(|d| {
            let terms: Vec<String> = d.terms.iter().map(ToString::to_string).collect();
            format!("{} {}", d.code, terms.join(" "))
        })
        .collect()
}

/// Runs, and checks that the summary counts what the reads give.
fn reason(r: &mut Reasoner) -> Summary {
    let s = r.reason();
    let all = lines(r.triples());
    assert_eq!(s.base + s.inferred, all.len(), "{s:?}");
    assert_eq!(r.triples().count(), all.len(), "each triple is read once");
    assert_eq!(s.inferred, r.inferred().count());
    assert_eq!(s.diagnostics, r.diagnostics().len());
    let mut out = Vec::new();
    r.write(&mut out, Format::NTriples, false).unwrap();
    assert_eq!(String::from_utf8(out).unwrap().lines().count(), all.len());
    s
}

fn closure(turtle: &str) -> BTreeSet<String> {
    let mut r = reasoner(turtle);
    reason(&mut r);
    lines(r.triples())
}

/// Every `owl:sameAs` triple among `members` (short forms).
fn clique(members: &[&str]) -> String {
    let mut out = String::new();
    for a in members {
        for b in members {
            out.push_str(&format!("{a} owl:sameAs {b}\n"));
        }
    }
    out
}

fn assert_contains(all: &BTreeSet<String>, wanted: &str) {
    let missing: Vec<String> = expand(wanted).into_iter().filter(|t| !all.contains(t)).collect();
    assert!(missing.is_empty(), "missing {missing:#?}\nin {all:#?}");
}

fn ex(r: &Reasoner, local: &str) -> u32 {
    r.terms().get_iri(&format!("http://example.org/{local}")).unwrap().0
}

#[test]
fn eq_sym_and_eq_trans_give_every_pair_of_a_class() {
    let mut r = reasoner("ex:x owl:sameAs ex:y . ex:y owl:sameAs ex:z .");
    reason(&mut r);
    let same: BTreeSet<String> = lines(r.triples())
        .into_iter()
        .filter(|t| t.contains("sameAs"))
        .collect();
    assert_eq!(same, expand(&clique(&["ex:x", "ex:y", "ex:z"])));
    // Stored once: one class, one `r owl:sameAs r`.
    let canon = r.store().canon();
    assert_eq!(canon.members(canon.find(ex(&r, "x"))).len(), 3);
    assert_eq!(r.store().table(vocab::OWL_SAME_AS.0).unwrap().by_subject().len(), 1);
}

#[test]
fn eq_rep_s_p_o_hold_for_every_member() {
    let all = closure("ex:a owl:sameAs ex:b . ex:p owl:sameAs ex:q . ex:o owl:sameAs ex:o2 . ex:a ex:p ex:o .");
    let mut wanted = String::new();
    for s in ["ex:a", "ex:b"] {
        for p in ["ex:p", "ex:q"] {
            for o in ["ex:o", "ex:o2"] {
                wanted.push_str(&format!("{s} {p} {o}\n"));
            }
        }
    }
    assert_contains(&all, &wanted);
    assert_eq!(all.len(), 8 + 3 * 4 + 2, "the triples, three cliques of two, the facts");
}

#[test]
fn a_term_is_the_same_as_itself_only_where_a_triple_says_so() {
    // No eq-ref: ex:a is in no `owl:sameAs` triple.
    let all = closure("ex:a ex:p ex:b . ex:x owl:sameAs ex:x .");
    assert!(all.contains(&one("ex:x owl:sameAs ex:x")));
    assert_eq!(all.len(), 2 + 2, "{all:#?}");
}

#[test]
fn a_term_the_same_as_rdf_type_types() {
    let mut r = reasoner("ex:t owl:sameAs rdf:type . ex:a ex:t ex:C . ex:C rdfs:subClassOf ex:D .");
    reason(&mut r);
    assert_contains(
        &lines(r.triples()),
        "ex:a rdf:type ex:C\nex:a rdf:type ex:D\nex:a ex:t ex:D\nrdf:type owl:sameAs ex:t\nex:t owl:sameAs ex:t",
    );
    // The vocabulary term represents its class: no restart was needed.
    assert_eq!(r.store().canon().find(ex(&r, "t")), vocab::RDF_TYPE.0);
}

#[test]
fn two_vocabulary_terms_made_equal_restart_the_run() {
    let mut r = reasoner("rdfs:subClassOf owl:sameAs owl:equivalentClass . ex:A rdfs:subClassOf ex:B . ex:x a ex:B .");
    reason(&mut r);
    // ex:A owl:equivalentClass ex:B, and cax-eqc2 types ex:x.
    assert_contains(
        &lines(r.triples()),
        "ex:A owl:equivalentClass ex:B\nex:B rdfs:subClassOf ex:A\nex:x rdf:type ex:A\n\
         owl:equivalentClass owl:sameAs rdfs:subClassOf",
    );
    let low = vocab::RDFS_SUB_CLASS_OF.0.min(vocab::OWL_EQUIVALENT_CLASS.0);
    assert_eq!(r.store().canon().find(vocab::OWL_EQUIVALENT_CLASS.0), low);
    // Made equal by an insert: the incremental run restarts too.
    let mut r = reasoner("ex:A rdfs:subClassOf ex:B . ex:x a ex:B .");
    reason(&mut r);
    r.insert(
        parse("rdfs:subClassOf owl:sameAs owl:equivalentClass .")
            .iter()
            .map(Triple::as_ref),
    );
    reason(&mut r);
    assert!(lines(r.triples()).contains(&one("ex:x rdf:type ex:A")));
}

#[test]
fn a_cell_the_same_as_rdf_nil_reads_on() {
    // ex:n is equal to rdf:nil without being it, so it can be a later cell
    // (the W3C rules test `?r != rdf:nil`): ex:l, then ex:n, whose first is
    // rdf:nil's. Members ex:A and ex:B, and a chain of two.
    let all = closure(
        "ex:C owl:intersectionOf ex:l . ex:l rdf:first ex:A ; rdf:rest rdf:nil .
         rdf:nil rdf:first ex:B ; rdf:rest rdf:nil . ex:n owl:sameAs rdf:nil . ex:y a ex:C .
         ex:p owl:propertyChainAxiom ex:l . ex:u ex:A ex:v . ex:v ex:B ex:w .",
    );
    assert_contains(
        &all,
        "ex:y rdf:type ex:A\nex:y rdf:type ex:B\nex:u ex:p ex:v\nex:u ex:p ex:w",
    );
}

#[test]
fn rdf_nil_alone_ends_a_list() {
    let all = closure(
        "ex:C owl:intersectionOf ex:l . ex:l rdf:first ex:A ; rdf:rest rdf:nil .
         rdf:nil rdf:first ex:B ; rdf:rest rdf:nil . ex:y a ex:C .",
    );
    assert!(all.contains(&one("ex:y rdf:type ex:A")));
    assert!(!all.contains(&one("ex:y rdf:type ex:B")));
}

#[test]
fn a_merged_away_predicate_s_table_moves() {
    let mut r = reasoner("ex:p owl:sameAs ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d . ex:q rdfs:domain ex:C .");
    reason(&mut r);
    assert_contains(
        &lines(r.triples()),
        "ex:a rdf:type ex:C\nex:c rdf:type ex:C\nex:a ex:q ex:b\nex:c ex:p ex:d\nex:p rdfs:domain ex:C",
    );
    // One of the two tables holds both pairs; the other is empty.
    let len = |p| r.store().table(p).map_or(0, |t| t.by_subject().len());
    let (p, q) = (len(ex(&r, "p")), len(ex(&r, "q")));
    assert_eq!((p.min(q), p.max(q)), (0, 2));
}

#[test]
fn literals_made_equal_share_their_triples() {
    // The reference evaluator's example: "a" and "b" made equal by prp-fp.
    let mut r =
        reasoner("ex:d a owl:FunctionalProperty . ex:x ex:d \"a\" , \"b\" . ex:y ex:q \"a\" . ex:i owl:sameAs \"c\" .");
    reason(&mut r);
    let all = lines(r.triples());
    assert_contains(&all, "ex:y ex:q \"b\"\nex:i owl:sameAs \"c\"\nex:i owl:sameAs ex:i");
    // Triples with a literal subject (`"c" owl:sameAs ex:i`) stay inside.
    assert!(!all.iter().any(|t| t.starts_with('"')), "{all:#?}");
}

#[test]
fn findings_name_every_member() {
    let mut r = reasoner(
        "ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:y . ex:x owl:sameAs ex:y .
         ex:A owl:disjointWith ex:B . ex:i a ex:A , ex:B . ex:i owl:sameAs ex:j .",
    );
    reason(&mut r);
    assert_eq!(
        findings(&r),
        expand("prp-irp ex:p ex:x\nprp-irp ex:p ex:y\ncax-dw ex:i ex:A ex:B\ncax-dw ex:j ex:A ex:B")
    );
}

#[test]
fn dt_not_type_checks_each_literal_of_a_typed_class() {
    // The typing first, then the merge: "abc" joins ex:o's class.
    let mut r = reasoner("ex:p rdfs:range xsd:integer . ex:s ex:p ex:o . ex:o owl:sameAs \"abc\" .");
    reason(&mut r);
    assert_eq!(findings(&r), expand("dt-not-type \"abc\" xsd:integer"));
}

/// Chaos, reduced (incremental seed 0x5EED_0105, iteration 8339 of 20,000):
/// the merge of "abc" and "3.5" rewrote only tuples the store already held,
/// so no table changed, and the round in which the datatype family checks
/// the moved literal against its class's typings never ran.
#[test]
fn a_literal_joining_a_typed_class_is_checked_when_nothing_else_changes() {
    let mut r = reasoner(
        "ex:d1 a owl:FunctionalProperty . ex:x ex:d0 \"3.5\"^^xsd:decimal . ex:x ex:d1 \"abc\"^^xsd:integer .",
    );
    reason(&mut r);
    r.insert(
        parse("ex:d0 rdfs:range xsd:string . ex:d0 rdfs:subPropertyOf ex:d1 .")
            .iter()
            .map(Triple::as_ref),
    );
    reason(&mut r);
    assert_eq!(
        findings(&r),
        expand(
            "dt-not-type \"abc\"^^xsd:integer xsd:integer\ndt-not-type \"abc\"^^xsd:integer xsd:string\n\
             dt-not-type \"3.5\"^^xsd:decimal xsd:string"
        )
    );
}

#[test]
fn incremental_runs_after_merges_equal_one_run() {
    const INPUT: &str = "
        ex:a owl:sameAs ex:b . ex:b ex:p ex:c . ex:p rdfs:domain ex:C . ex:C rdfs:subClassOf ex:D .
        ex:f a owl:FunctionalProperty . ex:a ex:f ex:o1 . ex:b ex:f ex:o2 . ex:o2 a ex:E .
        ex:t owl:sameAs rdf:type . ex:z ex:t ex:E . ex:E owl:disjointWith ex:D . ex:o1 a ex:D .
        rdfs:subPropertyOf owl:sameAs owl:equivalentProperty . ex:q rdfs:subPropertyOf ex:p . ex:k ex:p ex:m .";
    let triples = parse(INPUT);
    let mut whole = Reasoner::new();
    whole.insert(triples.iter().map(Triple::as_ref));
    let summary = reason(&mut whole);
    let expected = (lines(whole.triples()), findings(&whole));
    assert!(summary.diagnostics > 0, "{summary:?}");
    assert!(
        expected.0.contains(&one("ex:k ex:q ex:m")),
        "prp-eqp2 through the merged constant"
    );
    for batches in 1..=5 {
        for offset in 0..batches {
            let mut r = Reasoner::new();
            for b in 0..batches {
                let batch = triples.iter().enumerate().filter(|(i, _)| (i + offset) % batches == b);
                r.insert(batch.map(|(_, t)| t.as_ref()));
                reason(&mut r);
            }
            let got = (lines(r.triples()), findings(&r));
            assert_eq!(got, expected, "{batches} batches, offset {offset}");
        }
    }
}

#[test]
fn a_removal_splits_a_class() {
    let mut r = reasoner("ex:a owl:sameAs ex:b . ex:b owl:sameAs ex:c . ex:a ex:p ex:v .");
    reason(&mut r);
    assert!(lines(r.triples()).contains(&one("ex:c ex:p ex:v")));
    r.remove(parse("ex:b owl:sameAs ex:c .").iter().map(Triple::as_ref));
    let s = reason(&mut r);
    let all = lines(r.triples());
    assert_contains(&all, &clique(&["ex:a", "ex:b"]));
    assert!(!all.iter().any(|t| t.contains("example.org/c")), "{all:#?}");
    assert_eq!(s.base, 2);
    // Put back: the clique of three again, incrementally.
    r.insert(parse("ex:b owl:sameAs ex:c .").iter().map(Triple::as_ref));
    reason(&mut r);
    assert_contains(&lines(r.triples()), &clique(&["ex:a", "ex:b", "ex:c"]));
}

#[test]
fn the_harness_expands_as_the_reasoner_does() {
    let run = testing::run_with(crate::Switches::default(), "ex:a owl:sameAs ex:b . ex:a ex:p ex:c .");
    assert!(run.has("ex:b ex:p ex:c"));
    assert!(run.has("ex:b owl:sameAs ex:a"));
}
