//! The `property-chains` switch: prp-spo2 (`owl:propertyChainAxiom`), read
//! along the list paths [`View::chain`] joins.

use super::{EMPTY, Family};
use crate::evaluate::{Round, View};
use crate::vocab::*;

pub(super) const FAMILY: Family = Family {
    rules: &["prp-spo2"],
    derive,
    ..EMPTY
};

fn derive(v: &View, r: &mut Round) {
    for (p, head) in v.pairs(&iri(OWL_PROPERTY_CHAIN_AXIOM)) {
        let Some(pairs) = v.chain(head, r.room) else {
            r.overflow = true;
            return;
        };
        for (u, w) in pairs {
            r.derive("prp-spo2", &u, p, &w);
        }
    }
}
