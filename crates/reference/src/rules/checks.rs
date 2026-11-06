//! The `checks` switch: the rules that conclude `false` (cax-dw, cax-adc,
//! prp-pdw, prp-adp, prp-npa1, prp-npa2, cls-nothing2, cls-maxc1,
//! cls-maxqc1, cls-maxqc2, prp-asyp, prp-irp, cls-com, eq-diff1, eq-diff2,
//! eq-diff3), as the W3C tables write them, over the closure. dt-not-type,
//! in the same switch, is in `datatypes.rs`.
//!
//! The list rules (cax-adc, prp-adp, eq-diff2, eq-diff3) read positions
//! along the paths from the head to `rdf:nil` ([`before`]), each head once
//! a diagnosis ([`pairs_before`]).

use super::{EMPTY, Family, Found};
use crate::evaluate::View;
use crate::vocab::*;
use oxrdf::{Literal, Term};
use std::collections::{HashMap, HashSet};

const OWL: &str = "http://www.w3.org/2002/07/owl#";

pub(super) const FAMILY: Family = Family {
    diagnostic_rules: &[
        "cax-dw",
        "prp-pdw",
        "cls-nothing2",
        "prp-asyp",
        "prp-irp",
        "cls-com",
        "eq-diff1",
        "eq-diff2",
        "eq-diff3",
        "cax-adc",
        "prp-adp",
        "prp-npa1",
        "prp-npa2",
        "cls-maxc1",
        "cls-maxqc1",
        "cls-maxqc2",
    ],
    diagnose,
    ..EMPTY
};

/// An OWL term by its local name.
fn owl(local: &str) -> Term {
    iri(&format!("{OWL}{local}"))
}

fn diagnose(v: &View, found: &mut Vec<Found>) {
    let ty = iri(RDF_TYPE);
    for (c1, c2) in v.pairs(&iri(OWL_DISJOINT_WITH)) {
        for x in v.subjects(&ty, c1) {
            if v.has(x, &ty, c2) {
                found.push(("cax-dw", vec![x.clone(), c1.clone(), c2.clone()]));
            }
        }
    }
    for (p1, p2) in v.pairs(&iri(OWL_PROPERTY_DISJOINT_WITH)) {
        for (x, y) in v.pairs(p1) {
            if v.has(x, p2, y) {
                found.push(("prp-pdw", vec![p1.clone(), p2.clone(), x.clone(), y.clone()]));
            }
        }
    }
    for x in v.instances(OWL_NOTHING) {
        found.push(("cls-nothing2", vec![x.clone()]));
    }
    for p in v.instances(OWL_ASYMMETRIC_PROPERTY) {
        for (x, y) in v.pairs(p) {
            if v.has(y, p, x) {
                found.push(("prp-asyp", vec![p.clone(), x.clone(), y.clone()]));
            }
        }
    }
    for p in v.instances(OWL_IRREFLEXIVE_PROPERTY) {
        for (x, y) in v.pairs(p) {
            if x == y {
                found.push(("prp-irp", vec![p.clone(), x.clone()]));
            }
        }
    }
    for (c1, c2) in v.pairs(&iri(OWL_COMPLEMENT_OF)) {
        for x in v.subjects(&ty, c1) {
            if v.has(x, &ty, c2) {
                found.push(("cls-com", vec![c1.clone(), c2.clone(), x.clone()]));
            }
        }
    }
    let mut read = HashMap::new();
    different(v, &mut read, found);
    all_disjoint(v, &mut read, found);
    negative_assertions(v, found);
    max_zero(v, found);
}

/// eq-diff1 (x, y); eq-diff2 and eq-diff3 (x, zi, zj).
fn different(v: &View, read: &mut HashMap<Term, HashSet<(Term, Term)>>, found: &mut Vec<Found>) {
    let same = iri(OWL_SAME_AS);
    // dt-diff's triples (`literal_values.rs`) are owl:differentFrom triples
    // the closure does not list.
    for different in [owl("differentFrom"), unlisted_different_from()] {
        for (x, y) in v.pairs(&different) {
            if v.has(x, &same, y) {
                found.push(("eq-diff1", vec![x.clone(), y.clone()]));
            }
        }
    }
    for x in v.subjects(&iri(RDF_TYPE), &owl("AllDifferent")) {
        for (code, predicate) in [("eq-diff2", "members"), ("eq-diff3", "distinctMembers")] {
            for (zi, zj) in pairs_before(v, v.objects(x, &owl(predicate)), read) {
                if v.has(&zi, &same, &zj) {
                    found.push((code, vec![x.clone(), zi, zj]));
                }
            }
        }
    }
}

/// cax-adc (z, ci, cj) and prp-adp (pi, pj, u, v).
fn all_disjoint(v: &View, read: &mut HashMap<Term, HashSet<(Term, Term)>>, found: &mut Vec<Found>) {
    let (ty, members) = (iri(RDF_TYPE), owl("members"));
    for x in v.subjects(&ty, &owl("AllDisjointClasses")) {
        for (ci, cj) in pairs_before(v, v.objects(x, &members), read) {
            for z in v.subjects(&ty, &ci) {
                if v.has(z, &ty, &cj) {
                    found.push(("cax-adc", vec![z.clone(), ci.clone(), cj.clone()]));
                }
            }
        }
    }
    for x in v.subjects(&ty, &owl("AllDisjointProperties")) {
        for (pi, pj) in pairs_before(v, v.objects(x, &members), read) {
            for (u, w) in v.pairs(&pi) {
                if v.has(u, &pj, w) {
                    found.push(("prp-adp", vec![pi.clone(), pj.clone(), u.clone(), w.clone()]));
                }
            }
        }
    }
}

