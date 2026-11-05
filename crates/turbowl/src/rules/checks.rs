//! The checks family (switch `checks`): the rules that conclude `false`,
//! dt-not-type aside (it looks inside literals: `datatype.rs`, the same
//! switch). Version 1's are here: cax-dw, cls-com, cls-nothing2, prp-pdw,
//! prp-asyp and prp-irp. Ticket 05 adds cax-adc, cls-maxc1, cls-maxqc1,
//! cls-maxqc2, prp-adp, prp-npa1, prp-npa2, eq-diff1, eq-diff2 and
//! eq-diff3.
//!
//! The rules derive no triple: each pushes a finding through
//! [`Context::report`], with its terms in the order the public `Diagnostic`
//! documents (the oracle crate's `Finding` and the reference evaluator's
//! `Diagnostics` give the same orders).
//!
//! # Semi-naive
//!
//! Every finding is reported in the round its last premise arrives: each
//! rule joins the recent part of one premise with all of the others, for
//! every premise. A finding may be reported more than once (two premises
//! recent in the same round); the reasoner deduplicates.
//!
//! The rules read `rdf:type`, `owl:disjointWith` and `owl:complementOf` by
//! object, an index the engine builds lazily ([`Round::by_object`]): the
//! first round that asks gets `None`, and the round after, the index holds
//! every pair as recent. A rule that skips a round for want of an index
//! loses nothing, because every one of its findings has a premise in that
//! index and the rule reads that relation's recent part from the index
//! (cax-dw and cls-com read their axioms by object only).
//!
//! # Equality
//!
//! Every constant is read through the round's canon ([`Round::constant`],
//! ADR 0003), and the findings name representatives; the reasoner expands
//! each term to every member of its class.

use super::Context;
use crate::engine::{Pair, Round, Variable, gallop};
use crate::term::{TermId, vocab};

/// The checks have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the checks.
pub(crate) fn rules(round: &mut Round<'_>, cx: &mut Context<'_>) {
    // Every object index is asked for before any rule returns early, so
    // that all the missing ones are built for the next round together.
    let types = round.by_object(round.constant(vocab::RDF_TYPE.0));
    let disjoint = round.by_object(round.constant(vocab::OWL_DISJOINT_WITH.0));
    let complement = round.by_object(round.constant(vocab::OWL_COMPLEMENT_OF.0));

    property_disjoint(round, cx);
    let Some(types) = types else { return };
    if let Some(axioms) = disjoint {
        class_pairs(types, axioms, |c1, c2, x| cx.report("cax-dw", &[x, c1, c2]));
    }
    if let Some(axioms) = complement {
        class_pairs(types, axioms, |c1, c2, x| cx.report("cls-com", &[c1, c2, x]));
    }
    for &(_, x) in with_key(types.recent(), round.constant(vocab::OWL_NOTHING.0)) {
        cx.report("cls-nothing2", &[x]);
    }
    asymmetric(round, types, cx);
    irreflexive(round, types, cx);
}

/// The pairs of a sorted slice whose first field is `key`.
fn with_key(pairs: &[Pair], key: u32) -> &[Pair] {
    let from = gallop(pairs, |t| t.0 < key);
    &from[..from.partition_point(|t| t.0 == key)]
}

/// cax-dw and cls-com: `c1 R c2`, `x rdf:type c1`, `x rdf:type c2` gives
/// `found(c1, c2, x)`. `types` is `rdf:type` by object, `(class, x)`, and
/// `axioms` is `R` by object, `(c2, c1)` (see the module doc). Each case
/// intersects two classes' instances, sorted runs of `types`, by merging:
///
/// - a recent axiom: all instances of `c1` with all of `c2`;
/// - an earlier axiom: the recent instances of `c1` with all of `c2`, and
///   the earlier instances of `c1` with the recent ones of `c2`.
///
/// The axioms are schema: reading the earlier ones each round costs their
/// number, not the data's.
fn class_pairs(types: &Variable<Pair>, axioms: &Variable<Pair>, mut found: impl FnMut(u32, u32, u32)) {
    let individual = |t: &Pair| t.1;
    for &(c2, c1) in axioms.recent().iter() {
        for a in types.parts() {
            for b in types.parts() {
                common(with_key(a, c1), with_key(b, c2), individual, |(_, x)| found(c1, c2, x));
            }
        }
    }
    let recent = types.recent();
    if recent.is_empty() {
        return;
    }
    for part in axioms.stable() {
        for &(c2, c1) in part.iter() {
            let r1 = with_key(recent, c1);
            if !r1.is_empty() {
                for b in types.parts() {
                    common(r1, with_key(b, c2), individual, |(_, x)| found(c1, c2, x));
                }
            }
            let r2 = with_key(recent, c2);
            if !r2.is_empty() {
                for a in types.stable() {
                    common(with_key(a, c1), r2, individual, |(_, x)| found(c1, c2, x));
                }
            }
        }
    }
}

