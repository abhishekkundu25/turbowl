//! The `schema` group's minimal inputs: scm-eqc2, scm-spo, scm-eqp1,
//! scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv, scm-svf1,
//! scm-svf2, scm-avf1, scm-avf2, scm-int and scm-uni.
//!
//! Version 2's ticket 07 adds them. Until then the group has none of its
//! rules, and the two inputs here state that version 1 has neither scm-spo
//! nor scm-eqc2 (`scm-sco-cycle` also covers the core's scm-sco); the
//! ticket turns them around.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::Schema);

pub(super) const CASES: &[Case] = &[
    // scm-sco through a cycle gives A subClassOf A (OWL-RL does not: F1);
    // scm-eqc2 is not in version 1.
    Case {
        group: GROUP,
        name: "scm-sco-cycle",
        covers: "scm-sco, not scm-eqc2",
        turtle: "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
        expect: &["+V ex:A rdfs:subClassOf ex:A", "- ex:A owl:equivalentClass ex:B"],
    },
    // Rules version 1 does not implement add nothing.
    Case {
        group: GROUP,
        name: "not-scm-spo",
        covers: "not scm-spo",
        turtle: "ex:p1 rdfs:subPropertyOf ex:p2 . ex:p2 rdfs:subPropertyOf ex:p3 .",
        expect: &["- ex:p1 rdfs:subPropertyOf ex:p3", "+ ex:p1 rdfs:subPropertyOf ex:p2"],
    },
];
