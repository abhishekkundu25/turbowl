//! The `trivial` switch, off by default: triples that hold of every term,
//! class, property or literal, eq-ref, scm-cls, scm-op, scm-dp, prp-ap,
//! dt-type1 and dt-type2 (OWL 2 Profiles, Tables 4, 5, 8 and 9), as the W3C
//! tables write them. eq-ref is a rule like the others here: its triples
//! join every rule (the core adds them only when its closure is read, and
//! stores them where a rule could read them). It ranges over the terms of
//! the closure, which dt-diff's unlisted triples are no part of
//! (`literal_values.rs`). dt-type2 reads the datatype map
//! (`crate::datatype`).

use super::{EMPTY, Family};
use crate::datatype::DataValue;
use crate::evaluate::{Round, View};
use crate::vocab::*;
use oxrdf::Term;
use std::collections::HashSet;

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

/// The datatypes OWL 2 RL supports, as OWL 2 Profiles (Section 4.2.1) lists
/// them: dt-type1's. Not `owl:real` and `owl:rational`, which OWL 2 RL
/// excludes, nor RDF 1.1's `rdf:langString`.
const OWL_2_RL_DATATYPES: [&str; 32] = [
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral",
    "http://www.w3.org/2000/01/rdf-schema#Literal",
    "http://www.w3.org/2001/XMLSchema#decimal",
    "http://www.w3.org/2001/XMLSchema#integer",
    "http://www.w3.org/2001/XMLSchema#nonNegativeInteger",
    "http://www.w3.org/2001/XMLSchema#nonPositiveInteger",
    "http://www.w3.org/2001/XMLSchema#positiveInteger",
    "http://www.w3.org/2001/XMLSchema#negativeInteger",
    "http://www.w3.org/2001/XMLSchema#long",
    "http://www.w3.org/2001/XMLSchema#int",
    "http://www.w3.org/2001/XMLSchema#short",
    "http://www.w3.org/2001/XMLSchema#byte",
    "http://www.w3.org/2001/XMLSchema#unsignedLong",
    "http://www.w3.org/2001/XMLSchema#unsignedInt",
    "http://www.w3.org/2001/XMLSchema#unsignedShort",
    "http://www.w3.org/2001/XMLSchema#unsignedByte",
    "http://www.w3.org/2001/XMLSchema#float",
    "http://www.w3.org/2001/XMLSchema#double",
    "http://www.w3.org/2001/XMLSchema#string",
    "http://www.w3.org/2001/XMLSchema#normalizedString",
    "http://www.w3.org/2001/XMLSchema#token",
    "http://www.w3.org/2001/XMLSchema#language",
    "http://www.w3.org/2001/XMLSchema#Name",
    "http://www.w3.org/2001/XMLSchema#NCName",
    "http://www.w3.org/2001/XMLSchema#NMTOKEN",
    "http://www.w3.org/2001/XMLSchema#boolean",
    "http://www.w3.org/2001/XMLSchema#hexBinary",
    "http://www.w3.org/2001/XMLSchema#base64Binary",
    "http://www.w3.org/2001/XMLSchema#anyURI",
    "http://www.w3.org/2001/XMLSchema#dateTime",
    "http://www.w3.org/2001/XMLSchema#dateTimeStamp",
];

pub(super) const FAMILY: Family = Family {
    rules: &[
        "eq-ref", "prp-ap", "dt-type1", "dt-type2", "scm-cls", "scm-op", "scm-dp",
    ],
    derive,
    ..EMPTY
};

fn derive(v: &View, r: &mut Round) {
    let same = iri(OWL_SAME_AS);
    let ty = iri(RDF_TYPE);
    let unlisted = unlisted_different_from();
    for (s, p, o) in v.all.iter().filter(|(_, p, _)| *p != unlisted) {
        for t in [s, p, o] {
            r.derive("eq-ref", t, &same, t);
        }
    }
    let annotation_property = iri(&format!("{OWL}AnnotationProperty"));
    for p in ANNOTATION_PROPERTIES {
        r.derive("prp-ap", &iri(p), &ty, &annotation_property);
    }
    let datatype = iri(&format!("{RDFS}Datatype"));
    for dt in OWL_2_RL_DATATYPES {
        r.derive("dt-type1", &iri(dt), &ty, &datatype);
    }
    // dt-type2: each literal of the closure, and each the rules name, with
    // every datatype of the map whose value space holds its value.
    let named = rule_literals();
    let literals: HashSet<&Term> = v
        .all
        .iter()
        .flat_map(|(s, _, o)| [s, o])
        .chain(&named)
        .filter(|t| t.is_literal())
        .collect();
    for t in literals {
        if let Term::Literal(lt) = t
            && let Some(value) = DataValue::of(lt)
        {
            for dt in value.datatypes() {
                r.derive("dt-type2", t, &ty, &iri(&dt));
            }
        }
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
