//! The property family's tests: one minimal input per rule, chains, lists
//! derived during the run, incremental runs, a mixed input with the
//! reference evaluator's inferences written out, generated inputs against a
//! naive evaluator of the same rules, and the family's scaling.

use crate::rules::testing::{self, expand};
use crate::term::vocab;
use std::collections::BTreeSet;

fn run(turtle: &str) -> testing::Run {
    testing::run(&["property"], turtle)
}

fn inferred(turtle: &str) -> BTreeSet<String> {
    run(turtle).inferred()
}

// ---------------------------------------------------------------------------
// One rule each
// ---------------------------------------------------------------------------

#[test]
fn prp_dom_types_the_subject() {
    assert_eq!(
        inferred("ex:p rdfs:domain ex:C . ex:a ex:p ex:b . ex:a ex:p ex:c ."),
        expand("ex:a a ex:C")
    );
}

#[test]
fn prp_rng_types_the_object_a_literal_too() {
    assert_eq!(
        inferred("ex:p rdfs:range ex:C . ex:a ex:p ex:b . ex:a ex:p \"lit\" ."),
        expand(
            "ex:b a ex:C
             \"lit\" a ex:C"
        )
    );
}

#[test]
fn prp_fp_makes_the_objects_of_a_subject_the_same() {
    assert_eq!(
        inferred(
            "ex:p a owl:FunctionalProperty . ex:a ex:p ex:o1 . ex:a ex:p ex:o2 .
             ex:b ex:p ex:o3 . ex:q a owl:Class . ex:c ex:q ex:o4 . ex:c ex:q ex:o5 ."
        ),
        expand(
            "ex:o1 owl:sameAs ex:o1
             ex:o1 owl:sameAs ex:o2
             ex:o2 owl:sameAs ex:o1
             ex:o2 owl:sameAs ex:o2
             ex:o3 owl:sameAs ex:o3"
        )
    );
}

/// Two literal values of a functional property: a generalized `owl:sameAs`
/// between literals, and between a literal and an IRI.
#[test]
fn prp_fp_with_literal_objects() {
    assert_eq!(
        inferred("ex:p a owl:FunctionalProperty . ex:a ex:p \"x\" . ex:a ex:p \"y\" . ex:a ex:p ex:o ."),
        expand(
            "\"x\" owl:sameAs \"x\"
             \"x\" owl:sameAs \"y\"
             \"x\" owl:sameAs ex:o
             \"y\" owl:sameAs \"x\"
             \"y\" owl:sameAs \"y\"
             \"y\" owl:sameAs ex:o
             ex:o owl:sameAs \"x\"
             ex:o owl:sameAs \"y\"
             ex:o owl:sameAs ex:o"
        )
    );
}

/// prp-ifp equates only subjects whose objects are equal.
#[test]
fn prp_ifp_merges_only_subjects_with_an_equal_object() {
    assert_eq!(
        inferred(
            "ex:p a owl:InverseFunctionalProperty .
             ex:a ex:p ex:o . ex:b ex:p ex:o . ex:c ex:p ex:o2 ."
        ),
        expand(
            "ex:a owl:sameAs ex:a
             ex:a owl:sameAs ex:b
             ex:b owl:sameAs ex:a
             ex:b owl:sameAs ex:b
             ex:c owl:sameAs ex:c"
        )
    );
}

#[test]
fn prp_symp_reverses_a_pair() {
    assert_eq!(
        inferred("ex:p a owl:SymmetricProperty . ex:a ex:p ex:b . ex:c ex:q ex:d ."),
        expand("ex:b ex:p ex:a")
    );
}

/// A literal object of a symmetric property gives a literal subject.
#[test]
fn prp_symp_with_a_literal() {
    assert_eq!(
        inferred("ex:p a owl:SymmetricProperty . ex:a ex:p \"x\" ."),
        expand("\"x\" ex:p ex:a")
    );
}

#[test]
fn prp_trp_closes_a_chain() {
    assert_eq!(
        inferred("ex:p a owl:TransitiveProperty . ex:a ex:p ex:b . ex:b ex:p ex:c . ex:c ex:p ex:d ."),
        expand(
            "ex:a ex:p ex:c
             ex:a ex:p ex:d
             ex:b ex:p ex:d"
        )
    );
}

