//! The `trivial` group's minimal inputs: eq-ref, scm-cls, scm-op, scm-dp,
//! prp-ap and dt-type1, and eq-diff1 to eq-diff3 where eq-ref gives their
//! `owl:sameAs`. dt-type2 is version 2's ticket 10.
//!
//! The group is off by default, so these expectations are checked in the
//! complete configuration only. There, the cases of other groups that state
//! an exact closure (`=`, the core's `empty` and `no-thing-typing`) leave out
//! the triples this group's rules give ([`derives`]).

use super::Case;
use oxrdf::{Term, Triple};
use turbowl::Switch;

pub(super) const GROUP: Option<Switch> = Some(Switch::Trivial);

pub(super) const CASES: &[Case] = &[
    // Every IRI and blank node of the closure, in any position, the rules'
    // facts and eq-ref's own triples included.
    Case {
        group: GROUP,
        name: "eq-ref",
        covers: "eq-ref",
        turtle: "ex:a ex:p _:b . _:b ex:q \"v\" .",
        expect: &[
            "+ ex:a owl:sameAs ex:a",
            "+ ex:p owl:sameAs ex:p",
            "+ _:b0 owl:sameAs _:b0",
            "+ ex:q owl:sameAs ex:q",
            "+ rdf:type owl:sameAs rdf:type",
            "+ owl:Thing owl:sameAs owl:Thing",
            "+ owl:sameAs owl:sameAs owl:sameAs",
            "- ex:a owl:sameAs ex:b",
        ],
    },
    // A class of equal terms: its members' pairs, and eq-ref for the rest.
    Case {
        group: GROUP,
        name: "eq-ref-equal-terms",
        covers: "eq-ref, eq-sym, eq-trans",
        turtle: "ex:a owl:sameAs ex:b . ex:c ex:p ex:a .",
        expect: &[
            "+ ex:a owl:sameAs ex:a",
            "+ ex:b owl:sameAs ex:a",
            "+ ex:b owl:sameAs ex:b",
            "+ ex:c owl:sameAs ex:c",
            "+ ex:c ex:p ex:b",
            "- ex:c owl:sameAs ex:a",
        ],
    },
    // A schema that reads owl:sameAs as a property reads eq-ref's triples:
    // every term of the closure is in the domain.
    Case {
        group: GROUP,
        name: "eq-ref-read-by-a-rule",
        covers: "eq-ref, prp-dom, prp-spo1",
        turtle: "owl:sameAs rdfs:domain ex:C . owl:sameAs rdfs:subPropertyOf ex:self . ex:a ex:p ex:b .",
        expect: &[
            "+ ex:a rdf:type ex:C",
            "+ ex:p rdf:type ex:C",
            "+ ex:b rdf:type ex:C",
            "+ owl:Thing rdf:type ex:C",
            "+ ex:a ex:self ex:a",
            "+ ex:C ex:self ex:C",
            "- ex:a ex:self ex:b",
        ],
    },
    Case {
        group: GROUP,
        name: "scm-cls",
        covers: "scm-cls",
        turtle: "ex:C a owl:Class . ex:x a ex:D .",
        expect: &[
            "+ ex:C rdfs:subClassOf ex:C",
            "+ ex:C owl:equivalentClass ex:C",
            "+ ex:C rdfs:subClassOf owl:Thing",
            "+ owl:Nothing rdfs:subClassOf ex:C",
            "+ owl:Thing rdfs:subClassOf owl:Thing",
            "+ owl:Nothing rdfs:subClassOf owl:Thing",
            "- ex:D rdfs:subClassOf ex:D",
            "- ex:D rdfs:subClassOf owl:Thing",
        ],
    },
    Case {
        group: GROUP,
        name: "scm-op",
        covers: "scm-op",
        turtle: "ex:p a owl:ObjectProperty . ex:a ex:q ex:b .",
        expect: &[
            "+ ex:p rdfs:subPropertyOf ex:p",
            "+ ex:p owl:equivalentProperty ex:p",
            "- ex:q rdfs:subPropertyOf ex:q",
        ],
    },
    Case {
        group: GROUP,
        name: "scm-dp",
        covers: "scm-dp",
        turtle: "ex:d a owl:DatatypeProperty . ex:a ex:d \"1\"^^xsd:integer .",
        expect: &["+ ex:d rdfs:subPropertyOf ex:d", "+ ex:d owl:equivalentProperty ex:d"],
    },
    // The facts hold of an input that names none of them.
    Case {
        group: GROUP,
        name: "prp-ap",
        covers: "prp-ap",
        turtle: "ex:a ex:p ex:b .",
        expect: &[
            "+ rdfs:label rdf:type owl:AnnotationProperty",
            "+ rdfs:comment rdf:type owl:AnnotationProperty",
            "+ rdfs:seeAlso rdf:type owl:AnnotationProperty",
            "+ rdfs:isDefinedBy rdf:type owl:AnnotationProperty",
            "+ owl:deprecated rdf:type owl:AnnotationProperty",
            "+ owl:versionInfo rdf:type owl:AnnotationProperty",
            "+ owl:priorVersion rdf:type owl:AnnotationProperty",
            "+ owl:backwardCompatibleWith rdf:type owl:AnnotationProperty",
            "+ owl:incompatibleWith rdf:type owl:AnnotationProperty",
            "- ex:p rdf:type owl:AnnotationProperty",
        ],
    },
    // Every datatype OWL 2 RL supports (OWL 2 Profiles, Section 4.2.1), and
    // not owl:real and owl:rational, which it excludes, nor RDF 1.1's
    // rdf:langString (which OWL-RL types).
    Case {
        group: GROUP,
        name: "dt-type1",
        covers: "dt-type1",
        turtle: "ex:a ex:p ex:b .",
        expect: &[
            "+ xsd:string rdf:type rdfs:Datatype",
            "+ xsd:integer rdf:type rdfs:Datatype",
            "+ xsd:dateTimeStamp rdf:type rdfs:Datatype",
            "+ rdf:PlainLiteral rdf:type rdfs:Datatype",
            "+ rdf:XMLLiteral rdf:type rdfs:Datatype",
            "+ rdfs:Literal rdf:type rdfs:Datatype",
            "- owl:real rdf:type rdfs:Datatype",
            "- owl:rational rdf:type rdfs:Datatype",
            "- rdf:langString rdf:type rdfs:Datatype",
            "- xsd:gYear rdf:type rdfs:Datatype",
        ],
    },
    // With eq-ref, a term is owl:sameAs itself: differentFrom itself alone
    // is a finding.
    Case {
        group: GROUP,
        name: "eq-diff1-eq-ref",
        covers: "eq-diff1, eq-ref",
        turtle: "ex:a owl:differentFrom ex:a . ex:b owl:differentFrom ex:c .",
        expect: &["! eq-diff1 ex:a ex:a"],
    },
    // A member listed twice is at two positions; OWL-RL skips zi = zj (F6).
    Case {
        group: GROUP,
        name: "eq-diff2-eq-ref",
        covers: "eq-diff2, eq-diff3, eq-ref",
        turtle: "ex:d a owl:AllDifferent ; owl:members ( ex:a ex:b ex:a ) .
                 ex:e a owl:AllDifferent ; owl:distinctMembers ( ex:c ex:c ) .
                 ex:f a owl:AllDifferent ; owl:members ( ex:a ex:b ) .",
        expect: &["! eq-diff2 ex:d ex:a ex:a", "! eq-diff3 ex:e ex:c ex:c"],
    },
];

