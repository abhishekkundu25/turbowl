//! The `schema` group's minimal inputs: scm-eqc2, scm-spo, scm-eqp1,
//! scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv, scm-svf1,
//! scm-svf2, scm-avf1, scm-avf2, scm-int and scm-uni, one or more each.
//!
//! OWL-RL implements every schema rule, so most expectations hold in its
//! closure too (`+`). It never closes a cycle reflexively (fault F1), so the
//! reflexive triples a cycle gives, and their scm-eqc2 and scm-eqp2
//! consequences, are expected of the configuration's rules only (`+V`).
//!
//! The inputs whose rules infer class or property subsumptions are written
//! in OWL 2 DL, so that HermiT judges them: properties and classes are
//! declared, and each restriction is a blank node with a named class
//! equivalent to it (scm-eqc1 and scm-sco carry its subsumptions to the
//! names). An undeclared property or an IRI restriction is not parsed as
//! one, and HermiT then entails nothing about it.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::Schema);

pub(super) const CASES: &[Case] = &[
    // scm-sco through a cycle gives A subClassOf A (OWL-RL does not: F1),
    // and scm-eqc2 the equivalences, A owl:equivalentClass A among them.
    Case {
        group: GROUP,
        name: "scm-sco-cycle",
        covers: "scm-sco, scm-eqc2",
        turtle: "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
        expect: &[
            "+V ex:A rdfs:subClassOf ex:A",
            "+ ex:A owl:equivalentClass ex:B",
            "+ ex:B owl:equivalentClass ex:A",
            "+V ex:A owl:equivalentClass ex:A",
            "+V ex:B owl:equivalentClass ex:B",
        ],
    },
    // A subclass pair one way only: no equivalence.
    Case {
        group: GROUP,
        name: "scm-eqc2-one-way",
        covers: "scm-eqc2",
        turtle: "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C .",
        expect: &["- ex:A owl:equivalentClass ex:B", "- ex:B owl:equivalentClass ex:A"],
    },
    Case {
        group: GROUP,
        name: "scm-spo",
        covers: "scm-spo, prp-spo1",
        turtle: "ex:p1 a owl:ObjectProperty . ex:p2 a owl:ObjectProperty . ex:p3 a owl:ObjectProperty .
                 ex:p1 rdfs:subPropertyOf ex:p2 . ex:p2 rdfs:subPropertyOf ex:p3 . ex:a ex:p1 ex:b .",
        expect: &[
            "+ ex:p1 rdfs:subPropertyOf ex:p3",
            "+ ex:p1 rdfs:subPropertyOf ex:p2",
            "+ ex:a ex:p3 ex:b",
            "- ex:p3 rdfs:subPropertyOf ex:p1",
        ],
    },
    // scm-eqp1 gives both subproperty pairs; scm-spo closes the cycle
    // reflexively, and scm-eqp2 makes the pairs equivalent. (OWL-RL has the
    // reflexive triples here only through scm-op on the declarations: F1.)
    Case {
        group: GROUP,
        name: "scm-eqp1",
        covers: "scm-eqp1, scm-spo, scm-eqp2",
        turtle: "ex:p a owl:ObjectProperty . ex:q a owl:ObjectProperty . ex:p owl:equivalentProperty ex:q .",
        expect: &[
            "+ ex:p rdfs:subPropertyOf ex:q",
            "+ ex:q rdfs:subPropertyOf ex:p",
            "+ ex:q owl:equivalentProperty ex:p",
            "+V ex:p rdfs:subPropertyOf ex:p",
            "+V ex:p owl:equivalentProperty ex:p",
        ],
    },
    Case {
        group: GROUP,
        name: "scm-eqp2",
        covers: "scm-eqp2",
        turtle: "ex:p a owl:ObjectProperty . ex:q a owl:ObjectProperty . ex:r a owl:ObjectProperty .
                 ex:p rdfs:subPropertyOf ex:q . ex:q rdfs:subPropertyOf ex:p . ex:r rdfs:subPropertyOf ex:p .",
        expect: &[
            "+ ex:p owl:equivalentProperty ex:q",
            "+ ex:q owl:equivalentProperty ex:p",
            "- ex:r owl:equivalentProperty ex:p",
        ],
    },
    Case {
        group: GROUP,
        name: "scm-dom1",
        covers: "scm-dom1, prp-dom",
        turtle: "ex:p rdfs:domain ex:A . ex:A rdfs:subClassOf ex:B . ex:x ex:p ex:y .",
        expect: &[
            "+ ex:p rdfs:domain ex:B",
            "+ ex:x rdf:type ex:B",
            "- ex:y rdf:type ex:B",
        ],
    },
    Case {
        group: GROUP,
        name: "scm-dom2",
        covers: "scm-dom2, prp-dom",
        turtle: "ex:p rdfs:domain ex:A . ex:q rdfs:subPropertyOf ex:p . ex:x ex:q ex:y .",
        expect: &[
            "+ ex:q rdfs:domain ex:A",
            "+ ex:x rdf:type ex:A",
            "- ex:p rdfs:range ex:A",
        ],
    },
    Case {
        group: GROUP,
        name: "scm-rng1",
        covers: "scm-rng1, prp-rng",
        turtle: "ex:p rdfs:range ex:A . ex:A rdfs:subClassOf ex:B . ex:x ex:p ex:y .",
        expect: &["+ ex:p rdfs:range ex:B", "+ ex:y rdf:type ex:B", "- ex:x rdf:type ex:B"],
    },
    Case {
        group: GROUP,
        name: "scm-rng2",
        covers: "scm-rng2, prp-rng",
        turtle: "ex:p rdfs:range ex:A . ex:q rdfs:subPropertyOf ex:p . ex:x ex:q ex:y .",
        expect: &[
            "+ ex:q rdfs:range ex:A",
            "+ ex:y rdf:type ex:A",
            "- ex:p rdfs:domain ex:A",
        ],
    },
    // Equal values on a subproperty and its superproperty; a third
    // restriction with another value is unrelated.
    Case {
        group: GROUP,
        name: "scm-hv",
        covers: "scm-hv, scm-eqc1, scm-sco",
        turtle: "ex:p a owl:ObjectProperty . ex:q a owl:ObjectProperty . ex:p rdfs:subPropertyOf ex:q .
                 ex:v a owl:NamedIndividual . ex:w a owl:NamedIndividual .
                 ex:R1 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:hasValue ex:v ; owl:onProperty ex:p ] .
                 ex:R2 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:hasValue ex:v ; owl:onProperty ex:q ] .
                 ex:R3 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:hasValue ex:w ; owl:onProperty ex:q ] .",
        expect: &[
            "+ ex:R1 rdfs:subClassOf ex:R2",
            "- ex:R2 rdfs:subClassOf ex:R1",
            "- ex:R1 rdfs:subClassOf ex:R3",
        ],
    },
    // The subsumption reaches the data: x, of R1, is of R2.
    Case {
        group: GROUP,
        name: "scm-svf1",
        covers: "scm-svf1, scm-eqc1, scm-sco, cax-sco",
        turtle: "ex:p a owl:ObjectProperty . ex:A a owl:Class . ex:B a owl:Class . ex:A rdfs:subClassOf ex:B .
                 ex:R1 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:someValuesFrom ex:A ; owl:onProperty ex:p ] .
                 ex:R2 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:someValuesFrom ex:B ; owl:onProperty ex:p ] .
                 ex:x a ex:R1 .",
        expect: &[
            "+ ex:R1 rdfs:subClassOf ex:R2",
            "- ex:R2 rdfs:subClassOf ex:R1",
            "+ ex:x rdf:type ex:R2",
        ],
    },
    // One property and one filler: scm-svf1 needs A subClassOf A, which
    // only scm-cls (the `trivial` group, off by default) gives.
    Case {
        group: GROUP,
        name: "scm-svf1-one-filler",
        covers: "scm-svf1",
        turtle: "ex:r1 owl:someValuesFrom ex:A ; owl:onProperty ex:p .
                 ex:r2 owl:someValuesFrom ex:A ; owl:onProperty ex:p .",
        expect: &["- ex:r1 rdfs:subClassOf ex:r2", "- ex:r2 rdfs:subClassOf ex:r1"],
    },
    Case {
        group: GROUP,
        name: "scm-svf2",
        covers: "scm-svf2, scm-eqc1, scm-sco",
        turtle: "ex:p a owl:ObjectProperty . ex:q a owl:ObjectProperty . ex:p rdfs:subPropertyOf ex:q . ex:A a owl:Class .
                 ex:R1 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:someValuesFrom ex:A ; owl:onProperty ex:p ] .
                 ex:R2 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:someValuesFrom ex:A ; owl:onProperty ex:q ] .",
        expect: &["+ ex:R1 rdfs:subClassOf ex:R2", "- ex:R2 rdfs:subClassOf ex:R1"],
    },
    Case {
        group: GROUP,
        name: "scm-avf1",
        covers: "scm-avf1, scm-eqc1, scm-sco, cls-avf",
        turtle: "ex:p a owl:ObjectProperty . ex:A a owl:Class . ex:B a owl:Class . ex:A rdfs:subClassOf ex:B .
                 ex:R1 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:allValuesFrom ex:A ; owl:onProperty ex:p ] .
                 ex:R2 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:allValuesFrom ex:B ; owl:onProperty ex:p ] .
                 ex:x a ex:R1 ; ex:p ex:y .",
        expect: &[
            "+ ex:R1 rdfs:subClassOf ex:R2",
            "- ex:R2 rdfs:subClassOf ex:R1",
            "+ ex:y rdf:type ex:B",
        ],
    },
    // The superproperty's restriction is the subclass.
    Case {
        group: GROUP,
        name: "scm-avf2",
        covers: "scm-avf2, scm-eqc1, scm-sco",
        turtle: "ex:p a owl:ObjectProperty . ex:q a owl:ObjectProperty . ex:p rdfs:subPropertyOf ex:q . ex:A a owl:Class .
                 ex:R1 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:allValuesFrom ex:A ; owl:onProperty ex:p ] .
                 ex:R2 a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:allValuesFrom ex:A ; owl:onProperty ex:q ] .",
        expect: &["+ ex:R2 rdfs:subClassOf ex:R1", "- ex:R1 rdfs:subClassOf ex:R2"],
    },
    Case {
        group: GROUP,
        name: "scm-int",
        covers: "scm-int",
        turtle: "ex:C owl:intersectionOf ( ex:A ex:B ) .",
        expect: &[
            "+ ex:C rdfs:subClassOf ex:A",
            "+ ex:C rdfs:subClassOf ex:B",
            "- ex:A rdfs:subClassOf ex:C",
        ],
    },
    Case {
        group: GROUP,
        name: "scm-uni",
        covers: "scm-uni",
        turtle: "ex:C owl:unionOf ( ex:A ex:B ) .",
        expect: &[
            "+ ex:A rdfs:subClassOf ex:C",
            "+ ex:B rdfs:subClassOf ex:C",
            "- ex:C rdfs:subClassOf ex:A",
        ],
    },
    // A list whose last cell has no rdf:first has no members.
    Case {
        group: GROUP,
        name: "scm-int-ill-formed",
        covers: "scm-int",
        turtle: "ex:C owl:intersectionOf _:l1 . _:l1 rdf:first ex:A ; rdf:rest _:l2 . _:l2 rdf:rest rdf:nil .",
        expect: &["- ex:C rdfs:subClassOf ex:A"],
    },
];