/// A cycle closes to every pair, the reflexive ones included.
#[test]
fn prp_trp_closes_a_cycle() {
    assert_eq!(
        inferred("ex:p a owl:TransitiveProperty . ex:a ex:p ex:b . ex:b ex:p ex:a ."),
        expand(
            "ex:a ex:p ex:a
             ex:b ex:p ex:b"
        )
    );
}

#[test]
fn prp_spo1_copies_up_the_hierarchy() {
    assert_eq!(
        inferred("ex:p rdfs:subPropertyOf ex:q . ex:q rdfs:subPropertyOf ex:r . ex:a ex:p ex:b ."),
        expand(
            "ex:a ex:q ex:b
             ex:a ex:r ex:b"
        )
    );
}

/// A literal super-property gives triples with a literal predicate.
#[test]
fn prp_spo1_with_a_literal_super_property() {
    assert_eq!(
        inferred("ex:p rdfs:subPropertyOf \"x\" . ex:a ex:p ex:b ."),
        expand("ex:a \"x\" ex:b")
    );
}

#[test]
fn prp_eqp1_and_eqp2_copy_both_ways() {
    assert_eq!(
        inferred("ex:p owl:equivalentProperty ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d ."),
        expand(
            "ex:a ex:q ex:b
             ex:c ex:p ex:d"
        )
    );
}

#[test]
fn prp_inv1_and_inv2_copy_reversed_both_ways() {
    assert_eq!(
        inferred("ex:p owl:inverseOf ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d ."),
        expand(
            "ex:b ex:q ex:a
             ex:d ex:p ex:c"
        )
    );
}

/// A property that is its own inverse acts as a symmetric one.
#[test]
fn prp_inv_of_itself() {
    assert_eq!(
        inferred("ex:p owl:inverseOf ex:p . ex:a ex:p ex:b ."),
        expand("ex:b ex:p ex:a")
    );
}

/// A schema triple that arrives after the data joins all of it.
#[test]
fn schema_after_the_data_joins_all_of_it() {
    let mut r = run("ex:a ex:p ex:b . ex:b ex:p ex:c .");
    r.add("ex:p a owl:TransitiveProperty . ex:p rdfs:domain ex:C . ex:p owl:inverseOf ex:q .");
    assert_eq!(
        r.inferred(),
        expand(
            "ex:a ex:p ex:c
             ex:a a ex:C
             ex:b a ex:C
             ex:b ex:q ex:a
             ex:c ex:q ex:b
             ex:c ex:q ex:a"
        )
    );
}

// ---------------------------------------------------------------------------
// prp-spo2
// ---------------------------------------------------------------------------

/// `ex:p owl:propertyChainAxiom (ex:q1 ... ex:qn)`, a matching path
/// `ex:x0 ... ex:xn`, a path from `ex:y0` that stops one short, and a
/// path `ex:z0 ... ex:zn` with the properties in reverse order.
#[cfg(feature = "prp-spo2")]
fn chain_input(n: usize) -> String {
    let qs: Vec<String> = (1..=n).map(|i| format!("ex:q{i}")).collect();
    let mut turtle = format!("ex:p owl:propertyChainAxiom ( {} ) .\n", qs.join(" "));
    for i in 1..=n {
        turtle.push_str(&format!("ex:x{} ex:q{i} ex:x{i} .\n", i - 1));
        if i < n {
            turtle.push_str(&format!("ex:y{} ex:q{i} ex:y{i} .\n", i - 1));
        }
        turtle.push_str(&format!("ex:z{} ex:q{} ex:z{i} .\n", i - 1, n + 1 - i));
    }
    turtle
}

#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_chains_of_length_1_to_5() {
    for n in 1..=5 {
        let expected = if n == 1 {
            // A chain of one property: every pair of q1, the reversed path too.
            expand("ex:x0 ex:p ex:x1\nex:z0 ex:p ex:z1")
        } else {
            expand(&format!("ex:x0 ex:p ex:x{n}"))
        };
        assert_eq!(inferred(&chain_input(n)), expected, "a chain of length {n}");
    }
}

