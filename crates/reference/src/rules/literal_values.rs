//! The `literal-values` switch, off by default: literals compared by value,
//! dt-eq and dt-diff (OWL 2 Profiles, Table 8), through the datatype map
//! (`crate::datatype`), over the literals of the closure and those the
//! rules name ([`rule_literals`]).
//!
//! - dt-eq: `lt1 owl:sameAs lt2` for every two literals with the same value,
//!   each with itself too, as the W3C table writes it.
//! - dt-diff: `lt1 owl:differentFrom lt2` for every two literals with
//!   different values, as a triple the closure never lists (the phase 1
//!   spec, "dt-diff is never listed"): its predicate is
//!   [`unlisted_different_from`], not `owl:differentFrom`. eq-rep-s and
//!   eq-rep-o carry it like any triple, eq-ref leaves it out
//!   (`trivial.rs`), and eq-diff1 reads it as it reads `owl:differentFrom`
//!   (`checks.rs`); no other rule names it, and the returned closure leaves
//!   it out, a predicate that is not an IRI.

use super::{EMPTY, Family};
use crate::datatype::DataValue;
use crate::evaluate::{Round, View};
use crate::vocab::*;
use oxrdf::Term;
use std::collections::HashSet;

pub(super) const FAMILY: Family = Family {
    rules: &["dt-eq", "dt-diff"],
    derive,
    ..EMPTY
};

fn derive(v: &View, r: &mut Round) {
    let named = rule_literals();
    let mut literals: HashSet<&Term> = named.iter().collect();
    for (s, _, o) in &v.all {
        literals.extend([s, o].into_iter().filter(|t| matches!(t, Term::Literal(_))));
    }
    let valued: Vec<(&Term, DataValue)> = literals
        .into_iter()
        .filter_map(|t| match t {
            Term::Literal(lt) => DataValue::of(lt).map(|value| (t, value)),
            _ => None,
        })
        .collect();
    let (same, different) = (iri(OWL_SAME_AS), unlisted_different_from());
    for (lt1, v1) in &valued {
        for (lt2, v2) in &valued {
            if v1 == v2 {
                r.derive("dt-eq", lt1, &same, lt2);
            } else {
                r.derive("dt-diff", lt1, &different, lt2);
            }
        }
    }
}
