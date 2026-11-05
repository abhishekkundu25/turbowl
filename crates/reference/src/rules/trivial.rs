//! The `trivial` switch, off by default: triples that hold of every term,
//! class, property or literal, eq-ref, scm-cls, scm-op, scm-dp, prp-ap,
//! dt-type1 and dt-type2 (OWL 2 Profiles, Tables 4, 5, 8 and 9), as the W3C
//! tables write them. eq-ref is a rule like the others here: its triples
//! join every rule (the core adds them only when its closure is read, and
//! stores them where a rule could read them).
//!
//! To come: dt-type2, version 2's ticket 10, which reads the datatype map
//! (`crate::datatype`).

use super::{EMPTY, Family};
use crate::datatype::datatype_iris;
use crate::evaluate::{Round, View};
use crate::vocab::*;

const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";

/// The annotation properties of prp-ap.
const ANNOTATION_PROPERTIES: [&str; 9] = [
    "http://www.w3.org/2000/01/rdf-schema#label",
    "http://www.w3.org/2000/01/rdf-schema#comment",
    "http://www.w3.org/2000/01/rdf-schema#seeAlso",
    "http://www.w3.org/2000/01/rdf-schema#isDefinedBy",
    "http://www.w3.org/2002/07/owl#deprecated",
    "http://www.w3.org/2002/07/owl#versionInfo",
    "http://www.w3.org/2002/07/owl#priorVersion",
    "http://www.w3.org/2002/07/owl#backwardCompatibleWith",
    "http://www.w3.org/2002/07/owl#incompatibleWith",
];

pub(super) const FAMILY: Family = Family {
    rules: &["eq-ref", "prp-ap", "dt-type1", "scm-cls", "scm-op", "scm-dp"],
    derive,
    ..EMPTY
};

fn derive(v: &View, r: &mut Round) {
    let same = iri(OWL_SAME_AS);
    let ty = iri(RDF_TYPE);
    for (s, p, o) in &v.all {
        for t in [s, p, o] {
            r.derive("eq-ref", t, &same, t);
        }
    }
    let annotation_property = iri(&format!("{OWL}AnnotationProperty"));
    for p in ANNOTATION_PROPERTIES {
        r.derive("prp-ap", &iri(p), &ty, &annotation_property);
    }
    let datatype = iri(&format!("{RDFS}Datatype"));
    for dt in datatype_iris() {
        r.derive("dt-type1", &iri(&dt), &ty, &datatype);
    }
    let (sco, eqc) = (iri(RDFS_SUB_CLASS_OF), iri(OWL_EQUIVALENT_CLASS));
    let (thing, nothing) = (iri(OWL_THING), iri(OWL_NOTHING));
    for c in v.instances(OWL_CLASS) {
        r.derive("scm-cls", c, &sco, c);
        r.derive("scm-cls", c, &eqc, c);
        r.derive("scm-cls", c, &sco, &thing);
        r.derive("scm-cls", &nothing, &sco, c);
    }
    let (spo, eqp) = (iri(RDFS_SUB_PROPERTY_OF), iri(OWL_EQUIVALENT_PROPERTY));
    for (kind, rule) in [("ObjectProperty", "scm-op"), ("DatatypeProperty", "scm-dp")] {
        for p in v.instances(&format!("{OWL}{kind}")) {
            r.derive(rule, p, &spo, p);
            r.derive(rule, p, &eqp, p);
        }
    }
}