/// Each property of a chain arriving in its own run, in several orders:
/// each atom of the n-way join is the recent one in turn.
#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_atoms_arriving_one_run_at_a_time() {
    let schema = "ex:p owl:propertyChainAxiom (ex:q1 ex:q2 ex:q3 ex:q4) .";
    let edges: Vec<String> = (1..=4).map(|i| format!("ex:x{} ex:q{i} ex:x{i} .", i - 1)).collect();
    for order in [[0, 1, 2, 3], [3, 2, 1, 0], [1, 3, 0, 2], [2, 0, 3, 1]] {
        let mut r = run(schema);
        for &i in &order {
            assert!(!r.has("ex:x0 ex:p ex:x4"), "{order:?}");
            r.add(&edges[i]);
        }
        assert_eq!(r.inferred(), expand("ex:x0 ex:p ex:x4"), "{order:?}");
    }
}

/// The chain `(ex:p ex:q)` for `ex:p` itself: what it derives feeds it
/// again, but it is not transitivity.
#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_recursive_chain() {
    assert_eq!(
        inferred(
            "ex:p owl:propertyChainAxiom (ex:p ex:q) .
             ex:a ex:p ex:b . ex:b ex:q ex:c . ex:c ex:q ex:d . ex:d ex:p ex:e ."
        ),
        expand(
            "ex:a ex:p ex:c
             ex:a ex:p ex:d"
        )
    );
}

/// A chain that repeats a property: `(ex:q ex:q)` is the square of q.
#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_repeated_property() {
    assert_eq!(
        inferred("ex:p owl:propertyChainAxiom (ex:q ex:q) . ex:a ex:q ex:b . ex:b ex:q ex:c . ex:c ex:q ex:d ."),
        expand(
            "ex:a ex:p ex:c
             ex:b ex:p ex:d"
        )
    );
}

/// A list whose last cell points back into it never reaches `rdf:nil`: it
/// gives nothing, and the run ends.
#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_cyclic_list_gives_nothing() {
    let r = run("ex:p owl:propertyChainAxiom _:l1 .
         _:l1 rdf:first ex:q ; rdf:rest _:l2 .
         _:l2 rdf:first ex:r ; rdf:rest _:l1 .
         ex:a ex:q ex:b . ex:b ex:r ex:c . ex:c ex:q ex:d .");
    assert!(r.inferred().is_empty());
    assert!(r.rounds < 10, "{} rounds", r.rounds);
}

/// A cycle of cells that still reach `rdf:nil` (the second cell's rest is
/// both the first cell and nil) reads as the chains q r, q r q r, ..., as
/// the reference reads it.
#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_cycle_with_an_exit() {
    assert_eq!(
        inferred(
            "ex:p owl:propertyChainAxiom _:l1 .
             _:l1 rdf:first ex:q ; rdf:rest _:l2 .
             _:l2 rdf:first ex:r ; rdf:rest _:l1 , rdf:nil .
             ex:a ex:q ex:b . ex:b ex:r ex:c . ex:c ex:q ex:d . ex:d ex:r ex:e . ex:e ex:q ex:f ."
        ),
        expand(
            "ex:a ex:p ex:c
             ex:a ex:p ex:e
             ex:c ex:p ex:e"
        )
    );
}

/// A cell with two `rdf:first` gives a chain for each, and a cell with two
/// `rdf:rest` a chain for each path.
#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_branching_list() {
    assert_eq!(
        inferred(
            "ex:p owl:propertyChainAxiom _:l1 .
             _:l1 rdf:first ex:q , ex:s ; rdf:rest _:l2 , _:l3 .
             _:l2 rdf:first ex:r ; rdf:rest rdf:nil .
             _:l3 rdf:first ex:t ; rdf:rest rdf:nil .
             ex:a ex:q ex:b . ex:b ex:r ex:c . ex:b ex:t ex:d .
             ex:e ex:s ex:f . ex:f ex:r ex:g ."
        ),
        expand(
            "ex:a ex:p ex:c
             ex:a ex:p ex:d
             ex:e ex:p ex:g"
        )
    );
}

