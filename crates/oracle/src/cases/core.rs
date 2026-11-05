//! The core's minimal inputs: the class and property rules of version 1,
//! and how lists and literals are read.
//!
//! To come: cls-oo (`owl:oneOf`), version 2's ticket 08.

use super::Case;
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = None;

pub(super) const CASES: &[Case] = &[
    // cls-thing and cls-nothing1 are the only triples of an empty input.
    Case {
        group: GROUP,
        name: "empty",
        covers: "cls-thing, cls-nothing1",
        turtle: "",
        expect: &[
            "= owl:Thing rdf:type owl:Class",
            "= owl:Nothing rdf:type owl:Class",
            "+ owl:Thing rdf:type owl:Class",
            "+ owl:Nothing rdf:type owl:Class",
        ],
    },
    // No rule types every subject owl:Thing.
    Case {
        group: GROUP,
        name: "no-thing-typing",
        covers: "cls-thing",
        turtle: "ex:a ex:p ex:b .",
        expect: &[
            "= ex:a ex:p ex:b",
            "= owl:Thing rdf:type owl:Class",
            "= owl:Nothing rdf:type owl:Class",
        ],
    },
    Case {
        group: GROUP,
        name: "prp-dom",
        covers: "prp-dom",
        turtle: "ex:p rdfs:domain ex:C . ex:a ex:p ex:b .",
        expect: &["+ ex:a rdf:type ex:C", "- ex:b rdf:type ex:C"],
    },
    // A class range with an IRI object and a literal object: typing only,
    // no dt-not-type (ex:C is not a datatype).
    Case {
        group: GROUP,
        name: "prp-rng",
        covers: "prp-rng, dt-not-type",
        turtle: "ex:p rdfs:range ex:C . ex:a ex:p ex:b . ex:a ex:p \"lit\" .",
        expect: &["+ ex:b rdf:type ex:C", "- ex:a rdf:type ex:C"],
    },
    // A literal value: `"v" owl:sameAs ex:o1` has a literal subject and is
    // left out of the closure; its mirror is kept.
    Case {
        group: GROUP,
        name: "prp-fp",
        covers: "prp-fp, eq-sym, eq-trans",
        turtle: "ex:p a owl:FunctionalProperty . ex:a ex:p ex:o1 . ex:a ex:p ex:o2 . ex:a ex:p \"v\" .",
        expect: &[
            "+ ex:o1 owl:sameAs ex:o2",
            "+ ex:o2 owl:sameAs ex:o1",
            "+ ex:o1 owl:sameAs ex:o1",
            "+ ex:o1 owl:sameAs \"v\"",
        ],
    },
    Case {
        group: GROUP,
        name: "prp-ifp",
        covers: "prp-ifp",
        turtle: "ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:o . ex:b ex:p ex:o .",
        expect: &["+ ex:a owl:sameAs ex:b", "+ ex:b owl:sameAs ex:a"],
    },
    // Different objects: nothing is equal.
    Case {
        group: GROUP,
        name: "prp-ifp-different-objects",
        covers: "prp-ifp",
        turtle: "ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:o1 . ex:b ex:p ex:o2 .",
        expect: &["- ex:a owl:sameAs ex:b", "- ex:b owl:sameAs ex:a"],
    },
    Case {
        group: GROUP,
        name: "prp-symp",
        covers: "prp-symp",
        turtle: "ex:p a owl:SymmetricProperty . ex:a ex:p ex:b .",
        expect: &["+ ex:b ex:p ex:a"],
    },
    Case {
        group: GROUP,
        name: "prp-trp",
        covers: "prp-trp",
        turtle: "ex:p a owl:TransitiveProperty . ex:a ex:p ex:b . ex:b ex:p ex:c . ex:c ex:p ex:d .",
        expect: &["+ ex:a ex:p ex:c", "+ ex:a ex:p ex:d", "- ex:d ex:p ex:a"],
    },
    Case {
        group: GROUP,
        name: "prp-spo1",
        covers: "prp-spo1",
        turtle: "ex:p rdfs:subPropertyOf ex:q . ex:q rdfs:subPropertyOf ex:r . ex:a ex:p ex:b .",
        expect: &["+ ex:a ex:q ex:b", "+ ex:a ex:r ex:b", "- ex:a ex:p ex:c"],
    },
    Case {
        group: GROUP,
        name: "prp-eqp",
        covers: "prp-eqp1, prp-eqp2",
        turtle: "ex:p owl:equivalentProperty ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
        expect: &["+ ex:a ex:q ex:b", "+ ex:c ex:p ex:d"],
    },
    Case {
        group: GROUP,
        name: "prp-inv",
        covers: "prp-inv1, prp-inv2",
        turtle: "ex:p owl:inverseOf ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
        expect: &["+ ex:b ex:q ex:a", "+ ex:d ex:p ex:c"],
    },
    Case {
        group: GROUP,
        name: "cls-int",
        covers: "cls-int1, cls-int2",
        turtle: "ex:C owl:intersectionOf (ex:A ex:B) . ex:x a ex:A, ex:B . ex:y a ex:C . ex:z a ex:A .",
        expect: &[
            "+ ex:x rdf:type ex:C",
            "+ ex:y rdf:type ex:A",
            "+ ex:y rdf:type ex:B",
            "- ex:z rdf:type ex:C",
        ],
    },
    // A class with two intersection lists: both count.
    Case {
        group: GROUP,
        name: "two-intersections",
        covers: "cls-int1, cls-int2",
        turtle: "ex:C owl:intersectionOf (ex:A ex:B) . ex:C owl:intersectionOf (ex:D ex:E) .
                 ex:x a ex:A, ex:B . ex:y a ex:D, ex:E . ex:z a ex:A, ex:E .",
        expect: &[
            "+ ex:x rdf:type ex:C",
            "+ ex:y rdf:type ex:C",
            "+ ex:x rdf:type ex:D",
            "+ ex:y rdf:type ex:A",
            "- ex:z rdf:type ex:C",
        ],
    },
    // Two lists that share a tail: each head reads its own list.
    Case {
        group: GROUP,
        name: "shared-list-tail",
        covers: "cls-int1, lists",
        turtle: "ex:I owl:intersectionOf _:l1 . _:l1 rdf:first ex:A ; rdf:rest _:l2 . _:l2 rdf:first ex:B ; rdf:rest rdf:nil .
                 ex:J owl:intersectionOf _:l2 . ex:x a ex:B .",
        expect: &["+ ex:x rdf:type ex:J", "- ex:x rdf:type ex:I"],
    },
    // A list whose only path to rdf:nil passes a cell without an
    // rdf:first matches nothing (the W3C LIST[...] form): A is a member of
    // neither the union nor the intersection.
    Case {
        group: GROUP,
        name: "list-cell-without-first",
        covers: "cls-int1, cls-int2, cls-uni, lists",
        turtle: "ex:U owl:unionOf ex:l1 . ex:I owl:intersectionOf ex:l1 .
                 ex:l1 rdf:first ex:A ; rdf:rest ex:l2 . ex:l2 rdf:rest rdf:nil .
                 ex:x a ex:A . ex:y a ex:I .",
        expect: &[
            "- ex:x rdf:type ex:U",
            "- ex:x rdf:type ex:I",
            "- ex:y rdf:type ex:A",
            "- ex:y rdf:type ex:U",
        ],
    },
    // rdf:rest stops at rdf:nil: its own rdf:first and rdf:rest add no
    // member to a list that ends there.
    Case {
        group: GROUP,
        name: "list-rest-stops-at-nil",
        covers: "cls-int1, cls-int2, cls-uni, lists",
        turtle: "ex:U owl:unionOf ex:n1 . ex:I owl:intersectionOf ex:n1 .
                 ex:n1 rdf:first ex:A ; rdf:rest rdf:nil . rdf:nil rdf:first ex:B ; rdf:rest rdf:nil .
                 ex:x a ex:B . ex:y a ex:I . ex:z a ex:A .",
        expect: &[
            "+V ex:y rdf:type ex:A",
            "+V ex:z rdf:type ex:U",
            "+V ex:z rdf:type ex:I",
            "- ex:x rdf:type ex:U",
            "- ex:x rdf:type ex:I",
            "- ex:y rdf:type ex:B",
        ],
    },
    // rdf:nil can head a list and is read like any cell there, but a list
    // that ends at rdf:nil never reads on into it: B is a member of V's
    // union only.
    Case {
        group: GROUP,
        name: "list-nil-head",
        covers: "cls-uni, lists",
        turtle: "ex:U owl:unionOf ex:n1 . ex:n1 rdf:first ex:A ; rdf:rest rdf:nil .
                 ex:V owl:unionOf rdf:nil . rdf:nil rdf:first ex:B ; rdf:rest rdf:nil .
                 ex:x a ex:B . ex:z a ex:A .",
        expect: &["+V ex:x rdf:type ex:V", "+V ex:z rdf:type ex:U", "- ex:x rdf:type ex:U"],
    },
    // List triples derived during the run count: the list's rdf:first
    // comes from prp-spo1, its head from prp-eqp1.
    Case {
        group: GROUP,
        name: "derived-list",
        covers: "cls-int1, cls-uni, lists",
        turtle: "ex:first rdfs:subPropertyOf rdf:first . ex:inter owl:equivalentProperty owl:intersectionOf .
                 ex:C ex:inter _:l . _:l ex:first ex:A ; rdf:rest rdf:nil . ex:x a ex:A .
                 ex:U owl:unionOf _:m . _:m ex:first ex:A ; rdf:rest rdf:nil .",
        expect: &[
            "+V ex:C owl:intersectionOf _:b0",
            "+V ex:x rdf:type ex:C",
            "+V ex:x rdf:type ex:U",
        ],
    },
    Case {
        group: GROUP,
        name: "cls-uni",
        covers: "cls-uni",
        turtle: "ex:C owl:unionOf (ex:A ex:B) . ex:x a ex:A . ex:y a ex:B .",
        expect: &["+ ex:x rdf:type ex:C", "+ ex:y rdf:type ex:C"],
    },
    Case {
        group: GROUP,
        name: "cls-svf",
        covers: "cls-svf1, cls-svf2",
        turtle: "ex:R owl:onProperty ex:p ; owl:someValuesFrom ex:D . ex:T owl:onProperty ex:q ; owl:someValuesFrom owl:Thing .
                 ex:a ex:p ex:b . ex:b a ex:D . ex:c ex:p ex:e . ex:f ex:q ex:g .",
        expect: &["+ ex:a rdf:type ex:R", "- ex:c rdf:type ex:R", "+ ex:f rdf:type ex:T"],
    },
    Case {
        group: GROUP,
        name: "cls-avf",
        covers: "cls-avf",
        turtle: "ex:R owl:onProperty ex:p ; owl:allValuesFrom ex:D . ex:a a ex:R ; ex:p ex:b . ex:c ex:p ex:e .",
        expect: &["+ ex:b rdf:type ex:D", "- ex:e rdf:type ex:D"],
    },
    Case {
        group: GROUP,
        name: "cls-hv",
        covers: "cls-hv1, cls-hv2",
        turtle: "ex:R owl:onProperty ex:p ; owl:hasValue ex:v . ex:a a ex:R . ex:b ex:p ex:v .",
        expect: &["+ ex:a ex:p ex:v", "+ ex:b rdf:type ex:R"],
    },
    Case {
        group: GROUP,
        name: "cax-sco-scm-sco",
        covers: "cax-sco, scm-sco",
        turtle: "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C . ex:x a ex:A .",
        expect: &["+ ex:x rdf:type ex:B", "+ ex:x rdf:type ex:C", "+ ex:A rdfs:subClassOf ex:C"],
    },
    Case {
        group: GROUP,
        name: "cax-eqc-scm-eqc1",
        covers: "cax-eqc1, cax-eqc2, scm-eqc1",
        turtle: "ex:A owl:equivalentClass ex:B . ex:x a ex:A . ex:y a ex:B .",
        expect: &[
            "+ ex:x rdf:type ex:B",
            "+ ex:y rdf:type ex:A",
            "+ ex:A rdfs:subClassOf ex:B",
            "+ ex:B rdfs:subClassOf ex:A",
        ],
    },
    // No container-membership axioms (not OWL 2 RL rules).
    Case {
        group: GROUP,
        name: "no-container-axioms",
        covers: "not rdfs12",
        turtle: "ex:x rdf:_1 ex:y .",
        expect: &[
            "- rdf:_1 rdf:type rdfs:ContainerMembershipProperty",
            "- rdf:_1 rdfs:subPropertyOf rdfs:member",
            "- ex:x rdfs:member ex:y",
        ],
    },
    // Language tags survive derivation.
    Case {
        group: GROUP,
        name: "language-tag",
        covers: "prp-spo1",
        turtle: "ex:p rdfs:subPropertyOf rdfs:label . ex:a ex:p \"chat\"@fr .",
        expect: &["+ ex:a rdfs:label \"chat\"@fr"],
    },
];
