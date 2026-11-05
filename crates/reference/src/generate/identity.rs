//! The `identity` group's shapes, for prp-key, cls-maxc2, cls-maxqc3 and
//! cls-maxqc4: each states its axiom and data that fire it, now and then
//! data that does not.
//!
//! - A key: `C owl:hasKey (p1 [p2])` over object or data properties, two
//!   individuals of `C` with the same values, and now and then a third with
//!   one value different. The list is blank cells ending in `rdf:nil`; now
//!   and then a cell has a second `rdf:first` (two keys), or the list is
//!   cyclic (no key).
//! - `owl:maxCardinality` on a restriction, with `u` of its type and two
//!   values of its property; one time in six the number is 2, which fires
//!   nothing.
//! - `owl:maxQualifiedCardinality` with `owl:onClass` a class (cls-maxqc3,
//!   each value typed with it two times in three) or `owl:Thing`
//!   (cls-maxqc4); one time in six the number is 2.
//!
//! The shapes keep their lists to themselves (not in `Generator::lists`),
//! so that no other file's shapes find them.

use super::{Generator, Shape, datatypes, p_str};
use crate::vocab::*;
use oxrdf::{Literal, Term};

const OWL_HAS_KEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
const OWL_MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
const OWL_MAX_QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
const OWL_ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: None, weight: 3, draw: key },
    Shape { version_1: None, weight: 2, draw: max_cardinality },
    Shape { version_1: None, weight: 2, draw: max_qualified_cardinality },
];

/// `C owl:hasKey (p1 ... pn)`, and individuals of `C` with values.
fn key(g: &mut Generator) {
    let c = g.class();
    let properties: Vec<Term> = (0..1 + g.rng.below(2)).map(|_| g.property()).collect();
    let head = key_list(g, &properties);
    g.add(&c, OWL_HAS_KEY, &head);
    let values: Vec<Term> = properties.iter().map(|p| value(g, p)).collect();
    let others = if g.rng.one_in(3) { 3 } else { 2 };
    for k in 0..others {
        let x = g.individual();
        g.add(&x, RDF_TYPE, &c);
        for (i, (p, v)) in properties.iter().zip(&values).enumerate() {
            // The third differs in its last value.
            let v = if k == 2 && i + 1 == properties.len() {
                value(g, p)
            } else {
                v.clone()
            };
            g.add(&x, p_str(p), &v);
        }
    }
}

/// A value of `p`: an individual, or a literal for a data property.
fn value(g: &mut Generator, p: &Term) -> Term {
    if g.data_properties.contains(p) {
        datatypes::literal(g)
    } else {
        g.individual()
    }
}

/// The list of a key: blank cells ending in `rdf:nil`; one time in eight a
/// cell with a second first, one time in ten a cycle back to the head.
fn key_list(g: &mut Generator, properties: &[Term]) -> Term {
    let cells: Vec<Term> = properties.iter().map(|_| g.fresh()).collect();
    let cyclic = g.rng.one_in(10);
    for (i, (cell, p)) in cells.iter().zip(properties).enumerate() {
        g.add(cell, RDF_FIRST, p);
        let rest = match cells.get(i + 1) {
            Some(next) => next.clone(),
            None if cyclic => cells[0].clone(),
            None => iri(RDF_NIL),
        };
        g.add(cell, RDF_REST, &rest);
    }
    if g.rng.one_in(8) {
        let cell = g.rng.pick(&cells).clone();
        let other = g.property();
        g.add(&cell, RDF_FIRST, &other);
    }
    cells[0].clone()
}

/// The number of a cardinality restriction: 1, one time in six 2.
fn number(g: &mut Generator) -> Term {
    let n = if g.rng.one_in(6) { "2" } else { "1" };
    Term::from(Literal::new_typed_literal(n, named(XSD_NON_NEGATIVE_INTEGER)))
}