/// More chains than a list is walked into: the naive reading gives the
/// same. Twelve cells with two `rdf:first` each make 4,096 chains, and
/// one path of data matches.
#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_too_many_chains_are_read_naively() {
    let n = 12;
    assert!(1 << n > super::chain::MAX_CHAINS);
    let mut turtle = String::from("ex:p owl:propertyChainAxiom ex:l0 .\n");
    for i in 0..n {
        let rest = if i + 1 == n {
            "rdf:nil".to_owned()
        } else {
            format!("ex:l{}", i + 1)
        };
        turtle.push_str(&format!("ex:l{i} rdf:first ex:q{i} , ex:r{i} ; rdf:rest {rest} .\n"));
        let q = if i % 2 == 0 { "q" } else { "r" };
        turtle.push_str(&format!("ex:x{i} ex:{q}{i} ex:x{} .\n", i + 1));
    }
    assert_eq!(inferred(&turtle), expand(&format!("ex:x0 ex:p ex:x{n}")));
}

/// The axiom and a list cell's rest are derived during the run (prp-spo1
/// into `owl:propertyChainAxiom` and `rdf:rest`), and the chain then joins
/// data that was there from the start.
#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_list_derived_during_the_run() {
    assert_eq!(
        inferred(
            "ex:chain rdfs:subPropertyOf owl:propertyChainAxiom .
             ex:next rdfs:subPropertyOf rdf:rest .
             ex:p ex:chain _:l1 .
             _:l1 rdf:first ex:q ; ex:next _:l2 .
             _:l2 rdf:first ex:r ; rdf:rest rdf:nil .
             ex:a ex:q ex:b . ex:b ex:r ex:c ."
        ),
        expand(
            "ex:p owl:propertyChainAxiom _:l1
             _:l1 rdf:rest _:l2
             ex:a ex:p ex:c"
        )
    );
}

/// The data of a chain derived during the run, by other rules of the
/// family, round after round.
#[cfg(feature = "prp-spo2")]
#[test]
fn prp_spo2_data_derived_during_the_run() {
    assert_eq!(
        inferred(
            "ex:p owl:propertyChainAxiom (ex:q ex:r ex:s) .
             ex:q owl:inverseOf ex:qi . ex:s2 rdfs:subPropertyOf ex:s3 . ex:s3 owl:equivalentProperty ex:s .
             ex:b ex:qi ex:a . ex:b ex:r ex:c . ex:c ex:s2 ex:d ."
        ),
        expand(
            "ex:a ex:q ex:b
             ex:c ex:s3 ex:d
             ex:c ex:s ex:d
             ex:a ex:p ex:d"
        )
    );
}

// ---------------------------------------------------------------------------
// Incremental runs and a mixed input
// ---------------------------------------------------------------------------

/// Every rule at once, with IRI list cells (so that statements can be split
/// across documents).
const MIXED: &[&str] = &[
    "ex:knows rdfs:domain ex:Person .",
    "ex:knows rdfs:range ex:Agent .",
    "ex:knows owl:inverseOf ex:knownBy .",
    "ex:friend rdfs:subPropertyOf ex:knows .",
    "ex:near a owl:SymmetricProperty .",
    "ex:near owl:equivalentProperty ex:close .",
    "ex:part a owl:TransitiveProperty .",
    "ex:id a owl:InverseFunctionalProperty .",
    "ex:age a owl:FunctionalProperty .",
    "ex:uncle owl:propertyChainAxiom ex:l1 .",
    "ex:l1 rdf:first ex:parent .",
    "ex:l1 rdf:rest ex:l2 .",
    "ex:l2 rdf:first ex:brother .",
    "ex:l2 rdf:rest rdf:nil .",
    "ex:a ex:friend ex:b .",
    "ex:b ex:near ex:c .",
    "ex:c ex:close ex:d .",
    "ex:w ex:part ex:x .",
    "ex:x ex:part ex:y .",
    "ex:y ex:part ex:z .",
    "ex:a ex:id \"1\" .",
    "ex:a2 ex:id \"2\" .",
    "ex:a ex:age \"30\" .",
    "ex:kid ex:parent ex:mum .",
    "ex:mum ex:brother ex:bob .",
    "ex:bob ex:knownBy ex:kid .",
];

