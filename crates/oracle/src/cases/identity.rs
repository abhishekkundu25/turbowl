//! The `identity` group's minimal inputs: prp-key, cls-maxc2, cls-maxqc3
//! and cls-maxqc4.
//!
//! None yet: version 2's ticket 06 adds them.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::Identity);

pub(super) const CASES: &[Case] = &[];
