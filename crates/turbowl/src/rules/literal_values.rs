//! The literal-values family (switch `literal-values`, off by default):
//! literals compared by value (OWL 2 Profiles, Table 8), over the values of
//! the datatype map (`datatype/map.rs`: the 32 datatypes OWL 2 RL supports,
//! and language-tagged literals).
//!
//! - dt-eq: `lt1 owl:sameAs lt2` for every two literals with the same value
//!   (in the OWL 2 value spaces, where an `xsd:integer` value is an
//!   `xsd:decimal` value), each literal with itself included ([`dt_eq`]).
//!   With `equality` on, the equality family merges each class of equal
//!   literals in the canon, as for any `owl:sameAs`.
//! - dt-diff: `lt1 owl:differentFrom lt2` for every two literals with
//!   different values. Never listed: these triples are not in the closure,
//!   and no rule reads them but eq-diff1, which reports two literals with
//!   different values made `owl:sameAs` ([`dt_diff`]). This departs from
//!   the W3C rule set in what the closure lists, not in what is
//!   inconsistent; `crates/oracle/src/w3c.rs` lists it.
//!
//! # dt-eq
//!
//! No rule makes a literal, so the literals are those of the base: each is
//! handled once, in the round the store first holds it
//! ([`datatype::new_literals`], marked by its own `lt owl:sameAs lt`),
//! against the literals handled before that have the same value (the
//! literal members of the classes whose `r owl:sameAs r` is stored). With
//! `equality` on, each new literal is made `owl:sameAs` one literal of its
//! value, and the merges give the rest: a class of k equal literals costs k
//! triples and k merges. With it off, the W3C rule's triples are all
//! stored, k² for such a class, since no rule closes `owl:sameAs`.
//!
//! # dt-diff
//!
//! eq-diff1 reads dt-diff's triples, and with `equality` on, eq-rep-s and
//! eq-rep-o carry them to every term equal to their literals: in a class of
//! terms made equal that holds literals of two or more values, every two
//! members `x` and `y` (each with itself too) are `owl:sameAs` and
//! `owl:differentFrom`. So the family reports eq-diff1 for the class in
//! representatives (`x` and `y` both its representative), which the
//! reasoner expands to every pair of members. It checks a class when its
//! members change (the ids [`Round::moved`]), and every class in the
//! store's first round (a restart keeps the classes). With `equality` off,
//! a dt-diff triple stays between its two literals: eq-diff1 holds for
//! each stored `lt1 owl:sameAs lt2` between literals of different values.
//! The finding needs eq-diff1's switch, `checks`.

use super::Context;
use super::datatype::{
    self,
    map::{Reading, Value, value_of},
};
use crate::Switch;
use crate::engine::Round;
use crate::term::{Dictionary, Kind, TermId, vocab};
use rustc_hash::FxHashMap;

/// The literal-value rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the literal-values rules.
pub(crate) fn rules(round: &mut Round<'_>, cx: &mut Context<'_>) {
    let equality = cx.switches.is_on(Switch::Equality);
    dt_eq(round, cx.terms, equality);
    if cx.switches.is_on(Switch::Checks) {
        dt_diff(round, cx, equality);
    }
}

/// The value of a term, when it is a literal with one.
fn value(terms: &Dictionary, id: u32) -> Option<Value<'_>> {
    if terms.kind(TermId(id)) != Kind::Literal {
        return None;
    }
    match value_of(terms.term(TermId(id))) {
        Reading::Value(v) => Some(v),
        _ => None,
    }
}

/// dt-eq for the literals met for the first time (see the module doc).
fn dt_eq(round: &mut Round<'_>, terms: &Dictionary, equality: bool) {
    let same = round.constant(vocab::OWL_SAME_AS.0);
    let table = round.by_subject(same);
    let new = datatype::new_literals(round, terms, |r| table.is_some_and(|t| t.contains(&(r, r))));
    if new.is_empty() {
        return;
    }
    // For each value of a new literal: the literals handled before, and the
    // new ones.
    let mut by_value: FxHashMap<Value<'_>, (Vec<u32>, Vec<u32>)> = FxHashMap::default();
    for (lt, v) in new {
        by_value.entry(v).or_default().1.push(lt);
    }
    if round.number() > 1
        && let Some(table) = table
    {
        let canon = round.canon();
        for (s, o) in table.all().iter() {
            if s != o {
                continue;
            }
            for m in canon.members(s).iter() {
                if let Some((old, _)) = value(terms, m).and_then(|v| by_value.get_mut(&v)) {
                    old.push(m);
                }
            }
        }
    }
    for (old, new) in by_value.into_values() {
        if equality {
            let anchor = old.first().copied().unwrap_or(new[0]);
            for &lt in &new {
                round.emit(lt, same, lt);
                if lt != anchor {
                    round.emit(lt, same, anchor);
                }
            }
        } else {
            for &lt in &new {
                for &other in old.iter().chain(&new) {
                    round.emit(lt, same, other);
                    round.emit(other, same, lt);
                }
            }
        }
    }
}

