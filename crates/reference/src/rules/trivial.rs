//! The `trivial` switch, off by default: triples that hold of every term,
//! class, property or literal, eq-ref, scm-cls, scm-op, scm-dp, prp-ap,
//! dt-type1 and dt-type2 (OWL 2 Profiles, Tables 4, 5, 8 and 9).
//!
//! Empty: version 2's ticket 08 fills this file, and ticket 10 adds
//! dt-type2, which reads the datatype map (`crate::datatype`).

use super::{EMPTY, Family};

pub(super) const FAMILY: Family = EMPTY;
