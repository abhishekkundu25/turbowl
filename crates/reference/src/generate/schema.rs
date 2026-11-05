//! The `schema` group's shapes, for scm-eqc2, scm-spo, scm-eqp1,
//! scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv, scm-svf1,
//! scm-svf2, scm-avf1, scm-avf2, scm-int and scm-uni. The core's shapes
//! already state the axioms most of them read.
//!
//! None yet: version 2's ticket 07 adds them.
//!
//! A group with no shape draws nothing from the random generator, so every
//! seed gives the same input as before.

use super::Shape;

pub(super) const SHAPES: &[Shape] = &[];
