//! The minimal inputs of dt-not-type (the `checks` group): ill-typed
//! literals, and literals a rule types with a datatype whose value space
//! they are not in. Version 2's ticket 09 adds the other OWL 2 RL datatypes
//! here.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::Checks);

pub(super) const CASES: &[Case] = &[
    // A literal against its own datatype (OWL-RL accepts " 42": F3).
    Case {
        group: GROUP,
        name: "dt-not-type",
        covers: "dt-not-type",
        turtle: "ex:s ex:p \" 42\"^^xsd:int, \"2147483648\"^^xsd:int, \"abc\"^^xsd:integer, \"+5\"^^xsd:integer,
                 \"-7\"^^xsd:int, \"x\"^^ex:unknown .",
        expect: &[
            "! dt-not-type \" 42\"^^xsd:int xsd:int",
            "! dt-not-type \"2147483648\"^^xsd:int xsd:int",
            "! dt-not-type \"abc\"^^xsd:integer xsd:integer",
        ],
    },
    // A literal typed by prp-rng (OWL-RL does not check these: F5). An
    // IRI object is not a literal, and "5"^^xsd:int is an xsd:integer.
    Case {
        group: GROUP,
        name: "dt-not-type-range",
        covers: "prp-rng, dt-not-type",
        turtle: "ex:p rdfs:range xsd:integer . ex:s ex:p \"abc\", \"x\"@en, ex:o, \"5\"^^xsd:int .
                 ex:q rdfs:range ex:Unknown . ex:s ex:q \"1\" .",
        expect: &[
            "+ ex:o rdf:type xsd:integer",
            "! dt-not-type \"abc\" xsd:integer",
            "! dt-not-type \"x\"@en xsd:integer",
        ],
    },
];
