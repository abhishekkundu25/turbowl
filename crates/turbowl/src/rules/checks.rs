//! The checks family (switch `checks`): the rules that conclude `false`,
//! dt-not-type aside (it looks inside literals: `datatype.rs`, the same
//! switch): cax-dw, cax-adc, cls-com, cls-nothing2, cls-maxc1, cls-maxqc1,
//! cls-maxqc2, prp-irp, prp-asyp, prp-pdw, prp-adp, prp-npa1, prp-npa2,
//! eq-diff1, eq-diff2 and eq-diff3.
//!
//! The rules derive no triple: each pushes a finding through
//! [`Context::report`], with its terms in the order the public `Diagnostic`
//! documents (the oracle crate's `Finding` and the reference evaluator's
//! `Diagnostics` give the same orders). Each rule's function says its order.
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
//! (cax-dw and cls-com read their axioms by object only). cls-maxqc1 reads
//! its property by object too, and reads that property's recent pairs from
//! the index once it exists (see [`max_zero`]).
//!
//! # Lists
//!
//! cax-adc, prp-adp, eq-diff2 and eq-diff3 read the list of an
//! `owl:AllDisjointClasses`, `owl:AllDisjointProperties` or
//! `owl:AllDifferent` axiom by its positions ([`Positions`]), as the class
//! family reads its lists (`class/lists.rs`): the *live* cells are the
//! cells on some path from the head to `rdf:nil` whose every cell has an
//! `rdf:first`, and a position is a live cell. Two terms are at positions
//! `i < j` when a cell listing the second follows a cell listing the first
//! along live cells, one `rdf:rest` or more. In a list that loops through
//! live cells, a cell follows itself, so its terms are at two positions; a
//! cyclic list that never reaches `rdf:nil` has no position. `rdf:nil` ends
//! a list, and is read on as a cell when `owl:sameAs` gives it a class
//! with other members, as in `class/lists.rs`. Lists are schema-sized:
//! they are read again in each round in which one of their premises may
//! have changed.
//!
//! # Equality
//!
//! Every constant is read through the round's canon ([`Round::constant`],
//! ADR 0003), and the findings name representatives; the reasoner expands
//! each term to every member of its class. eq-diff1, eq-diff2 and eq-diff3
//! read `owl:sameAs` as the store holds it: with the `equality` switch on, a
//! class of two or more is one stored `r owl:sameAs r`, and a lone
//! `x owl:sameAs x` exists only when a triple states it; with the switch
//! off, the stated pairs as they are.
//!
//! eq-ref (`x owl:sameAs x` for every term, switch `trivial`) is an output
//! rule, mostly never stored (`trivial.rs`), so these three rules do not
//! find it in the store. With `trivial` on ([`Context::switches`]), they
//! treat every term as `owl:sameAs` itself, as eq-ref makes it: a recent
//! `x owl:differentFrom x` alone gives eq-diff1, and a term at two
//! positions of a changed `owl:AllDifferent` list gives eq-diff2 or
//! eq-diff3. Both are in representatives, so `a owl:differentFrom b` with
//! `a` and `b` made equal is `r owl:differentFrom r`.

use super::Context;
use crate::Switch;
use crate::engine::{Pair, Round, Variable, View, gallop};
use crate::term::{DocId, TermId, vocab};
use rustc_hash::FxHashMap;

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
    different(round, cx);
    negative_assertions(round, cx);
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
    let lists = Lists::new(round);
    all_different(round, types, &lists, cx);
    all_disjoint_classes(round, types, &lists, cx);
    all_disjoint_properties(round, types, &lists, cx);
    max_zero(round, types, cx);
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

/// Calls `f(x, y)` for each pair both tables hold that this round can find:
/// every pair when `whole` (a recent axiom), otherwise each table's recent
/// part with the other table (recent with recent once).
fn intersect(a: &Variable<Pair>, b: &Variable<Pair>, whole: bool, mut f: impl FnMut(u32, u32)) {
    if whole {
        for pa in a.parts() {
            for pb in b.parts() {
                common(pa, pb, |&t| t, |(x, y)| f(x, y));
            }
        }
        return;
    }
    let (ra, rb) = (a.recent(), b.recent());
    if !ra.is_empty() {
        for pb in b.parts() {
            common(ra, pb, |&t| t, |(x, y)| f(x, y));
        }
    }
    if !rb.is_empty() {
        for pa in a.stable() {
            common(pa, rb, |&t| t, |(x, y)| f(x, y));
        }
    }
}

/// prp-pdw: `p1 owl:propertyDisjointWith p2`, `x p1 y`, `x p2 y` gives
/// (p1, p2, x, y). A recent axiom intersects the two tables whole; an
/// earlier one intersects each table's recent part with the other table.
/// The axioms are schema: reading the earlier ones each round costs their
/// number, not the data's.
fn property_disjoint(round: &Round<'_>, cx: &mut Context<'_>) {
    let Some(axioms) = round.by_subject(round.constant(vocab::OWL_PROPERTY_DISJOINT_WITH.0)) else {
        return;
    };
    let tables = |p1: u32, p2: u32| Some((round.by_subject(p1)?, round.by_subject(p2)?));
    let parts = axioms.stable().iter().map(|p| (&p[..], false));
    for (part, whole) in parts.chain([(&axioms.recent()[..], true)]) {
        for &(p1, p2) in part {
            let Some((t1, t2)) = tables(p1, p2) else { continue };
            intersect(t1, t2, whole, |x, y| cx.report("prp-pdw", &[p1, p2, x, y]));
        }
    }
}

