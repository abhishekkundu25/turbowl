//! The `equality` group's minimal inputs: eq-sym, eq-trans, eq-rep-s,
//! eq-rep-p and eq-rep-o, with `owl:sameAs` on vocabulary terms (ADR 0003's
//! amendment), lists and literals. Some also need the core, the
//! `property-chains` group (`eq-nil-cell`) or the `checks` group's findings
//! (`eq-literal-typed`, `eq-findings`; a finding is expected only where its
//! code's group is on).

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::Equality);

pub(super) const CASES: &[Case] = &[
    Case {
        group: GROUP,
        name: "eq-sym-trans",
        covers: "eq-sym, eq-trans",
        turtle: "ex:x owl:sameAs ex:y . ex:y owl:sameAs ex:z .",
        expect: &[
            "+ ex:y owl:sameAs ex:x",
            "+ ex:x owl:sameAs ex:z",
            "+ ex:z owl:sameAs ex:x",
            "+ ex:x owl:sameAs ex:x",
            "+ ex:z owl:sameAs ex:z",
        ],
    },
    Case {
        group: GROUP,
        name: "eq-rep-s-p-o",
        covers: "eq-rep-s, eq-rep-p, eq-rep-o",
        turtle: "ex:a owl:sameAs ex:b . ex:p owl:sameAs ex:q . ex:o owl:sameAs ex:o2 . ex:a ex:p ex:o .",
        expect: &[
            "+ ex:b ex:p ex:o",
            "+ ex:a ex:q ex:o",
            "+ ex:a ex:p ex:o2",
            "+ ex:b ex:q ex:o2",
        ],
    },
    // `owl:sameAs` on vocabulary terms (ADR 0003's amendment). A
    // property the same as rdf:type types, and is typed through.
    Case {
        group: GROUP,
        name: "eq-vocabulary-term",
        covers: "eq-rep-p, eq-rep-s, eq-rep-o, cax-sco",
        turtle: "ex:t owl:sameAs rdf:type . ex:a ex:t ex:C . ex:C rdfs:subClassOf ex:D .",
        expect: &[
            "+V ex:a rdf:type ex:C",
            "+V ex:a rdf:type ex:D",
            "+V ex:a ex:t ex:D",
            "+V rdf:type owl:sameAs ex:t",
            "+V ex:t owl:sameAs ex:t",
        ],
    },
    // Two vocabulary terms made equal: subClassOf axioms are
    // equivalentClass axioms too, and cax-eqc2 types the other way.
    Case {
        group: GROUP,
        name: "eq-two-vocabulary-terms",
        covers: "eq-rep-p, cax-eqc2, scm-eqc1",
        turtle: "rdfs:subClassOf owl:sameAs owl:equivalentClass . ex:A rdfs:subClassOf ex:B . ex:x a ex:B .",
        expect: &[
            "+V ex:A owl:equivalentClass ex:B",
            "+V ex:B rdfs:subClassOf ex:A",
            "+V ex:x rdf:type ex:A",
            "+V owl:equivalentClass owl:sameAs rdfs:subClassOf",
        ],
    },
    // A property the same as owl:sameAs makes its pairs equal.
    Case {
        group: GROUP,
        name: "eq-same-as-a-property",
        covers: "eq-rep-p, eq-rep-s, eq-sym",
        turtle: "ex:same owl:sameAs owl:sameAs . ex:a ex:same ex:b . ex:b ex:p ex:c .",
        expect: &[
            "+V ex:a ex:p ex:c",
            "+V ex:b owl:sameAs ex:a",
            "+V ex:b ex:same ex:a",
            "+V owl:sameAs ex:same ex:same",
        ],
    },
    // A class the same as owl:Thing is a someValuesFrom filler of cls-svf2.
    Case {
        group: GROUP,
        name: "eq-thing",
        covers: "eq-rep-o, cls-svf2",
        turtle: "ex:T owl:sameAs owl:Thing . ex:r owl:someValuesFrom ex:T ; owl:onProperty ex:p . ex:a ex:p ex:b .",
        expect: &["+V ex:r owl:someValuesFrom owl:Thing", "+V ex:a rdf:type ex:r"],
    },
    // A term the same as rdf:nil but not rdf:nil is a list cell (the W3C
    // rules test `?r != rdf:nil`): ex:l's rest ex:n has rdf:nil's first.
    Case {
        group: GROUP,
        name: "eq-nil-cell",
        covers: "eq-rep-o, cls-int2, prp-spo2, lists",
        turtle: "ex:C owl:intersectionOf ex:l . ex:l rdf:first ex:A ; rdf:rest rdf:nil .
                 rdf:nil rdf:first ex:B ; rdf:rest rdf:nil . ex:n owl:sameAs rdf:nil . ex:y a ex:C .
                 ex:p owl:propertyChainAxiom ex:l . ex:u ex:A ex:v . ex:v ex:B ex:w .",
        expect: &[
            "+V ex:y rdf:type ex:A",
            "+V ex:y rdf:type ex:B",
            "+V ex:u ex:p ex:v",
            "+V ex:u ex:p ex:w",
        ],
    },
    // A literal made equal to a typed term is typed: dt-not-type names
    // the literal (OWL-RL does not check typed literals: F5).
    Case {
        group: GROUP,
        name: "eq-literal-typed",
        covers: "eq-rep-s, dt-not-type",
        turtle: "ex:p rdfs:range xsd:integer . ex:s ex:p ex:o . ex:o owl:sameAs \"abc\" .",
        expect: &["+V ex:o owl:sameAs \"abc\"", "! dt-not-type \"abc\" xsd:integer"],
    },
    // Findings name every member of a class.
    Case {
        group: GROUP,
        name: "eq-findings",
        covers: "eq-rep-s, eq-rep-o, prp-irp, cax-dw",
        turtle: "ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:y . ex:x owl:sameAs ex:y .
                 ex:A owl:disjointWith ex:B . ex:i a ex:A , ex:B . ex:i owl:sameAs ex:j .",
        expect: &[
            "! prp-irp ex:p ex:x",
            "! prp-irp ex:p ex:y",
            "! cax-dw ex:i ex:A ex:B",
            "! cax-dw ex:j ex:A ex:B",
        ],
    },
];
