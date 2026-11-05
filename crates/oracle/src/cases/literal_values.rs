//! The `literal-values` group's minimal inputs: dt-eq and dt-diff.
//!
//! None yet: version 2's ticket 10 adds them.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::LiteralValues);

pub(super) const CASES: &[Case] = &[];