/// The reference evaluator's inferences on [`MIXED`] (`crates/reference`,
/// `evaluate_counted`), less what the families other than this one add:
/// `owl:Thing` and `owl:Nothing` typed `owl:Class` (cls-thing,
/// cls-nothing1). The eq-* rules add nothing here: each `owl:sameAs` is
/// reflexive. The last line has a literal subject: the reference counts it
/// as its one triple that is not RDF (`not_rdf`) and leaves it out of the
/// closure it returns.
const MIXED_INFERRED: &str = "
    ex:a ex:knows ex:b
    ex:a a ex:Person
    ex:b a ex:Agent
    ex:b ex:knownBy ex:a
    ex:c ex:near ex:b
    ex:b ex:close ex:c
    ex:c ex:close ex:b
    ex:c ex:near ex:d
    ex:d ex:close ex:c
    ex:d ex:near ex:c
    ex:w ex:part ex:y
    ex:w ex:part ex:z
    ex:x ex:part ex:z
    ex:a owl:sameAs ex:a
    ex:a2 owl:sameAs ex:a2
    ex:kid ex:uncle ex:bob
    ex:kid ex:knows ex:bob
    ex:kid a ex:Person
    ex:bob a ex:Agent
    \"30\" owl:sameAs \"30\"
";

#[cfg(feature = "prp-spo2")]
#[test]
fn mixed_input_matches_the_reference() {
    assert_eq!(run(&MIXED.join("\n")).inferred(), expand(MIXED_INFERRED));
}

/// The mixed input split across runs, one statement per run, forwards,
/// backwards and interleaved, and in two halves, gives the closure of one
/// run.
#[test]
fn incremental_runs_give_the_closure_of_one_run() {
    let whole = run(&MIXED.join("\n")).closure();
    let n = MIXED.len();
    let orders: Vec<Vec<usize>> = vec![
        (0..n).collect(),
        (0..n).rev().collect(),
        (0..n).step_by(2).chain((1..n).step_by(2)).collect(),
    ];
    for order in orders {
        let mut r = run("");
        for &i in &order {
            r.add(MIXED[i]);
        }
        assert_eq!(r.closure(), whole, "{order:?}");
    }
    let mut r = run(&MIXED[n / 2..].join("\n"));
    r.add(&MIXED[..n / 2].join("\n"));
    assert_eq!(r.closure(), whole);
}

// ---------------------------------------------------------------------------
// Generated inputs against a naive evaluator
// ---------------------------------------------------------------------------

mod generated {
    use super::*;
    use crate::scaling::Rng;

    type Triple = (u32, u32, u32);

    /// Ids for generated terms, past the vocabulary. They are never printed.
    const BASE: u32 = 1_000_000;
    const TERMS: usize = 10;

    const CHARACTERISTICS: [u32; 4] = [
        vocab::OWL_FUNCTIONAL_PROPERTY.0,
        vocab::OWL_INVERSE_FUNCTIONAL_PROPERTY.0,
        vocab::OWL_SYMMETRIC_PROPERTY.0,
        vocab::OWL_TRANSITIVE_PROPERTY.0,
    ];

    fn term(rng: &mut Rng) -> u32 {
        BASE + rng.below(TERMS)
    }

    /// A term, now and then a property characteristic (so that a derived
    /// `rdf:type` can declare one).
    fn object(rng: &mut Rng) -> u32 {
        if rng.below(6) == 0 {
            CHARACTERISTICS[rng.below(4) as usize]
        } else {
            term(rng)
        }
    }

