//! The minimal inputs of dt-not-type (the `checks` group): ill-typed
//! literals of the datatypes of the OWL 2 RL map, and literals a rule types
//! with a datatype whose value space they are not in.
//!
//! The literals of `dt-not-type-map` are ones OWL-RL reads as the W3C
//! datatypes define them; its departures are oracle faults with tests of
//! their own (F3, F7 to F12 in `differences.rs`), and the map's edge cases
//! are tested in the core and the reference evaluator
//! (`crates/turbowl/src/rules/datatype/edge_cases.rs`). No `xsd:token` or
//! `xsd:normalizedString` form that rdflib rewrites (F12) is among them:
//! the Python bindings take rdflib terms, so `crates/python/tests` could
//! not give the bindings the input the Rust library reads.

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
    // Every datatype of the map against its own lexical space: bounds of
    // the integer subtypes, exponents, patterns, time zones.
    Case {
        group: GROUP,
        name: "dt-not-type-map",
        covers: "dt-not-type",
        turtle: "ex:s ex:p \"1e3\"^^xsd:decimal, \"1.0\"^^xsd:integer, \"128\"^^xsd:byte,
                 \"-1\"^^xsd:nonNegativeInteger, \"0\"^^xsd:positiveInteger, \"70000\"^^xsd:unsignedShort,
                 \"9223372036854775808\"^^xsd:long, \"yes\"^^xsd:boolean, \"1e\"^^xsd:double,
                 \"abc\"^^xsd:float, \"en_GB\"^^xsd:language, \"1a\"^^xsd:Name, \"a:b\"^^xsd:NCName,
                 \"a b\"^^xsd:NMTOKEN, \"0G\"^^xsd:hexBinary, \"Q!==\"^^xsd:base64Binary,
                 \"2000-13-01T00:00:00\"^^xsd:dateTime, \"2000-01-01T00:00:00\"^^xsd:dateTimeStamp,
                 \"abc\"^^rdf:PlainLiteral .
                 ex:s ex:q \"-5\"^^xsd:negativeInteger, \"0\"^^xsd:nonPositiveInteger, \"4294967295\"^^xsd:unsignedInt,
                 \"-32768\"^^xsd:short, \"18446744073709551615\"^^xsd:unsignedLong, \"1.5\"^^xsd:decimal,
                 \"127\"^^xsd:byte, \"1.5E3\"^^xsd:double, \"2.5\"^^xsd:float, \"true\"^^xsd:boolean,
                 \"en-GB\"^^xsd:language, \"0FB7\"^^xsd:hexBinary, \"QUJD\"^^xsd:base64Binary,
                 \"http://example.org/\"^^xsd:anyURI, \"2000-01-01T00:00:00Z\"^^xsd:dateTimeStamp,
                 \"2000-01-01T12:30:00.5\"^^xsd:dateTime, \"chat@fr\"^^rdf:PlainLiteral, \"a b\"^^xsd:token,
                 \"_a-1\"^^xsd:NCName, \"a:b\"^^xsd:Name, \"1a\"^^xsd:NMTOKEN, \"a b\"^^xsd:normalizedString .",
        expect: &[
            "! dt-not-type \"1e3\"^^xsd:decimal xsd:decimal",
            "! dt-not-type \"1.0\"^^xsd:integer xsd:integer",
            "! dt-not-type \"128\"^^xsd:byte xsd:byte",
            "! dt-not-type \"-1\"^^xsd:nonNegativeInteger xsd:nonNegativeInteger",
            "! dt-not-type \"0\"^^xsd:positiveInteger xsd:positiveInteger",
            "! dt-not-type \"70000\"^^xsd:unsignedShort xsd:unsignedShort",
            "! dt-not-type \"9223372036854775808\"^^xsd:long xsd:long",
            "! dt-not-type \"yes\"^^xsd:boolean xsd:boolean",
            "! dt-not-type \"1e\"^^xsd:double xsd:double",
            "! dt-not-type \"abc\"^^xsd:float xsd:float",
            "! dt-not-type \"en_GB\"^^xsd:language xsd:language",
            "! dt-not-type \"1a\"^^xsd:Name xsd:Name",
            "! dt-not-type \"a:b\"^^xsd:NCName xsd:NCName",
            "! dt-not-type \"a b\"^^xsd:NMTOKEN xsd:NMTOKEN",
            "! dt-not-type \"0G\"^^xsd:hexBinary xsd:hexBinary",
            "! dt-not-type \"Q!==\"^^xsd:base64Binary xsd:base64Binary",
            "! dt-not-type \"2000-13-01T00:00:00\"^^xsd:dateTime xsd:dateTime",
            "! dt-not-type \"2000-01-01T00:00:00\"^^xsd:dateTimeStamp xsd:dateTimeStamp",
            "! dt-not-type \"abc\"^^rdf:PlainLiteral rdf:PlainLiteral",
        ],
    },
    // A literal typed by prp-rng (OWL-RL does not check these: F5). An
    // IRI object is not a literal, and "5"^^xsd:int is an xsd:integer.
    // The integers are inside the decimals, and a byte holds only its range.
    // (A double in a decimal range is left to the unit tables and chaos:
    // OWL-RL reports that clash as cax-dw, from its own datatype
    // disjointness axioms, which the F5 entry would hide.)
    Case {
        group: GROUP,
        name: "dt-not-type-range",
        covers: "prp-rng, dt-not-type",
        turtle: "ex:p rdfs:range xsd:integer . ex:s ex:p \"abc\", \"x\"@en, ex:o, \"5\"^^xsd:int .
                 ex:q rdfs:range ex:Unknown . ex:s ex:q \"1\" .
                 ex:r rdfs:range xsd:decimal . ex:s ex:r \"2\"^^xsd:int, \"2.5\"^^xsd:decimal .
                 ex:b rdfs:range xsd:byte . ex:s ex:b \"300\"^^xsd:integer, \"5\"^^xsd:unsignedLong .",
        expect: &[
            "+ ex:o rdf:type xsd:integer",
            "! dt-not-type \"abc\" xsd:integer",
            "! dt-not-type \"x\"@en xsd:integer",
            "! dt-not-type \"300\"^^xsd:integer xsd:byte",
        ],
    },
    // The datatypes OWL 2 RL supports (OWL 2 Profiles, Section 4.2.1):
    // owl:real and owl:rational are outside the map, so their literals are
    // never ill-typed; rdf:langString is not checked as a datatype; a
    // language-tagged literal keeps its value (an rdf:PlainLiteral's), so it
    // is not an xsd:string. OWL-RL checks no range-typed literal (F5).
    Case {
        group: GROUP,
        name: "dt-not-type-owl-2-rl",
        covers: "dt-not-type, prp-rng",
        turtle: "ex:s ex:p \"1\"^^owl:real , \"1/3\"^^owl:rational , \"1/0\"^^owl:rational .
                 ex:q rdfs:range xsd:string . ex:s ex:q \"chat\"@fr .
                 ex:r rdfs:range rdf:langString . ex:s ex:r \"abc\" .",
        expect: &["! dt-not-type \"chat\"@fr xsd:string"],
    },
];
