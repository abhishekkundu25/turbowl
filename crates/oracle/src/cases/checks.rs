//! The `checks` group's minimal inputs: one per rule that concludes `false`
//! (cax-dw, prp-pdw, prp-asyp, prp-irp, cls-nothing2, cls-com). dt-not-type,
//! in the same group, has its inputs in `datatypes.rs`.
//!
//! To come, in version 2's ticket 05: cax-adc, cls-maxc1, cls-maxqc1,
//! cls-maxqc2, prp-adp, prp-npa1, prp-npa2, eq-diff1, eq-diff2, eq-diff3.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::Checks);

pub(super) const CASES: &[Case] = &[
    // Inconsistencies.
    Case {
        group: GROUP,
        name: "cax-dw",
        covers: "cax-dw",
        turtle: "ex:A owl:disjointWith ex:B . ex:i a ex:A, ex:B .",
        expect: &["! cax-dw ex:i ex:A ex:B"],
    },
    Case {
        group: GROUP,
        name: "prp-pdw",
        covers: "prp-pdw",
        turtle: "ex:p owl:propertyDisjointWith ex:q . ex:x ex:p ex:y ; ex:q ex:y .",
        expect: &["! prp-pdw ex:p ex:q ex:x ex:y"],
    },
    Case {
        group: GROUP,
        name: "prp-asyp",
        covers: "prp-asyp",
        turtle: "ex:p a owl:AsymmetricProperty . ex:x ex:p ex:y . ex:y ex:p ex:x .",
        expect: &["! prp-asyp ex:p ex:x ex:y", "! prp-asyp ex:p ex:y ex:x"],
    },
    Case {
        group: GROUP,
        name: "prp-irp",
        covers: "prp-irp",
        turtle: "ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:x .",
        expect: &["! prp-irp ex:p ex:x"],
    },
    Case {
        group: GROUP,
        name: "cls-nothing2",
        covers: "cls-nothing2",
        turtle: "ex:x a owl:Nothing .",
        expect: &["! cls-nothing2 ex:x"],
    },
    // OWL-RL names ex:C2 for ex:x (F2).
    Case {
        group: GROUP,
        name: "cls-com",
        covers: "cls-com",
        turtle: "ex:C1 owl:complementOf ex:C2 . ex:x a ex:C1, ex:C2 .",
        expect: &["! cls-com ex:C1 ex:C2 ex:x"],
    },
];
