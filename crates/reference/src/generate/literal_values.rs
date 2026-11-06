//! The `literal-values` group's shapes, for dt-eq and dt-diff, drawn from
//! this file's own stream: literals of one value in several forms
//! ([`VALUES`]: lexical forms, datatypes, language tags), and the ways two
//! literals become the same (prp-fp, or an IRI `owl:sameAs` both):
//!
//! - two literals as values of individuals: dt-eq makes them the same when
//!   their values are, and with equality on, each gets the other's triples;
//! - a functional data property with two values: prp-fp makes them the same,
//!   and when their values differ, dt-diff with eq-diff1 reports it;
//! - an IRI `owl:sameAs` a literal: the IRI joins the literal's class, and
//!   so its value's other literals (and dt-type2's typings).

use super::{Generator, Shape, datatypes, p_str};
use crate::vocab::*;
use oxrdf::{Literal, Term};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const RDF_PLAIN_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral";

/// Groups of literals, the literals of a group of one value: (lexical
/// form, datatype's local name in `xsd:`, or a language tag after `@`, or
/// `PlainLiteral`).
const VALUES: &[&[(&str, &str)]] = &[
    &[
        ("1", "integer"),
        ("01", "integer"),
        ("1.0", "decimal"),
        ("+1", "byte"),
        ("1", "unsignedLong"),
    ],
    &[
        ("0", "integer"),
        ("-0", "decimal"),
        (".0", "decimal"),
        ("0", "nonPositiveInteger"),
    ],
    &[("2", "int"), ("2.00", "decimal")],
    &[("1", "double"), ("1.0E0", "double"), ("10e-1", "double")],
    &[("1", "float"), ("1.0", "float")],
    &[("a", "string"), ("a", "token"), ("a", "NCName"), ("a@", "PlainLiteral")],
    &[("chat", "@fr"), ("chat", "@FR"), ("chat@fr", "PlainLiteral")],
    &[("true", "boolean"), ("1", "boolean")],
    &[("0FB7", "hexBinary"), ("0fb7", "hexBinary")],
    &[("QQ==", "base64Binary"), ("Q Q = =", "base64Binary")],
    &[
        ("2000-01-01T00:00:00Z", "dateTime"),
        ("2000-01-01T00:00:00+00:00", "dateTimeStamp"),
    ],
    &[("2000-01-01T24:00:00", "dateTime"), ("2000-01-02T00:00:00", "dateTime")],
];

#[rustfmt::skip]
pub(super) const SHAPES: &[Shape] = &[
    Shape { version_1: None, weight: 4, draw: values },
    Shape { version_1: None, weight: 3, draw: functional_values },
    Shape { version_1: None, weight: 2, draw: same_as_a_literal },
];

/// A literal of the [`VALUES`] table.
fn literal((lexical, kind): (&str, &str)) -> Term {
    if let Some(tag) = kind.strip_prefix('@') {
        Term::from(Literal::new_language_tagged_literal_unchecked(lexical, tag))
    } else if kind == "PlainLiteral" {
        Term::from(Literal::new_typed_literal(lexical, named(RDF_PLAIN_LITERAL)))
    } else {
        Term::from(Literal::new_typed_literal(lexical, named(&format!("{XSD}{kind}"))))
    }
}

/// Two literals: mostly of one value, else of two values of the table, or
/// one of the table and an edge case of the map.
fn pair(g: &mut Generator) -> [Term; 2] {
    let group = *g.rng.pick(VALUES);
    let first = literal(*g.rng.pick(group));
    let second = match g.rng.below(4) {
        0 => {
            let other = *g.rng.pick(VALUES);
            literal(*g.rng.pick(other))
        }
        1 => datatypes::edge_literal(g),
        _ => literal(*g.rng.pick(group)),
    };
    [first, second]
}

/// `i d lt1 . j d' lt2`.
fn values(g: &mut Generator) {
    for lt in pair(g) {
        let (i, d) = (g.individual(), g.data_property());
        g.add(&i, p_str(&d), &lt);
    }
}

/// A functional data property and two values of one individual: prp-fp
/// makes them the same.
fn functional_values(g: &mut Generator) {
    let d = g.data_property();
    g.add(&d, RDF_TYPE, &iri(OWL_FUNCTIONAL_PROPERTY));
    let i = g.individual();
    for lt in pair(g) {
        g.add(&i, p_str(&d), &lt);
    }
}

/// `i owl:sameAs lt`, now and then of two literals.
fn same_as_a_literal(g: &mut Generator) {
    let i = g.individual();
    let [first, second] = pair(g);
    g.add(&i, OWL_SAME_AS, &first);
    if g.rng.one_in(3) {
        g.add(&i, OWL_SAME_AS, &second);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Rng, generate_parts};
    use crate::datatype::DataValue;
    use crate::evaluate::evaluate_within;
    use oxrdf::Term;
    use std::collections::{BTreeMap, HashMap, HashSet};
    use turbowl::Switches;

    /// Over the chaos test's first 300 seeds (both seeds, sizes 1 to 48 as
    /// it draws them), many inputs hold two different literals of one value
    /// (dt-eq makes them the same), and the reference evaluator, in the
    /// complete configuration, reports eq-diff1 between two literals of
    /// different values (dt-diff) on many, and types literals by dt-type2
    /// on most.
    #[test]
    fn the_first_300_seeds_exercise_dt_eq_dt_diff_and_dt_type2() {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for seed in [0x5EED_0005, 0x5EED_0105] {
            for i in 0..300 {
                let mut rng = Rng::at(seed, i);
                let size = 1 + rng.below(48);
                let (_, input) = generate_parts(rng.next_u64(), size);
                let mut values: HashMap<DataValue, HashSet<&Term>> = HashMap::new();
                for t in &input {
                    if let Term::Literal(lt) = &t.object
                        && let Some(v) = DataValue::of(lt)
                    {
                        values.entry(v).or_default().insert(&t.object);
                    }
                }
                if values.values().any(|lts| lts.len() > 1) {
                    *counts.entry("dt-eq").or_default() += 1;
                }
                let base: HashSet<_> = input.iter().cloned().collect();
                let Some(e) = evaluate_within(&base, Switches::complete(), 20_000) else {
                    *counts.entry("too large").or_default() += 1;
                    continue;
                };
                if e.diagnostics.iter().any(|(code, terms)| {
                    code == "eq-diff1" && matches!(terms.as_slice(), [Term::Literal(a), Term::Literal(b)] if a != b)
                }) {
                    *counts.entry("dt-diff").or_default() += 1;
                }
                if e.fired.contains_key("dt-type2") {
                    *counts.entry("dt-type2").or_default() += 1;
                }
            }
        }
        println!("inputs: {counts:?}");
        for (rule, least) in [("dt-eq", 200), ("dt-diff", 100), ("dt-type2", 500)] {
            assert!(counts.get(rule).copied().unwrap_or(0) >= least, "{rule}: {counts:?}");
        }
    }
}
