//! The `literal-values` switch, off by default: literals compared by value,
//! dt-eq and dt-diff (OWL 2 Profiles, Table 8), through the datatype map
//! (`crate::datatype`).
//!
//! Empty: version 2's ticket 10 fills this file.

use super::{EMPTY, Family};

pub(super) const FAMILY: Family = EMPTY;
