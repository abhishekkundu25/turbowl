//! The `identity` switch: keys and cardinalities that make individuals the
//! same, prp-key (`owl:hasKey`), cls-maxc2 (`owl:maxCardinality 1`),
//! cls-maxqc3 and cls-maxqc4 (`owl:maxQualifiedCardinality 1`, with
//! `owl:onClass` a class or `owl:Thing`; OWL 2 Profiles, Tables 5 and 6).
//!
//! Empty: version 2's ticket 06 fills this file. The rules derive
//! `owl:sameAs`, which the `equality` switch's rules then close.

use super::{EMPTY, Family};

pub(super) const FAMILY: Family = EMPTY;
