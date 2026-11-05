//! The `identity` group's minimal inputs: prp-key, cls-maxc2, cls-maxqc3
//! and cls-maxqc4.
//!
//! OWL-RL (owlrl 7.6.2) derives more than these rules on some of them: its
//! prp-key leaves the last property of a key unchecked, and its cardinality
//! rules compare the number's value, not the term. So what must not be
//! derived is written `-`, which OWL-RL is not held to.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::Identity);

pub(super) const CASES: &[Case] = &[
    // New-Feature-Keys-003's shape: a key of one data property. An
    // individual with the value but not of the class, and one of the class
    // with another value, stay apart; each keyed individual is the same as
    // itself.
    Case {
        group: GROUP,
        name: "prp-key",
        covers: "prp-key",
        turtle: "ex:Person owl:hasKey ( ex:ssn ) .
            ex:a a ex:Person ; ex:ssn \"1\" .
            ex:b a ex:Person ; ex:ssn \"1\" .
            ex:c ex:ssn \"1\" .
            ex:d a ex:Person ; ex:ssn \"2\" .",
        expect: &[
            "+ ex:a owl:sameAs ex:b",
            "+ ex:b owl:sameAs ex:a",
            "+V ex:d owl:sameAs ex:d",
            "- ex:a owl:sameAs ex:c",
            "- ex:a owl:sameAs ex:d",
        ],
    },
    // A key of two properties: both must share a value, any of the values;
    // an individual without the second property has no key.
    Case {
        group: GROUP,
        name: "prp-key-two-properties",
        covers: "prp-key",
        turtle: "ex:P owl:hasKey ( ex:first ex:last ) .
            ex:a a ex:P ; ex:first ex:n1 ; ex:last ex:n2 .
            ex:b a ex:P ; ex:first ex:n1 ; ex:last ex:n2 .
            ex:c a ex:P ; ex:first ex:n1 ; ex:last ex:n3 .
            ex:d a ex:P ; ex:first ex:n4 , ex:n1 ; ex:last ex:n2 .
            ex:e a ex:P ; ex:first ex:n1 .",
        expect: &[
            "+ ex:a owl:sameAs ex:b",
            "+ ex:a owl:sameAs ex:d",
            "+ ex:d owl:sameAs ex:b",
            "- ex:a owl:sameAs ex:c",
            "- ex:a owl:sameAs ex:e",
        ],
    },
    // A key value made equal by owl:sameAs: the owners are the same.
    Case {
        group: GROUP,
        name: "prp-key-equal-values",
        covers: "prp-key, eq-rep-o",
        turtle: "ex:P owl:hasKey ( ex:k ) .
            ex:a a ex:P ; ex:k ex:v1 .
            ex:b a ex:P ; ex:k ex:v2 .
            ex:v1 owl:sameAs ex:v2 .",
        expect: &["+ ex:a owl:sameAs ex:b", "+ ex:b owl:sameAs ex:a"],
    },
    Case {
        group: GROUP,
        name: "cls-maxc2",
        covers: "cls-maxc2",
        turtle: "ex:R owl:maxCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .
            ex:u a ex:R ; ex:p ex:y1 , ex:y2 .
            ex:v ex:p ex:z1 , ex:z2 .",
        expect: &[
            "+ ex:y1 owl:sameAs ex:y2",
            "+ ex:y2 owl:sameAs ex:y1",
            "+ ex:y1 owl:sameAs ex:y1",
            "- ex:z1 owl:sameAs ex:z2",
        ],
    },
    // The number is the term "1"^^xsd:nonNegativeInteger: 2, or 1 as an
    // xsd:integer, fires nothing.
    Case {
        group: GROUP,
        name: "cls-maxc2-other-numbers",
        covers: "cls-maxc2",
        turtle: "ex:S owl:maxCardinality \"2\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .
            ex:T owl:maxCardinality \"1\"^^xsd:integer ; owl:onProperty ex:p .
            ex:u a ex:S , ex:T ; ex:p ex:y1 , ex:y2 .",
        expect: &["- ex:y1 owl:sameAs ex:y2", "- ex:y2 owl:sameAs ex:y1"],
    },
    // Only the values typed with the class count.
    Case {
        group: GROUP,
        name: "cls-maxqc3",
        covers: "cls-maxqc3",
        turtle: "ex:R owl:maxQualifiedCardinality \"1\"^^xsd:nonNegativeInteger ;
                owl:onProperty ex:p ; owl:onClass ex:C .
            ex:u a ex:R ; ex:p ex:y1 , ex:y2 , ex:y3 .
            ex:y1 a ex:C . ex:y2 a ex:C .",
        expect: &[
            "+ ex:y1 owl:sameAs ex:y2",
            "+ ex:y2 owl:sameAs ex:y1",
            "- ex:y1 owl:sameAs ex:y3",
        ],
    },
    Case {
        group: GROUP,
        name: "cls-maxqc4",
        covers: "cls-maxqc4",
        turtle: "ex:R owl:maxQualifiedCardinality \"1\"^^xsd:nonNegativeInteger ;
                owl:onProperty ex:p ; owl:onClass owl:Thing .
            ex:u a ex:R ; ex:p ex:y1 , ex:y2 .",
        expect: &["+ ex:y1 owl:sameAs ex:y2", "+ ex:y2 owl:sameAs ex:y1"],
    },
];
