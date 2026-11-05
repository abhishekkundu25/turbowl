//! The `equality` group's shapes: `owl:sameAs`, mostly between
//! individuals, chained into cliques; now and then between two classes or
//! two properties of the same kind; and now and then on a vocabulary term
//! (ADR 0003's amendment): a term made equal to a rule constant, two rule
//! constants made equal, a list cell made equal to `rdf:nil` or to another
//! cell.

use super::{Generator, Shape};
use crate::vocab::*;

/// Rule constants `owl:sameAs` may name (ADR 0003's amendment): the
/// predicates the rules read, the classes they test, and the datatypes.
const VOCABULARY: &[&str] = &[
    RDF_TYPE,
    RDFS_SUB_CLASS_OF,
    RDFS_SUB_PROPERTY_OF,
    RDFS_DOMAIN,
    RDFS_RANGE,
    OWL_SAME_AS,
    OWL_EQUIVALENT_CLASS,
    OWL_EQUIVALENT_PROPERTY,
    OWL_INVERSE_OF,
    OWL_ON_PROPERTY,
    OWL_SOME_VALUES_FROM,
    OWL_ALL_VALUES_FROM,
    OWL_HAS_VALUE,
    OWL_INTERSECTION_OF,
    OWL_UNION_OF,
    OWL_DISJOINT_WITH,
    RDF_FIRST,
    RDF_REST,
    RDF_NIL,
    OWL_THING,
    OWL_NOTHING,
    OWL_FUNCTIONAL_PROPERTY,
    OWL_TRANSITIVE_PROPERTY,
    OWL_SYMMETRIC_PROPERTY,
    XSD_INTEGER,
    XSD_INT,
    XSD_STRING,
];

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: Some(15), weight: 4, draw: same_as },
];

/// `a owl:sameAs b`.
fn same_as(g: &mut Generator) {
    let (a, b) = match g.rng.below(13) {
        0 => (g.class(), g.class()),
        1 => (g.object_property(), g.object_property()),
        10 => {
            let a = match g.rng.below(4) {
                0 => g.class(),
                1 => g.individual(),
                _ => g.property(),
            };
            (a, iri(g.rng.pick(VOCABULARY)))
        }
        11 => (iri(g.rng.pick(VOCABULARY)), iri(g.rng.pick(VOCABULARY))),
        12 => {
            let cell = if g.lists.is_empty() {
                g.individual()
            } else {
                let list = g.rng.pick(&g.lists).clone();
                list[g.rng.below(list.len())].clone()
            };
            let other = match g.rng.below(4) {
                0 | 1 => iri(RDF_NIL),
                2 if !g.lists.is_empty() => {
                    let list = g.rng.pick(&g.lists).clone();
                    list[g.rng.below(list.len())].clone()
                }
                _ => g.individual(),
            };
            (cell, other)
        }
        _ => (g.individual(), g.individual()),
    };
    g.add(&a, OWL_SAME_AS, &b);
}
