//! cls-svf1, cls-svf2, cls-avf, cls-hv1 and cls-hv2: the rules that read a
//! restriction `x` with its `owl:onProperty p`.
//!
//! The restrictions are schema-sized: each round they are joined again
//! from `owl:someValuesFrom` (or `allValuesFrom`, `hasValue`) and
//! `owl:onProperty`, and each one is marked new when that join is new this
//! round (semi-naively: one of its two triples is recent). A new restriction
//! is joined with all of the instance data; an old one only with what is
//! recent. Restrictions are grouped by `p`, so each rule looks up a
//! property's table once per round, outside the tuple loops.

use super::{TYPE, range};
use crate::engine::{Pair, Round, Variable, View, join, join_views};
use crate::term::vocab;

const ON_PROPERTY: u32 = vocab::OWL_ON_PROPERTY.0;
const SOME_VALUES_FROM: u32 = vocab::OWL_SOME_VALUES_FROM.0;
const ALL_VALUES_FROM: u32 = vocab::OWL_ALL_VALUES_FROM.0;
const HAS_VALUE: u32 = vocab::OWL_HAS_VALUE.0;
const THING: u32 = vocab::OWL_THING.0;

/// The restriction rules, for one round. `instances` is `rdf:type` by object.
pub(super) fn rules(round: &mut Round<'_>, instances: Option<&Variable<Pair>>) {
    let Some(on_property) = round.by_subject(ON_PROPERTY) else {
        return;
    };
    let types = round.by_subject(TYPE).map(Variable::all);
    if let Some(svf) = round.by_subject(SOME_VALUES_FROM) {
        let restrictions = restrictions(svf, on_property);
        cls_svf1(round, &restrictions, instances, types.as_ref());
        cls_svf2(round, &restrictions);
    }
    if let Some(avf) = round.by_subject(ALL_VALUES_FROM) {
        let restrictions = restrictions(avf, on_property);
        cls_avf(round, &restrictions, instances, types.as_ref());
    }
    if let Some(hv) = round.by_subject(HAS_VALUE) {
        let restrictions = restrictions(hv, on_property);
        cls_hv1(round, &restrictions, instances);
        cls_hv2(round, &restrictions);
    }
}

/// `x owl:onProperty p` with `x <filler> y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Restriction {
    p: u32,
    /// New this round: joined with all of the instance data.
    new: bool,
    y: u32,
    x: u32,
}

/// The restrictions of `fillers` (`(x, y)` pairs) and `owl:onProperty`,
/// sorted by property.
fn restrictions(fillers: &Variable<Pair>, on_property: &Variable<Pair>) -> Vec<Restriction> {
    let mut all = Vec::new();
    join_views(&fillers.stable_view(), &on_property.stable_view(), |x, y, p| {
        all.push(Restriction { p, new: false, y, x });
    });
    join(fillers, on_property, |x, y, p| {
        all.push(Restriction { p, new: true, y, x })
    });
    all.sort_unstable();
    all
}

/// The restrictions grouped by property.
fn by_property(restrictions: &[Restriction]) -> impl Iterator<Item = (u32, &[Restriction])> {
    restrictions
        .chunk_by(|a, b| a.p == b.p)
        .map(|group| (group[0].p, group))
}

/// The `(y, x)` pairs of a group's old (`new` false) or new restrictions,
/// sorted.
fn fillers(group: &[Restriction], new: bool, only: impl Fn(&Restriction) -> bool, out: &mut Vec<Pair>) {
    out.clear();
    out.extend(group.iter().filter(|r| r.new == new && only(r)).map(|r| (r.y, r.x)));
    out.sort_unstable();
}

/// Calls `f` with each run of pairs that share a key.
fn runs(mut pairs: &[Pair], mut f: impl FnMut(u32, &[Pair])) {
    while let Some(&(k, _)) = pairs.first() {
        let n = pairs.iter().take_while(|t| t.0 == k).count();
        let (run, rest) = pairs.split_at(n);
        f(k, run);
        pairs = rest;
    }
}

/// cls-svf1: `x owl:someValuesFrom y`, `x owl:onProperty p`, `u p v`,
/// `v rdf:type y` → `u rdf:type x`. Join on v, with `p` by object: p's
/// recent pairs with every type of v; p's stable pairs with the recent
/// instances of y; for a new restriction, all of p with all instances of y.
fn cls_svf1(
    round: &mut Round<'_>,
    restrictions: &[Restriction],
    instances: Option<&Variable<Pair>>,
    types: Option<&View<'_, Pair>>,
) {
    let (Some(instances), Some(types)) = (instances, types) else {
        return;
    };
    let (mut old, mut new) = (Vec::new(), Vec::new());
    let mut matched: Vec<u32> = Vec::new();
    for (p, group) in by_property(restrictions) {
        let Some(edges) = round.by_object(p) else {
            continue;
        };
        fillers(group, false, |_| true, &mut old);
        fillers(group, true, |_| true, &mut new);
        if !old.is_empty() {
            runs(edges.recent(), |v, run| {
                matched.clear();
                types.for_each_value(v, |c| matched.extend(range(&old, c).iter().map(|&(_, x)| x)));
                for &x in &matched {
                    for &(_, u) in run {
                        round.emit(u, TYPE, x);
                    }
                }
            });
            for &(y, x) in &old {
                for &(_, v) in range(instances.recent(), y) {
                    for part in edges.stable() {
                        for &(_, u) in range(part, v) {
                            round.emit(u, TYPE, x);
                        }
                    }
                }
            }
        }
        for &(y, x) in &new {
            for part in instances.parts() {
                for &(_, v) in range(part, y) {
                    for p_part in edges.parts() {
                        for &(_, u) in range(p_part, v) {
                            round.emit(u, TYPE, x);
                        }
                    }
                }
            }
        }
    }
}

