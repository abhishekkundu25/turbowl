//! The `property-chains` group's shapes: `owl:propertyChainAxiom` over one
//! to four object properties, on a list that is well formed (now and then
//! with IRI cells or a tail shared with an earlier list) or cyclic.

use super::{Generator, Shape};
use crate::vocab::*;
use oxrdf::Term;

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: Some(11), weight: 3, draw: chain },
];

/// `p owl:propertyChainAxiom (p1 ... pn)`.
fn chain(g: &mut Generator) {
    let p = g.object_property();
    let members: Vec<Term> = (0..1 + g.rng.below(4)).map(|_| g.object_property()).collect();
    let head = if g.rng.one_in(8) {
        g.cyclic_list(&members)
    } else {
        g.list(&members)
    };
    g.add(&p, OWL_PROPERTY_CHAIN_AXIOM, &head);
}
