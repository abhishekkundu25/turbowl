//! The `checks` group's shapes: `owl:disjointWith`,
//! `owl:propertyDisjointWith`, `owl:complementOf`, and individuals typed
//! `owl:Nothing` (version 1's); `owl:differentFrom`, `owl:AllDifferent`,
//! `owl:AllDisjointClasses`, `owl:AllDisjointProperties`, negative property
//! assertions and restrictions of cardinality zero (version 2's, drawn from
//! this file's own stream). Each new shape now and then adds the data that
//! breaks its axiom, so chaos reaches every rule's finding. The asymmetric
//! and irreflexive characteristics are drawn by the core's characteristics
//! shape, and the literals dt-not-type checks by `datatypes.rs`.

use super::{Generator, Shape, datatypes, p_str};
use crate::vocab::*;
use oxrdf::{Literal, Term};

const OWL: &str = "http://www.w3.org/2002/07/owl#";
const OWL_DIFFERENT_FROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
const OWL_MEMBERS: &str = "http://www.w3.org/2002/07/owl#members";
const OWL_DISTINCT_MEMBERS: &str = "http://www.w3.org/2002/07/owl#distinctMembers";
const OWL_SOURCE_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#sourceIndividual";
const OWL_ASSERTION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#assertionProperty";
const OWL_TARGET_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#targetIndividual";
const OWL_TARGET_VALUE: &str = "http://www.w3.org/2002/07/owl#targetValue";
const OWL_MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
const OWL_MAX_QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
const OWL_ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: Some(16), weight: 2, draw: disjoint_classes },
    Shape { version_1: Some(17), weight: 1, draw: disjoint_properties },
    Shape { version_1: Some(18), weight: 1, draw: complement },
    Shape { version_1: Some(19), weight: 1, draw: nothing },
    Shape { version_1: None, weight: 2, draw: different },
    Shape { version_1: None, weight: 2, draw: all_different },
    Shape { version_1: None, weight: 2, draw: all_disjoint_classes },
    Shape { version_1: None, weight: 2, draw: all_disjoint_properties },
    Shape { version_1: None, weight: 2, draw: negative_assertion },
    Shape { version_1: None, weight: 2, draw: max_zero },
];

/// `C owl:disjointWith D`.
fn disjoint_classes(g: &mut Generator) {
    let (c, d) = (g.class(), g.superclass());
    g.add(&c, OWL_DISJOINT_WITH, &d);
}

/// `p owl:propertyDisjointWith q`.
fn disjoint_properties(g: &mut Generator) {
    let (p, q) = (g.property(), g.property());
    g.add(&p, OWL_PROPERTY_DISJOINT_WITH, &q);
}

/// `C owl:complementOf D`.
fn complement(g: &mut Generator) {
    let (c, d) = (g.class(), g.superclass());
    g.add(&c, OWL_COMPLEMENT_OF, &d);
}

/// `i a owl:Nothing`.
fn nothing(g: &mut Generator) {
    let i = g.individual();
    g.add(&i, RDF_TYPE, &iri(OWL_NOTHING));
}

fn owl(local: &str) -> Term {
    iri(&format!("{OWL}{local}"))
}

/// `a owl:differentFrom b`, `a` and `b` sometimes the same, and sometimes
/// `a owl:sameAs b` or `b owl:sameAs a` (eq-diff1).
fn different(g: &mut Generator) {
    let (a, b) = (g.individual(), g.individual());
    g.add(&a, OWL_DIFFERENT_FROM, &b);
    match g.rng.below(4) {
        0 => g.add(&a, OWL_SAME_AS, &b),
        1 => g.add(&b, OWL_SAME_AS, &a),
        _ => {}
    }
}

/// Two to four individuals, now and then one of them twice.
fn individuals(g: &mut Generator) -> Vec<Term> {
    let mut members: Vec<Term> = (0..2 + g.rng.below(3)).map(|_| g.individual()).collect();
    if g.rng.one_in(4) {
        let again = g.rng.pick(&members).clone();
        members.push(again);
    }
    members
}

/// A list of `members` for an all-different or all-disjoint axiom: mostly
/// well formed, now and then cyclic (no position).
fn axiom_list(g: &mut Generator, members: &[Term]) -> Term {
    if g.rng.one_in(10) {
        g.cyclic_list(members)
    } else {
        g.list(members)
    }
}

/// `x a owl:AllDifferent ; owl:members (...)` or `owl:distinctMembers`, and
/// half the time `owl:sameAs` between two members (eq-diff2, eq-diff3).
fn all_different(g: &mut Generator) {
    let members = individuals(g);
    let (x, head) = (g.fresh(), axiom_list(g, &members));
    g.add(&x, RDF_TYPE, &owl("AllDifferent"));
    let predicate = if g.rng.one_in(2) {
        OWL_MEMBERS
    } else {
        OWL_DISTINCT_MEMBERS
    };
    g.add(&x, predicate, &head);
    if g.rng.one_in(2) {
        let (a, b) = (g.rng.pick(&members).clone(), g.rng.pick(&members).clone());
        g.add(&a, OWL_SAME_AS, &b);
    }
}

/// `x a owl:AllDisjointClasses ; owl:members (...)`, and half the time an
/// individual typed with two of the classes (cax-adc).
fn all_disjoint_classes(g: &mut Generator) {
    let members: Vec<Term> = (0..2 + g.rng.below(3)).map(|_| g.class()).collect();
    let (x, head) = (g.fresh(), axiom_list(g, &members));
    g.add(&x, RDF_TYPE, &owl("AllDisjointClasses"));
    g.add(&x, OWL_MEMBERS, &head);
    if g.rng.one_in(2) {
        let i = g.individual();
        for _ in 0..2 {
            let c = g.rng.pick(&members).clone();
            g.add(&i, RDF_TYPE, &c);
        }
    }
}

