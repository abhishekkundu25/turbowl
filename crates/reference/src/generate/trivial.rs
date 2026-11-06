//! The `trivial` group's shapes, for eq-ref, scm-cls, scm-op, scm-dp,
//! prp-ap, dt-type1 and dt-type2, drawn from this file's own stream:
//!
//! - a declaration: `C a owl:Class`, `p a owl:ObjectProperty` or
//!   `d a owl:DatatypeProperty` (scm-cls, scm-op, scm-dp), now and then of
//!   the other kind of property, and now and then a label (an annotation
//!   property of prp-ap);
//! - a schema that reads `owl:sameAs` as a property: a domain, a range, a
//!   super- or inverse property, or a restriction on it. These read eq-ref's
//!   `x owl:sameAs x` for every term, which the core then stores
//!   (`turbowl`'s `rules/trivial.rs`). The class of the domain or range and
//!   the super- or inverse property are this shape's own (`ex:Same`,
//!   `ex:same`), outside the pools: a pool's class or property may hold a
//!   key or be functional, and over every term of the closure that makes
//!   the reference evaluator's prp-key and prp-fp, which compare every two
//!   instances, too slow for chaos.
//!
//! - a datatype where a class goes, which reads dt-type2's typings: the
//!   filler of an `owl:someValuesFrom` on a data property (cls-svf1), or a
//!   subclass (cax-sco); now and then a range on `rdf:type` itself, which
//!   types every class of a typing, the datatypes of dt-type2 included.
//!
//! prp-ap and dt-type1 have empty bodies and need no shape; eq-ref fires on
//! every input; `x owl:differentFrom x` and a member listed twice in an
//! `owl:AllDifferent` (eq-diff1 to eq-diff3 with eq-ref) are drawn by
//! `checks.rs`. dt-type2 fires on every input with a literal, which most
//! shapes draw (`datatypes.rs`, `literal_values.rs`).

use super::{Generator, Shape, datatypes, p_str};
use crate::vocab::*;

const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const OWL_DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: None, weight: 8, draw: declaration },
    Shape { version_1: None, weight: 1, draw: same_as_as_a_property },
    Shape { version_1: None, weight: 3, draw: datatype_as_a_class },
];

/// Datatypes of the map, for [`datatype_as_a_class`]: numbers whose spaces
/// nest, strings, and the top data range.
const DATATYPES: &[&str] = &[
    "http://www.w3.org/2001/XMLSchema#decimal",
    "http://www.w3.org/2001/XMLSchema#integer",
    "http://www.w3.org/2001/XMLSchema#short",
    "http://www.w3.org/2001/XMLSchema#unsignedByte",
    "http://www.w3.org/2001/XMLSchema#nonPositiveInteger",
    "http://www.w3.org/2001/XMLSchema#double",
    "http://www.w3.org/2001/XMLSchema#string",
    "http://www.w3.org/2001/XMLSchema#token",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral",
    "http://www.w3.org/2000/01/rdf-schema#Literal",
];

/// A datatype where a class goes: `r owl:someValuesFrom dt ; owl:onProperty
/// d` with a value of `d` (cls-svf1 reads the literal's typing), or
/// `dt rdfs:subClassOf C` (cax-sco); now and then `rdf:type rdfs:range C`.
fn datatype_as_a_class(g: &mut Generator) {
    let dt = iri(g.rng.pick(DATATYPES));
    match g.rng.below(7) {
        0..=3 => {
            let (r, d) = (g.fresh(), g.data_property());
            g.add(&r, OWL_SOME_VALUES_FROM, &dt);
            g.add(&r, OWL_ON_PROPERTY, &d);
            let c = g.class();
            g.add(&r, RDFS_SUB_CLASS_OF, &c);
            let (i, lt) = (g.individual(), datatypes::literal(g));
            g.add(&i, p_str(&d), &lt);
        }
        4 | 5 => {
            let c = g.class();
            g.add(&dt, RDFS_SUB_CLASS_OF, &c);
        }
        _ => {
            let c = iri("http://example.org/Typed");
            g.add(&iri(RDF_TYPE), RDFS_RANGE, &c);
        }
    }
}

