//! The class family's tests: one minimal input per rule, the list cases,
//! generalized triples, incremental runs, a mixed input whose closure is the
//! reference evaluator's, and the scaling test. Expectations were checked
//! against `turtle_reasoner_reference::evaluate` on the same inputs.

use crate::rules::testing::{self, Run, expand};

/// The heads of cls-thing and cls-nothing1, inferred by every run.
const FACTS: &str = "owl:Thing a owl:Class
                     owl:Nothing a owl:Class";

fn run(turtle: &str) -> Run {
    testing::run(&["class"], turtle)
}

/// Asserts that the class family infers exactly `expected` (and the facts)
/// from `turtle`.
#[track_caller]
fn infers(turtle: &str, expected: &str) {
    let run = run(turtle);
    let mut want = expand(expected);
    want.extend(expand(FACTS));
    assert_eq!(run.inferred(), want, "input:\n{turtle}");
}

// ---------------------------------------------------------------------------
// One minimal input per rule
// ---------------------------------------------------------------------------

#[test]
fn cls_thing_and_cls_nothing1_hold_for_an_empty_base() {
    infers("", "");
}

#[test]
fn cax_sco() {
    infers("ex:A rdfs:subClassOf ex:B . ex:x a ex:A . ex:y a ex:B .", "ex:x a ex:B");
}

#[test]
fn cax_eqc1() {
    // scm-eqc1 and scm-sco add the subclass pairs.
    infers(
        "ex:A owl:equivalentClass ex:B . ex:x a ex:A .",
        "ex:x a ex:B
         ex:A rdfs:subClassOf ex:B
         ex:B rdfs:subClassOf ex:A
         ex:A rdfs:subClassOf ex:A
         ex:B rdfs:subClassOf ex:B",
    );
}

#[test]
fn cax_eqc2() {
    infers(
        "ex:A owl:equivalentClass ex:B . ex:x a ex:B .",
        "ex:x a ex:A
         ex:A rdfs:subClassOf ex:B
         ex:B rdfs:subClassOf ex:A
         ex:A rdfs:subClassOf ex:A
         ex:B rdfs:subClassOf ex:B",
    );
}

#[test]
fn scm_sco() {
    infers(
        "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C . ex:C rdfs:subClassOf ex:D .",
        "ex:A rdfs:subClassOf ex:C
         ex:A rdfs:subClassOf ex:D
         ex:B rdfs:subClassOf ex:D",
    );
}

#[test]
fn scm_sco_on_a_cycle_closes_it_reflexively() {
    infers(
        "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
        "ex:A rdfs:subClassOf ex:A
         ex:B rdfs:subClassOf ex:B",
    );
}

#[test]
fn scm_eqc1() {
    infers(
        "ex:A owl:equivalentClass ex:B .",
        "ex:A rdfs:subClassOf ex:B
         ex:B rdfs:subClassOf ex:A
         ex:A rdfs:subClassOf ex:A
         ex:B rdfs:subClassOf ex:B",
    );
}

#[test]
fn cls_int1() {
    infers(
        "ex:C owl:intersectionOf ( ex:A ex:B ) .
         ex:x a ex:A , ex:B .
         ex:y a ex:A .
         ex:z a ex:B .",
        "ex:x a ex:C",
    );
}

#[test]
fn cls_int2() {
    infers(
        "ex:C owl:intersectionOf ( ex:A ex:B ) . ex:x a ex:C .",
        "ex:x a ex:A
         ex:x a ex:B",
    );
}

#[test]
fn cls_uni() {
    infers(
        "ex:C owl:unionOf ( ex:A ex:B ) . ex:x a ex:A . ex:y a ex:B . ex:z a ex:C .",
        "ex:x a ex:C
         ex:y a ex:C",
    );
}

#[test]
fn cls_svf1() {
    infers(
        "ex:R owl:someValuesFrom ex:B ; owl:onProperty ex:p .
         ex:u ex:p ex:v . ex:v a ex:B .
         ex:w ex:p ex:z .",
        "ex:u a ex:R",
    );
}

#[test]
fn cls_svf2_with_owl_thing() {
    // No typing of z is needed: the filler is owl:Thing.
    infers(
        "ex:R owl:someValuesFrom owl:Thing ; owl:onProperty ex:p .
         ex:u ex:p ex:v . ex:w ex:p ex:z . ex:w ex:q ex:z .",
        "ex:u a ex:R
         ex:w a ex:R",
    );
}

