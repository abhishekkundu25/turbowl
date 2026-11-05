//! The `checks` group's shapes: `owl:disjointWith`,
//! `owl:propertyDisjointWith`, `owl:complementOf`, and individuals typed
//! `owl:Nothing`. The asymmetric and irreflexive characteristics are drawn
//! by the core's characteristics shape, and the literals dt-not-type checks
//! by `datatypes.rs`.
//!
//! To come, in version 2's ticket 05: the shapes of cax-adc, cls-maxc1,
//! cls-maxqc1, cls-maxqc2, prp-adp, prp-npa1, prp-npa2, eq-diff1, eq-diff2
//! and eq-diff3.

use super::{Generator, Shape};
use crate::vocab::*;

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: Some(16), weight: 2, draw: disjoint_classes },
    Shape { version_1: Some(17), weight: 1, draw: disjoint_properties },
    Shape { version_1: Some(18), weight: 1, draw: complement },
    Shape { version_1: Some(19), weight: 1, draw: nothing },
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
