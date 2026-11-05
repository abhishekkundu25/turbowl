//! The core's shapes: data, the class and property hierarchies,
//! equivalences, domains and ranges, property characteristics, inverses,
//! intersections, unions, restrictions, and container membership.
//!
//! The characteristics shape draws the `checks` group's
//! `owl:AsymmetricProperty` and `owl:IrreflexiveProperty` too, and the range
//! shape a datatype (`datatypes.rs`) half the time: one shape each in
//! version 1's table, kept whole so that every seed's input stays the same.
//!
//! To come: `owl:oneOf` for cls-oo, version 2's ticket 08.

use super::{Generator, Shape, datatypes, p_str};
use crate::vocab::*;
use oxrdf::Term;

/// Container membership predicates, and names like them (`_0`, `_a`): no
/// rule of the set gives them a meaning, so they derive nothing.
const MEMBERSHIP: &[&str] = &["_1", "_2", "_3", "_01", "_0", "_a"];

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: Some(0), weight: 14, draw: typed },
    Shape { version_1: Some(1), weight: 14, draw: related },
    Shape { version_1: Some(3), weight: 6, draw: subclass },
    Shape { version_1: Some(4), weight: 2, draw: equivalent_classes },
    Shape { version_1: Some(5), weight: 3, draw: domain },
    Shape { version_1: Some(6), weight: 3, draw: range },
    Shape { version_1: Some(7), weight: 6, draw: characteristic },
    Shape { version_1: Some(8), weight: 3, draw: subproperty },
    Shape { version_1: Some(9), weight: 2, draw: equivalent_properties },
    Shape { version_1: Some(10), weight: 2, draw: inverse },
    Shape { version_1: Some(12), weight: 2, draw: intersection },
    Shape { version_1: Some(13), weight: 2, draw: union },
    Shape { version_1: Some(14), weight: 5, draw: restriction },
    Shape { version_1: Some(20), weight: 2, draw: membership },
];

/// `i a C`.
fn typed(g: &mut Generator) {
    let (i, c) = (g.individual(), g.superclass());
    g.add(&i, RDF_TYPE, &c);
}

/// `i p j`.
fn related(g: &mut Generator) {
    let (i, p, j) = (g.individual(), g.object_property(), g.individual());
    g.add(&i, p_str(&p), &j);
}

/// `C rdfs:subClassOf D`.
fn subclass(g: &mut Generator) {
    let (c, d) = (g.class(), g.superclass());
    g.add(&c, RDFS_SUB_CLASS_OF, &d);
}

/// `C owl:equivalentClass D`.
fn equivalent_classes(g: &mut Generator) {
    let (c, d) = (g.class(), g.class());
    g.add(&c, OWL_EQUIVALENT_CLASS, &d);
}

/// `p rdfs:domain C`.
fn domain(g: &mut Generator) {
    let (p, c) = (g.property(), g.superclass());
    g.add(&p, RDFS_DOMAIN, &c);
}

/// `p rdfs:range C`, or `d rdfs:range` a datatype.
fn range(g: &mut Generator) {
    if g.rng.one_in(2) {
        let d = g.data_property();
        let t = datatypes::datatype(g);
        g.add(&d, RDFS_RANGE, &t);
    } else {
        let (p, c) = (g.object_property(), g.class());
        g.add(&p, RDFS_RANGE, &c);
    }
}

/// A property characteristic.
fn characteristic(g: &mut Generator) {
    let (p, characteristic) = match g.rng.below(7) {
        0 => (g.object_property(), OWL_SYMMETRIC_PROPERTY),
        1 => (g.object_property(), OWL_TRANSITIVE_PROPERTY),
        2 => (g.object_property(), OWL_FUNCTIONAL_PROPERTY),
        3 => (g.data_property(), OWL_FUNCTIONAL_PROPERTY),
        4 => (g.object_property(), OWL_INVERSE_FUNCTIONAL_PROPERTY),
        5 => (g.object_property(), OWL_ASYMMETRIC_PROPERTY),
        _ => (g.object_property(), OWL_IRREFLEXIVE_PROPERTY),
    };
    g.add(&p, RDF_TYPE, &iri(characteristic));
}

/// Two properties of the same kind, mostly object properties.
fn two_properties(g: &mut Generator) -> (Term, Term) {
    if g.rng.one_in(4) {
        (g.data_property(), g.data_property())
    } else {
        (g.object_property(), g.object_property())
    }
}

/// `p rdfs:subPropertyOf q`.
fn subproperty(g: &mut Generator) {
    let (p, q) = two_properties(g);
    g.add(&p, RDFS_SUB_PROPERTY_OF, &q);
}

/// `p owl:equivalentProperty q`.
fn equivalent_properties(g: &mut Generator) {
    let (p, q) = two_properties(g);
    g.add(&p, OWL_EQUIVALENT_PROPERTY, &q);
}

/// `p owl:inverseOf q`.
fn inverse(g: &mut Generator) {
    let (p, q) = (g.object_property(), g.object_property());
    g.add(&p, OWL_INVERSE_OF, &q);
}

/// A list of two or three classes, now and then restrictions.
fn class_list(g: &mut Generator) -> Term {
    let members: Vec<Term> = (0..2 + g.rng.below(2))
        .map(|_| {
            if g.rng.one_in(4) {
                g.restriction()
            } else {
                g.superclass()
            }
        })
        .collect();
    g.list(&members)
}

/// `C owl:intersectionOf (...)`.
fn intersection(g: &mut Generator) {
    let c = g.class();
    let head = class_list(g);
    g.add(&c, OWL_INTERSECTION_OF, &head);
}

/// `C owl:unionOf (...)`.
fn union(g: &mut Generator) {
    let c = g.class();
    let head = class_list(g);
    g.add(&c, OWL_UNION_OF, &head);
}

/// A restriction, tied to a class (below it, above it or equivalent) or
/// typing an individual.
fn restriction(g: &mut Generator) {
    let r = g.restriction();
    match g.rng.below(4) {
        0 => {
            let c = g.class();
            g.add(&r, RDFS_SUB_CLASS_OF, &c);
        }
        1 => {
            let c = g.class();
            g.add(&c, RDFS_SUB_CLASS_OF, &r);
        }
        2 => {
            let c = g.class();
            g.add(&c, OWL_EQUIVALENT_CLASS, &r);
        }
        _ => {
            let i = g.individual();
            g.add(&i, RDF_TYPE, &r);
        }
    }
}

/// A container membership triple, `_:c rdf:_1 i`.
fn membership(g: &mut Generator) {
    let container = g.fresh();
    let p = named(&format!("{RDF}{}", g.rng.pick(MEMBERSHIP)));
    let i = g.individual();
    g.add_with(&container, &p, &i);
}
