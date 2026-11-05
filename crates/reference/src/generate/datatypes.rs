//! Literals and datatypes, for dt-not-type (the `checks` group): data
//! property values, well-formed and ill-formed values of the checked
//! datatypes and values of unchecked ones, and the datatypes a data
//! property's range names (the core's range shape draws them). Version 2's
//! ticket 09 adds the other OWL 2 RL datatypes here.

use super::{Generator, Shape, p_str};
use crate::vocab::*;
use oxrdf::{Literal, Term};

/// Literals the data properties take: well-formed and ill-formed values of
/// the checked datatypes, and values of unchecked ones.
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

/// Datatypes a data property's range may name: the five dt-not-type checks
/// and two it does not.
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