/// dt-diff, as eq-diff1 reads it (see the module doc).
fn dt_diff(round: &Round<'_>, cx: &mut Context<'_>, equality: bool) {
    let terms = cx.terms;
    let canon = round.canon();
    if !equality {
        let same = round.constant(vocab::OWL_SAME_AS.0);
        let Some(table) = round.by_subject(same) else { return };
        for &(x, y) in table.recent().iter() {
            if x != y
                && let (Some(vx), Some(vy)) = (value(terms, x), value(terms, y))
                && vx != vy
            {
                cx.report("eq-diff1", &[x, y]);
            }
        }
        return;
    }
    let mut classes: Vec<u32> = if round.number() == 1 {
        canon.classes().map(|(r, _)| r).collect()
    } else {
        round.moved().iter().map(|&m| canon.find(m)).collect()
    };
    classes.sort_unstable();
    classes.dedup();
    for r in classes {
        let mut values = canon.members(r).iter().filter_map(|m| value(terms, m));
        if let Some(first) = values.next()
            && values.any(|v| v != first)
        {
            cx.report("eq-diff1", &[r, r]);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::testing::{self, Run, expand};
    use crate::term::vocab;
    use std::collections::BTreeSet;

    /// Literals of equal and different values, and literals with none: of a
    /// datatype OWL 2 RL leaves out (`owl:real`, `owl:rational`), outside
    /// the map, ill-typed, or typed `rdfs:Literal`.
    const LITERALS: &str = "ex:x ex:p \"1\"^^xsd:integer , \"01\"^^xsd:integer , \"1.0\"^^xsd:decimal ,
        \"+1\"^^xsd:byte , \"2\"^^xsd:integer , \"1\"^^xsd:double , \"a\" , \"a\"^^xsd:token , \"chat\"@fr ,
        \"chat@fr\"^^rdf:PlainLiteral , \"1\"^^owl:real , \"1/1\"^^owl:rational , \"1\"^^ex:dt ,
        \"abc\"^^xsd:integer , \"1\"^^rdfs:Literal .";

    /// `x owl:sameAs y` for every two members of each group, in the
    /// short notation.
    fn same_as(groups: &[&[&str]]) -> BTreeSet<String> {
        let mut lines = String::new();
        for group in groups {
            for x in *group {
                for y in *group {
                    lines.push_str(&format!("{x} owl:sameAs {y}\n"));
                }
            }
        }
        expand(&lines)
    }

    /// The classes of equal values in [`LITERALS`], with the literals the
    /// rules name (`datatype::RULE_LITERALS`).
    fn classes() -> BTreeSet<String> {
        same_as(&[
            &[
                "\"1\"^^xsd:integer",
                "\"01\"^^xsd:integer",
                "\"1.0\"^^xsd:decimal",
                "\"+1\"^^xsd:byte",
                "\"1\"^^xsd:nonNegativeInteger",
            ],
            &["\"0\"^^xsd:nonNegativeInteger"],
            &["\"2\"^^xsd:integer"],
            &["\"1\"^^xsd:double"],
            &["\"a\"", "\"a\"^^xsd:token"],
            &["\"chat\"@fr", "\"chat@fr\"^^rdf:PlainLiteral"],
        ])
    }

    /// dt-eq: every two literals with the same value, each with itself
    /// included; the same triples with the equality rules as without.
    #[test]
    fn dt_eq_makes_literals_of_one_value_the_same() {
        let same = "<http://www.w3.org/2002/07/owl#sameAs>";
        for families in [&["equality", "literal-values"][..], &["literal-values"]] {
            let run = testing::run(families, LITERALS);
            let inferred: BTreeSet<String> = run.inferred().into_iter().filter(|t| t.contains(same)).collect();
            assert_eq!(inferred, classes(), "{families:?}");
        }
        // With equality, a class in the canon: eq-rep-o gives each its
        // triples, the literal the rules name too.
        let mut run = testing::run(&["equality", "literal-values"], LITERALS);
        assert!(run.has("ex:x ex:p \"1\"^^xsd:nonNegativeInteger"));
        assert!(!testing::run(&["literal-values"], LITERALS).has("ex:x ex:p \"1\"^^xsd:nonNegativeInteger"));
        let one = run.dict.intern_typed_literal("1", vocab::XSD_INTEGER);
        assert_eq!(run.store.canon().members(run.store.canon().find(one.0)).len(), 5);
    }

    /// A literal of a later run joins the literals of its value, with
    /// equality and without; a literal of a new value is alone.
    #[test]
    fn dt_eq_in_an_incremental_run() {
        for families in [&["equality", "literal-values"][..], &["literal-values"]] {
            let mut run = testing::run(families, "ex:x ex:p \"1\"^^xsd:integer , \"01\"^^xsd:integer .");
            run.add("ex:y ex:q \"1.00\"^^xsd:decimal , \"3\"^^xsd:short .");
            run.add("ex:z ex:q \"+3\"^^xsd:integer .");
            // With equality, eq-rep-o copies the base triples too.
            let same = "<http://www.w3.org/2002/07/owl#sameAs>";
            let inferred: BTreeSet<String> = run.inferred().into_iter().filter(|t| t.contains(same)).collect();
            assert_eq!(
                inferred,
                same_as(&[
                    &[
                        "\"1\"^^xsd:integer",
                        "\"01\"^^xsd:integer",
                        "\"1.00\"^^xsd:decimal",
                        "\"1\"^^xsd:nonNegativeInteger",
                    ],
                    &["\"0\"^^xsd:nonNegativeInteger"],
                    &["\"3\"^^xsd:short", "\"+3\"^^xsd:integer"],
                ]),
                "{families:?}"
            );
        }
    }

    /// A restart keeps the classes, and a class's representative can hide
    /// its literals: they are read from the canon in the store's first
    /// round.
    #[test]
    fn dt_eq_after_a_restart() {
        // `rdfs:seeAlso owl:sameAs rdfs:label` makes two rule constants
        // equal: the run starts again with ex:a and ex:b representing the
        // literals.
        let run = testing::run(
            &["equality", "literal-values"],
            "ex:a owl:sameAs \"1\"^^xsd:integer . ex:b owl:sameAs \"01\"^^xsd:integer .
             rdfs:seeAlso owl:sameAs rdfs:label .",
        );
        assert!(run.has("ex:a owl:sameAs ex:b"));
        assert!(run.has("\"01\"^^xsd:integer owl:sameAs ex:a"));
    }

    fn findings(families: &[&str], turtle: &str) -> BTreeSet<String> {
        testing::run(families, turtle).findings()
    }

    /// dt-diff with eq-diff1: two literals of different values made the
    /// same. With equality, every two members of their class, and each with
    /// itself, are the same and different.
    #[test]
    fn dt_diff_gives_eq_diff1_for_literals_of_different_values_made_the_same() {
        let input = "ex:d a owl:FunctionalProperty . ex:x ex:d \"3\"^^xsd:integer , \"4\"^^xsd:integer .
                     ex:y ex:d \"5\"^^xsd:integer , \"05\"^^xsd:integer .";
        assert_eq!(
            findings(&["property", "equality", "literal-values", "checks"], input),
            expand(
                "eq-diff1 \"3\"^^xsd:integer \"4\"^^xsd:integer
                 eq-diff1 \"4\"^^xsd:integer \"3\"^^xsd:integer
                 eq-diff1 \"3\"^^xsd:integer \"3\"^^xsd:integer
                 eq-diff1 \"4\"^^xsd:integer \"4\"^^xsd:integer"
            )
        );
        // Without equality, only the pairs prp-fp makes the same.
        assert_eq!(
            findings(&["property", "literal-values", "checks"], input),
            expand(
                "eq-diff1 \"3\"^^xsd:integer \"4\"^^xsd:integer
                 eq-diff1 \"4\"^^xsd:integer \"3\"^^xsd:integer"
            )
        );
        // eq-diff1 is a check: nothing without `checks`; dt-diff is
        // `literal-values`: nothing without it.
        assert!(findings(&["property", "equality", "literal-values"], input).is_empty());
        assert!(findings(&["property", "equality", "checks"], input).is_empty());
    }

    /// An IRI made the same as two literals of different values is in
    /// their class; literals without a value never differ.
    #[test]
    fn dt_diff_in_a_class_with_an_iri() {
        let found = findings(
            &["equality", "literal-values", "checks"],
            "ex:a owl:sameAs \"3\"^^xsd:integer , \"3.5\"^^xsd:decimal .
             ex:b owl:sameAs \"1\"^^owl:real , \"x\"^^ex:dt , \"2\"^^owl:rational , \"abc\"^^xsd:integer .",
        );
        let members = ["ex:a", "\"3\"^^xsd:integer", "\"3.5\"^^xsd:decimal"];
        let mut lines = String::new();
        for x in members {
            for y in members {
                lines.push_str(&format!("eq-diff1 {x} {y}\n"));
            }
        }
        assert_eq!(found, expand(&lines));
    }

    /// `"1"^^xsd:integer owl:sameAs "2"^^xsd:integer`, a generalized triple
    /// (inserted: Turtle cannot write it), is reported; in a later run too.
    #[test]
    fn a_stated_same_as_between_two_values_is_reported() {
        for families in [
            &["equality", "literal-values", "checks"][..],
            &["literal-values", "checks"],
        ] {
            let mut run: Run = testing::run(families, "ex:x ex:p \"1\"^^xsd:integer , \"2\"^^xsd:integer .");
            assert!(run.findings().is_empty());
            let one = run.dict.intern_typed_literal("1", vocab::XSD_INTEGER).0;
            let two = run.dict.intern_typed_literal("2", vocab::XSD_INTEGER).0;
            run.store.insert(one, vocab::OWL_SAME_AS.0, two);
            run.add("");
            assert!(
                run.findings().contains(
                    &expand("eq-diff1 \"1\"^^xsd:integer \"2\"^^xsd:integer")
                        .pop_first()
                        .unwrap()
                ),
                "{families:?}: {:?}",
                run.findings()
            );
        }
    }
}
