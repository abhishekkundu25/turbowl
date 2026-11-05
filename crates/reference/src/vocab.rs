//! The IRIs the rules and the generator use, written out in full.

use oxrdf::{NamedNode, Term};

pub(crate) const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub(crate) const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub(crate) const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
pub(crate) const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
pub(crate) const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
pub(crate) const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
pub(crate) const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
pub(crate) const RDF_XML_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral";

pub(crate) const RDFS_SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub(crate) const RDFS_SUB_PROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
pub(crate) const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
pub(crate) const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
pub(crate) const RDFS_MEMBER: &str = "http://www.w3.org/2000/01/rdf-schema#member";
pub(crate) const RDFS_CONTAINER_MEMBERSHIP_PROPERTY: &str =
    "http://www.w3.org/2000/01/rdf-schema#ContainerMembershipProperty";

pub(crate) const OWL_THING: &str = "http://www.w3.org/2002/07/owl#Thing";
pub(crate) const OWL_NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
pub(crate) const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
pub(crate) const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
pub(crate) const OWL_SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
pub(crate) const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
pub(crate) const OWL_EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
pub(crate) const OWL_INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
pub(crate) const OWL_SYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
pub(crate) const OWL_TRANSITIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
pub(crate) const OWL_FUNCTIONAL_PROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
pub(crate) const OWL_INVERSE_FUNCTIONAL_PROPERTY: &str = "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
pub(crate) const OWL_ASYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AsymmetricProperty";
pub(crate) const OWL_IRREFLEXIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#IrreflexiveProperty";
pub(crate) const OWL_PROPERTY_CHAIN_AXIOM: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";
pub(crate) const OWL_PROPERTY_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#propertyDisjointWith";
pub(crate) const OWL_INTERSECTION_OF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
pub(crate) const OWL_UNION_OF: &str = "http://www.w3.org/2002/07/owl#unionOf";
pub(crate) const OWL_COMPLEMENT_OF: &str = "http://www.w3.org/2002/07/owl#complementOf";
pub(crate) const OWL_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
pub(crate) const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
pub(crate) const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
pub(crate) const OWL_ALL_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
pub(crate) const OWL_HAS_VALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";

pub(crate) const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
pub(crate) const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
pub(crate) const XSD_INT: &str = "http://www.w3.org/2001/XMLSchema#int";
pub(crate) const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";

/// An IRI as a term. The constants above are valid IRIs.
pub(crate) fn iri(s: &str) -> Term {
    Term::NamedNode(named(s))
}

pub(crate) fn named(s: &str) -> NamedNode {
    NamedNode::new_unchecked(s)
}
