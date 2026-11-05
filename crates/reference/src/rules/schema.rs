//! The `schema` switch: the schema rules beyond the class hierarchy,
//! scm-eqc2, scm-spo, scm-eqp1, scm-eqp2, scm-dom1, scm-dom2, scm-rng1,
//! scm-rng2, scm-hv, scm-svf1, scm-svf2, scm-avf1, scm-avf2, scm-int and
//! scm-uni (OWL 2 Profiles, Table 9), each read as the table states it.

use super::{EMPTY, Family};
use crate::evaluate::{Round, View};
use crate::vocab::*;
use oxrdf::Term;
use std::collections::{HashMap, HashSet};

pub(super) const FAMILY: Family = Family {
    rules: &[
        "scm-eqc2", "scm-spo", "scm-eqp1", "scm-eqp2", "scm-dom1", "scm-dom2", "scm-rng1", "scm-rng2", "scm-hv",
        "scm-svf1", "scm-svf2", "scm-avf1", "scm-avf2", "scm-int", "scm-uni",
    ],
    derive,
    ..EMPTY
};

fn derive(v: &View, r: &mut Round) {
    let sco = iri(RDFS_SUB_CLASS_OF);
    let spo = iri(RDFS_SUB_PROPERTY_OF);
    let eqp = iri(OWL_EQUIVALENT_PROPERTY);
    let on_property = iri(OWL_ON_PROPERTY);

    for (c1, c2) in v.pairs(&sco) {
        if v.has(c2, &sco, c1) {
            r.derive("scm-eqc2", c1, &iri(OWL_EQUIVALENT_CLASS), c2);
        }
    }
    for (p1, p2) in v.pairs(&spo) {
        for p3 in v.objects(p2, &spo) {
            r.derive("scm-spo", p1, &spo, p3);
        }
    }
    for (p1, p2) in v.pairs(&eqp) {
        r.derive("scm-eqp1", p1, &spo, p2);
        r.derive("scm-eqp1", p2, &spo, p1);
    }
    for (p1, p2) in v.pairs(&spo) {
        if v.has(p2, &spo, p1) {
            r.derive("scm-eqp2", p1, &eqp, p2);
        }
    }
    for (axiom, by_class, by_property) in [
        (iri(RDFS_DOMAIN), "scm-dom1", "scm-dom2"),
        (iri(RDFS_RANGE), "scm-rng1", "scm-rng2"),
    ] {
        for (p, c1) in v.pairs(&axiom) {
            for c2 in v.objects(c1, &sco) {
                r.derive(by_class, p, &axiom, c2);
            }
        }
        for (p2, c) in v.pairs(&axiom) {
            for p1 in v.subjects(&spo, p2) {
                r.derive(by_property, p1, &axiom, c);
            }
        }
    }

    // The restriction rules, for each filler predicate. A restriction c has
    // keys (p, y): `c owl:onProperty p` and `c <filler> y`. c1 is below c2
    // when a key of c2 is reachable from a key (p, y1) of c1: (p, y2) with
    // `y1 rdfs:subClassOf y2` (scm-svf1, scm-avf1), or (p2, y1) with
    // `p rdfs:subPropertyOf p2` (scm-hv, scm-svf2; scm-avf2 the other way
    // round). The reachable keys are collected per c1 first: equality can
    // give a restriction many aliases with many keys each.
    for (filler, by_class, by_property) in [
        (OWL_SOME_VALUES_FROM, Some("scm-svf1"), "scm-svf2"),
        (OWL_ALL_VALUES_FROM, Some("scm-avf1"), "scm-avf2"),
        (OWL_HAS_VALUE, None, "scm-hv"),
    ] {
        let mut keys: HashMap<&Term, Vec<(&Term, &Term)>> = HashMap::new();
        let mut on: HashMap<(&Term, &Term), Vec<&Term>> = HashMap::new();
        for (c, y) in v.pairs(&iri(filler)) {
            for p in v.objects(c, &on_property) {
                keys.entry(c).or_default().push((p, y));
                on.entry((p, y)).or_default().push(c);
            }
        }
        for (c1, keys) in &keys {
            let (mut along_classes, mut along_properties) = (HashSet::new(), HashSet::new());
            for &(p, y1) in keys {
                if by_class.is_some() {
                    along_classes.extend(v.objects(y1, &sco).iter().map(|y2| (p, y2)));
                }
                along_properties.extend(v.objects(p, &spo).iter().map(|p2| (p2, y1)));
            }
            let classes = |reached: HashSet<(&Term, &Term)>| -> HashSet<&Term> {
                reached.iter().filter_map(|k| on.get(k)).flatten().copied().collect()
            };
            if let Some(rule) = by_class {
                for c2 in classes(along_classes) {
                    r.derive(rule, c1, &sco, c2);
                }
            }
            for c2 in classes(along_properties) {
                if filler == OWL_ALL_VALUES_FROM {
                    r.derive(by_property, c2, &sco, c1);
                } else {
                    r.derive(by_property, c1, &sco, c2);
                }
            }
        }
    }

    for (c, head) in v.pairs(&iri(OWL_INTERSECTION_OF)) {
        for k in v.members(head) {
            r.derive("scm-int", c, &sco, &k);
        }
    }
    for (c, head) in v.pairs(&iri(OWL_UNION_OF)) {
        for k in v.members(head) {
            r.derive("scm-uni", &k, &sco, c);
        }
    }
}