    /// A small input: data over a few terms (any term in any position),
    /// every kind of property axiom, chain lists (branching, cyclic, with
    /// several firsts), and sub-properties of the vocabulary, which make
    /// schema and list triples during the run.
    fn input(rng: &mut Rng) -> Vec<Triple> {
        use vocab::*;
        let meta = [
            RDF_FIRST,
            RDF_REST,
            OWL_PROPERTY_CHAIN_AXIOM,
            RDF_TYPE,
            RDFS_SUB_PROPERTY_OF,
            RDFS_DOMAIN,
            OWL_INVERSE_OF,
            OWL_SAME_AS,
        ];
        let size = 4 + rng.below(24) as usize;
        let mut out = Vec::new();
        for _ in 0..size {
            let (a, b, c) = (term(rng), term(rng), object(rng));
            let t = match rng.below(14) {
                0..=4 => (a, b, c),
                5 => (a, RDFS_DOMAIN.0, c),
                6 => (a, RDFS_RANGE.0, c),
                7 => (a, RDF_TYPE.0, CHARACTERISTICS[rng.below(4) as usize]),
                8 => (a, RDFS_SUB_PROPERTY_OF.0, b),
                9 => (a, OWL_EQUIVALENT_PROPERTY.0, b),
                10 => (a, OWL_INVERSE_OF.0, b),
                11 => (a, OWL_PROPERTY_CHAIN_AXIOM.0, b),
                12 => {
                    if rng.below(2) == 0 {
                        (a, RDF_FIRST.0, b)
                    } else {
                        let tail = if rng.below(3) == 0 { RDF_NIL.0 } else { b };
                        (a, RDF_REST.0, tail)
                    }
                }
                _ => (a, RDFS_SUB_PROPERTY_OF.0, meta[rng.below(meta.len()) as usize].0),
            };
            out.push(t);
        }
        // Half the time a well-formed list of two or three cells, so that
        // chains fire often.
        if rng.below(2) == 0 {
            let len = 2 + rng.below(2);
            let cells: Vec<u32> = (0..len).map(|i| BASE + 100 + i).collect();
            out.push((term(rng), OWL_PROPERTY_CHAIN_AXIOM.0, cells[0]));
            for (i, &cell) in cells.iter().enumerate() {
                out.push((cell, RDF_FIRST.0, term(rng)));
                let tail = cells.get(i + 1).copied().unwrap_or(RDF_NIL.0);
                out.push((cell, RDF_REST.0, tail));
            }
        }
        out
    }

    /// The property rules as the reference evaluator writes them: naive
    /// loops over the whole closure, to a fixpoint.
    fn naive(base: &[Triple]) -> BTreeSet<Triple> {
        use vocab::*;
        let mut all: BTreeSet<Triple> = base.iter().copied().collect();
        loop {
            let pairs = |p: u32| -> Vec<(u32, u32)> { all.iter().filter(|t| t.1 == p).map(|t| (t.0, t.2)).collect() };
            let objects =
                |s: u32, p: u32| -> Vec<u32> { all.iter().filter(|t| t.0 == s && t.1 == p).map(|t| t.2).collect() };
            let typed = |c: u32| -> Vec<u32> {
                all.iter()
                    .filter(|t| t.1 == RDF_TYPE.0 && t.2 == c)
                    .map(|t| t.0)
                    .collect()
            };
            let mut new: Vec<Triple> = Vec::new();
            for (p, c) in pairs(RDFS_DOMAIN.0) {
                new.extend(pairs(p).into_iter().map(|(x, _)| (x, RDF_TYPE.0, c)));
            }
            for (p, c) in pairs(RDFS_RANGE.0) {
                new.extend(pairs(p).into_iter().map(|(_, y)| (y, RDF_TYPE.0, c)));
            }
            for p in typed(OWL_FUNCTIONAL_PROPERTY.0) {
                for (x, y1) in pairs(p) {
                    new.extend(objects(x, p).into_iter().map(|y2| (y1, OWL_SAME_AS.0, y2)));
                }
            }
            for p in typed(OWL_INVERSE_FUNCTIONAL_PROPERTY.0) {
                let ps = pairs(p);
                for &(x1, y) in &ps {
                    new.extend(ps.iter().filter(|t| t.1 == y).map(|&(x2, _)| (x1, OWL_SAME_AS.0, x2)));
                }
            }
            for p in typed(OWL_SYMMETRIC_PROPERTY.0) {
                new.extend(pairs(p).into_iter().map(|(x, y)| (y, p, x)));
            }
            for p in typed(OWL_TRANSITIVE_PROPERTY.0) {
                for (x, y) in pairs(p) {
                    new.extend(objects(y, p).into_iter().map(|z| (x, p, z)));
                }
            }
            for (p1, p2) in pairs(RDFS_SUB_PROPERTY_OF.0) {
                new.extend(pairs(p1).into_iter().map(|(x, y)| (x, p2, y)));
            }
            if cfg!(feature = "prp-spo2") {
                for (p, head) in pairs(OWL_PROPERTY_CHAIN_AXIOM.0) {
                    new.extend(chain(&pairs, &objects, head).into_iter().map(|(u, w)| (u, p, w)));
                }
            }
            for (p1, p2) in pairs(OWL_EQUIVALENT_PROPERTY.0) {
                new.extend(pairs(p1).into_iter().map(|(x, y)| (x, p2, y)));
                new.extend(pairs(p2).into_iter().map(|(x, y)| (x, p1, y)));
            }
            for (p1, p2) in pairs(OWL_INVERSE_OF.0) {
                new.extend(pairs(p1).into_iter().map(|(x, y)| (y, p2, x)));
                new.extend(pairs(p2).into_iter().map(|(x, y)| (y, p1, x)));
            }
            let before = all.len();
            all.extend(new);
            if all.len() == before {
                return all;
            }
        }
    }