/// A restriction on a property drawn for it, an individual of its type,
/// and two values of the property: (restriction, individual, values).
fn restricted(g: &mut Generator, number_of: &str, p: &Term) -> (Term, Term, [Term; 2]) {
    let r = g.fresh();
    g.add(&r, RDF_TYPE, &iri(OWL_RESTRICTION));
    let n = number(g);
    g.add(&r, number_of, &n);
    g.add(&r, OWL_ON_PROPERTY, p);
    let u = g.individual();
    g.add(&u, RDF_TYPE, &r);
    let values = [value(g, p), value(g, p)];
    for v in &values {
        g.add(&u, p_str(p), v);
    }
    (r, u, values)
}

/// `R owl:maxCardinality n ; owl:onProperty p`, `u a R ; p y1, y2`.
fn max_cardinality(g: &mut Generator) {
    let p = if g.rng.one_in(5) {
        g.data_property()
    } else {
        g.object_property()
    };
    restricted(g, OWL_MAX_CARDINALITY, &p);
}

/// `R owl:maxQualifiedCardinality n ; owl:onProperty p ; owl:onClass C`
/// (or `owl:Thing`), `u a R ; p y1, y2`, the values typed `C` now and then.
fn max_qualified_cardinality(g: &mut Generator) {
    let p = g.object_property();
    let (r, _, values) = restricted(g, OWL_MAX_QUALIFIED_CARDINALITY, &p);
    if g.rng.one_in(3) {
        g.add(&r, OWL_ON_CLASS, &iri(OWL_THING));
    } else {
        let c = g.class();
        g.add(&r, OWL_ON_CLASS, &c);
        for y in &values {
            if !g.rng.one_in(3) {
                g.add(y, RDF_TYPE, &c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Rng, generate_parts};
    use crate::evaluate::evaluate_within;
    use std::collections::{BTreeMap, HashSet};
    use turbowl::Switches;

    /// Over the chaos test's first 300 seeds (both seeds, sizes 1 to 48 as
    /// it draws them), the inputs hold each shape's axiom, and the
    /// reference evaluator fires each of the group's rules on some.
    #[test]
    fn the_first_300_seeds_exercise_every_identity_rule() {
        let axioms = [
            ("hasKey", "owl#hasKey>"),
            ("maxCardinality 1", "owl#maxCardinality> \"1\""),
            (
                "maxQualifiedCardinality 1 on owl:Thing",
                "owl#onClass> <http://www.w3.org/2002/07/owl#Thing>",
            ),
            ("maxQualifiedCardinality", "owl#maxQualifiedCardinality> \"1\""),
        ];
        let mut drawn: BTreeMap<&str, usize> = BTreeMap::new();
        let mut fired: BTreeMap<&str, usize> = BTreeMap::new();
        for seed in [0x5EED_0005, 0x5EED_0105] {
            for i in 0..300 {
                let mut rng = Rng::at(seed, i);
                let size = 1 + rng.below(48);
                let (_, input) = generate_parts(rng.next_u64(), size);
                let lines: Vec<String> = input.iter().map(ToString::to_string).collect();
                for (name, text) in axioms {
                    if lines.iter().any(|l| l.contains(text)) {
                        *drawn.entry(name).or_default() += 1;
                    }
                }
                let base: HashSet<_> = input.into_iter().collect();
                if let Some(e) = evaluate_within(&base, Switches::complete(), 20_000) {
                    for rule in ["prp-key", "cls-maxc2", "cls-maxqc3", "cls-maxqc4"] {
                        if e.fired.contains_key(rule) {
                            *fired.entry(rule).or_default() += 1;
                        }
                    }
                }
            }
        }
        println!("inputs with each axiom: {drawn:?}\ninputs where each rule fires: {fired:?}");
        for (name, _) in axioms {
            assert!(drawn.get(name).copied().unwrap_or(0) >= 20, "{name}: {drawn:?}");
        }
        for rule in ["prp-key", "cls-maxc2", "cls-maxqc3", "cls-maxqc4"] {
            assert!(fired.get(rule).copied().unwrap_or(0) >= 10, "{rule}: {fired:?}");
        }
    }
}