/// Calls `f(pair)` with each pair of `a` whose key some pair of `b` has.
/// Both are sorted by `key` without repeats; the merge gallops over gaps.
fn common<K: Ord>(mut a: &[Pair], mut b: &[Pair], key: impl Fn(&Pair) -> K, mut f: impl FnMut(Pair)) {
    while let (Some(x), Some(y)) = (a.first(), b.first()) {
        let (kx, ky) = (key(x), key(y));
        match kx.cmp(&ky) {
            std::cmp::Ordering::Less => a = gallop(a, |t| key(t) < ky),
            std::cmp::Ordering::Greater => b = gallop(b, |t| key(t) < kx),
            std::cmp::Ordering::Equal => {
                f(*x);
                a = &a[1..];
                b = &b[1..];
            }
        }
    }
}

/// prp-pdw: `p1 owl:propertyDisjointWith p2`, `x p1 y`, `x p2 y` gives
/// (p1, p2, x, y). A recent axiom intersects the two tables whole; an
/// earlier one intersects each table's recent part with the other table
/// (recent with recent once). The axioms are schema: reading the earlier
/// ones each round costs their number, not the data's.
fn property_disjoint(round: &Round<'_>, cx: &mut Context<'_>) {
    let Some(axioms) = round.by_subject(round.constant(vocab::OWL_PROPERTY_DISJOINT_WITH.0)) else {
        return;
    };
    let tables = |p1: u32, p2: u32| Some((round.by_subject(p1)?, round.by_subject(p2)?));
    for &(p1, p2) in axioms.recent().iter() {
        let Some((t1, t2)) = tables(p1, p2) else { continue };
        for a in t1.parts() {
            for b in t2.parts() {
                common(a, b, |&t| t, |(x, y)| cx.report("prp-pdw", &[p1, p2, x, y]));
            }
        }
    }
    for part in axioms.stable() {
        for &(p1, p2) in part.iter() {
            let Some((t1, t2)) = tables(p1, p2) else { continue };
            let (r1, r2) = (t1.recent(), t2.recent());
            if !r1.is_empty() {
                for b in t2.parts() {
                    common(r1, b, |&t| t, |(x, y)| cx.report("prp-pdw", &[p1, p2, x, y]));
                }
            }
            if !r2.is_empty() {
                for a in t1.stable() {
                    common(a, r2, |&t| t, |(x, y)| cx.report("prp-pdw", &[p1, p2, x, y]));
                }
            }
        }
    }
}

/// Calls `f(p, table, whole)` for each property `p` typed `class` (a
/// constant, read through the round) that has a table: with `whole` when
/// the typing is recent (the rule reads the whole table), without it when
/// it is earlier and the table has a recent part (the rule reads that part).
fn properties_of<'a>(
    round: &Round<'a>,
    types: &'a Variable<Pair>,
    class: TermId,
    mut f: impl FnMut(u32, &'a Variable<Pair>, bool),
) {
    let class = TermId(round.constant(class.0));
    for &(_, p) in with_key(types.recent(), class.0) {
        if let Some(table) = round.by_subject(p) {
            f(p, table, true);
        }
    }
    for part in types.stable() {
        for &(_, p) in with_key(part, class.0) {
            if let Some(table) = round.by_subject(p).filter(|t| !t.recent().is_empty()) {
                f(p, table, false);
            }
        }
    }
}