    /// The reference's `View::chain`: for each cell reachable from `head`,
    /// the pairs the list from there to `rdf:nil` joins, to a fixpoint.
    fn chain(
        pairs: &impl Fn(u32) -> Vec<(u32, u32)>,
        objects: &impl Fn(u32, u32) -> Vec<u32>,
        head: u32,
    ) -> BTreeSet<(u32, u32)> {
        use std::collections::BTreeMap;
        use vocab::*;
        let mut cells = vec![head];
        let mut i = 0;
        while i < cells.len() {
            for tail in objects(cells[i], RDF_REST.0) {
                if !cells.contains(&tail) {
                    cells.push(tail);
                }
            }
            i += 1;
        }
        let mut joined: BTreeMap<u32, BTreeSet<(u32, u32)>> = BTreeMap::new();
        let mut changed = true;
        while changed {
            changed = false;
            for &cell in &cells {
                let mut found = Vec::new();
                for q in objects(cell, RDF_FIRST.0) {
                    for tail in objects(cell, RDF_REST.0) {
                        if tail == RDF_NIL.0 {
                            found.extend(pairs(q));
                        } else if let Some(after) = joined.get(&tail) {
                            for (u, x) in pairs(q) {
                                found.extend(after.iter().filter(|t| t.0 == x).map(|&(_, v)| (u, v)));
                            }
                        }
                    }
                }
                let set = joined.entry(cell).or_default();
                for pair in found {
                    changed |= set.insert(pair);
                }
            }
        }
        joined.remove(&head).unwrap_or_default()
    }

    fn closure(r: &testing::Run) -> BTreeSet<Triple> {
        r.store.triples().collect()
    }

    /// Runs `batches` of generalized triples, one run each.
    fn run_batches(batches: &[&[Triple]]) -> testing::Run {
        let mut r = run("");
        for batch in batches {
            for &(s, p, o) in *batch {
                if r.base.insert([s, p, o]) {
                    r.store.insert(s, p, o);
                }
            }
            r.add("");
        }
        r
    }

