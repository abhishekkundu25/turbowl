//! The `equality` switch: `owl:sameAs` makes terms equal (eq-sym, eq-trans,
//! eq-rep-s, eq-rep-p, eq-rep-o), as the W3C tables write the rules, with
//! no representatives.

use super::{EMPTY, Family};
use crate::evaluate::{Round, View};
use crate::vocab::*;

pub(super) const FAMILY: Family = Family {
    rules: &["eq-sym", "eq-trans", "eq-rep-s", "eq-rep-p", "eq-rep-o"],
    derive,
    ..EMPTY
};

fn derive(v: &View, r: &mut Round) {
    let same = iri(OWL_SAME_AS);
    for (x, y) in v.pairs(&same) {
        r.derive("eq-sym", y, &same, x);
        for z in v.objects(y, &same) {
            r.derive("eq-trans", x, &same, z);
        }
        for (p, o) in v.about(x) {
            r.derive("eq-rep-s", y, p, o);
        }
        for (s, o) in v.pairs(x) {
            r.derive("eq-rep-p", s, y, o);
        }
        for (s, p) in v.pointing_at(x) {
            r.derive("eq-rep-o", s, p, y);
        }
    }
}
