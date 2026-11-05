//! The `identity` group's shapes, for prp-key, cls-maxc2, cls-maxqc3 and
//! cls-maxqc4.
//!
//! None yet: version 2's ticket 06 adds them.
//!
//! A group with no shape draws nothing from the random generator, so every
//! seed gives the same input as before.

use super::Shape;

pub(super) const SHAPES: &[Shape] = &[];