    #[test]
    fn generated_inputs_match_a_naive_evaluation() {
        for seed in 0..2000 {
            let mut rng = Rng::new(seed);
            let input = input(&mut rng);
            let expected = naive(&input);
            let whole = run_batches(&[&input]);
            assert_eq!(closure(&whole), expected, "seed {seed}: {input:?}");
            // The same input in random batches, one run each.
            let mut batches: Vec<&[Triple]> = Vec::new();
            let mut rest = &input[..];
            while !rest.is_empty() {
                let k = 1 + rng.below(rest.len().min(4)) as usize;
                let (batch, tail) = rest.split_at(k);
                batches.push(batch);
                rest = tail;
            }
            let split = run_batches(&batches);
            assert_eq!(closure(&split), expected, "seed {seed}, in batches: {input:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Scaling
// ---------------------------------------------------------------------------

mod scaling {
    use crate::engine::Store;
    use crate::rules::Context;
    use crate::scaling::linear;
    use crate::term::{Dictionary, vocab};

    /// Schema terms, then instance terms: ids past the vocabulary, never
    /// printed.
    const SCHEMA: u32 = 500_000;
    const DATA: u32 = 1_000_000;

    const KNOWS: u32 = SCHEMA;
    const PERSON: u32 = SCHEMA + 1;
    const AGENT: u32 = SCHEMA + 2;
    const HAS: u32 = SCHEMA + 3;
    const OF: u32 = SCHEMA + 4;
    const NEAR: u32 = SCHEMA + 5;
    const FUN: u32 = SCHEMA + 6;
    const IFUN: u32 = SCHEMA + 7;
    const SUB: u32 = SCHEMA + 8;
    const SUPER: u32 = SCHEMA + 9;
    const EQ1: u32 = SCHEMA + 10;
    const EQ2: u32 = SCHEMA + 11;
    const PART: u32 = SCHEMA + 12;
    const UNCLE: u32 = SCHEMA + 13;
    const PARENT: u32 = SCHEMA + 14;
    const BROTHER: u32 = SCHEMA + 15;
    const L1: u32 = SCHEMA + 16;
    const L2: u32 = SCHEMA + 17;

    const SCHEMA_TRIPLES: usize = 14;
    const BASE_PER_UNIT: usize = 13;
    /// Per unit: a typed by the domain, b by the range, b OF a, b NEAR a;
    /// four `owl:sameAs` among a and b (b's reflexive one from both FUN and
    /// IFUN); a SUPER c, a EQ2 d, three PART pairs, and a UNCLE c (prp-spo2).
    const INFERRED_PER_UNIT: usize = 14;

    /// A fixed schema with every rule of the family, and `n` units of data,
    /// each with its own four individuals: the closure is linear in `n`
    /// (prp-trp closes n disjoint chains of three edges, not one long one).
    fn store(n: usize) -> Store {
        use vocab::*;
        let mut store = Store::new();
        for (s, p, o) in [
            (KNOWS, RDFS_DOMAIN.0, PERSON),
            (KNOWS, RDFS_RANGE.0, AGENT),
            (HAS, OWL_INVERSE_OF.0, OF),
            (NEAR, RDF_TYPE.0, OWL_SYMMETRIC_PROPERTY.0),
            (FUN, RDF_TYPE.0, OWL_FUNCTIONAL_PROPERTY.0),
            (IFUN, RDF_TYPE.0, OWL_INVERSE_FUNCTIONAL_PROPERTY.0),
            (SUB, RDFS_SUB_PROPERTY_OF.0, SUPER),
            (EQ1, OWL_EQUIVALENT_PROPERTY.0, EQ2),
            (PART, RDF_TYPE.0, OWL_TRANSITIVE_PROPERTY.0),
            (UNCLE, OWL_PROPERTY_CHAIN_AXIOM.0, L1),
            (L1, RDF_FIRST.0, PARENT),
            (L1, RDF_REST.0, L2),
            (L2, RDF_FIRST.0, BROTHER),
            (L2, RDF_REST.0, RDF_NIL.0),
        ] {
            store.insert(s, p, o);
        }
        for i in 0..n as u32 {
            let [a, b, c, d] = [0, 1, 2, 3].map(|k| DATA + 4 * i + k);
            for (s, p, o) in [
                (a, KNOWS, b),
                (a, HAS, b),
                (a, NEAR, b),
                (a, FUN, b),
                (a, IFUN, c),
                (b, IFUN, c),
                (a, SUB, c),
                (a, EQ1, d),
                (a, PART, b),
                (b, PART, c),
                (c, PART, d),
                (a, PARENT, b),
                (b, BROTHER, c),
            ] {
                store.insert(s, p, o);
            }
        }
        store
    }

    fn reason(mut store: Store, terms: &Dictionary) -> Store {
        let mut findings = Vec::new();
        let mut cx = Context {
            terms,
            findings: &mut findings,
        };
        store.run(|round| super::super::rules(round, &mut cx));
        store
    }

    /// With a fixed schema, the family's run (not the parse) grows linearly
    /// with the data: 256 to 4,096 units, 3 to 53 thousand base triples.
    #[test]
    fn scaling_property_family_in_the_data() {
        let _lock = crate::scaling::lock();
        let terms = Dictionary::new();
        let sizes = [256, 512, 1024, 2048, 4096];
        let inferred = if cfg!(feature = "prp-spo2") {
            INFERRED_PER_UNIT
        } else {
            INFERRED_PER_UNIT - 1
        };
        for n in [sizes[0], sizes[4]] {
            let len = reason(store(n), &terms).len();
            assert_eq!(len, SCHEMA_TRIPLES + n * (BASE_PER_UNIT + inferred), "{n} units");
        }
        linear("scaling_property_family_in_the_data", &sizes, store, |s| {
            reason(s, &terms)
        });
    }
}