/// prp-asyp: `p rdf:type owl:AsymmetricProperty`, `x p y`, `y p x` gives
/// (p, x, y). A recent pair `(x, y)` whose reverse is held is the last
/// premise of two findings, (p, x, y) and (p, y, x).
fn asymmetric(round: &Round<'_>, types: &Variable<Pair>, cx: &mut Context<'_>) {
    properties_of(round, types, vocab::OWL_ASYMMETRIC_PROPERTY, |p, table, whole| {
        if whole {
            for part in table.parts() {
                for &(x, y) in part {
                    if table.contains(&(y, x)) {
                        cx.report("prp-asyp", &[p, x, y]);
                    }
                }
            }
        } else {
            for &(x, y) in table.recent().iter() {
                if table.contains(&(y, x)) {
                    cx.report("prp-asyp", &[p, x, y]);
                    if x != y {
                        cx.report("prp-asyp", &[p, y, x]);
                    }
                }
            }
        }
    });
}

/// prp-irp: `p rdf:type owl:IrreflexiveProperty`, `x p x` gives (p, x).
fn irreflexive(round: &Round<'_>, types: &Variable<Pair>, cx: &mut Context<'_>) {
    properties_of(round, types, vocab::OWL_IRREFLEXIVE_PROPERTY, |p, table, whole| {
        let mut check = |part: &[Pair]| {
            for &(x, y) in part {
                if x == y {
                    cx.report("prp-irp", &[p, x]);
                }
            }
        };
        if whole {
            table.parts().for_each(&mut check);
        } else {
            check(table.recent());
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Store, join};
    use crate::rules::testing::{self, Run, expand};
    use crate::rules::{Family, datatype};
    use crate::term::Dictionary;
    use std::collections::BTreeSet;

    fn run(turtle: &str) -> Run {
        testing::run(&["checks"], turtle)
    }

    fn findings(turtle: &str) -> BTreeSet<String> {
        run(turtle).findings()
    }

    /// The id of a literal, interned if new.
    fn literal(run: &mut Run, lexical: &str, datatype: TermId) -> u32 {
        run.dict.intern_typed_literal(lexical, datatype).0
    }

    fn iri(run: &mut Run, iri: &str) -> u32 {
        run.dict.intern_iri(iri).0
    }

    #[test]
    fn cax_dw_names_the_individual_then_the_classes_as_stated() {
        assert_eq!(
            findings("ex:A owl:disjointWith ex:B . ex:x a ex:A , ex:B . ex:y a ex:A ."),
            expand("cax-dw ex:x ex:A ex:B")
        );
        // A class disjoint with itself has no instance.
        assert_eq!(
            findings("ex:A owl:disjointWith ex:A . ex:x a ex:A ."),
            expand("cax-dw ex:x ex:A ex:A")
        );
    }

    #[test]
    fn prp_pdw_names_the_properties_as_stated_then_subject_and_object() {
        assert_eq!(
            findings(
                "ex:p owl:propertyDisjointWith ex:q .
                 ex:x ex:p ex:y ; ex:q ex:y .
                 ex:x ex:p ex:z . ex:z ex:q ex:x ."
            ),
            expand("prp-pdw ex:p ex:q ex:x ex:y")
        );
    }

    #[test]
    fn cls_nothing2_names_the_individual() {
        assert_eq!(
            findings("ex:x a owl:Nothing . ex:y a ex:C ."),
            expand("cls-nothing2 ex:x")
        );
    }

    #[test]
    fn prp_asyp_names_the_property_subject_and_object_both_ways() {
        assert_eq!(
            findings(
                "ex:p a owl:AsymmetricProperty .
                 ex:x ex:p ex:y . ex:y ex:p ex:x . ex:y ex:p ex:z .
                 ex:s ex:p ex:s .
                 ex:a ex:q ex:b . ex:b ex:q ex:a ."
            ),
            expand(
                "prp-asyp ex:p ex:x ex:y
                 prp-asyp ex:p ex:y ex:x
                 prp-asyp ex:p ex:s ex:s"
            )
        );
    }

    #[test]
    fn prp_irp_names_the_property_and_the_individual() {
        assert_eq!(
            findings(
                "ex:p a owl:IrreflexiveProperty .
                 ex:x ex:p ex:x . ex:x ex:p ex:y . ex:z ex:q ex:z ."
            ),
            expand("prp-irp ex:p ex:x")
        );
    }

    #[test]
    fn cls_com_names_the_classes_as_stated_then_the_individual() {
        assert_eq!(
            findings("ex:A owl:complementOf ex:B . ex:x a ex:A , ex:B . ex:y a ex:B ."),
            expand("cls-com ex:A ex:B ex:x")
        );
    }

    #[test]
    fn findings_on_generalized_terms() {
        let mut run = run("ex:A owl:disjointWith ex:B . ex:C owl:complementOf ex:D .
             ex:p a owl:IrreflexiveProperty , owl:AsymmetricProperty .
             ex:q owl:propertyDisjointWith ex:r .
             _:b1 a ex:A , ex:B .");
        let ty = vocab::RDF_TYPE.0;
        let five = literal(&mut run, "5", vocab::XSD_INTEGER);
        let s = literal(&mut run, "s", vocab::XSD_STRING);
        let [a, b, c, d, p, q, r] =
            ["A", "B", "C", "D", "p", "q", "r"].map(|local| iri(&mut run, &format!("http://example.org/{local}")));
        for (x, pred, o) in [
            (five, ty, a),
            (five, ty, b),
            (s, ty, c),
            (s, ty, d),
            (s, ty, vocab::OWL_NOTHING.0),
            (five, p, five),
            (five, p, s),
            (s, p, five),
            (s, q, five),
            (s, r, five),
        ] {
            run.store.insert(x, pred, o);
        }
        run.add("");
        assert_eq!(
            run.findings(),
            expand(
                "cax-dw _:b1 ex:A ex:B
                 cax-dw \"5\"^^xsd:integer ex:A ex:B
                 cls-com ex:C ex:D \"s\"
                 cls-nothing2 \"s\"
                 prp-irp ex:p \"5\"^^xsd:integer
                 prp-asyp ex:p \"5\"^^xsd:integer \"5\"^^xsd:integer
                 prp-asyp ex:p \"5\"^^xsd:integer \"s\"
                 prp-asyp ex:p \"s\" \"5\"^^xsd:integer
                 prp-pdw ex:q ex:r \"s\" \"5\"^^xsd:integer"
            )
        );
    }

    /// Each finding's premises in two parts: the finding appears only once
    /// the second part is added, whichever part comes last.
    #[test]
    fn a_finding_is_found_when_its_last_premise_arrives() {
        let cases = [
            (
                "ex:A owl:disjointWith ex:B . ex:x a ex:A .",
                "ex:x a ex:B .",
                "cax-dw ex:x ex:A ex:B",
            ),
            (
                "ex:x a ex:A , ex:B .",
                "ex:A owl:disjointWith ex:B .",
                "cax-dw ex:x ex:A ex:B",
            ),
            (
                "ex:A owl:disjointWith ex:B . ex:x a ex:B .",
                "ex:x a ex:A .",
                "cax-dw ex:x ex:A ex:B",
            ),
            (
                "ex:A owl:complementOf ex:B . ex:x a ex:B .",
                "ex:x a ex:A .",
                "cls-com ex:A ex:B ex:x",
            ),
            (
                "ex:x a ex:A , ex:B .",
                "ex:A owl:complementOf ex:B .",
                "cls-com ex:A ex:B ex:x",
            ),
            (
                "ex:p owl:propertyDisjointWith ex:q . ex:x ex:p ex:y .",
                "ex:x ex:q ex:y .",
                "prp-pdw ex:p ex:q ex:x ex:y",
            ),
            (
                "ex:p owl:propertyDisjointWith ex:q . ex:x ex:q ex:y .",
                "ex:x ex:p ex:y .",
                "prp-pdw ex:p ex:q ex:x ex:y",
            ),
            (
                "ex:x ex:p ex:y ; ex:q ex:y .",
                "ex:p owl:propertyDisjointWith ex:q .",
                "prp-pdw ex:p ex:q ex:x ex:y",
            ),
            (
                "ex:p a owl:AsymmetricProperty . ex:x ex:p ex:y .",
                "ex:y ex:p ex:x .",
                "prp-asyp ex:p ex:x ex:y\nprp-asyp ex:p ex:y ex:x",
            ),
            (
                "ex:x ex:p ex:y . ex:y ex:p ex:x .",
                "ex:p a owl:AsymmetricProperty .",
                "prp-asyp ex:p ex:x ex:y\nprp-asyp ex:p ex:y ex:x",
            ),
            (
                "ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:y .",
                "ex:x ex:p ex:x .",
                "prp-irp ex:p ex:x",
            ),
            (
                "ex:x ex:p ex:x .",
                "ex:p a owl:IrreflexiveProperty .",
                "prp-irp ex:p ex:x",
            ),
            ("ex:x a ex:C .", "ex:x a owl:Nothing .", "cls-nothing2 ex:x"),
        ];
        for (first, second, expected) in cases {
            let mut run = run(first);
            assert!(run.findings().is_empty(), "{first} alone: {:?}", run.findings());
            run.add(second);
            assert_eq!(run.findings(), expand(expected), "{first} then {second}");
        }
    }

    /// `x ex:next y` and `y rdf:type c` give `x rdf:type c`: types that
    /// arrive one round after another, after the object indexes exist.
    fn backwards(round: &mut Round<'_>, cx: &mut Context<'_>) {
        let next = cx.terms.get_iri("http://example.org/next").expect("in the input").0;
        let (Some(next), Some(types)) = (round.by_object(next), round.by_subject(vocab::RDF_TYPE.0)) else {
            return;
        };
        let mut derived = Vec::new();
        join(next, types, |_, x, c| derived.push((x, c)));
        for (x, c) in derived {
            round.emit(x, vocab::RDF_TYPE.0, c);
        }
    }

    static BACKWARDS: Family = Family {
        name: "backwards",
        switch: None,
        rules: backwards,
        facts: &[],
    };

    #[test]
    fn premises_derived_in_later_rounds_are_joined() {
        let run = testing::run_families(
            vec![&BACKWARDS, testing::family("checks")],
            "ex:A owl:disjointWith ex:B . ex:C owl:complementOf ex:D .
             ex:x0 ex:next ex:x1 . ex:x1 ex:next ex:x2 . ex:x2 ex:next ex:x3 . ex:x3 ex:next ex:x4 .
             ex:x4 a ex:A , ex:C , owl:Nothing . ex:x0 a ex:B . ex:x1 a ex:D .",
        );
        assert!(run.rounds >= 5, "{} rounds", run.rounds);
        let mut expected = expand(
            "cax-dw ex:x0 ex:A ex:B
             cls-com ex:C ex:D ex:x1
             cls-com ex:C ex:D ex:x0",
        );
        for i in 0..5 {
            expected.extend(expand(&format!("cls-nothing2 ex:x{i}")));
        }
        assert_eq!(run.findings(), expected);
    }

    // -----------------------------------------------------------------------
    // Scaling
    // -----------------------------------------------------------------------

    /// A fixed schema (disjoint and complementary classes, disjoint,
    /// asymmetric and irreflexive properties) over `n` individuals, about
    /// 13 triples each, literals included, with a known number of findings:
    /// the dictionary and the store, its triples queued.
    fn instances(n: usize) -> (Dictionary, Store, usize) {
        let mut d = Dictionary::new();
        let mut store = Store::new();
        let ex = |d: &mut Dictionary, local: &str| d.intern_iri(&format!("http://example.org/{local}")).0;
        let [a, b, c, comp, p, q, asym, irr, age, label] =
            ["A", "B", "C", "D", "p", "q", "asym", "irr", "age", "label"].map(|l| ex(&mut d, l));
        let ty = vocab::RDF_TYPE.0;
        store.insert(a, vocab::OWL_DISJOINT_WITH.0, b);
        store.insert(c, vocab::OWL_COMPLEMENT_OF.0, comp);
        store.insert(p, vocab::OWL_PROPERTY_DISJOINT_WITH.0, q);
        store.insert(asym, ty, vocab::OWL_ASYMMETRIC_PROPERTY.0);
        store.insert(irr, ty, vocab::OWL_IRREFLEXIVE_PROPERTY.0);
        let x: Vec<u32> = (0..=n).map(|i| ex(&mut d, &format!("x{i}"))).collect();
        for i in 0..n {
            let (xi, next) = (x[i], x[i + 1]);
            let every = |k: usize| i % k == 0;
            store.insert(xi, ty, a);
            if every(8) {
                store.insert(xi, ty, b);
            }
            if every(2) {
                store.insert(xi, ty, c);
            }
            if every(16) {
                store.insert(xi, ty, comp);
            }
            if every(32) {
                store.insert(xi, ty, vocab::OWL_NOTHING.0);
            }
            store.insert(xi, p, next);
            if every(4) {
                store.insert(xi, q, next);
            }
            store.insert(xi, asym, next);
            if every(8) {
                store.insert(next, asym, xi);
            }
            store.insert(xi, irr, next);
            if every(16) {
                store.insert(xi, irr, xi);
            }
            let years = d.intern_typed_literal(&i.to_string(), vocab::XSD_INT).0;
            store.insert(xi, age, years);
            store.insert(years, ty, vocab::XSD_INTEGER.0);
            let name = d.intern_typed_literal(&format!("x {i}"), vocab::XSD_STRING).0;
            store.insert(xi, label, name);
            if every(8) {
                let bad = d.intern_typed_literal(&format!("{i}a"), vocab::XSD_INTEGER).0;
                store.insert(xi, age, bad);
            }
            if every(16) {
                store.insert(name, ty, vocab::XSD_INTEGER.0);
            }
        }
        // cax-dw, cls-com, cls-nothing2, prp-pdw, prp-asyp (both ways),
        // prp-irp, dt-not-type (ill-typed, then typed).
        let expected = n / 8 + n / 16 + n / 32 + n / 4 + 2 * (n / 8) + n / 16 + n / 8 + n / 16;
        (d, store, expected)
    }

    /// The rounds of the `checks` switch's families (this one and
    /// `datatype`) over `input`: the seconds spent in them (the engine's own
    /// steps not counted), and the distinct findings.
    fn timed_run((d, mut store, _): (Dictionary, Store, usize)) -> (f64, usize) {
        let mut findings = Vec::new();
        let mut spent = std::time::Duration::ZERO;
        let mut cx = Context {
            terms: &d,
            findings: &mut findings,
        };
        store.run(|round| {
            let start = std::time::Instant::now();
            rules(round, &mut cx);
            datatype::rules(round, &mut cx);
            spent += start.elapsed();
        });
        findings.sort_unstable();
        findings.dedup();
        (spent.as_secs_f64(), findings.len())
    }

    /// With a fixed schema, the switch's run is linear in the instance
    /// data. Only the families' calls are timed: the engine's sorting of
    /// the input into tables is the engine's own scaling test.
    #[test]
    fn scaling_checks_linear_in_instances() {
        let _lock = crate::scaling::lock();
        const LEAST: usize = 1_024;
        let sizes = [LEAST, 2 * LEAST, 4 * LEAST, 8 * LEAST, 16 * LEAST];
        for n in sizes {
            let input = instances(n);
            let expected = input.2;
            assert_eq!(timed_run(input).1, expected, "findings at {n}");
        }
        // Each measurement runs the same number of individuals in all
        // (16 inputs of the least size, one of the largest), so short runs
        // are not timed against the clock alone.
        crate::scaling::check(
            "scaling_checks_linear_in_instances",
            crate::scaling::Growth::Linear,
            &sizes,
            |n| {
                let reps = 16 * LEAST / n;
                let inputs: Vec<_> = (0..reps).map(|_| instances(n)).collect();
                inputs.into_iter().map(|i| timed_run(i).0).sum::<f64>() / reps as f64
            },
        );
    }
}