/// `x a owl:AllDisjointProperties ; owl:members (...)`, and half the time
/// two of the properties between the same individuals (prp-adp).
fn all_disjoint_properties(g: &mut Generator) {
    let members: Vec<Term> = (0..2 + g.rng.below(3)).map(|_| g.object_property()).collect();
    let (x, head) = (g.fresh(), axiom_list(g, &members));
    g.add(&x, RDF_TYPE, &owl("AllDisjointProperties"));
    g.add(&x, OWL_MEMBERS, &head);
    if g.rng.one_in(2) {
        let (u, v) = (g.individual(), g.individual());
        for _ in 0..2 {
            let p = g.rng.pick(&members).clone();
            g.add(&u, p_str(&p), &v);
        }
    }
}

/// A negative assertion: on an object property with
/// `owl:targetIndividual`, or on a data property with `owl:targetValue`,
/// and half the time the triple it denies (prp-npa1, prp-npa2).
fn negative_assertion(g: &mut Generator) {
    let (x, source) = (g.fresh(), g.individual());
    g.add(&x, RDF_TYPE, &owl("NegativePropertyAssertion"));
    g.add(&x, OWL_SOURCE_INDIVIDUAL, &source);
    let (p, target, kind) = if g.rng.one_in(2) {
        (g.object_property(), g.individual(), OWL_TARGET_INDIVIDUAL)
    } else {
        (g.data_property(), datatypes::literal(g), OWL_TARGET_VALUE)
    };
    g.add(&x, OWL_ASSERTION_PROPERTY, &p);
    g.add(&x, kind, &target);
    if g.rng.one_in(2) {
        g.add(&source, p_str(&p), &target);
    }
}

/// A restriction of cardinality zero: `owl:maxCardinality`, or
/// `owl:maxQualifiedCardinality` on a class or `owl:Thing`; now and then
/// the cardinality is another literal (`"1"`, `"00"`, an `xsd:integer`
/// zero), which these rules do not read. Half the time an instance with a
/// value (cls-maxc1, cls-maxqc1, cls-maxqc2).
fn max_zero(g: &mut Generator) {
    let r = g.fresh();
    let cardinality = match g.rng.below(8) {
        0 => Literal::new_typed_literal("1", named(XSD_NON_NEGATIVE_INTEGER)),
        1 => Literal::new_typed_literal("00", named(XSD_NON_NEGATIVE_INTEGER)),
        2 => Literal::new_typed_literal("0", named(XSD_INTEGER)),
        _ => Literal::new_typed_literal("0", named(XSD_NON_NEGATIVE_INTEGER)),
    };
    let data = g.rng.one_in(4);
    let p = if data { g.data_property() } else { g.object_property() };
    g.add(&r, RDF_TYPE, &iri(OWL_RESTRICTION));
    g.add(&r, OWL_ON_PROPERTY, &p);
    let class = if g.rng.one_in(2) {
        g.add(&r, OWL_MAX_CARDINALITY, &cardinality.into());
        None
    } else {
        g.add(&r, OWL_MAX_QUALIFIED_CARDINALITY, &cardinality.into());
        let c = if g.rng.one_in(3) { iri(OWL_THING) } else { g.class() };
        g.add(&r, OWL_ON_CLASS, &c);
        Some(c)
    };
    if g.rng.one_in(2) {
        let u = g.individual();
        g.add(&u, RDF_TYPE, &r);
        let y = if data { datatypes::literal(g) } else { g.individual() };
        g.add(&u, p_str(&p), &y);
        if let Some(c) = class
            && !data
            && g.rng.one_in(2)
        {
            g.add(&y, RDF_TYPE, &c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Rng, generate_parts};
    use crate::evaluate_within;
    use std::collections::{BTreeMap, HashSet};
    use turbowl::Switches;

    /// The inputs of the chaos test's first 300 seeds (both seeds, sizes 1
    /// to 48 as it draws them), evaluated in the default configuration:
    /// each new rule's construct shows in many of them, and each new rule
    /// finds something in some of them.
    #[test]
    fn the_first_300_seeds_reach_every_new_rule() {
        let constructs = [
            "differentFrom",
            "AllDifferent",
            "AllDisjointClasses",
            "AllDisjointProperties",
            "sourceIndividual",
            "maxCardinality",
            "maxQualifiedCardinality",
        ];
        let rules = [
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
        let mut shown: BTreeMap<&str, usize> = BTreeMap::new();
        let mut found: BTreeMap<&str, usize> = BTreeMap::new();
        for seed in [0x5EED_0005, 0x5EED_0105] {
            for i in 0..300 {
                let mut rng = Rng::at(seed, i);
                let size = 1 + rng.below(48);
                let (_, input) = generate_parts(rng.next_u64(), size);
                let text: String = input.iter().map(|t| format!("{t}\n")).collect();
                for c in constructs {
                    *shown.entry(c).or_default() += usize::from(text.contains(&format!("owl#{c}>")));
                }
                let triples: HashSet<_> = input.into_iter().collect();
                let Some(evaluation) = evaluate_within(&triples, Switches::default(), 200_000) else {
                    continue;
                };
                for r in rules {
                    *found.entry(r).or_default() +=
                        usize::from(evaluation.diagnostics.iter().any(|(code, _)| code == r));
                }
            }
        }
        println!("constructs: {shown:?}\nfindings: {found:?}");
        for c in constructs {
            assert!(shown[c] >= 30, "{c} shows in {} inputs: {shown:?}", shown[c]);
        }
        for r in rules {
            assert!(found[r] >= 3, "{r} finds something in {} inputs: {found:?}", found[r]);
        }
    }
}
