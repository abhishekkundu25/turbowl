//! The `checks` group's minimal inputs: one per rule that concludes `false`
//! (cax-dw, cax-adc, prp-pdw, prp-adp, prp-asyp, prp-irp, prp-npa1,
//! prp-npa2, cls-nothing2, cls-com, cls-maxc1, cls-maxqc1, cls-maxqc2,
//! eq-diff1, eq-diff2, eq-diff3).
//! dt-not-type, in the same group, has its inputs in `datatypes.rs`.
//!
//! The equality inputs run with `equality` on (both configurations), so a
//! finding names every member of a class.

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
    // A class of two: every pair of its members is the same and different.
    Case {
        group: GROUP,
        name: "eq-diff1",
        covers: "eq-diff1",
        turtle: "ex:a owl:differentFrom ex:b . ex:a owl:sameAs ex:b .",
        expect: &[
            "! eq-diff1 ex:a ex:a",
            "! eq-diff1 ex:a ex:b",
            "! eq-diff1 ex:b ex:a",
            "! eq-diff1 ex:b ex:b",
        ],
    },
    // OWL-RL reads one rdf:first per cell and skips zi = zj (F6).
    Case {
        group: GROUP,
        name: "eq-diff2",
        covers: "eq-diff2",
        turtle: "ex:d a owl:AllDifferent ; owl:members ( ex:a ex:b ex:c ) . ex:a owl:sameAs ex:b .",
        expect: &[
            "! eq-diff2 ex:d ex:a ex:a",
            "! eq-diff2 ex:d ex:a ex:b",
            "! eq-diff2 ex:d ex:b ex:a",
            "! eq-diff2 ex:d ex:b ex:b",
        ],
    },
    // OWL-RL reads one rdf:first per cell and skips zi = zj (F6).
    Case {
        group: GROUP,
        name: "eq-diff3",
        covers: "eq-diff3",
        turtle: "ex:d a owl:AllDifferent ; owl:distinctMembers ( ex:a ex:b ex:c ) . ex:a owl:sameAs ex:b .",
        expect: &[
            "! eq-diff3 ex:d ex:a ex:a",
            "! eq-diff3 ex:d ex:a ex:b",
            "! eq-diff3 ex:d ex:b ex:a",
            "! eq-diff3 ex:d ex:b ex:b",
        ],
    },
    Case {
        group: GROUP,
        name: "cax-adc",
        covers: "cax-adc",
        turtle: "ex:x a owl:AllDisjointClasses ; owl:members ( ex:A ex:B ex:C ) .
            ex:i a ex:A, ex:C . ex:j a ex:B .",
        expect: &["! cax-adc ex:i ex:A ex:C"],
    },
    // Three properties: OWL-RL pairs only the first two (F6).
    Case {
        group: GROUP,
        name: "prp-adp",
        covers: "prp-adp",
        turtle: "ex:x a owl:AllDisjointProperties ; owl:members ( ex:p ex:q ex:r ) .
            ex:u ex:p ex:v ; ex:q ex:v . ex:u ex:r ex:w .",
        expect: &["! prp-adp ex:p ex:q ex:u ex:v"],
    },
    Case {
        group: GROUP,
        name: "prp-npa1",
        covers: "prp-npa1",
        turtle: "ex:n a owl:NegativePropertyAssertion ; owl:sourceIndividual ex:a ;
            owl:assertionProperty ex:p ; owl:targetIndividual ex:b .
            ex:a ex:p ex:b , ex:c .",
        expect: &["! prp-npa1 ex:a ex:p ex:b"],
    },
    Case {
        group: GROUP,
        name: "prp-npa2",
        covers: "prp-npa2",
        turtle: "ex:n a owl:NegativePropertyAssertion ; owl:sourceIndividual ex:a ;
            owl:assertionProperty ex:age ; owl:targetValue \"5\"^^xsd:integer .
            ex:a ex:age \"5\"^^xsd:integer , \"6\"^^xsd:integer .",
        expect: &["! prp-npa2 ex:a ex:age \"5\"^^xsd:integer"],
    },
    Case {
        group: GROUP,
        name: "cls-maxc1",
        covers: "cls-maxc1",
        turtle: "ex:R owl:maxCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .
            ex:u a ex:R ; ex:p ex:y . ex:w a ex:R . ex:v ex:p ex:y .",
        expect: &["! cls-maxc1 ex:R ex:u ex:y"],
    },
    Case {
        group: GROUP,
        name: "cls-maxqc1",
        covers: "cls-maxqc1",
        turtle: "ex:R owl:maxQualifiedCardinality \"0\"^^xsd:nonNegativeInteger ;
            owl:onProperty ex:p ; owl:onClass ex:C .
            ex:u a ex:R ; ex:p ex:y , ex:z . ex:y a ex:C .",
        expect: &["! cls-maxqc1 ex:R ex:u ex:y"],
    },
    Case {
        group: GROUP,
        name: "cls-maxqc2",
        covers: "cls-maxqc2",
        turtle: "ex:R owl:maxQualifiedCardinality \"0\"^^xsd:nonNegativeInteger ;
            owl:onProperty ex:p ; owl:onClass owl:Thing .
            ex:u a ex:R ; ex:p ex:y .",
        expect: &["! cls-maxqc2 ex:R ex:u ex:y"],
    },
];
