//! Literals and datatypes, for dt-not-type (the `checks` group): data
//! property values, well-formed and ill-formed values, and the datatypes a
//! data property's range names.
//!
//! Version 1's shape and tables ([`LITERALS`], [`DATATYPES`], drawn by
//! version 1's shapes here and in `core.rs`) stay as they were. The shapes
//! added since draw literals of every datatype of the OWL 2 RL map, at the
//! edges of their lexical spaces (`crate::datatype::EDGE_CASES`), and type
//! them with any datatype of the map: through a range (prp-rng), and through
//! a functional data property that makes two of them the same (prp-fp), so
//! a literal joins a class typed with a datatype.

use super::{Generator, Shape, p_str};
use crate::datatype::{EDGE_CASES, datatype_iris};
use crate::vocab::*;
use oxrdf::{Literal, Term};

/// Version 1's literals: well-formed and ill-formed values of the five
/// datatypes version 1 checked, and two others.
const LITERALS: &[(&str, &str)] = &[
    ("42", XSD_INT),
    ("-7", XSD_INT),
    (" 42", XSD_INT),
    ("2147483648", XSD_INT),
    ("+5", XSD_INTEGER),
    ("12", XSD_INTEGER),
    ("abc", XSD_INTEGER),
    ("hello", XSD_STRING),
    ("", XSD_STRING),
    ("<a>t</a>", RDF_XML_LITERAL),
    ("<a>t</b>", RDF_XML_LITERAL),
    ("plain", RDF_XML_LITERAL),
    ("3.5", XSD_DECIMAL),
    ("1", "http://example.org/dt"),
];

/// Version 1's datatypes a data property's range may name: the five it
/// checked and two others.
const DATATYPES: &[&str] = &[
    XSD_INTEGER,
    XSD_INT,
    XSD_STRING,
    RDF_LANG_STRING,
    RDF_XML_LITERAL,
    XSD_DECIMAL,
    "http://example.org/dt",
];

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: Some(2), weight: 6, draw: value },
    Shape { version_1: None, weight: 4, draw: edge_value },
    Shape { version_1: None, weight: 3, draw: ranged_value },
    Shape { version_1: None, weight: 2, draw: functional_values },
];

/// `i d literal`.
fn value(g: &mut Generator) {
    let (i, d, l) = (g.individual(), g.data_property(), literal(g));
    g.add(&i, p_str(&d), &l);
}

/// A literal: one of [`LITERALS`], a language-tagged string or a simple
/// literal.
pub(super) fn literal(g: &mut Generator) -> Term {
    match g.rng.below(LITERALS.len() + 2) {
        n if n < LITERALS.len() => {
            let (value, datatype) = LITERALS[n];
            Term::from(Literal::new_typed_literal(value, named(datatype)))
        }
        n if n == LITERALS.len() => Term::from(Literal::new_language_tagged_literal_unchecked("chat", "fr")),
        _ => Term::from(Literal::new_simple_literal("x")),
    }
}

/// A datatype a range may name.
pub(super) fn datatype(g: &mut Generator) -> Term {
    iri(g.rng.pick(DATATYPES))
}

/// `i d literal`, the literal of any datatype of the map.
fn edge_value(g: &mut Generator) {
    let (i, d, l) = (g.individual(), g.data_property(), edge_literal(g));
    g.add(&i, p_str(&d), &l);
}

/// `d rdfs:range dt` for any datatype of the map, and `i d literal`:
/// prp-rng types the literal with `dt`.
fn ranged_value(g: &mut Generator) {
    let d = g.data_property();
    let dt = any_datatype(g);
    g.add(&d, RDFS_RANGE, &dt);
    let (i, l) = (g.individual(), edge_literal(g));
    g.add(&i, p_str(&d), &l);
}

/// A functional data property with a range and two values of one
/// individual: with equality on, prp-fp makes the two literals the same, so
/// each is checked against the range the other is typed with.
fn functional_values(g: &mut Generator) {
    let d = g.data_property();
    g.add(&d, RDF_TYPE, &iri(OWL_FUNCTIONAL_PROPERTY));
    let dt = any_datatype(g);
    g.add(&d, RDFS_RANGE, &dt);
    let i = g.individual();
    for _ in 0..2 {
        let l = edge_literal(g);
        g.add(&i, p_str(&d), &l);
    }
}

/// A datatype of the map, or now and then one outside it.
fn any_datatype(g: &mut Generator) -> Term {
    let all: Vec<String> = datatype_iris().collect();
    if g.rng.one_in(12) {
        iri("http://example.org/dt")
    } else {
        iri(g.rng.pick(&all))
    }
}

/// A literal at an edge of a datatype of the map, or a language-tagged
/// string, a simple literal, or a literal typed `rdfs:Literal` or with a
/// datatype outside the map.
fn edge_literal(g: &mut Generator) -> Term {
    let typed = |value: &str, datatype: &str| Term::from(Literal::new_typed_literal(value, named(datatype)));
    match g.rng.below(16) {
        0 => {
            let (text, tag) = *g.rng.pick(&[("chat", "fr"), ("a", "EN-gb"), ("", "en"), ("5", "x-1")]);
            Term::from(Literal::new_language_tagged_literal_unchecked(text, tag))
        }
        1 => Term::from(Literal::new_simple_literal(*g.rng.pick(&["x", "", "5", "a b", " a"]))),
        2 => typed(g.rng.pick(&["x", "5"]), "http://www.w3.org/2000/01/rdf-schema#Literal"),
        3 => typed("1", "http://example.org/dt"),
        _ => {
            let (datatype, cases) = *g.rng.pick(EDGE_CASES);
            let (value, _) = *g.rng.pick(cases);
            typed(value, datatype)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::generate;
    use crate::datatype::EDGE_CASES;
    use crate::evaluate::evaluate_within;
    use oxrdf::Term;
    use std::collections::{BTreeSet, HashSet};
    use turbowl::Switches;

    /// Over the first seeds, the inputs hold literals of every datatype of
    /// the map, and the complete closure has dt-not-type findings of both
    /// halves: against a literal's own datatype and against another.
    #[test]
    fn inputs_reach_every_datatype_and_both_halves() {
        let mut datatypes = BTreeSet::new();
        let (mut own, mut other) = (0, 0);
        for seed in 0..300 {
            let input: HashSet<_> = generate(seed, 1 + (seed as usize % 48)).into_iter().collect();
            for t in &input {
                if let Term::Literal(l) = &t.object {
                    datatypes.insert(l.datatype().as_str().to_owned());
                }
            }
            let Some(e) = evaluate_within(&input, Switches::complete(), 20_000) else {
                continue;
            };
            for (code, terms) in &e.diagnostics {
                if let (true, [Term::Literal(l), Term::NamedNode(d)]) = (code == "dt-not-type", terms.as_slice()) {
                    if l.datatype() == d.as_ref() {
                        own += 1;
                    } else {
                        other += 1;
                    }
                }
            }
        }
        for (datatype, _) in EDGE_CASES {
            assert!(datatypes.contains(*datatype), "no literal of {datatype}");
        }
        assert!(
            own > 100 && other > 20,
            "findings: {own} against their own datatype, {other} against another"
        );
    }
}
