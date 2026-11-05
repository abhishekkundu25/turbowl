//! The `trivial` group's minimal inputs: eq-ref, scm-cls, scm-op, scm-dp,
//! prp-ap, dt-type1 and dt-type2.
//!
//! Version 2's tickets 08 and 10 add them. Until then the group has none of
//! its rules, and the input here states that version 1 has no eq-ref; the
//! ticket turns it around.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::Trivial);

pub(super) const CASES: &[Case] = &[Case {
    group: GROUP,
    name: "not-eq-ref",
    covers: "not eq-ref",
    turtle: "ex:a ex:p ex:b .",
    expect: &["- ex:a owl:sameAs ex:a", "- ex:p owl:sameAs ex:p"],
}];