/// True when `t` has the form of a head of this group's rules: eq-ref's
/// `x owl:sameAs x`, scm-cls's `c rdfs:subClassOf c`, `c owl:equivalentClass
/// c`, `c rdfs:subClassOf owl:Thing` and `owl:Nothing rdfs:subClassOf c`,
/// scm-op's and scm-dp's `p rdfs:subPropertyOf p` and `p
/// owl:equivalentProperty p`, prp-ap's `p rdf:type owl:AnnotationProperty`
/// and dt-type1's `d rdf:type rdfs:Datatype`.
pub(super) fn derives(t: &Triple) -> bool {
    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
    const OWL: &str = "http://www.w3.org/2002/07/owl#";
    let is = |term: &Term, ns: &str, local: &str| matches!(term, Term::NamedNode(n) if n.as_str().strip_prefix(ns) == Some(local));
    let (subject, p) = (Term::from(t.subject.clone()), Term::from(t.predicate.clone()));
    let reflexive = subject == t.object;
    if ["sameAs", "equivalentClass", "equivalentProperty"]
        .iter()
        .any(|l| is(&p, OWL, l))
        || is(&p, RDFS, "subPropertyOf")
    {
        reflexive
    } else if is(&p, RDFS, "subClassOf") {
        reflexive || is(&t.object, OWL, "Thing") || is(&subject, OWL, "Nothing")
    } else {
        t.predicate.as_str() == RDF_TYPE
            && (is(&t.object, OWL, "AnnotationProperty") || is(&t.object, RDFS, "Datatype"))
    }
}

#[cfg(test)]
mod tests {
    use super::derives;
    use crate::cases::expand;
    use crate::closure::triple;

    #[test]
    fn derives_knows_the_heads_of_the_groups_rules() {
        for t in [
            "ex:a owl:sameAs ex:a",
            "ex:C rdfs:subClassOf ex:C",
            "ex:C owl:equivalentClass ex:C",
            "ex:C rdfs:subClassOf owl:Thing",
            "owl:Nothing rdfs:subClassOf ex:C",
            "ex:p rdfs:subPropertyOf ex:p",
            "ex:p owl:equivalentProperty ex:p",
            "rdfs:label rdf:type owl:AnnotationProperty",
            "xsd:string rdf:type rdfs:Datatype",
        ] {
            assert!(derives(&triple(&expand(t))), "{t}");
        }
        for t in [
            "ex:a owl:sameAs ex:b",
            "ex:C rdfs:subClassOf ex:D",
            "ex:p rdfs:subPropertyOf ex:q",
            "ex:a ex:p ex:a",
            "ex:a rdf:type ex:C",
            "owl:Thing rdf:type owl:Class",
        ] {
            assert!(!derives(&triple(&expand(t))), "{t}");
        }
    }
}