/// prp-npa1 and prp-npa2: (i1, p, i2).
fn negative_assertions(v: &View, found: &mut Vec<Found>) {
    for (x, i1) in v.pairs(&owl("sourceIndividual")) {
        for p in v.objects(x, &owl("assertionProperty")) {
            for (code, target) in [("prp-npa1", "targetIndividual"), ("prp-npa2", "targetValue")] {
                for i2 in v.objects(x, &owl(target)) {
                    if v.has(i1, p, i2) {
                        found.push((code, vec![i1.clone(), p.clone(), i2.clone()]));
                    }
                }
            }
        }
    }
}

/// cls-maxc1, cls-maxqc1 and cls-maxqc2: (x, u, y).
fn max_zero(v: &View, found: &mut Vec<Found>) {
    let xsd_non_negative_integer = named("http://www.w3.org/2001/XMLSchema#nonNegativeInteger");
    let zero: Term = Literal::new_typed_literal("0", xsd_non_negative_integer).into();
    let (ty, on_property) = (iri(RDF_TYPE), iri(OWL_ON_PROPERTY));
    let mut values = |x: &Term, c: Option<&Term>, code: &'static str| {
        for p in v.objects(x, &on_property) {
            for u in v.subjects(&ty, x) {
                for y in v.objects(u, p) {
                    if c.is_none_or(|c| v.has(y, &ty, c)) {
                        found.push((code, vec![x.clone(), u.clone(), y.clone()]));
                    }
                }
            }
        }
    };
    for x in v.subjects(&owl("maxCardinality"), &zero) {
        values(x, None, "cls-maxc1");
    }
    for x in v.subjects(&owl("maxQualifiedCardinality"), &zero) {
        for c in v.objects(x, &owl("onClass")) {
            values(x, Some(c), "cls-maxqc1");
            if *c == iri(OWL_THING) {
                values(x, None, "cls-maxqc2");
            }
        }
    }
}

/// The pairs (a, b) of the list at `head` with `a` at a position before
/// one of `b`: along a path from `head` to `rdf:nil` whose every cell has
/// an `rdf:first`, the cell of `a` comes before the cell of `b`, one
/// `rdf:rest` or more (a path may pass a cell twice). `rdf:rest` stops at
/// `rdf:nil`.
fn before(v: &View, head: &Term) -> HashSet<(Term, Term)> {
    let (first, rest, nil) = (iri(RDF_FIRST), iri(RDF_REST), iri(RDF_NIL));
    let tails = |cell: &Term| -> Vec<Term> { v.objects(cell, &rest).iter().filter(|t| **t != nil).cloned().collect() };
    // The cells reached from the head, then the well-formed ones: an
    // rdf:first, and a rest that is rdf:nil or well formed.
    let mut cells = vec![head.clone()];
    let mut i = 0;
    while i < cells.len() {
        for t in tails(&cells[i]) {
            if !cells.contains(&t) {
                cells.push(t);
            }
        }
        i += 1;
    }
    let mut well_formed: HashSet<Term> = HashSet::new();
    loop {
        let known = well_formed.len();
        for cell in &cells {
            let ends = v
                .objects(cell, &rest)
                .iter()
                .any(|t| *t == nil || well_formed.contains(t));
            if !v.objects(cell, &first).is_empty() && ends {
                well_formed.insert(cell.clone());
            }
        }
        if well_formed.len() == known {
            break;
        }
    }
    // The cells after `cell` along well-formed cells.
    let after = |cell: &Term| {
        let mut seen: Vec<Term> = Vec::new();
        let mut todo = tails(cell);
        while let Some(c) = todo.pop() {
            if well_formed.contains(&c) && !seen.contains(&c) {
                todo.extend(tails(&c));
                seen.push(c);
            }
        }
        seen
    };
    let mut pairs = HashSet::new();
    if !well_formed.contains(head) {
        return pairs;
    }
    let mut live = vec![head.clone()];
    live.extend(after(head).into_iter().filter(|c| c != head));
    for l in &live {
        // The members after `l`, each once: when equality makes the cells
        // aliases of each other, every cell is after `l` and holds every
        // member.
        let later: HashSet<&Term> = after(l).iter().flat_map(|m| v.objects(m, &first)).collect();
        for a in v.objects(l, &first) {
            for b in &later {
                pairs.insert((a.clone(), (*b).clone()));
            }
        }
    }
    pairs
}

/// The pairs [`before`] gives over the lists at `heads`, each head read once
/// a diagnosis (`read`): equality gives a list's head many aliases, each a
/// head of the same list, and the same heads come back for each axiom.
fn pairs_before(v: &View, heads: &[Term], read: &mut HashMap<Term, HashSet<(Term, Term)>>) -> HashSet<(Term, Term)> {
    let mut pairs = HashSet::new();
    for head in heads {
        pairs.extend(
            read.entry(head.clone())
                .or_insert_with(|| before(v, head))
                .iter()
                .cloned(),
        );
    }
    pairs
}
