//! The `schema` group's shapes, for scm-eqc2, scm-spo, scm-eqp1,
//! scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv, scm-svf1,
//! scm-svf2, scm-avf1, scm-avf2, scm-int and scm-uni. The core's shapes
//! already state the hierarchies, domains, ranges, intersections and unions
//! most of them read; the shapes here add what the rest need:
//!
//! - two restrictions of one kind that a rule relates: on one property with
//!   fillers in a subclass pair (scm-svf1, scm-avf1), or with one filler on
//!   properties in a subproperty pair (scm-hv, scm-svf2, scm-avf2). Now and
//!   then the pair is left out, or names one class or property twice, so
//!   the rule needs a reflexive premise that only a cycle or the `trivial`
//!   group gives. Each restriction is tied to a class or an individual, so
//!   the subsumptions reach the data;
//! - two classes or two properties stated each below the other (scm-eqc2,
//!   scm-eqp2).

use super::{Generator, Shape, datatypes};
use crate::vocab::*;
use oxrdf::Term;

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: None, weight: 4, draw: restriction_pair },
    Shape { version_1: None, weight: 1, draw: mutual },
];

/// Two restrictions of one kind, related by a subclass pair of their
/// fillers or a subproperty pair of their properties.
fn restriction_pair(g: &mut Generator) {
    let filler = *g.rng.pick(&[OWL_SOME_VALUES_FROM, OWL_ALL_VALUES_FROM, OWL_HAS_VALUE]);
    let stated = !g.rng.one_in(4);
    let (r1, r2) = if filler != OWL_HAS_VALUE && g.rng.one_in(2) {
        // One property, two fillers.
        let p = g.property();
        let (y1, y2) = (g.class(), g.superclass());
        if stated {
            g.add(&y1, RDFS_SUB_CLASS_OF, &y2);
        }
        (restriction(g, &p, filler, &y1), restriction(g, &p, filler, &y2))
    } else {
        // One filler, two properties of one kind.
        let data = g.rng.one_in(4);
        let (p1, p2) = if data {
            (g.data_property(), g.data_property())
        } else {
            (g.object_property(), g.object_property())
        };
        let y = match (filler, data) {
            (OWL_HAS_VALUE, true) => datatypes::literal(g),
            (OWL_HAS_VALUE, false) => g.individual(),
            _ => g.superclass(),
        };
        if stated {
            g.add(&p1, RDFS_SUB_PROPERTY_OF, &p2);
        }
        (restriction(g, &p1, filler, &y), restriction(g, &p2, filler, &y))
    };
    tie(g, &r1);
    tie(g, &r2);
}

/// A blank restriction `[ owl:onProperty p ; <filler> y ]`.
fn restriction(g: &mut Generator, p: &Term, filler: &str, y: &Term) -> Term {
    let r = g.fresh();
    g.add(&r, RDF_TYPE, &iri(OWL_RESTRICTION));
    g.add(&r, OWL_ON_PROPERTY, p);
    g.add(&r, filler, y);
    r
}

/// Ties a restriction to a class (below it, above it or equivalent) or to
/// an individual it types.
fn tie(g: &mut Generator, r: &Term) {
    match g.rng.below(4) {
        0 => {
            let c = g.class();
            g.add(r, RDFS_SUB_CLASS_OF, &c);
        }
        1 => {
            let c = g.class();
            g.add(&c, RDFS_SUB_CLASS_OF, r);
        }
        2 => {
            let c = g.class();
            g.add(&c, OWL_EQUIVALENT_CLASS, r);
        }
        _ => {
            let i = g.individual();
            g.add(&i, RDF_TYPE, r);
        }
    }
}

/// `C rdfs:subClassOf D . D rdfs:subClassOf C`, or the same for two
/// properties of one kind.
fn mutual(g: &mut Generator) {
    let (a, b, predicate) = if g.rng.one_in(2) {
        (g.class(), g.class(), RDFS_SUB_CLASS_OF)
    } else if g.rng.one_in(4) {
        (g.data_property(), g.data_property(), RDFS_SUB_PROPERTY_OF)
    } else {
        (g.object_property(), g.object_property(), RDFS_SUB_PROPERTY_OF)
    };
    g.add(&a, predicate, &b);
    g.add(&b, predicate, &a);
}