/// eq-diff1: `x owl:sameAs y`, `x owl:differentFrom y` gives (x, y), as
/// `x owl:differentFrom y` states them: the two tables intersected. With
/// `equality` on, the only `owl:sameAs` pairs are `r owl:sameAs r` (see
/// the module doc), so this finds a stored `r owl:differentFrom r` that
/// meets one. With `trivial` on, every recent `x owl:differentFrom x`
/// meets eq-ref's `x owl:sameAs x`.
fn different(round: &Round<'_>, cx: &mut Context<'_>) {
    let Some(different) = round.by_subject(round.constant(vocab::OWL_DIFFERENT_FROM.0)) else {
        return;
    };
    if cx.switches.is_on(Switch::Trivial) {
        for &(x, y) in different.recent().iter().filter(|(x, y)| x == y) {
            cx.report("eq-diff1", &[x, y]);
        }
    }
    if let Some(same) = round.by_subject(round.constant(vocab::OWL_SAME_AS.0)) {
        intersect(different, same, false, |x, y| cx.report("eq-diff1", &[x, y]));
    }
}

/// prp-npa1 and prp-npa2: `x owl:sourceIndividual i1`, `x
/// owl:assertionProperty p`, and `x owl:targetIndividual i2` (prp-npa1) or
/// `x owl:targetValue i2` (prp-npa2), with `i1 p i2`, give (i1, p, i2):
/// the source, the property and the target, the triple the negative
/// assertion denies. An assertion with a recent part looks the triple up
/// in all of `p`'s table, an earlier one in its recent part. The
/// assertions are read each round, as prp-pdw's axioms are.
fn negative_assertions(round: &Round<'_>, cx: &mut Context<'_>) {
    let table = |iri: TermId| round.by_subject(round.constant(iri.0));
    let (Some(sources), Some(properties)) = (
        table(vocab::OWL_SOURCE_INDIVIDUAL),
        table(vocab::OWL_ASSERTION_PROPERTY),
    ) else {
        return;
    };
    let all_properties = properties.all();
    for (code, target) in [
        ("prp-npa1", vocab::OWL_TARGET_INDIVIDUAL),
        ("prp-npa2", vocab::OWL_TARGET_VALUE),
    ] {
        let Some(targets) = table(target) else { continue };
        let all_targets = targets.all();
        let parts = sources.stable().iter().map(|p| (&p[..], false));
        for (part, recent) in parts.chain([(&sources.recent()[..], true)]) {
            for &(x, i1) in part {
                all_properties.for_each_value(x, |p| {
                    let Some(facts) = round.by_subject(p) else { return };
                    let recent = recent || properties.recent().contains(&(x, p));
                    all_targets.for_each_value(x, |i2| {
                        let found = if recent || targets.recent().contains(&(x, i2)) {
                            facts.contains(&(i1, i2))
                        } else {
                            facts.recent().contains(&(i1, i2))
                        };
                        if found {
                            cx.report(code, &[i1, p, i2]);
                        }
                    });
                });
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

/// The `(x, head)` pairs of the axioms `x rdf:type class`, `x predicate
/// head` (both constants, read through the round), each with whether one
/// of its two triples is recent.
fn axioms(round: &Round<'_>, types: &Variable<Pair>, class: TermId, predicate: TermId) -> Vec<(u32, u32, bool)> {
    let class = round.constant(class.0);
    let Some(heads) = round.by_subject(round.constant(predicate.0)) else {
        return Vec::new();
    };
    let all = heads.all();
    let mut out = Vec::new();
    let parts = types.stable().iter().map(|p| (&p[..], false));
    for (part, recent) in parts.chain([(&types.recent()[..], true)]) {
        for &(_, x) in with_key(part, class) {
            all.for_each_value(x, |head| {
                out.push((x, head, recent || heads.recent().contains(&(x, head))));
            });
        }
    }
    out
}

/// eq-diff2 and eq-diff3: `x rdf:type owl:AllDifferent`, `x owl:members
/// (z1 ... zn)` (eq-diff2) or `x owl:distinctMembers (z1 ... zn)`
/// (eq-diff3), and `zi owl:sameAs zj` with `i < j`, give (x, zi, zj): the
/// axiom, then the two terms, the one at the earlier position first. A
/// changed list (a recent axiom triple, or a cell with a recent
/// `rdf:first` or `rdf:rest`) is joined with all of `owl:sameAs`, an
/// unchanged one with its recent part. With `trivial` on, a term is
/// `owl:sameAs` itself (eq-ref), so a term at two positions of a changed
/// list gives (x, z, z).
fn all_different(round: &Round<'_>, types: &Variable<Pair>, lists: &Lists<'_>, cx: &mut Context<'_>) {
    let same = round.by_subject(round.constant(vocab::OWL_SAME_AS.0));
    let reflexive = cx.switches.is_on(Switch::Trivial);
    if same.is_none() && !reflexive {
        return;
    }
    let all_same = same.map_or_else(|| View::one(&[]), Variable::all);
    let recent_same: &[Pair] = same.map_or(&[], |s| &s.recent()[..]);
    for (code, predicate) in [
        ("eq-diff2", vocab::OWL_MEMBERS),
        ("eq-diff3", vocab::OWL_DISTINCT_MEMBERS),
    ] {
        for (x, head, recent) in axioms(round, types, vocab::OWL_ALL_DIFFERENT, predicate) {
            if !recent && lists.touched.is_empty() && recent_same.is_empty() {
                continue;
            }
            let list = lists.read(head);
            if recent || list.touched {
                for a in list.terms() {
                    all_same.for_each_value(a, |b| {
                        if list.ordered(a, b) {
                            cx.report(code, &[x, a, b]);
                        }
                    });
                    if reflexive && list.ordered(a, a) {
                        cx.report(code, &[x, a, a]);
                    }
                }
            } else {
                for &(a, b) in recent_same {
                    if list.ordered(a, b) {
                        cx.report(code, &[x, a, b]);
                    }
                }
            }
        }
    }
}

/// cax-adc: `x rdf:type owl:AllDisjointClasses`, `x owl:members (c1 ...
/// cn)`, `z rdf:type ci`, `z rdf:type cj` with `i < j` give (z, ci, cj):
/// the individual, then the two classes, the one at the earlier position
/// first (as cax-dw names them). A changed list is joined with all the
/// instances of its classes, an unchanged one with the recent ones; each
/// such instance's classes in the list are then read by subject.
fn all_disjoint_classes(round: &Round<'_>, types: &Variable<Pair>, lists: &Lists<'_>, cx: &mut Context<'_>) {
    let Some(by_subject) = round.by_subject(round.constant(vocab::RDF_TYPE.0)) else {
        return;
    };
    let by_subject = by_subject.all();
    for (_, head, recent) in axioms(round, types, vocab::OWL_ALL_DISJOINT_CLASSES, vocab::OWL_MEMBERS) {
        if !recent && lists.touched.is_empty() && types.recent().is_empty() {
            continue;
        }
        let list = lists.read(head);
        let whole = recent || list.touched;
        let mut individuals = Vec::new();
        for c in list.terms() {
            if whole {
                for part in types.parts() {
                    individuals.extend(with_key(part, c).iter().map(|t| t.1));
                }
            } else {
                individuals.extend(with_key(types.recent(), c).iter().map(|t| t.1));
            }
        }
        individuals.sort_unstable();
        individuals.dedup();
        let mut classes = Vec::new();
        for z in individuals {
            classes.clear();
            by_subject.for_each_value(z, |c| {
                if list.contains(c) {
                    classes.push(c);
                }
            });
            for &ci in &classes {
                for &cj in &classes {
                    if list.ordered(ci, cj) {
                        cx.report("cax-adc", &[z, ci, cj]);
                    }
                }
            }
        }
    }
}

/// prp-adp: `x rdf:type owl:AllDisjointProperties`, `x owl:members (p1 ...
/// pn)`, `u pi v`, `u pj v` with `i < j` give (pi, pj, u, v): the two
/// properties, the one at the earlier position first, then the subject and
/// the object (as prp-pdw names them). Each ordered pair of the list's
/// properties intersects their tables, whole for a changed list.
fn all_disjoint_properties(round: &Round<'_>, types: &Variable<Pair>, lists: &Lists<'_>, cx: &mut Context<'_>) {
    for (_, head, recent) in axioms(round, types, vocab::OWL_ALL_DISJOINT_PROPERTIES, vocab::OWL_MEMBERS) {
        let list = lists.read(head);
        let whole = recent || list.touched;
        let tables: Vec<(u32, &Variable<Pair>)> =
            list.terms().filter_map(|p| Some((p, round.by_subject(p)?))).collect();
        for &(pi, ti) in &tables {
            for &(pj, tj) in &tables {
                if list.ordered(pi, pj) {
                    intersect(ti, tj, whole, |u, v| cx.report("prp-adp", &[pi, pj, u, v]));
                }
            }
        }
    }
}

/// cls-maxc1, cls-maxqc1 and cls-maxqc2, each with the finding (x, u, y):
/// the restriction, the individual and the value.
///
/// - cls-maxc1: `x owl:maxCardinality "0"^^xsd:nonNegativeInteger`, `x
///   owl:onProperty p`, `u rdf:type x`, `u p y`.
/// - cls-maxqc1: `x owl:maxQualifiedCardinality
///   "0"^^xsd:nonNegativeInteger`, `x owl:onProperty p`, `x owl:onClass
///   c`, `u rdf:type x`, `u p y`, `y rdf:type c`.
/// - cls-maxqc2: the same with `x owl:onClass owl:Thing` and no `y
///   rdf:type owl:Thing`. With `owl:onClass owl:Thing`, cls-maxqc1 holds
///   too where `y rdf:type owl:Thing` is stated or derived.
///
/// The literal is the term the W3C table writes, as it is (`"00"` or an
/// `xsd:integer` zero is another term), read through the round as every
/// constant is. The restrictions are schema, read each round; [`restricted`]
/// joins their instances with the property. cls-maxqc1's third premise,
/// `y rdf:type c`, reaches `u` through `p` by object: when that index is
/// missing it is asked for, and the round after, its pairs are all recent
/// and the rule reads `p`'s recent pairs from it, so none is missed.
fn max_zero(round: &mut Round<'_>, types: &Variable<Pair>, cx: &mut Context<'_>) {
    let zero = oxrdf::LiteralRef::new_typed_literal("0", oxrdf::vocab::xsd::NON_NEGATIVE_INTEGER);
    let Some(zero) = cx.terms.get_term(zero.into(), DocId(0)) else {
        return;
    };
    // The literal is no vocabulary term: when a merge changes its class,
    // every restriction is joined whole ([`Round::class_moved`]).
    let moved = round.class_moved(zero.0);
    let zero = round.constant(zero.0);
    let constant = |round: &Round<'_>, iri: TermId| round.constant(iri.0);
    let Some(on_property) = round.by_subject(constant(round, vocab::OWL_ON_PROPERTY)) else {
        return;
    };
    // (x, p, c, whole): c is `u32::MAX` for cls-maxc1.
    let mut restrictions = Vec::new();
    let mut with_property = |x: u32, recent: bool, c: u32, c_recent: bool| {
        on_property.all().for_each_value(x, |p| {
            let whole = moved || recent || c_recent || on_property.recent().contains(&(x, p));
            restrictions.push((x, p, c, whole));
        });
    };
    if let Some(table) = round.by_subject(constant(round, vocab::OWL_MAX_CARDINALITY)) {
        for (x, recent) in valued(table, zero) {
            with_property(x, recent, u32::MAX, false);
        }
    }
    let qualified = round.by_subject(constant(round, vocab::OWL_MAX_QUALIFIED_CARDINALITY));
    let on_class = round.by_subject(constant(round, vocab::OWL_ON_CLASS));
    if let (Some(table), Some(on_class)) = (qualified, on_class) {
        for (x, recent) in valued(table, zero) {
            on_class.all().for_each_value(x, |c| {
                with_property(x, recent, c, on_class.recent().contains(&(x, c)));
            });
        }
    }
    let thing = constant(round, vocab::OWL_THING);
    for (x, p, c, whole) in restrictions {
        let Some(table) = round.by_subject(p) else { continue };
        let recent = || table.recent().iter().copied();
        if c == u32::MAX {
            restricted(types, x, table, recent(), whole, |u, y| {
                cx.report("cls-maxc1", &[x, u, y])
            });
            continue;
        }
        if c == thing {
            restricted(types, x, table, recent(), whole, |u, y| {
                cx.report("cls-maxqc2", &[x, u, y])
            });
        }
        let by_object = round.by_object(p);
        let typed = |u: u32, y: u32, cx: &mut Context<'_>| {
            if types.contains(&(c, y)) {
                cx.report("cls-maxqc1", &[x, u, y]);
            }
        };
        match by_object {
            Some(values) => {
                let reversed = values.recent().iter().map(|&(y, u)| (u, y));
                restricted(types, x, table, reversed, whole, |u, y| typed(u, y, cx));
                if !whole {
                    for &(_, y) in with_key(types.recent(), c) {
                        for part in values.parts() {
                            for &(_, u) in with_key(part, y) {
                                if types.contains(&(x, u)) {
                                    cx.report("cls-maxqc1", &[x, u, y]);
                                }
                            }
                        }
                    }
                }
            }
            None => restricted(types, x, table, recent(), whole, |u, y| typed(u, y, cx)),
        }
    }
}

/// The subjects of `table`'s pairs whose object is `value`, each with
/// whether its pair is recent.
fn valued(table: &Variable<Pair>, value: u32) -> Vec<(u32, bool)> {
    let parts = table.stable().iter().map(|p| (&p[..], false));
    parts
        .chain([(&table.recent()[..], true)])
        .flat_map(|(part, recent)| part.iter().filter(move |t| t.1 == value).map(move |t| (t.0, recent)))
        .collect()
}

/// Calls `f(u, y)` for each `u rdf:type x`, `u p y` this round can find
/// (`types` by object, `table` is `p`'s by subject): every one when
/// `whole`, otherwise the recent instances of `x` with all of `p`, and the
/// recent pairs of `p` (`recent`, as `(u, y)`) whose `u` is an instance.
fn restricted(
    types: &Variable<Pair>,
    x: u32,
    table: &Variable<Pair>,
    recent: impl Iterator<Item = Pair>,
    whole: bool,
    mut f: impl FnMut(u32, u32),
) {
    let values = |u: u32, f: &mut dyn FnMut(u32, u32)| {
        for part in table.parts() {
            for &(_, y) in with_key(part, u) {
                f(u, y);
            }
        }
    };
    if whole {
        for part in types.parts() {
            for &(_, u) in with_key(part, x) {
                values(u, &mut f);
            }
        }
        return;
    }
    for &(_, u) in with_key(types.recent(), x) {
        values(u, &mut f);
    }
    for (u, y) in recent {
        if types.contains(&(x, u)) {
            f(u, y);
        }
    }
}

/// Reads the lists of the all-different and all-disjoint axioms for one
/// round (see the module doc).
#[derive(Debug)]
struct Lists<'a> {
    first: View<'a, Pair>,
    rest: View<'a, Pair>,
    /// The representative of `rdf:nil`, and whether it is read on as a cell.
    nil: u32,
    nil_is_a_cell: bool,
    /// Cells with a recent `rdf:first` or `rdf:rest`, sorted.
    touched: Vec<u32>,
}

impl<'a> Lists<'a> {
    fn new(round: &Round<'a>) -> Self {
        let first = round.by_subject(round.constant(vocab::RDF_FIRST.0));
        let rest = round.by_subject(round.constant(vocab::RDF_REST.0));
        let nil = round.constant(vocab::RDF_NIL.0);
        let mut touched = Vec::new();
        for v in [first, rest].into_iter().flatten() {
            touched.extend(v.recent().iter().map(|&(cell, _)| cell));
        }
        touched.sort_unstable();
        touched.dedup();
        let view = |v: Option<&'a Variable<Pair>>| v.map_or_else(|| View::one(&[]), Variable::all);
        Self {
            first: view(first),
            rest: view(rest),
            nil,
            nil_is_a_cell: round.canon().members(nil).len() > 1,
            touched,
        }
    }

    /// The positions of the list at `head`.
    fn read(&self, head: u32) -> Positions {
        // The cells, breadth first along rdf:rest from the head, each once.
        let mut cells = vec![head];
        let mut index: FxHashMap<u32, u32> = FxHashMap::default();
        index.insert(head, 0);
        let (mut firsts, mut rests, mut to_nil) = (Vec::new(), Vec::new(), Vec::new());
        let mut touched = false;
        let mut i = 0;
        while i < cells.len() {
            let term = cells[i];
            touched |= self.touched.binary_search(&term).is_ok();
            let mut first = Vec::new();
            self.first.for_each_value(term, |k| first.push(k));
            let (mut rest, mut nil) = (Vec::new(), false);
            self.rest.for_each_value(term, |tail| {
                if tail == self.nil {
                    nil = true;
                    if !self.nil_is_a_cell {
                        return;
                    }
                }
                let next = *index.entry(tail).or_insert_with(|| {
                    cells.push(tail);
                    (cells.len() - 1) as u32
                });
                rest.push(next as usize);
            });
            firsts.push(first);
            rests.push(rest);
            to_nil.push(nil);
            i += 1;
        }
        // Well formed: an rdf:first, and a rest that is rdf:nil or well
        // formed. A backward pass settles a simple list; repeat until
        // nothing changes.
        let n = cells.len();
        let mut well_formed = vec![false; n];
        loop {
            let mut grew = false;
            for i in (0..n).rev() {
                if !well_formed[i] && !firsts[i].is_empty() && (to_nil[i] || rests[i].iter().any(|&r| well_formed[r])) {
                    well_formed[i] = true;
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        // The live cells, numbered breadth first from a well-formed head
        // through well-formed cells, and each one's live successors.
        let mut position = vec![u32::MAX; n];
        let mut live = Vec::new();
        if well_formed[0] {
            position[0] = 0;
            live.push(0);
        }
        let mut k = 0;
        while k < live.len() {
            for &r in &rests[live[k]] {
                if well_formed[r] && position[r] == u32::MAX {
                    position[r] = live.len() as u32;
                    live.push(r);
                }
            }
            k += 1;
        }
        let next: Vec<Vec<u32>> = live
            .iter()
            .map(|&c| {
                rests[c]
                    .iter()
                    .filter(|&&r| well_formed[r])
                    .map(|&r| position[r])
                    .collect()
            })
            .collect();
        let mut cells_of: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
        for (k, &c) in live.iter().enumerate() {
            for &term in &firsts[c] {
                cells_of.entry(term).or_default().push(k as u32);
            }
        }
        // A chain (one live successor each, always later): positions are
        // the numbering. Otherwise, the cells after each cell.
        let chain = next
            .iter()
            .enumerate()
            .all(|(k, n)| n.len() <= 1 && n.iter().all(|&s| s as usize > k));
        let after = (!chain).then(|| {
            let words = live.len().div_ceil(64);
            (0..live.len())
                .map(|k| {
                    let mut bits = vec![0u64; words];
                    let mut stack = next[k].clone();
                    while let Some(s) = stack.pop() {
                        let (w, b) = (s as usize / 64, 1u64 << (s % 64));
                        if bits[w] & b == 0 {
                            bits[w] |= b;
                            stack.extend_from_slice(&next[s as usize]);
                        }
                    }
                    bits
                })
                .collect()
        });
        Positions {
            cells_of,
            after,
            touched,
        }
    }
}

/// The positions of one list (see the module doc).
#[derive(Debug)]
struct Positions {
    /// Each listed term, and the live cells that list it, numbered breadth
    /// first from the head, in increasing order.
    cells_of: FxHashMap<u32, Vec<u32>>,
    /// `None` for a chain, whose numbering is the list order; otherwise,
    /// for each live cell, the cells after it (a bitset).
    after: Option<Vec<Vec<u64>>>,
    /// A cell has a recent `rdf:first` or `rdf:rest`.
    touched: bool,
}

impl Positions {
    /// The listed terms.
    fn terms(&self) -> impl Iterator<Item = u32> + '_ {
        self.cells_of.keys().copied()
    }

    fn contains(&self, term: u32) -> bool {
        self.cells_of.contains_key(&term)
    }

    /// Whether `a` and `b` are at positions `i < j`: a cell listing `b`
    /// follows a cell listing `a`.
    fn ordered(&self, a: u32, b: u32) -> bool {
        let (Some(at_a), Some(at_b)) = (self.cells_of.get(&a), self.cells_of.get(&b)) else {
            return false;
        };
        match &self.after {
            None => at_a[0] < at_b[at_b.len() - 1],
            Some(after) => at_a.iter().any(|&i| {
                at_b.iter()
                    .any(|&j| after[i as usize][j as usize / 64] >> (j % 64) & 1 == 1)
            }),
        }
    }
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

    fn with_equality(turtle: &str) -> BTreeSet<String> {
        testing::run(&["checks", "equality"], turtle).findings()
    }

    #[test]
    fn eq_diff1_names_the_terms_as_stated_in_different_from() {
        // Equality off: the stated pairs.
        assert_eq!(
            findings(
                "ex:a owl:differentFrom ex:b ; owl:sameAs ex:b .
                 ex:c owl:differentFrom ex:d . ex:d owl:sameAs ex:c .
                 ex:e owl:differentFrom ex:e . ex:f owl:differentFrom ex:f ; owl:sameAs ex:f ."
            ),
            expand(
                "eq-diff1 ex:a ex:b
                 eq-diff1 ex:f ex:f"
            )
        );
        // Equality on: a class of two holds `r owl:sameAs r`, so every pair
        // of its members; a lone `x owl:differentFrom x` holds nothing.
        assert_eq!(
            with_equality("ex:a owl:differentFrom ex:b . ex:b owl:sameAs ex:a . ex:e owl:differentFrom ex:e ."),
            expand(
                "eq-diff1 ex:a ex:a
                 eq-diff1 ex:a ex:b
                 eq-diff1 ex:b ex:a
                 eq-diff1 ex:b ex:b"
            )
        );
    }

    #[test]
    fn eq_diff2_and_eq_diff3_name_the_axiom_then_the_terms_in_list_order() {
        // Equality off: `zi owl:sameAs zj` as stated, with i < j.
        assert_eq!(
            findings(
                "_:d a owl:AllDifferent ; owl:members ( ex:a ex:b ex:c ) .
                 _:e a owl:AllDifferent ; owl:distinctMembers ( ex:a ex:b ex:c ) .
                 ex:a owl:sameAs ex:c . ex:c owl:sameAs ex:b . ex:x owl:sameAs ex:a ."
            ),
            expand(
                "eq-diff2 _:d ex:a ex:c
                 eq-diff3 _:e ex:a ex:c"
            )
        );
        // Equality on: the members of the class, each way.
        assert_eq!(
            with_equality("_:d a owl:AllDifferent ; owl:members ( ex:a ex:b ex:c ) . ex:c owl:sameAs ex:a ."),
            expand(
                "eq-diff2 _:d ex:a ex:a
                 eq-diff2 _:d ex:a ex:c
                 eq-diff2 _:d ex:c ex:a
                 eq-diff2 _:d ex:c ex:c"
            )
        );
    }

    /// A term listed twice is at two positions; without eq-ref, it is
    /// `owl:sameAs` itself only where a triple says so.
    #[test]
    fn a_term_listed_twice_needs_a_stated_same_as() {
        let input = "_:d a owl:AllDifferent ; owl:members ( ex:a ex:b ex:a ) .";
        assert!(with_equality(input).is_empty());
        assert_eq!(
            with_equality(&format!("{input} ex:a owl:sameAs ex:a .")),
            expand("eq-diff2 _:d ex:a ex:a")
        );
    }

    /// With `trivial` on, every term is `owl:sameAs` itself (eq-ref, never
    /// stored): `x owl:differentFrom x` alone, and a term listed twice, are
    /// findings, with `equality` on or off, and in a list completed by a
    /// later run.
    #[test]
    fn with_trivial_every_term_is_the_same_as_itself() {
        let input = "ex:a owl:differentFrom ex:a . ex:b owl:differentFrom ex:c .
            _:d a owl:AllDifferent ; owl:members ( ex:a ex:b ex:a ) .
            _:e a owl:AllDifferent ; owl:distinctMembers ( ex:c ex:c ) .
            _:f a owl:AllDifferent ; owl:members ( ex:a ex:b ) .";
        let expected = expand(
            "eq-diff1 ex:a ex:a
             eq-diff2 _:d ex:a ex:a
             eq-diff3 _:e ex:c ex:c",
        );
        for families in [&["checks", "trivial"][..], &["checks", "equality", "trivial"]] {
            assert_eq!(testing::run(families, input).findings(), expected, "{families:?}");
        }
        assert!(with_equality(input).is_empty());
        // In representatives: `ex:g` and `ex:h` are made equal, so the
        // stated `ex:g owl:differentFrom ex:h` meets the class's pairs.
        assert_eq!(
            testing::run(
                &["checks", "equality", "trivial"],
                "ex:g owl:differentFrom ex:h . ex:g owl:sameAs ex:h ."
            )
            .findings(),
            expand(
                "eq-diff1 ex:g ex:g
                 eq-diff1 ex:g ex:h
                 eq-diff1 ex:h ex:g
                 eq-diff1 ex:h ex:h"
            )
        );
        // The list's last cell comes in a later, incremental, run.
        let mut run = testing::run(
            &["checks", "trivial"],
            "_:d a owl:AllDifferent ; owl:members ex:l1 . ex:l1 rdf:first ex:a ; rdf:rest ex:l2 .",
        );
        assert!(run.findings().is_empty());
        run.add("ex:l2 rdf:first ex:a ; rdf:rest rdf:nil .");
        assert_eq!(run.findings(), expand("eq-diff2 _:d ex:a ex:a"));
    }

    /// Positions follow the live cells: a list that loops back through
    /// live cells lists its terms again; a branch orders only along its
    /// path; a cycle that never reaches `rdf:nil`, or a cell without a
    /// first, has no position.
    #[test]
    fn positions_follow_the_paths_to_nil() {
        let all_different =
            |list: &str, same: &str| findings(&format!("_:d a owl:AllDifferent ; owl:members _:l1 . {list} {same}"));
        let looping = "_:l1 rdf:first ex:a ; rdf:rest _:l2 . _:l2 rdf:first ex:b ; rdf:rest _:l1 , rdf:nil .";
        assert_eq!(
            all_different(looping, "ex:b owl:sameAs ex:a . ex:b owl:sameAs ex:b ."),
            expand(
                "eq-diff2 _:d ex:b ex:a
                 eq-diff2 _:d ex:b ex:b"
            )
        );
        let branching = "_:l1 rdf:first ex:a ; rdf:rest _:l2 , _:l3 .
             _:l2 rdf:first ex:b ; rdf:rest rdf:nil . _:l3 rdf:first ex:c ; rdf:rest rdf:nil .";
        assert_eq!(
            all_different(
                branching,
                "ex:a owl:sameAs ex:c . ex:b owl:sameAs ex:c . ex:c owl:sameAs ex:b ."
            ),
            expand("eq-diff2 _:d ex:a ex:c")
        );
        let cyclic = "_:l1 rdf:first ex:a ; rdf:rest _:l2 . _:l2 rdf:first ex:b ; rdf:rest _:l1 .";
        assert!(all_different(cyclic, "ex:a owl:sameAs ex:b . ex:a owl:sameAs ex:a .").is_empty());
        let gap = "_:l1 rdf:first ex:a ; rdf:rest _:l2 . _:l2 rdf:rest _:l3 .
             _:l3 rdf:first ex:b ; rdf:rest rdf:nil .";
        assert!(all_different(gap, "ex:a owl:sameAs ex:b .").is_empty());
    }

    #[test]
    fn cax_adc_names_the_individual_then_the_classes_in_list_order() {
        assert_eq!(
            findings(
                "_:x a owl:AllDisjointClasses ; owl:members ( ex:A ex:B ex:C ) .
                 ex:z a ex:C , ex:A . ex:y a ex:B . ex:w a ex:A , ex:B , ex:C .
                 _:o a owl:AllDisjointClasses ; owl:members ( ex:D ex:D ) . ex:v a ex:D ."
            ),
            expand(
                "cax-adc ex:z ex:A ex:C
                 cax-adc ex:w ex:A ex:B
                 cax-adc ex:w ex:A ex:C
                 cax-adc ex:w ex:B ex:C
                 cax-adc ex:v ex:D ex:D"
            )
        );
    }

    #[test]
    fn prp_adp_names_the_properties_in_list_order_then_subject_and_object() {
        assert_eq!(
            findings(
                "_:x a owl:AllDisjointProperties ; owl:members ( ex:p ex:q ex:r ) .
                 ex:u ex:r ex:v ; ex:p ex:v . ex:u ex:q ex:w . ex:s ex:q ex:t . ex:t ex:r ex:s ."
            ),
            expand("prp-adp ex:p ex:r ex:u ex:v")
        );
    }

    #[test]
    fn prp_npa_names_source_property_and_target() {
        assert_eq!(
            findings(
                "_:n1 owl:sourceIndividual ex:a ; owl:assertionProperty ex:p ; owl:targetIndividual ex:b .
                 _:n2 owl:sourceIndividual ex:a ; owl:assertionProperty ex:age ; owl:targetValue 5 .
                 _:n3 owl:sourceIndividual ex:a ; owl:assertionProperty ex:p ; owl:targetIndividual ex:c .
                 _:n4 owl:sourceIndividual ex:a ; owl:assertionProperty ex:p ; owl:targetValue ex:b .
                 ex:a ex:p ex:b ; ex:age 5 . ex:c ex:p ex:a ."
            ),
            expand(
                "prp-npa1 ex:a ex:p ex:b
                 prp-npa2 ex:a ex:age \"5\"^^xsd:integer
                 prp-npa2 ex:a ex:p ex:b"
            )
        );
    }

    /// The cardinality is the literal the W3C table writes, as it is.
    #[test]
    fn max_cardinality_zero_names_restriction_individual_and_value() {
        assert_eq!(
            findings(
                "ex:R owl:maxCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .
                 ex:S owl:maxCardinality \"00\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .
                 ex:T owl:maxCardinality 0 ; owl:onProperty ex:p .
                 ex:U owl:maxCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .
                 ex:u a ex:R , ex:S , ex:T , ex:U ; ex:p ex:y . ex:w a ex:R ."
            ),
            expand("cls-maxc1 ex:R ex:u ex:y")
        );
    }

    /// A merge can make a stable restriction's cardinality the literal of
    /// the rule: the rule then joins every restriction again (a chaos find:
    /// dt-eq made `"0"^^xsd:nonNegativeInteger` the same as an `xsd:integer`
    /// zero interned before it, which stayed the representative).
    #[test]
    fn max_cardinality_zero_after_a_merge_moves_the_literal() {
        let input = "ex:R owl:maxCardinality \"00\"^^xsd:integer ; owl:onProperty ex:p .
                     ex:u a ex:R ; ex:p ex:y .";
        let found = testing::run(
            &["checks", "equality", "literal-values"],
            &format!("{input} ex:s ex:q \"0\"^^xsd:nonNegativeInteger ."),
        )
        .findings();
        assert_eq!(found, expand("cls-maxc1 ex:R ex:u ex:y"));
        // Made the same by prp-fp, a round later.
        let found = testing::run(
            &["property", "checks", "equality"],
            &format!("{input} ex:d a owl:FunctionalProperty . ex:s ex:d \"00\"^^xsd:integer , \"0\"^^xsd:nonNegativeInteger ."),
        )
        .findings();
        assert!(
            found.contains(&expand("cls-maxc1 ex:R ex:u ex:y").pop_first().unwrap()),
            "{found:?}"
        );
    }

    #[test]
    fn max_qualified_cardinality_zero_names_restriction_individual_and_value() {
        assert_eq!(
            findings(
                "ex:Q owl:maxQualifiedCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p ;
                     owl:onClass ex:C .
                 ex:T owl:maxQualifiedCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:q ;
                     owl:onClass owl:Thing .
                 ex:u a ex:Q , ex:T ; ex:p ex:y , ex:z ; ex:q ex:y , ex:t . ex:y a ex:C . ex:t a owl:Thing ."
            ),
            expand(
                "cls-maxqc1 ex:Q ex:u ex:y
                 cls-maxqc2 ex:T ex:u ex:y
                 cls-maxqc2 ex:T ex:u ex:t
                 cls-maxqc1 ex:T ex:u ex:t"
            )
        );
    }

    /// A finding on a merged term names every member of its class.
    #[test]
    fn findings_expand_per_class_member() {
        assert_eq!(
            with_equality(
                "_:x a owl:AllDisjointClasses ; owl:members ( ex:A ex:B ) . ex:z a ex:A , ex:B .
                 ex:z owl:sameAs ex:z2 ."
            ),
            expand(
                "cax-adc ex:z ex:A ex:B
                 cax-adc ex:z2 ex:A ex:B"
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
            (
                "ex:a owl:differentFrom ex:b .",
                "ex:a owl:sameAs ex:b .",
                "eq-diff1 ex:a ex:b",
            ),
            (
                "ex:a owl:sameAs ex:b .",
                "ex:a owl:differentFrom ex:b .",
                "eq-diff1 ex:a ex:b",
            ),
            (
                "ex:d a owl:AllDifferent ; owl:members ( ex:a ex:b ) .",
                "ex:a owl:sameAs ex:b .",
                "eq-diff2 ex:d ex:a ex:b",
            ),
            (
                "ex:d owl:members ( ex:a ex:b ) . ex:a owl:sameAs ex:b .",
                "ex:d a owl:AllDifferent .",
                "eq-diff2 ex:d ex:a ex:b",
            ),
            (
                "ex:d a owl:AllDifferent . ex:a owl:sameAs ex:b .",
                "ex:d owl:distinctMembers ( ex:a ex:b ) .",
                "eq-diff3 ex:d ex:a ex:b",
            ),
            (
                "ex:d a owl:AllDifferent ; owl:members ex:l1 . ex:l1 rdf:first ex:a ; rdf:rest ex:l2 .
                 ex:l2 rdf:first ex:b . ex:a owl:sameAs ex:b .",
                "ex:l2 rdf:rest rdf:nil .",
                "eq-diff2 ex:d ex:a ex:b",
            ),
            (
                "ex:x a owl:AllDisjointClasses ; owl:members ( ex:A ex:B ) . ex:z a ex:A .",
                "ex:z a ex:B .",
                "cax-adc ex:z ex:A ex:B",
            ),
            (
                "ex:z a ex:A , ex:B .",
                "ex:x a owl:AllDisjointClasses ; owl:members ( ex:A ex:B ) .",
                "cax-adc ex:z ex:A ex:B",
            ),
            (
                "ex:x a owl:AllDisjointProperties ; owl:members ( ex:p ex:q ) . ex:u ex:q ex:v .",
                "ex:u ex:p ex:v .",
                "prp-adp ex:p ex:q ex:u ex:v",
            ),
            (
                "ex:u ex:p ex:v ; ex:q ex:v .",
                "ex:x a owl:AllDisjointProperties ; owl:members ( ex:p ex:q ) .",
                "prp-adp ex:p ex:q ex:u ex:v",
            ),
            (
                "ex:n owl:sourceIndividual ex:a ; owl:assertionProperty ex:p ; owl:targetIndividual ex:b .",
                "ex:a ex:p ex:b .",
                "prp-npa1 ex:a ex:p ex:b",
            ),
            (
                "ex:a ex:p ex:b . ex:n owl:sourceIndividual ex:a ; owl:assertionProperty ex:p .",
                "ex:n owl:targetIndividual ex:b .",
                "prp-npa1 ex:a ex:p ex:b",
            ),
            (
                "ex:n owl:sourceIndividual ex:a ; owl:assertionProperty ex:age ; owl:targetValue 5 .",
                "ex:a ex:age 5 .",
                "prp-npa2 ex:a ex:age \"5\"^^xsd:integer",
            ),
            (
                "ex:R owl:maxCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p . ex:u a ex:R .",
                "ex:u ex:p ex:y .",
                "cls-maxc1 ex:R ex:u ex:y",
            ),
            (
                "ex:R owl:maxCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p . ex:u ex:p ex:y .",
                "ex:u a ex:R .",
                "cls-maxc1 ex:R ex:u ex:y",
            ),
            (
                "ex:u a ex:R ; ex:p ex:y . ex:R owl:onProperty ex:p .",
                "ex:R owl:maxCardinality \"0\"^^xsd:nonNegativeInteger .",
                "cls-maxc1 ex:R ex:u ex:y",
            ),
            (
                "ex:R owl:maxQualifiedCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p ;
                 owl:onClass ex:C . ex:u a ex:R ; ex:p ex:y .",
                "ex:y a ex:C .",
                "cls-maxqc1 ex:R ex:u ex:y",
            ),
            (
                "ex:R owl:maxQualifiedCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p ;
                 owl:onClass ex:C . ex:u a ex:R . ex:y a ex:C .",
                "ex:u ex:p ex:y .",
                "cls-maxqc1 ex:R ex:u ex:y",
            ),
            (
                "ex:R owl:maxQualifiedCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p ;
                 owl:onClass ex:C . ex:u ex:p ex:y . ex:y a ex:C .",
                "ex:u a ex:R .",
                "cls-maxqc1 ex:R ex:u ex:y",
            ),
            (
                "ex:R owl:maxQualifiedCardinality \"0\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .
                 ex:u a ex:R ; ex:p ex:y .",
                "ex:R owl:onClass owl:Thing .",
                "cls-maxqc2 ex:R ex:u ex:y",
            ),
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
            switches: crate::Switches::default(),
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

    /// One `owl:AllDifferent` axiom over a list of `n` terms, read both as
    /// `owl:members` and as `owl:distinctMembers`: every 64th position lists
    /// the term before it again, stated `owl:sameAs` itself, and every 32nd
    /// term is stated `owl:sameAs` the next one. The dictionary, the store
    /// (equality off: the pairs as stated) and the number of findings.
    fn all_different_list(n: usize) -> (Dictionary, Store, usize) {
        let mut d = Dictionary::new();
        let mut store = Store::new();
        let ex = |d: &mut Dictionary, local: &str| d.intern_iri(&format!("http://example.org/{local}")).0;
        let (members, distinct) = (ex(&mut d, "members"), ex(&mut d, "distinct"));
        let same = vocab::OWL_SAME_AS.0;
        let cells: Vec<u32> = (0..n).map(|i| ex(&mut d, &format!("l{i}"))).collect();
        let mut terms: Vec<u32> = Vec::with_capacity(n);
        let mut found = 0;
        for i in 0..n {
            if i % 64 == 63 {
                let again = terms[i - 1];
                terms.push(again);
                store.insert(again, same, again);
                found += 1;
            } else {
                terms.push(ex(&mut d, &format!("z{i}")));
            }
        }
        for i in (0..n - 2).step_by(32) {
            store.insert(terms[i], same, terms[i + 1]);
            found += 1;
        }
        for (i, (&cell, &term)) in cells.iter().zip(&terms).enumerate() {
            store.insert(cell, vocab::RDF_FIRST.0, term);
            let rest = cells.get(i + 1).copied().unwrap_or(vocab::RDF_NIL.0);
            store.insert(cell, vocab::RDF_REST.0, rest);
        }
        for (axiom, predicate) in [
            (members, vocab::OWL_MEMBERS.0),
            (distinct, vocab::OWL_DISTINCT_MEMBERS.0),
        ] {
            store.insert(axiom, vocab::RDF_TYPE.0, vocab::OWL_ALL_DIFFERENT.0);
            store.insert(axiom, predicate, cells[0]);
        }
        (d, store, 2 * found)
    }

    /// eq-diff2 and eq-diff3 read a list's positions once, so their run is
    /// linear in the list's length, not quadratic in its pairs.
    #[test]
    fn scaling_eq_diff_linear_in_list_length() {
        let _lock = crate::scaling::lock();
        const LEAST: usize = 2_048;
        let sizes = [LEAST, 2 * LEAST, 4 * LEAST, 8 * LEAST, 16 * LEAST];
        for n in sizes {
            let input = all_different_list(n);
            let expected = input.2;
            assert_eq!(timed_run(input).1, expected, "findings at {n}");
        }
        crate::scaling::check(
            "scaling_eq_diff_linear_in_list_length",
            crate::scaling::Growth::Linear,
            &sizes,
            |n| {
                let reps = 16 * LEAST / n;
                let inputs: Vec<_> = (0..reps).map(|_| all_different_list(n)).collect();
                inputs.into_iter().map(|i| timed_run(i).0).sum::<f64>() / reps as f64
            },
        );
    }
}