/// `C a owl:Class`, `p a owl:ObjectProperty` or `d a owl:DatatypeProperty`.
fn declaration(g: &mut Generator) {
    let (term, kind) = match g.rng.below(6) {
        0..=2 => (g.class(), OWL_CLASS),
        3 => (g.object_property(), OWL_OBJECT_PROPERTY),
        4 => (g.data_property(), OWL_DATATYPE_PROPERTY),
        _ => {
            let kind = if g.rng.one_in(2) {
                OWL_OBJECT_PROPERTY
            } else {
                OWL_DATATYPE_PROPERTY
            };
            (g.property(), kind)
        }
    };
    g.add(&term, RDF_TYPE, &iri(kind));
    if g.rng.one_in(4) {
        let label = datatypes::literal(g);
        g.add(&term, RDFS_LABEL, &label);
    }
}

/// A schema triple that reads `owl:sameAs` as a property.
fn same_as_as_a_property(g: &mut Generator) {
    let same = iri(OWL_SAME_AS);
    let (class, property) = (iri("http://example.org/Same"), iri("http://example.org/same"));
    match g.rng.below(5) {
        0 => g.add(&same, RDFS_DOMAIN, &class),
        1 => g.add(&same, RDFS_RANGE, &class),
        2 => g.add(&same, RDFS_SUB_PROPERTY_OF, &property),
        3 => g.add(&property, OWL_INVERSE_OF, &same),
        _ => {
            let (r, c) = (g.fresh(), g.class());
            g.add(&r, OWL_ON_PROPERTY, &same);
            g.add(&r, OWL_SOME_VALUES_FROM, &c);
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
    /// it draws them), the inputs hold each shape's axioms, and the
    /// reference evaluator fires each of the group's rules on some.
    #[test]
    fn the_first_300_seeds_exercise_every_trivial_rule() {
        const SAME_AS: &str = "<http://www.w3.org/2002/07/owl#sameAs>";
        type Holds = fn(&str) -> bool;
        let axioms: [(&str, Holds); 4] = [
            ("owl:Class", |l| l.ends_with("<http://www.w3.org/2002/07/owl#Class>")),
            ("owl:ObjectProperty", |l| {
                l.ends_with("<http://www.w3.org/2002/07/owl#ObjectProperty>")
            }),
            ("owl:DatatypeProperty", |l| {
                l.ends_with("<http://www.w3.org/2002/07/owl#DatatypeProperty>")
            }),
            ("owl:sameAs as a property", |l| {
                l.starts_with(SAME_AS) || (l.ends_with(SAME_AS) && !l.contains(&format!("> {SAME_AS} ")))
            }),
        ];
        let rules = [
            "eq-ref", "scm-cls", "scm-op", "scm-dp", "prp-ap", "dt-type1", "dt-type2",
        ];
        let mut drawn: BTreeMap<&str, usize> = BTreeMap::new();
        let mut fired: BTreeMap<&str, usize> = BTreeMap::new();
        for seed in [0x5EED_0005, 0x5EED_0105] {
            for i in 0..300 {
                let mut rng = Rng::at(seed, i);
                let size = 1 + rng.below(48);
                let (_, input) = generate_parts(rng.next_u64(), size);
                let lines: Vec<String> = input.iter().map(ToString::to_string).collect();
                for (name, holds) in axioms {
                    if lines.iter().any(|l| holds(l)) {
                        *drawn.entry(name).or_default() += 1;
                    }
                }
                let base: HashSet<_> = input.into_iter().collect();
                if let Some(e) = evaluate_within(&base, Switches::complete(), 20_000) {
                    for rule in rules {
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
        for rule in rules {
            assert!(fired.get(rule).copied().unwrap_or(0) >= 20, "{rule}: {fired:?}");
        }
    }
}
