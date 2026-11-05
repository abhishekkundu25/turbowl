//! The `checks` switch: the rules that conclude `false` (cax-dw, prp-pdw,
//! cls-nothing2, prp-asyp, prp-irp, cls-com). dt-not-type, in the same
//! switch, is in `datatypes.rs`.
//!
//! To come, in version 2's ticket 05: cax-adc, cls-maxc1, cls-maxqc1,
//! cls-maxqc2, prp-adp, prp-npa1, prp-npa2, eq-diff1, eq-diff2, eq-diff3.

use super::{EMPTY, Family, Found};
use crate::evaluate::View;
use crate::vocab::*;

pub(super) const FAMILY: Family = Family {
    diagnostic_rules: &["cax-dw", "prp-pdw", "cls-nothing2", "prp-asyp", "prp-irp", "cls-com"],
    diagnose,
    ..EMPTY
};

fn diagnose(v: &View, found: &mut Vec<Found>) {
    let ty = iri(RDF_TYPE);
    for (c1, c2) in v.pairs(&iri(OWL_DISJOINT_WITH)) {
        for x in v.subjects(&ty, c1) {
            if v.has(x, &ty, c2) {
                found.push(("cax-dw", vec![x.clone(), c1.clone(), c2.clone()]));
            }
        }
    }
    for (p1, p2) in v.pairs(&iri(OWL_PROPERTY_DISJOINT_WITH)) {
        for (x, y) in v.pairs(p1) {
            if v.has(x, p2, y) {
                found.push(("prp-pdw", vec![p1.clone(), p2.clone(), x.clone(), y.clone()]));
            }
        }
    }
    for x in v.instances(OWL_NOTHING) {
        found.push(("cls-nothing2", vec![x.clone()]));
    }
    for p in v.instances(OWL_ASYMMETRIC_PROPERTY) {
        for (x, y) in v.pairs(p) {
            if v.has(y, p, x) {
                found.push(("prp-asyp", vec![p.clone(), x.clone(), y.clone()]));
            }
        }
    }
    for p in v.instances(OWL_IRREFLEXIVE_PROPERTY) {
        for (x, y) in v.pairs(p) {
            if x == y {
                found.push(("prp-irp", vec![p.clone(), x.clone()]));
            }
        }
    }
    for (c1, c2) in v.pairs(&iri(OWL_COMPLEMENT_OF)) {
        for x in v.subjects(&ty, c1) {
            if v.has(x, &ty, c2) {
                found.push(("cls-com", vec![c1.clone(), c2.clone(), x.clone()]));
            }
        }
    }
}
