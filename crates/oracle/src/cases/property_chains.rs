//! The `property-chains` group's minimal inputs: prp-spo2, with the list
//! paths it reads.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::PropertyChains);

pub(super) const CASES: &[Case] = &[
    Case {
        group: GROUP,
        name: "prp-spo2",
        covers: "prp-spo2",
        turtle: "ex:p owl:propertyChainAxiom (ex:q ex:r ex:s) . ex:t owl:propertyChainAxiom (ex:q) .
                 ex:a ex:q ex:b . ex:b ex:r ex:c . ex:c ex:s ex:d . ex:b ex:s ex:e .",
        expect: &["+ ex:a ex:p ex:d", "+ ex:a ex:t ex:b", "- ex:a ex:p ex:e", "- ex:b ex:p ex:d"],
    },
    // The chain of BJP-004's shape is not transitivity.
    Case {
        group: GROUP,
        name: "chain-no-transitivity",
        covers: "prp-spo2",
        turtle: "ex:p owl:propertyChainAxiom (ex:p ex:q) . ex:a ex:p ex:b . ex:b ex:q ex:c . ex:c ex:p ex:d .",
        expect: &["+ ex:a ex:p ex:c", "- ex:a ex:p ex:d"],
    },
    // A cyclic chain list never reaches rdf:nil: it matches nothing, and
    // the run terminates (OWL-RL fails on it: F4).
    Case {
        group: GROUP,
        name: "chain-cyclic-list",
        covers: "prp-spo2",
        turtle: "ex:p owl:propertyChainAxiom ex:l1 . ex:l1 rdf:first ex:q ; rdf:rest ex:l2 . ex:l2 rdf:first ex:r ; rdf:rest ex:l1 .
                 ex:a ex:q ex:b . ex:b ex:r ex:c .",
        expect: &["- ex:a ex:p ex:c", "- ex:a ex:p ex:b"],
    },
    // The same for property chains: (q) ends at rdf:nil and does not read
    // on into the chain rdf:nil heads.
    Case {
        group: GROUP,
        name: "chain-nil-head",
        covers: "prp-spo2, lists",
        turtle: "ex:p owl:propertyChainAxiom ex:l1 . ex:l1 rdf:first ex:q ; rdf:rest rdf:nil .
                 ex:p2 owl:propertyChainAxiom rdf:nil . rdf:nil rdf:first ex:r ; rdf:rest rdf:nil .
                 ex:a ex:q ex:b . ex:b ex:r ex:c .",
        expect: &["+V ex:a ex:p ex:b", "+V ex:b ex:p2 ex:c", "- ex:a ex:p ex:c"],
    },
];