#[test]
fn cls_avf() {
    infers(
        "ex:R owl:allValuesFrom ex:B ; owl:onProperty ex:p .
         ex:u a ex:R ; ex:p ex:v ; ex:q ex:o .
         ex:w ex:p ex:z .",
        "ex:v a ex:B",
    );
}

#[test]
fn cls_hv1() {
    infers(
        "ex:R owl:hasValue ex:v ; owl:onProperty ex:p . ex:u a ex:R .",
        "ex:u ex:p ex:v",
    );
}

#[test]
fn cls_hv2() {
    infers(
        "ex:R owl:hasValue ex:v ; owl:onProperty ex:p . ex:u ex:p ex:v . ex:w ex:p ex:z .",
        "ex:u a ex:R",
    );
}

#[test]
fn restrictions_may_be_blank_nodes() {
    infers(
        "ex:C rdfs:subClassOf _:r . _:r owl:someValuesFrom ex:B ; owl:onProperty ex:p .
         ex:x a ex:C . ex:u ex:p ex:v . ex:v a ex:B .",
        "ex:x a _:r
         ex:u a _:r",
    );
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

#[test]
fn a_class_with_two_intersection_lists_reads_both() {
    infers(
        "ex:C owl:intersectionOf ( ex:A ex:B ) , ( ex:D ex:E ) .
         ex:x a ex:A , ex:B .
         ex:y a ex:D , ex:E .
         ex:z a ex:A , ex:E .",
        "ex:x a ex:C
         ex:y a ex:C
         ex:x a ex:D
         ex:x a ex:E
         ex:y a ex:A
         ex:y a ex:B",
    );
}

#[test]
fn a_class_with_two_union_lists_reads_both() {
    infers(
        "ex:C owl:unionOf ( ex:A ex:B ) , ( ex:D ) . ex:x a ex:B . ex:y a ex:D .",
        "ex:x a ex:C
         ex:y a ex:C",
    );
}

#[test]
fn two_lists_sharing_a_tail() {
    // (A D) and (B D), the cell holding D shared.
    infers(
        "ex:C1 owl:intersectionOf _:a . _:a rdf:first ex:A ; rdf:rest _:t .
         ex:C2 owl:intersectionOf _:b . _:b rdf:first ex:B ; rdf:rest _:t .
         _:t rdf:first ex:D ; rdf:rest rdf:nil .
         ex:U owl:unionOf _:b .
         ex:x a ex:A , ex:D .
         ex:y a ex:B , ex:D .
         ex:z a ex:D .
         ex:w a ex:C1 .",
        "ex:x a ex:C1
         ex:y a ex:C2
         ex:w a ex:A
         ex:w a ex:D
         ex:w a ex:U
         ex:x a ex:U
         ex:y a ex:U
         ex:z a ex:U",
    );
}

#[test]
fn a_cyclic_list_matches_nothing() {
    infers(
        "ex:C owl:intersectionOf _:l1 . ex:U owl:unionOf _:l1 .
         _:l1 rdf:first ex:A ; rdf:rest _:l2 .
         _:l2 rdf:first ex:B ; rdf:rest _:l1 .
         ex:x a ex:A , ex:B . ex:y a ex:C .",
        "",
    );
}

#[test]
fn a_cycle_with_an_exit_to_nil_is_a_list() {
    // _:l2 rests on both _:l1 and rdf:nil: the path l1 l2 nil reads (A B).
    infers(
        "ex:C owl:intersectionOf _:l1 .
         _:l1 rdf:first ex:A ; rdf:rest _:l2 .
         _:l2 rdf:first ex:B ; rdf:rest _:l1 , rdf:nil .
         ex:x a ex:A , ex:B . ex:y a ex:C . ex:z a ex:A .",
        "ex:x a ex:C
         ex:y a ex:A
         ex:y a ex:B",
    );
}

#[test]
fn a_cell_with_two_members_needs_one_of_them() {
    // cls-int1 needs B or A at _:l1; cls-int2 then gives x both.
    infers(
        "ex:C owl:intersectionOf _:l1 .
         _:l1 rdf:first ex:A , ex:B ; rdf:rest _:l2 .
         _:l2 rdf:first ex:D ; rdf:rest rdf:nil .
         ex:x a ex:B , ex:D . ex:y a ex:A . ex:w a ex:C .",
        "ex:x a ex:C
         ex:x a ex:A
         ex:w a ex:A
         ex:w a ex:B
         ex:w a ex:D",
    );
}

#[test]
fn cls_int1_draws_candidates_from_either_side() {
    // A has many instances, B one: the first run draws candidates from B's
    // instances (the pivot); the later additions are few, so they are drawn
    // from the recent instances of the members.
    let mut run = run("ex:C owl:intersectionOf ( ex:A ex:B ) .
         ex:a1 a ex:A , ex:B . ex:a2 a ex:A . ex:a3 a ex:A . ex:a4 a ex:A . ex:a5 a ex:A .");
    assert!(run.has("ex:a1 a ex:C"));
    assert!(!run.has("ex:a2 a ex:C"));
    run.add("ex:a2 a ex:B .");
    run.add("ex:n a ex:B . ex:m a ex:A .");
    run.add("ex:n a ex:A .");
    let mut want = expand(
        "ex:a1 a ex:C
         ex:a2 a ex:C
         ex:n a ex:C",
    );
    want.extend(expand(FACTS));
    assert_eq!(run.inferred(), want);
}

#[test]
fn two_rest_paths_need_only_one_typed_throughout() {
    // From _:l1, (A B) by _:l2 and (A D) by _:l3. x has (A D), so cls-int1
    // holds, and cls-int2 then gives it every member, B too.
    infers(
        "ex:C owl:intersectionOf _:l1 .
         _:l1 rdf:first ex:A ; rdf:rest _:l2 , _:l3 .
         _:l2 rdf:first ex:B ; rdf:rest rdf:nil .
         _:l3 rdf:first ex:D ; rdf:rest rdf:nil .
         ex:x a ex:A , ex:D . ex:y a ex:B , ex:D .",
        "ex:x a ex:C
         ex:x a ex:B",
    );
}

#[test]
fn a_cell_off_the_path_to_nil_is_no_member() {
    // _:l2 leads nowhere: its member B is not read; _:l3 ends the list.
    infers(
        "ex:C owl:intersectionOf _:l1 . ex:U owl:unionOf _:l1 .
         _:l1 rdf:first ex:A ; rdf:rest _:l2 , _:l3 .
         _:l2 rdf:first ex:B .
         _:l3 rdf:first ex:D ; rdf:rest rdf:nil .
         ex:x a ex:A , ex:D . ex:y a ex:C . ex:z a ex:B .",
        "ex:x a ex:C
         ex:x a ex:U
         ex:y a ex:A
         ex:y a ex:D
         ex:y a ex:U",
    );
}

#[test]
fn the_empty_list_has_no_members() {
    infers(
        "ex:C owl:intersectionOf rdf:nil . ex:U owl:unionOf () .
         ex:x a ex:C . ex:y a ex:U .",
        "",
    );
}

#[test]
fn a_list_derived_during_the_run_counts() {
    // cls-hv1 derives the intersection pair, the list cells' rdf:first and
    // rdf:rest, and so the list (A B) of ex:C, one round at a time.
    infers(
        "ex:Head owl:hasValue ex:l1 ; owl:onProperty owl:intersectionOf .
         ex:F1 owl:hasValue ex:A ; owl:onProperty rdf:first .
         ex:F2 owl:hasValue ex:B ; owl:onProperty rdf:first .
         ex:R1 owl:hasValue ex:l2 ; owl:onProperty rdf:rest .
         ex:R2 owl:hasValue rdf:nil ; owl:onProperty rdf:rest .
         ex:C a ex:Head . ex:l1 a ex:F1 , ex:R1 . ex:l2 a ex:F2 , ex:R2 .
         ex:x a ex:A , ex:B . ex:y a ex:C . ex:z a ex:A .",
        "ex:C owl:intersectionOf ex:l1
         ex:l1 rdf:first ex:A
         ex:l1 rdf:rest ex:l2
         ex:l2 rdf:first ex:B
         ex:l2 rdf:rest rdf:nil
         ex:x a ex:C
         ex:y a ex:A
         ex:y a ex:B",
    );
}

#[test]
fn a_union_list_cell_derived_later_adds_its_member() {
    // The list (A) of ex:U gets a second cell, holding B, from cls-hv1 in the
    // second round, after the union has been read once.
    infers(
        "ex:U owl:unionOf ex:l1 . ex:l1 rdf:first ex:A ; rdf:rest ex:l2 .
         ex:l2 rdf:first ex:B .
         ex:End owl:hasValue rdf:nil ; owl:onProperty rdf:rest .
         ex:l2 a ex:Pre . ex:Pre rdfs:subClassOf ex:End .
         ex:x a ex:A . ex:y a ex:B .",
        "ex:l2 a ex:End
         ex:l2 rdf:rest rdf:nil
         ex:x a ex:U
         ex:y a ex:U",
    );
}

// ---------------------------------------------------------------------------
// Restrictions
// ---------------------------------------------------------------------------

#[test]
fn svf_avf_and_hv_on_the_same_property() {
    infers(
        "ex:S owl:someValuesFrom ex:B ; owl:onProperty ex:p .
         ex:V owl:allValuesFrom ex:D ; owl:onProperty ex:p .
         ex:H owl:hasValue ex:o ; owl:onProperty ex:p .
         ex:u a ex:V , ex:H ; ex:p ex:v . ex:v a ex:B .
         ex:w ex:p ex:o .",
        "ex:u a ex:S
         ex:u ex:p ex:o
         ex:v a ex:D
         ex:o a ex:D
         ex:w a ex:H",
    );
}

#[test]
fn restrictions_feed_each_other() {
    // hv1 gives u p o, svf1 then types u, cax-sco carries it up, avf types o.
    infers(
        "ex:H owl:hasValue ex:o ; owl:onProperty ex:p .
         ex:S owl:someValuesFrom ex:B ; owl:onProperty ex:p .
         ex:S rdfs:subClassOf ex:V .
         ex:V owl:allValuesFrom ex:D ; owl:onProperty ex:p .
         ex:o a ex:B . ex:u a ex:H .",
        "ex:u ex:p ex:o
         ex:u a ex:S
         ex:u a ex:V
         ex:o a ex:D",
    );
}

#[test]
fn a_restriction_with_two_properties_reads_both() {
    infers(
        "ex:R owl:someValuesFrom owl:Thing ; owl:onProperty ex:p , ex:q .
         ex:u ex:p ex:v . ex:w ex:q ex:z .",
        "ex:u a ex:R
         ex:w a ex:R",
    );
}

// ---------------------------------------------------------------------------
// Generalized triples
// ---------------------------------------------------------------------------

#[test]
fn cls_avf_types_a_literal_and_rules_join_on_it() {
    infers(
        "ex:R owl:allValuesFrom ex:B ; owl:onProperty ex:p .
         ex:B rdfs:subClassOf ex:D .
         ex:u a ex:R ; ex:p \"5\"^^xsd:integer .",
        "\"5\"^^xsd:integer a ex:B
         \"5\"^^xsd:integer a ex:D",
    );
}

#[test]
fn cls_hv1_derives_a_triple_whose_predicate_is_a_literal() {
    infers(
        "ex:R owl:hasValue ex:v ; owl:onProperty \"p\" . ex:u a ex:R .",
        "ex:u \"p\" ex:v",
    );
}

#[test]
fn a_literal_subject_inserted_as_a_generalized_triple_is_classified() {
    let mut run = run("ex:A rdfs:subClassOf ex:B . ex:x ex:p \"5\"^^xsd:integer . ex:y a ex:A .");
    let five = run
        .dict
        .get_term(
            oxrdf::LiteralRef::new_typed_literal("5", oxrdf::vocab::xsd::INTEGER).into(),
            crate::term::DocId::NONE,
        )
        .expect("interned")
        .0;
    let a = run.dict.get_iri("http://example.org/A").expect("interned").0;
    run.store.insert(five, crate::term::vocab::RDF_TYPE.0, a);
    run.add("");
    assert!(run.has("\"5\"^^xsd:integer a ex:B"));
}

// ---------------------------------------------------------------------------
// Incremental runs
// ---------------------------------------------------------------------------

/// A mixed input touching every rule, written with IRIs only so that it can
/// be split across documents (blank nodes are scoped per document).
const MIXED: &[&str] = &[
    "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C . ex:C owl:equivalentClass ex:E .",
    "ex:I owl:intersectionOf ex:i1 . ex:i1 rdf:first ex:B ; rdf:rest ex:i2 .",
    "ex:i2 rdf:first ex:K ; rdf:rest rdf:nil .",
    "ex:N owl:unionOf ex:n1 . ex:n1 rdf:first ex:K ; rdf:rest ex:n2 . ex:n2 rdf:first ex:E ; rdf:rest rdf:nil .",
    "ex:S owl:someValuesFrom ex:C ; owl:onProperty ex:p .",
    "ex:T owl:someValuesFrom owl:Thing ; owl:onProperty ex:q .",
    "ex:V owl:allValuesFrom ex:K ; owl:onProperty ex:p .",
    "ex:H owl:hasValue ex:o ; owl:onProperty ex:q . ex:S rdfs:subClassOf ex:V .",
    "ex:x a ex:A ; ex:p ex:y . ex:y a ex:A .",
    "ex:z a ex:I . ex:w ex:q ex:o . ex:k a ex:H .",
];

/// The class family's inferences from `MIXED`, as the reference evaluator
/// computes them.
const MIXED_INFERRED: &str = "
    ex:A rdfs:subClassOf ex:C
    ex:A rdfs:subClassOf ex:E
    ex:B rdfs:subClassOf ex:E
    ex:C rdfs:subClassOf ex:E
    ex:E rdfs:subClassOf ex:C
    ex:C rdfs:subClassOf ex:C
    ex:E rdfs:subClassOf ex:E
    ex:x a ex:B
    ex:x a ex:C
    ex:x a ex:E
    ex:x a ex:N
    ex:x a ex:S
    ex:x a ex:V
    ex:y a ex:B
    ex:y a ex:C
    ex:y a ex:E
    ex:y a ex:N
    ex:y a ex:K
    ex:y a ex:I
    ex:z a ex:B
    ex:z a ex:K
    ex:z a ex:C
    ex:z a ex:E
    ex:z a ex:N
    ex:w a ex:T
    ex:w a ex:H
    ex:k ex:q ex:o
    ex:k a ex:T
";

#[test]
fn the_mixed_input_gives_the_reference_closure() {
    infers(&MIXED.concat(), MIXED_INFERRED);
}

#[test]
fn split_inputs_give_the_closure_of_one_run() {
    let whole = run(&MIXED.concat()).closure();
    // Every part in order, in reverse order, and data before schema.
    let orders: [Vec<usize>; 3] = [
        (0..MIXED.len()).collect(),
        (0..MIXED.len()).rev().collect(),
        vec![8, 9, 4, 5, 6, 7, 0, 3, 2, 1],
    ];
    for order in orders {
        let mut run = run("");
        for &i in &order {
            run.add(MIXED[i]);
        }
        assert_eq!(run.closure(), whole, "order {order:?}");
    }
}

#[test]
fn a_restriction_added_after_the_data_reads_all_of_it() {
    let mut run = run("ex:u ex:p ex:v . ex:v a ex:B . ex:u a ex:R2 . ex:w ex:p ex:o .");
    run.add("ex:R owl:someValuesFrom ex:B ; owl:onProperty ex:p .");
    run.add("ex:R2 owl:allValuesFrom ex:D ; owl:onProperty ex:p .");
    run.add("ex:R3 owl:hasValue ex:o ; owl:onProperty ex:p .");
    run.add("ex:R4 owl:someValuesFrom owl:Thing ; owl:onProperty ex:p .");
    let mut want = expand(
        "ex:u a ex:R
         ex:v a ex:D
         ex:w a ex:R3
         ex:u a ex:R4
         ex:w a ex:R4",
    );
    want.extend(expand(FACTS));
    assert_eq!(run.inferred(), want);
}

#[test]
fn data_added_after_a_run_is_joined_with_the_schema() {
    // Explicit blank labels: `( ... )` would get new ones in each run.
    let schema = "ex:A rdfs:subClassOf ex:B .
                  ex:C owl:intersectionOf _:i1 . _:i1 rdf:first ex:B ; rdf:rest _:i2 .
                  _:i2 rdf:first ex:D ; rdf:rest rdf:nil .
                  ex:U owl:unionOf _:u1 . _:u1 rdf:first ex:D ; rdf:rest rdf:nil .
                  ex:R owl:hasValue ex:o ; owl:onProperty ex:p .";
    let data = ["ex:x a ex:A .", "ex:x a ex:D .", "ex:y ex:p ex:o .", "ex:z a ex:R ."];
    let mut split = run(schema);
    for d in data {
        split.add(d);
    }
    let whole = run(&format!("{schema} {}", data.concat()));
    assert_eq!(split.closure(), whole.closure());
    assert!(split.has("ex:x a ex:C"));
    assert!(split.has("ex:x a ex:U"));
    assert!(split.has("ex:y a ex:R"));
    assert!(split.has("ex:z ex:p ex:o"));
}

// ---------------------------------------------------------------------------
// Scaling
// ---------------------------------------------------------------------------

mod scaling {
    use crate::engine::Store;
    use crate::rules::{Context, class};
    use crate::scaling::{Rng, linear, lock};
    use crate::term::{Dictionary, vocab};

    const TYPE: u32 = vocab::RDF_TYPE.0;
    const SCO: u32 = vocab::RDFS_SUB_CLASS_OF.0;
    /// Term ids past the vocabulary; the class rules never look a term up.
    const BASE: u32 = 100_000;
    /// A binary class tree 7 levels below its root, as deep as Brick 1.1.
    const DEPTH: u32 = 7;
    const CLASSES: u32 = (1 << (DEPTH + 1)) - 1;
    const LEAVES: u32 = 1 << DEPTH;

    /// Class `i` of the tree: 0 is the root, `2i + 1` and `2i + 2` its children.
    fn class(i: u32) -> u32 {
        BASE + i
    }

    /// A fixed schema: the tree, two equivalences, an intersection, a union,
    /// and one restriction of each kind on two properties.
    fn schema(store: &mut Store) -> (u32, u32) {
        let mut next = BASE + CLASSES;
        let mut fresh = || {
            next += 1;
            next
        };
        for i in 1..CLASSES {
            store.insert(class(i), SCO, class((i - 1) / 2));
        }
        let (p, q, tag) = (fresh(), fresh(), fresh());
        for c in [class(5), class(40)] {
            store.insert(fresh(), vocab::OWL_EQUIVALENT_CLASS.0, c);
        }
        let list = |store: &mut Store, members: &[u32], cells: [u32; 2]| {
            store.insert(cells[0], vocab::RDF_FIRST.0, members[0]);
            store.insert(cells[0], vocab::RDF_REST.0, cells[1]);
            store.insert(cells[1], vocab::RDF_FIRST.0, members[1]);
            store.insert(cells[1], vocab::RDF_REST.0, vocab::RDF_NIL.0);
            cells[0]
        };
        let head = list(store, &[class(1), class(9)], [fresh(), fresh()]);
        store.insert(fresh(), vocab::OWL_INTERSECTION_OF.0, head);
        let head = list(store, &[class(130), class(200)], [fresh(), fresh()]);
        store.insert(fresh(), vocab::OWL_UNION_OF.0, head);
        let restriction = |store: &mut Store, x: u32, kind: u32, filler: u32, on: u32| {
            store.insert(x, kind, filler);
            store.insert(x, vocab::OWL_ON_PROPERTY.0, on);
        };
        restriction(store, fresh(), vocab::OWL_SOME_VALUES_FROM.0, class(2), p);
        restriction(store, fresh(), vocab::OWL_SOME_VALUES_FROM.0, vocab::OWL_THING.0, q);
        let avf = fresh();
        restriction(store, avf, vocab::OWL_ALL_VALUES_FROM.0, class(6), p);
        store.insert(class(4), SCO, avf);
        let hv = fresh();
        restriction(store, hv, vocab::OWL_HAS_VALUE.0, tag, q);
        store.insert(class(11), SCO, hv);
        (p, q)
    }

    /// The schema and `n` individuals, each typed with a random leaf and
    /// linked by `p` to another individual.
    fn input(n: usize) -> Store {
        let mut store = Store::new();
        let (p, _) = schema(&mut store);
        let mut rng = Rng::new(7);
        let first = BASE * 2;
        for i in 0..n as u32 {
            let x = first + i;
            store.insert(x, TYPE, class(CLASSES - LEAVES + rng.below(LEAVES as usize)));
            store.insert(x, p, first + rng.below(n));
        }
        store
    }

    /// The class family's run to the fixpoint is linear in the instance data
    /// under a fixed schema.
    #[test]
    fn scaling_class_family_run() {
        let _lock = lock();
        let dict = Dictionary::new();
        let close = |mut store: Store| {
            let mut findings = Vec::new();
            let mut cx = Context {
                terms: &dict,
                findings: &mut findings,
            };
            let rounds = store.run(|round| class::rules(round, &mut cx));
            (rounds, store)
        };
        let sizes = [1_000, 2_000, 4_000, 8_000, 16_000];
        for n in sizes {
            let (_, store) = close(input(n));
            let types = store.table(TYPE).map_or(0, |t| t.by_subject().len());
            // At least the leaf and its 7 ancestors per individual.
            assert!(types >= 8 * n, "{types} types for {n} individuals");
        }
        linear("scaling_class_family_run", &sizes, input, close);
    }
}
