//! The core, always on: the class and property rules of version 1 (cax-sco,
//! cax-eqc1, cax-eqc2, cls-thing, cls-nothing1, cls-int1, cls-int2, cls-uni,
//! cls-svf1, cls-svf2, cls-avf, cls-hv1, cls-hv2, prp-dom, prp-rng, prp-fp,
//! prp-ifp, prp-symp, prp-trp, prp-spo1, prp-eqp1, prp-eqp2, prp-inv1,
//! prp-inv2, scm-sco, scm-eqc1).
//!
//! To come: cls-oo (`owl:oneOf`), version 2's ticket 08.

use super::{EMPTY, Family};
use crate::evaluate::{Round, View};
use crate::vocab::*;
use oxrdf::Term;
use std::collections::HashSet;

pub(super) const FAMILY: Family = Family {
    rules: &[
        "cls-thing",
        "cls-nothing1",
        "prp-dom",
        "prp-rng",
        "prp-fp",
        "prp-ifp",
        "prp-symp",
        "prp-trp",
        "prp-spo1",
        "prp-eqp1",
        "prp-eqp2",
        "prp-inv1",
        "prp-inv2",
        "cax-sco",
        "cax-eqc1",
        "cax-eqc2",
        "scm-sco",
        "scm-eqc1",
        "cls-int1",
        "cls-int2",
        "cls-uni",
        "cls-svf1",
        "cls-svf2",
        "cls-avf",
        "cls-hv1",
        "cls-hv2",
    ],
    derive,
    ..EMPTY
};

fn derive(v: &View, r: &mut Round) {
    // cls-thing, cls-nothing1: rules with an empty body.
    let ty = iri(RDF_TYPE);
    r.derive("cls-thing", &iri(OWL_THING), &ty, &iri(OWL_CLASS));
    r.derive("cls-nothing1", &iri(OWL_NOTHING), &ty, &iri(OWL_CLASS));
    properties(v, r);
    classes(v, r);
}

fn properties(v: &View, r: &mut Round) {
    let ty = iri(RDF_TYPE);
    let same = iri(OWL_SAME_AS);

    for (p, c) in v.pairs(&iri(RDFS_DOMAIN)) {
        for (x, _) in v.pairs(p) {
            r.derive("prp-dom", x, &ty, c);
        }
    }
    for (p, c) in v.pairs(&iri(RDFS_RANGE)) {
        for (_, y) in v.pairs(p) {
            r.derive("prp-rng", y, &ty, c);
        }
    }
    for p in v.instances(OWL_FUNCTIONAL_PROPERTY) {
        for (x, y1) in v.pairs(p) {
            for y2 in v.objects(x, p) {
                r.derive("prp-fp", y1, &same, y2);
            }
        }
    }
    for p in v.instances(OWL_INVERSE_FUNCTIONAL_PROPERTY) {
        for (x1, y) in v.pairs(p) {
            for x2 in v.subjects(p, y) {
                r.derive("prp-ifp", x1, &same, x2);
            }
        }
    }
    for p in v.instances(OWL_SYMMETRIC_PROPERTY) {
        for (x, y) in v.pairs(p) {
            r.derive("prp-symp", y, p, x);
        }
    }
    for p in v.instances(OWL_TRANSITIVE_PROPERTY) {
        for (x, y) in v.pairs(p) {
            for z in v.objects(y, p) {
                r.derive("prp-trp", x, p, z);
            }
        }
    }
    for (p1, p2) in v.pairs(&iri(RDFS_SUB_PROPERTY_OF)) {
        for (x, y) in v.pairs(p1) {
            r.derive("prp-spo1", x, p2, y);
        }
    }
    for (p1, p2) in v.pairs(&iri(OWL_EQUIVALENT_PROPERTY)) {
        for (x, y) in v.pairs(p1) {
            r.derive("prp-eqp1", x, p2, y);
        }
        for (x, y) in v.pairs(p2) {
            r.derive("prp-eqp2", x, p1, y);
        }
    }
    for (p1, p2) in v.pairs(&iri(OWL_INVERSE_OF)) {
        for (x, y) in v.pairs(p1) {
            r.derive("prp-inv1", y, p2, x);
        }
        for (x, y) in v.pairs(p2) {
            r.derive("prp-inv2", y, p1, x);
        }
    }
}

fn classes(v: &View, r: &mut Round) {
    let ty = iri(RDF_TYPE);
    let sco = iri(RDFS_SUB_CLASS_OF);
    let on_property = iri(OWL_ON_PROPERTY);

    for (c1, c2) in v.pairs(&sco) {
        for x in v.subjects(&ty, c1) {
            r.derive("cax-sco", x, &ty, c2);
        }
        for c3 in v.objects(c2, &sco) {
            r.derive("scm-sco", c1, &sco, c3);
        }
    }
    for (c1, c2) in v.pairs(&iri(OWL_EQUIVALENT_CLASS)) {
        for x in v.subjects(&ty, c1) {
            r.derive("cax-eqc1", x, &ty, c2);
        }
        for x in v.subjects(&ty, c2) {
            r.derive("cax-eqc2", x, &ty, c1);
        }
        r.derive("scm-eqc1", c1, &sco, c2);
        r.derive("scm-eqc1", c2, &sco, c1);
    }
    for (c, head) in v.pairs(&iri(OWL_INTERSECTION_OF)) {
        let members = v.members(head);
        let candidates: HashSet<&Term> = members.iter().flat_map(|k| v.subjects(&ty, k)).collect();
        let cells = v.cells(head);
        for y in candidates {
            if v.every_member(head, &cells, |k| v.has(y, &ty, k)) {
                r.derive("cls-int1", y, &ty, c);
            }
        }
        for y in v.subjects(&ty, c) {
            for k in &members {
                r.derive("cls-int2", y, &ty, k);
            }
        }
    }
    for (c, head) in v.pairs(&iri(OWL_UNION_OF)) {
        for k in v.members(head) {
            for y in v.subjects(&ty, &k) {
                r.derive("cls-uni", y, &ty, c);
            }
        }
    }
    let thing = iri(OWL_THING);
    for (x, y) in v.pairs(&iri(OWL_SOME_VALUES_FROM)) {
        for p in v.objects(x, &on_property) {
            for (u, value) in v.pairs(p) {
                if v.has(value, &ty, y) {
                    r.derive("cls-svf1", u, &ty, x);
                }
                if *y == thing {
                    r.derive("cls-svf2", u, &ty, x);
                }
            }
        }
    }
    for (x, y) in v.pairs(&iri(OWL_ALL_VALUES_FROM)) {
        for p in v.objects(x, &on_property) {
            for u in v.subjects(&ty, x) {
                for value in v.objects(u, p) {
                    r.derive("cls-avf", value, &ty, y);
                }
            }
        }
    }
    for (x, y) in v.pairs(&iri(OWL_HAS_VALUE)) {
        for p in v.objects(x, &on_property) {
            for u in v.subjects(&ty, x) {
                r.derive("cls-hv1", u, p, y);
            }
            for u in v.subjects(p, y) {
                r.derive("cls-hv2", u, &ty, x);
            }
        }
    }
}
