//! The `identity` switch: keys and cardinalities that make individuals the
//! same, prp-key (`owl:hasKey`), cls-maxc2 (`owl:maxCardinality 1`),
//! cls-maxqc3 and cls-maxqc4 (`owl:maxQualifiedCardinality 1`, with
//! `owl:onClass` a class or `owl:Thing`; OWL 2 Profiles, Tables 5 and 6).
//!
//! As the W3C tables write them, over every pair: prp-key compares two
//! individuals of a key's class along the list's paths
//! ([`View::every_member`]) when they share a value of some property of the
//! key (every path has a cell, so no other pair can match: this only skips
//! pairs that cannot), and the cardinality rules match the term
//! `"1"^^xsd:nonNegativeInteger` itself. The `owl:sameAs` they derive is
//! closed by the `equality` switch's rules.

use super::{EMPTY, Family};
use crate::evaluate::{Round, View};
use crate::vocab::*;
use oxrdf::{Literal, Term};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};

const OWL_HAS_KEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
const OWL_MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
const OWL_MAX_QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
const OWL_ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";

pub(super) const FAMILY: Family = Family {
    rules: &["prp-key", "cls-maxc2", "cls-maxqc3", "cls-maxqc4"],
    derive,
    ..EMPTY
};

fn derive(v: &View, r: &mut Round) {
    let ty = iri(RDF_TYPE);
    let same = iri(OWL_SAME_AS);
    // prp-key: c owl:hasKey (p1 ... pn), x and y of type c, x pi zi and
    // y pi zi for every i => x owl:sameAs y.
    for (c, head) in v.pairs(&iri(OWL_HAS_KEY)) {
        let properties = v.members(head);
        if properties.is_empty() {
            // No path from the head reaches rdf:nil: no key.
            continue;
        }
        let cells = v.cells(head);
        let individuals = v.subjects(&ty, c);
        // Each individual's values of each property of the key.
        let values: Vec<Vec<HashSet<&Term>>> = individuals
            .iter()
            .map(|x| properties.iter().map(|p| v.objects(x, p).iter().collect()).collect())
            .collect();
        // The pairs that share a value of some property of the key.
        let mut sharing: HashMap<(usize, &Term), Vec<usize>> = HashMap::new();
        for (i, xs) in values.iter().enumerate() {
            for (k, zs) in xs.iter().enumerate() {
                for z in zs {
                    sharing.entry((k, *z)).or_default().push(i);
                }
            }
        }
        let mut pairs: Vec<(usize, usize)> = sharing
            .values()
            .flat_map(|group| group.iter().flat_map(move |&i| group.iter().map(move |&j| (i, j))))
            .collect();
        pairs.sort_unstable();
        pairs.dedup();
        for (i, j) in pairs {
            let ((x, xs), (y, ys)) = ((&individuals[i], &values[i]), (&individuals[j], &values[j]));
            if v.has(x, &same, y) {
                // Known: equality makes most pairs so after a round.
                continue;
            }
            let share = |p: &Term| {
                properties
                    .iter()
                    .zip(xs.iter().zip(ys))
                    .any(|(q, (a, b))| q == p && !a.is_disjoint(b))
            };
            if v.every_member(head, &cells, share) {
                r.derive("prp-key", x, &same, y);
            }
        }
    }
    // cls-maxc2: x owl:maxCardinality 1, x owl:onProperty p, u rdf:type x,
    // u p y1, u p y2 => y1 owl:sameAs y2.
    // cls-maxqc3: x owl:maxQualifiedCardinality 1, x owl:onProperty p,
    // x owl:onClass c, u rdf:type x, u p y1, y1 rdf:type c, u p y2,
    // y2 rdf:type c => y1 owl:sameAs y2. cls-maxqc4: c is owl:Thing, and
    // y1 and y2 need no type.
    // Aliases of x, p, c and u, which the eq-* rules multiply, give the
    // same values: each (rule, p, c, u) is read once, and each (rule,
    // values) paired once.
    let one = Term::Literal(Literal::new_typed_literal("1", named(XSD_NON_NEGATIVE_INTEGER)));
    let on_property = iri(OWL_ON_PROPERTY);
    let thing = iri(OWL_THING);
    let mut restrictions: Vec<(&'static str, &Term, &Term, Option<&Term>)> = Vec::new();
    for x in v.subjects(&iri(OWL_MAX_CARDINALITY), &one) {
        for p in v.objects(x, &on_property) {
            restrictions.push(("cls-maxc2", x, p, None));
        }
    }
    for x in v.subjects(&iri(OWL_MAX_QUALIFIED_CARDINALITY), &one) {
        for p in v.objects(x, &on_property) {
            for c in v.objects(x, &iri(OWL_ON_CLASS)) {
                restrictions.push(("cls-maxqc3", x, p, Some(c)));
                if *c == thing {
                    restrictions.push(("cls-maxqc4", x, p, None));
                }
            }
        }
    }
    let mut seen = HashSet::new();
    let mut reads = Vec::new();
    for (rule, x, p, c) in restrictions {
        for u in v.subjects(&ty, x) {
            if seen.insert((rule, p, c, u)) {
                reads.push((rule, p, c, u));
            }
        }
    }
    let mut seen: HashSet<(&'static str, Vec<&Term>)> = HashSet::new();
    let mut groups: Vec<(&'static str, Vec<&Term>)> = Vec::new();
    for (rule, p, c, u) in reads {
        let mut values: Vec<&Term> = v
            .objects(u, p)
            .iter()
            .filter(|y| c.is_none_or(|c| v.has(y, &ty, c)))
            .collect();
        values.sort_by_cached_key(|t| {
            let mut h = DefaultHasher::new();
            t.hash(&mut h);
            h.finish()
        });
        values.dedup();
        if seen.insert((rule, values.clone())) {
            groups.push((rule, values));
        }
    }
    for (rule, values) in &groups {
        for y1 in values {
            for y2 in values {
                r.derive(rule, y1, &same, y2);
            }
        }
    }
}