/// cls-svf2: `x owl:someValuesFrom owl:Thing`, `x owl:onProperty p`, `u p v`
/// → `u rdf:type x`. The subjects of p's recent pairs, or of all of them for
/// a new restriction.
fn cls_svf2(round: &mut Round<'_>, restrictions: &[Restriction]) {
    let (mut old, mut new) = (Vec::new(), Vec::new());
    for (p, group) in by_property(restrictions) {
        fillers(group, false, |r| r.y == THING, &mut old);
        fillers(group, true, |r| r.y == THING, &mut new);
        if old.is_empty() && new.is_empty() {
            continue;
        }
        let Some(edges) = round.by_subject(p) else {
            continue;
        };
        let mut subjects = |part: &[Pair], xs: &[Pair]| {
            runs(part, |u, _| {
                for &(_, x) in xs {
                    round.emit(u, TYPE, x);
                }
            });
        };
        subjects(edges.recent(), &old);
        if !new.is_empty() {
            for part in edges.parts() {
                subjects(part, &new);
            }
        }
    }
}

/// cls-avf: `x owl:allValuesFrom y`, `x owl:onProperty p`, `u rdf:type x`,
/// `u p v` → `v rdf:type y`. Join on u, with `p` by subject: the recent
/// instances of x with p's stable pairs; p's recent pairs with every type of
/// u; for a new restriction, all instances of x with all of p.
fn cls_avf(
    round: &mut Round<'_>,
    restrictions: &[Restriction],
    instances: Option<&Variable<Pair>>,
    types: Option<&View<'_, Pair>>,
) {
    let (mut old, mut new) = (Vec::new(), Vec::new());
    let mut matched: Vec<u32> = Vec::new();
    for (p, group) in by_property(restrictions) {
        let Some(edges) = round.by_subject(p) else {
            continue;
        };
        // (x, y) pairs here: the join is on x.
        let classes = |new: bool, out: &mut Vec<Pair>| {
            out.clear();
            out.extend(group.iter().filter(|r| r.new == new).map(|r| (r.x, r.y)));
            out.sort_unstable();
        };
        classes(false, &mut old);
        classes(true, &mut new);
        if let Some(instances) = instances {
            for &(x, y) in &old {
                for &(_, u) in range(instances.recent(), x) {
                    for part in edges.stable() {
                        for &(_, v) in range(part, u) {
                            round.emit(v, TYPE, y);
                        }
                    }
                }
            }
            for &(x, y) in &new {
                for part in instances.parts() {
                    for &(_, u) in range(part, x) {
                        for p_part in edges.parts() {
                            for &(_, v) in range(p_part, u) {
                                round.emit(v, TYPE, y);
                            }
                        }
                    }
                }
            }
        }
        if let Some(types) = types
            && !old.is_empty()
        {
            runs(edges.recent(), |u, run| {
                matched.clear();
                types.for_each_value(u, |c| matched.extend(range(&old, c).iter().map(|&(_, y)| y)));
                for &y in &matched {
                    for &(_, v) in run {
                        round.emit(v, TYPE, y);
                    }
                }
            });
        }
    }
}

/// cls-hv1: `x owl:hasValue y`, `x owl:onProperty p`, `u rdf:type x` →
/// `u p y`. The recent instances of x, or all of them for a new restriction.
fn cls_hv1(round: &mut Round<'_>, restrictions: &[Restriction], instances: Option<&Variable<Pair>>) {
    let Some(instances) = instances else { return };
    for r in restrictions {
        if r.new {
            for part in instances.parts() {
                for &(_, u) in range(part, r.x) {
                    round.emit(u, r.p, r.y);
                }
            }
        } else {
            for &(_, u) in range(instances.recent(), r.x) {
                round.emit(u, r.p, r.y);
            }
        }
    }
}

/// cls-hv2: `x owl:hasValue y`, `x owl:onProperty p`, `u p y` →
/// `u rdf:type x`. p's recent pairs, looked up by object among the group's
/// values; all of p's pairs for a new restriction. Reading p by subject
/// spares an object index of p, which cls-hv1 makes large.
fn cls_hv2(round: &mut Round<'_>, restrictions: &[Restriction]) {
    let (mut old, mut new) = (Vec::new(), Vec::new());
    for (p, group) in by_property(restrictions) {
        let Some(edges) = round.by_subject(p) else {
            continue;
        };
        fillers(group, false, |_| true, &mut old);
        fillers(group, true, |_| true, &mut new);
        let mut matches = |part: &[Pair], values: &[Pair]| {
            for &(u, v) in part {
                for &(_, x) in range(values, v) {
                    round.emit(u, TYPE, x);
                }
            }
        };
        if !old.is_empty() {
            matches(edges.recent(), &old);
        }
        if !new.is_empty() {
            for part in edges.parts() {
                matches(part, &new);
            }
        }
    }
}
