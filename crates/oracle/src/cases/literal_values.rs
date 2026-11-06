//! The `literal-values` group's minimal inputs: dt-eq and dt-diff.
//!
//! The group is off by default, so these expectations are checked in the
//! complete configuration only. OWL-RL implements neither rule (F15), so
//! the expectations are `+V`, the configuration's rules.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::LiteralValues);

pub(super) const CASES: &[Case] = &[
    // Literals of one value are the same: "1"^^xsd:integer, "01"^^xsd:integer
    // and "1.0"^^xsd:decimal (an xsd:integer value is an xsd:decimal value),
    // and the cardinality the rules name; each gets the others' triples.
    Case {
        group: GROUP,
        name: "dt-eq",
        covers: "dt-eq, eq-rep-o",
        turtle:
            "ex:s ex:p \"1\"^^xsd:integer . ex:t ex:q \"01\"^^xsd:integer , \"1.0\"^^xsd:decimal , \"2\"^^xsd:integer ,
                 \"1\"^^xsd:double .",
        expect: &[
            "+V ex:s ex:p \"01\"^^xsd:integer",
            "+V ex:s ex:p \"1.0\"^^xsd:decimal",
            "+V ex:s ex:p \"1\"^^xsd:nonNegativeInteger",
            "+V ex:t ex:q \"1\"^^xsd:integer",
            "- ex:s ex:p \"2\"^^xsd:integer",
            "- ex:s ex:p \"1\"^^xsd:double",
        ],
    },
    // Terms made the same as literals of one value are the same.
    Case {
        group: GROUP,
        name: "dt-eq-iri",
        covers: "dt-eq, eq-sym, eq-trans",
        turtle: "ex:a owl:sameAs \"7\"^^xsd:integer . ex:b owl:sameAs \"7.00\"^^xsd:decimal .
                 ex:c owl:sameAs \"7\"^^xsd:float .",
        expect: &[
            "+V ex:a owl:sameAs ex:b",
            "+V ex:b owl:sameAs ex:a",
            "+V ex:a owl:sameAs \"7.00\"^^xsd:decimal",
            "- ex:a owl:sameAs ex:c",
        ],
    },
    // With dt-eq, an xsd:integer 1 is the cardinality cls-maxc2 names.
    Case {
        group: GROUP,
        name: "dt-eq-cardinality",
        covers: "dt-eq, eq-rep-o, cls-maxc2",
        turtle: "ex:T owl:maxCardinality \"1\"^^xsd:integer ; owl:onProperty ex:p .
                 ex:u a ex:T ; ex:p ex:y1 , ex:y2 .",
        expect: &[
            "+V ex:T owl:maxCardinality \"1\"^^xsd:nonNegativeInteger",
            "+V ex:y1 owl:sameAs ex:y2",
            "+V ex:y2 owl:sameAs ex:y1",
        ],
    },
    // "1"^^xsd:integer owl:sameAs "2"^^xsd:integer (by prp-fp) is reported:
    // with the equality rules, every two members of the class, the
    // cardinality "1" (dt-eq) among them, and each with itself.
    Case {
        group: GROUP,
        name: "dt-diff",
        covers: "dt-diff, eq-diff1, prp-fp",
        turtle: "ex:d a owl:FunctionalProperty . ex:x ex:d \"1\"^^xsd:integer , \"2\"^^xsd:integer .",
        expect: &[
            "! eq-diff1 \"1\"^^xsd:integer \"2\"^^xsd:integer",
            "! eq-diff1 \"2\"^^xsd:integer \"1\"^^xsd:integer",
            "! eq-diff1 \"1\"^^xsd:integer \"1\"^^xsd:integer",
            "! eq-diff1 \"2\"^^xsd:integer \"2\"^^xsd:integer",
            "! eq-diff1 \"1\"^^xsd:nonNegativeInteger \"1\"^^xsd:nonNegativeInteger",
            "! eq-diff1 \"1\"^^xsd:nonNegativeInteger \"1\"^^xsd:integer",
            "! eq-diff1 \"1\"^^xsd:nonNegativeInteger \"2\"^^xsd:integer",
            "! eq-diff1 \"1\"^^xsd:integer \"1\"^^xsd:nonNegativeInteger",
            "! eq-diff1 \"2\"^^xsd:integer \"1\"^^xsd:nonNegativeInteger",
        ],
    },
    // dt-diff is never listed: no owl:differentFrom between literals, nor
    // what eq-rep-s makes of it for a term equal to one.
    Case {
        group: GROUP,
        name: "dt-diff-never-listed",
        covers: "dt-diff",
        turtle: "ex:a owl:sameAs \"5\"^^xsd:integer . ex:b ex:p \"6\"^^xsd:integer .",
        expect: &[
            "- ex:a owl:differentFrom \"6\"^^xsd:integer",
            "- ex:a owl:differentFrom ex:a",
        ],
    },
];
