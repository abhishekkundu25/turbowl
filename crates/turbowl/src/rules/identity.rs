//! The identity family (switch `identity`): the rules that make
//! individuals the same, prp-key (`owl:hasKey`), cls-maxc2
//! (`owl:maxCardinality 1`), cls-maxqc3 and cls-maxqc4
//! (`owl:maxQualifiedCardinality 1`, with `owl:onClass` a class or
//! `owl:Thing`; OWL 2 Profiles, Tables 5 and 6).
//!
//! The rules derive `owl:sameAs` in their W3C forms: every pair the body
//! matches, `y owl:sameAs y` included. With the `equality` switch on, the
//! equality family merges what they make equal through the canon, like any
//! `owl:sameAs` (ADR 0003): a group of k equal terms is k² triples in the
//! round that finds it, and one stored triple after the merge.
//!
//! # prp-key
//!
//! `c owl:hasKey (p1 ... pn)`: two individuals of `c` with a common value
//! for every `pi` are the same. The key list is read as the W3C `LIST[...]`
//! pattern in Datalog form (`class/lists.rs`): a path of cells from the
//! head to `rdf:nil`, each cell giving one of its `rdf:first`s. The values
//! of each property are independent, so a path matters only through the
//! set of properties it names: the key's *property sets*, read again each
//! round (lists are schema-sized). Ending a path at `rdf:nil` is always
//! weaker than reading on, so the representative of `rdf:nil` ends a path
//! even when its class holds other terms.
//!
//! For each property set, the individuals of `c` are grouped by hashing
//! their tuples of values (one tuple for each choice of a value per
//! property), and the members of a group are the same: no two individuals
//! are compared. Semi-naive: a tuple is recent when the individual's type,
//! one of its values, the `owl:hasKey` triple or a cell of the list is
//! recent, and a group pairs its recent members with all of its members. A
//! key with nothing recent is skipped; one with something recent groups its
//! whole class again, at the cost of the class's size.
//!
//! # cls-maxc2, cls-maxqc3, cls-maxqc4
//!
//! A restriction `x` with `owl:maxCardinality` or
//! `owl:maxQualifiedCardinality` `"1"^^xsd:nonNegativeInteger` (that term,
//! read through the canon, as the W3C table writes it), `owl:onProperty p`
//! and, when qualified, `owl:onClass c`: the values of `p` of an individual
//! `u` of type `x` (those typed `c`, unless `c` is `owl:Thing`) are the
//! same. Restrictions are schema-sized and joined again each round; one is
//! new when one of its triples is recent. Per restriction, each `u` is read
//! once: all of its values are paired when the restriction is new or `u`'s
//! type `x` is recent; otherwise its recent values with all of them, `u`
//! coming from `p`'s recent pairs (and, qualified, from the recent
//! instances of `c`, through `p`'s object index). Each `u` costs its own
//! values: the work is linear in the data, plus the pairs it derives.
//!
//! Object indexes are lazy ([`Round::by_object`]): a rule that finds one
//! missing skips its join that round, and the next round, where the whole
//! index is recent, reads every tuple of it as recent (`class.rs`).
//!
//! Constants are read through the round's canon ([`Round::constant`]).

use super::Context;
use crate::engine::{Pair, Round, Variable, View, gallop};
use crate::term::{DocId, TermId, vocab};
use oxrdf::LiteralRef;
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;

/// The identity rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the identity rules.
pub(crate) fn rules(round: &mut Round<'_>, cx: &mut Context<'_>) {
    let ty = round.constant(vocab::RDF_TYPE.0);
    // `rdf:type` by object, (class, individual).
    let Some(instances) = round.by_object(ty) else {
        return;
    };
    let same_as = round.constant(vocab::OWL_SAME_AS.0);
    prp_key(round, same_as, instances);
    let one = LiteralRef::new_typed_literal("1", oxrdf::vocab::xsd::NON_NEGATIVE_INTEGER);
    if let Some(one) = cx.terms.get_term(one.into(), DocId::NONE) {
        let one = round.constant(one.0);
        let types = round.by_subject(ty).map_or_else(|| View::one(&[]), Variable::all);
        cardinalities(round, same_as, one, instances, &types);
    }
}

/// The pairs of a sorted part whose first term is `k`.
fn range(part: &[Pair], k: u32) -> &[Pair] {
    let start = part.partition_point(|t| t.0 < k);
    let rest = &part[start..];
    let after = gallop(rest, |t| t.0 <= k);
    &rest[..rest.len() - after.len()]
}

/// A variable's sorted parts, each with whether it is the recent one.
fn tagged(v: &Variable<Pair>) -> impl Iterator<Item = (&[Pair], bool)> {
    v.stable()
        .iter()
        .map(|b| (&b[..], false))
        .chain(Some((&v.recent()[..], true)))
}

/// Pushes `(value, recent)` for each pair `(k, value)` of `v`.
fn values_of(v: &Variable<Pair>, k: u32, out: &mut Vec<(u32, bool)>) {
    for (part, recent) in tagged(v) {
        out.extend(range(part, k).iter().map(|&(_, value)| (value, recent)));
    }
}

/// Emits `a owl:sameAs b` and `b owl:sameAs a` for each `a` of `recent`
/// and `b` of `all`.
fn pair_up(round: &mut Round<'_>, same_as: u32, recent: impl Iterator<Item = u32>, all: &[u32]) {
    for a in recent {
        for &b in all {
            round.emit(a, same_as, b);
            round.emit(b, same_as, a);
        }
    }
}

// ---------------------------------------------------------------------------
// prp-key
// ---------------------------------------------------------------------------

/// prp-key: `c owl:hasKey (p1 ... pn)`, `x` and `y` of type `c`, `x pi zi`
/// and `y pi zi` for every `i` → `x owl:sameAs y`.
fn prp_key(round: &mut Round<'_>, same_as: u32, instances: &Variable<Pair>) {
    let Some(keys) = round.by_subject(round.constant(vocab::OWL_HAS_KEY.0)) else {
        return;
    };
    let first = round.by_subject(round.constant(vocab::RDF_FIRST.0));
    let rest = round.by_subject(round.constant(vocab::RDF_REST.0));
    let mut touched: Vec<u32> = [first, rest]
        .into_iter()
        .flatten()
        .flat_map(|v| v.recent().iter().map(|&(cell, _)| cell))
        .collect();
    touched.sort_unstable();
    touched.dedup();
    let lists = Lists {
        first: first.map_or_else(|| View::one(&[]), Variable::all),
        rest: rest.map_or_else(|| View::one(&[]), Variable::all),
        nil: round.constant(vocab::RDF_NIL.0),
        touched,
    };
    let mut keyed = Keyed::default();
    for (part, recent) in tagged(keys) {
        for &(class, head) in part {
            let (sets, touched) = lists.property_sets(head);
            for set in &sets {
                keyed.group(round, same_as, instances, class, set, recent || touched);
            }
        }
    }
}

/// The `rdf:first` and `rdf:rest` tables, to read key lists.
struct Lists<'a> {
    first: View<'a, Pair>,
    rest: View<'a, Pair>,
    /// The representative of `rdf:nil`.
    nil: u32,
    /// Cells with a recent `rdf:first` or `rdf:rest`, sorted.
    touched: Vec<u32>,
}

impl Lists<'_> {
    /// The property sets of the list at `head` (each sorted, without
    /// duplicates), and whether a cell of the list is touched this round.
    ///
    /// A cell's sets are `{p}` for each first `p` when a rest is `rdf:nil`,
    /// and `{p} ∪ s` for each first `p` and each set `s` of a rest that is
    /// not `rdf:nil`, to a fixpoint: a cycle adds nothing a shorter path
    /// does not give, and a list that never reaches `rdf:nil` has none.
    fn property_sets(&self, head: u32) -> (BTreeSet<Vec<u32>>, bool) {
        // The cells, breadth first along rdf:rest, each once, never past
        // rdf:nil; with their firsts, and their rests as cell indexes.
        let mut cells = vec![head];
        let mut firsts: Vec<Vec<u32>> = Vec::new();
        let mut rests: Vec<(bool, Vec<usize>)> = Vec::new();
        let mut i = 0;
        while i < cells.len() {
            let cell = cells[i];
            let mut own = Vec::new();
            self.first.for_each_value(cell, |p| own.push(p));
            let mut to_nil = false;
            let mut next = Vec::new();
            self.rest.for_each_value(cell, |tail| {
                if tail == self.nil {
                    to_nil = true;
                    return;
                }
                let at = cells.iter().position(|&c| c == tail).unwrap_or_else(|| {
                    cells.push(tail);
                    cells.len() - 1
                });
                next.push(at);
            });
            firsts.push(own);
            rests.push((to_nil, next));
            i += 1;
        }
        let touched = cells.iter().any(|c| self.touched.binary_search(c).is_ok());
        let mut sets: Vec<BTreeSet<Vec<u32>>> = vec![BTreeSet::new(); cells.len()];
        let mut found = Vec::new();
        loop {
            let mut grew = false;
            for i in (0..cells.len()).rev() {
                let (to_nil, next) = &rests[i];
                for &p in &firsts[i] {
                    if *to_nil {
                        found.push(vec![p]);
                    }
                    for &r in next {
                        for s in &sets[r] {
                            let mut with = s.clone();
                            if let Err(at) = with.binary_search(&p) {
                                with.insert(at, p);
                            }
                            found.push(with);
                        }
                    }
                }
                for s in found.drain(..) {
                    grew |= sets[i].insert(s);
                }
            }
            if !grew {
                break;
            }
        }
        (std::mem::take(&mut sets[0]), touched)
    }
}

/// Buffers for grouping the individuals of a class, reused across keys.
#[derive(Debug, Default)]
struct Keyed {
    /// Every tuple, one after the other, as many terms each as the set has
    /// properties.
    tuples: Vec<u32>,
    /// For each tuple: its individual, and whether the tuple is recent.
    owners: Vec<(u32, bool)>,
    /// One individual's `(value, recent)` for each property in turn, and
    /// where each property's values start.
    values: Vec<(u32, bool)>,
    starts: Vec<usize>,
}

impl Keyed {
    /// prp-key for one key class and one property set: groups the
    /// individuals of `class` by their tuples of values, and makes the
    /// members of a group with a recent tuple the same. `changed`: the key
    /// itself is recent, so every tuple is.
    fn group(
        &mut self,
        round: &mut Round<'_>,
        same_as: u32,
        instances: &Variable<Pair>,
        class: u32,
        set: &[u32],
        changed: bool,
    ) {
        let store = round.store();
        let Some(tables) = set
            .iter()
            .map(|&p| store.table(p).map(|t| t.by_subject()))
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        let recent_class = !range(instances.recent(), class).is_empty();
        if !changed && !recent_class && tables.iter().all(|t| t.recent().is_empty()) {
            return;
        }
        self.tuples.clear();
        self.owners.clear();
        for (part, recent_type) in tagged(instances) {
            for &(_, x) in range(part, class) {
                self.tuples_of(x, &tables, changed || recent_type);
            }
        }
        let mut groups: FxHashMap<&[u32], Vec<(u32, bool)>> = FxHashMap::default();
        for (tuple, &owner) in self.tuples.chunks_exact(set.len()).zip(&self.owners) {
            groups.entry(tuple).or_default().push(owner);
        }
        let mut all = Vec::new();
        for members in groups.values() {
            if members.iter().any(|m| m.1) {
                all.clear();
                all.extend(members.iter().map(|m| m.0));
                pair_up(round, same_as, members.iter().filter(|m| m.1).map(|m| m.0), &all);
            }
        }
    }

    /// Pushes every tuple of `x`'s values in `tables`, one value from each,
    /// recent when `recent` or one of its values is.
    fn tuples_of(&mut self, x: u32, tables: &[&Variable<Pair>], recent: bool) {
        self.values.clear();
        self.starts.clear();
        for table in tables {
            let start = self.values.len();
            values_of(table, x, &mut self.values);
            if self.values.len() == start {
                return;
            }
            self.starts.push(start);
        }
        self.starts.push(self.values.len());
        let n = tables.len();
        // An odometer over the choices, one digit per property.
        let mut at: Vec<usize> = self.starts[..n].to_vec();
        loop {
            let mut fresh = recent;
            for &i in &at {
                let (value, r) = self.values[i];
                self.tuples.push(value);
                fresh |= r;
            }
            self.owners.push((x, fresh));
            let mut digit = 0;
            loop {
                if digit == n {
                    return;
                }
                at[digit] += 1;
                if at[digit] < self.starts[digit + 1] {
                    break;
                }
                at[digit] = self.starts[digit];
                digit += 1;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// cls-maxc2, cls-maxqc3, cls-maxqc4
// ---------------------------------------------------------------------------

/// A restriction to at most one value of `p`, for the individuals of `x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AtMostOne {
    p: u32,
    x: u32,
    /// The class the values count in (`owl:onClass`); `None` for
    /// `owl:maxCardinality` and for `owl:onClass owl:Thing`.
    class: Option<u32>,
    /// One of its triples is recent: joined with all of the data.
    new: bool,
}

/// cls-maxc2: `x owl:maxCardinality "1"^^xsd:nonNegativeInteger`,
/// `x owl:onProperty p`, `u rdf:type x`, `u p y1`, `u p y2` →
/// `y1 owl:sameAs y2`.
///
/// cls-maxqc3: `x owl:maxQualifiedCardinality "1"^^xsd:nonNegativeInteger`,
/// `x owl:onProperty p`, `x owl:onClass c`, `u rdf:type x`, `u p y1`,
/// `y1 rdf:type c`, `u p y2`, `y2 rdf:type c` → `y1 owl:sameAs y2`.
///
/// cls-maxqc4: as cls-maxqc3 with `owl:onClass owl:Thing`, without the
/// typing of `y1` and `y2` (what cls-maxqc3 then derives is a part of it).
fn cardinalities(round: &mut Round<'_>, same_as: u32, one: u32, instances: &Variable<Pair>, types: &View<'_, Pair>) {
    // (u, whether all of u's values are paired).
    let mut candidates: Vec<(u32, bool)> = Vec::new();
    let mut values: Vec<(u32, bool)> = Vec::new();
    let mut all: Vec<u32> = Vec::new();
    for r in restrictions(round, one) {
        let Some(edges) = round.by_subject(r.p) else {
            continue;
        };
        let typed = |u: u32| types.contains(&(u, r.x));
        candidates.clear();
        for (part, recent) in tagged(instances) {
            if r.new || recent {
                candidates.extend(range(part, r.x).iter().map(|&(_, u)| (u, true)));
            }
        }
        // A qualified restriction reads p by object: the recent instances
        // of its class find their subjects there.
        let qualified = match r.class {
            Some(class) => match round.by_object(r.p) {
                Some(os) => Some((class, os)),
                None => continue,
            },
            None => None,
        };
        match qualified {
            Some((class, os)) => {
                candidates.extend(os.recent().iter().filter(|t| typed(t.1)).map(|&(_, u)| (u, false)));
                for &(_, y) in range(instances.recent(), class) {
                    for part in os.parts() {
                        candidates.extend(range(part, y).iter().filter(|t| typed(t.1)).map(|&(_, u)| (u, false)));
                    }
                }
            }
            None => {
                let mut last = None;
                for &(u, _) in edges.recent().iter() {
                    if last != Some(u) && typed(u) {
                        candidates.push((u, false));
                    }
                    last = Some(u);
                }
            }
        }
        // Each u once, with all of its values paired if one reading says so.
        candidates.sort_unstable_by_key(|&(u, whole)| (u, !whole));
        candidates.dedup_by_key(|c| c.0);
        for &(u, whole) in &candidates {
            values.clear();
            match qualified {
                Some((class, os)) => {
                    for part in edges.parts() {
                        for &(_, y) in range(part, u) {
                            if instances.contains(&(class, y)) {
                                let recent = os.recent().contains(&(y, u)) || instances.recent().contains(&(class, y));
                                values.push((y, recent));
                            }
                        }
                    }
                }
                None => values_of(edges, u, &mut values),
            }
            all.clear();
            all.extend(values.iter().map(|v| v.0));
            if whole {
                for &a in &all {
                    for &b in &all {
                        round.emit(a, same_as, b);
                    }
                }
            } else {
                pair_up(round, same_as, values.iter().filter(|v| v.1).map(|v| v.0), &all);
            }
        }
    }
}

/// The restrictions to at most one value: `owl:maxCardinality` and
/// `owl:maxQualifiedCardinality` with `one`, joined with `owl:onProperty`
/// (and `owl:onClass`).
fn restrictions(round: &Round<'_>, one: u32) -> Vec<AtMostOne> {
    let mut out = Vec::new();
    let Some(on_property) = round.by_subject(round.constant(vocab::OWL_ON_PROPERTY.0)) else {
        return out;
    };
    let on_class = round.by_subject(round.constant(vocab::OWL_ON_CLASS.0));
    let thing = round.constant(vocab::OWL_THING.0);
    let (mut properties, mut classes) = (Vec::new(), Vec::new());
    for (predicate, qualified) in [
        (vocab::OWL_MAX_CARDINALITY, false),
        (vocab::OWL_MAX_QUALIFIED_CARDINALITY, true),
    ] {
        let Some(max) = round.by_subject(round.constant(predicate.0)) else {
            continue;
        };
        for (part, recent) in tagged(max) {
            for &(x, _) in part.iter().filter(|t| t.1 == one) {
                properties.clear();
                values_of(on_property, x, &mut properties);
                classes.clear();
                match (qualified, on_class) {
                    (false, _) => classes.push((thing, false)),
                    (true, Some(on_class)) => values_of(on_class, x, &mut classes),
                    (true, None) => {}
                }
                for &(p, recent_p) in &properties {
                    for &(c, recent_c) in &classes {
                        out.push(AtMostOne {
                            p,
                            x,
                            class: (c != thing).then_some(c),
                            new: recent || recent_p || recent_c,
                        });
                    }
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use crate::rules::testing::{self, expand};
    use crate::{Switch, Switches};

    /// The identity rules, with the families they lean on: the class rules
    /// (types through the hierarchy) and equality (the merges).
    fn run(turtle: &str) -> testing::Run {
        testing::run(&["class", "equality", "identity"], turtle)
    }

    #[track_caller]
    fn has_all(run: &testing::Run, lines: &str) {
        let closure = run.closure();
        let missing: Vec<_> = expand(lines).into_iter().filter(|t| !closure.contains(t)).collect();
        assert!(missing.is_empty(), "missing {missing:#?}");
    }

    #[track_caller]
    fn has_none(run: &testing::Run, lines: &str) {
        let closure = run.closure();
        let found: Vec<_> = expand(lines).into_iter().filter(|t| closure.contains(t)).collect();
        assert!(found.is_empty(), "derived {found:#?}");
    }

    #[test]
    fn a_key_makes_individuals_with_the_same_value_the_same() {
        let run = run("ex:Person owl:hasKey ( ex:ssn ) .
             ex:a a ex:Person ; ex:ssn \"1\" . ex:b a ex:Person ; ex:ssn \"1\" .
             ex:c ex:ssn \"1\" . ex:d a ex:Person ; ex:ssn \"2\" .");
        has_all(&run, "ex:a owl:sameAs ex:b\nex:b owl:sameAs ex:a\nex:d owl:sameAs ex:d");
        has_none(&run, "ex:a owl:sameAs ex:c\nex:a owl:sameAs ex:d");
    }

    /// Every property of the key shares a value, any of its values.
    #[test]
    fn a_key_of_two_properties_matches_on_both() {
        let run = run("ex:P owl:hasKey ( ex:f ex:l ) .
             ex:a a ex:P ; ex:f \"A\" ; ex:l \"B\" .
             ex:b a ex:P ; ex:f \"A\" ; ex:l \"B\" .
             ex:c a ex:P ; ex:f \"A\" ; ex:l \"C\" .
             ex:d a ex:P ; ex:f \"X\" , \"A\" ; ex:l \"B\" .
             ex:e a ex:P ; ex:f \"A\" .");
        has_all(&run, "ex:a owl:sameAs ex:b\nex:a owl:sameAs ex:d");
        has_none(&run, "ex:a owl:sameAs ex:c\nex:a owl:sameAs ex:e\nex:e owl:sameAs ex:e");
    }

    /// A cell with two firsts gives two keys; a list that never reaches
    /// `rdf:nil` gives none, and so does the empty list.
    #[test]
    fn key_lists_are_read_as_paths() {
        let run = run("ex:P owl:hasKey _:l1 . _:l1 rdf:first ex:f , ex:g ; rdf:rest rdf:nil .
             ex:a a ex:P ; ex:g \"1\" . ex:b a ex:P ; ex:g \"1\" .
             ex:Q owl:hasKey _:c1 . _:c1 rdf:first ex:f ; rdf:rest _:c1 .
             ex:x a ex:Q ; ex:f \"1\" . ex:y a ex:Q ; ex:f \"1\" .
             ex:R owl:hasKey rdf:nil . ex:u a ex:R . ex:v a ex:R .");
        has_all(&run, "ex:a owl:sameAs ex:b");
        has_none(&run, "ex:x owl:sameAs ex:y\nex:u owl:sameAs ex:v");
    }

    /// Whatever arrives last, across incremental runs, the closure is the
    /// one of a single run.
    #[test]
    fn a_key_holds_whatever_arrives_last() {
        let parts = [
            "ex:P owl:hasKey _:k1 . _:k1 rdf:first ex:f ; rdf:rest _:k2 . _:k2 rdf:first ex:l ; rdf:rest rdf:nil .",
            "ex:a a ex:P ; ex:f \"A\" .",
            "ex:b ex:f \"A\" ; ex:l \"B\" .",
            "ex:a ex:l \"B\" . ex:b a ex:P .",
        ];
        let whole = run(&parts.join("\n")).closure();
        assert!(whole.is_superset(&expand("ex:a owl:sameAs ex:b")));
        for order in [[0, 1, 2, 3], [3, 2, 1, 0], [1, 3, 0, 2]] {
            let mut r = run("");
            for i in order {
                r.add(parts[i]);
            }
            assert_eq!(r.closure(), whole, "order {order:?}");
        }
    }

    #[test]
    fn max_cardinality_one_makes_the_values_the_same() {
        let run = run(
            "ex:R owl:maxCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .
             ex:u a ex:R ; ex:p ex:y1 , ex:y2 .
             ex:v ex:p ex:z1 , ex:z2 .
             ex:S owl:maxCardinality \"2\"^^xsd:nonNegativeInteger ; owl:onProperty ex:q .
             ex:T owl:maxCardinality \"1\"^^xsd:integer ; owl:onProperty ex:q .
             ex:w a ex:S , ex:T ; ex:q ex:a , ex:b .",
        );
        has_all(&run, "ex:y1 owl:sameAs ex:y2\nex:y2 owl:sameAs ex:y1");
        has_none(&run, "ex:z1 owl:sameAs ex:z2\nex:a owl:sameAs ex:b");
    }

    #[test]
    fn max_qualified_cardinality_one_counts_the_values_in_the_class() {
        let run = run(
            "ex:R owl:maxQualifiedCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p ; owl:onClass ex:C .
             ex:u a ex:R ; ex:p ex:y1 , ex:y2 , ex:y3 . ex:y1 a ex:C . ex:y2 a ex:C .
             ex:S owl:maxQualifiedCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onProperty ex:q ; owl:onClass owl:Thing .
             ex:v a ex:S ; ex:q ex:z1 , ex:z2 .",
        );
        has_all(&run, "ex:y1 owl:sameAs ex:y2\nex:z1 owl:sameAs ex:z2");
        has_none(&run, "ex:y1 owl:sameAs ex:y3\nex:y3 owl:sameAs ex:y3");
    }

    /// A value typed last, in a later run, still counts.
    #[test]
    fn a_value_typed_later_counts() {
        let mut r = run(
            "ex:R owl:maxQualifiedCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p ; owl:onClass ex:C .
             ex:u a ex:R ; ex:p ex:y1 , ex:y2 . ex:y1 a ex:C .",
        );
        has_none(&r, "ex:y1 owl:sameAs ex:y2");
        r.add("ex:D rdfs:subClassOf ex:C . ex:y2 a ex:D .");
        has_all(&r, "ex:y1 owl:sameAs ex:y2");
    }

    /// In the default configuration the merges reach the other rules: a key
    /// value made equal makes its owners equal.
    #[test]
    fn identity_merges_reach_the_other_rules() {
        let input = "ex:P owl:hasKey ( ex:k ) .
             ex:a a ex:P ; ex:k ex:v1 . ex:b a ex:P ; ex:k ex:v2 .
             ex:R owl:maxCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .
             ex:u a ex:R ; ex:p ex:v1 , ex:v2 . ex:a ex:q ex:o .";
        let on = testing::run_with(Switches::default(), input);
        has_all(&on, "ex:v1 owl:sameAs ex:v2\nex:a owl:sameAs ex:b\nex:b ex:q ex:o");
        let off = testing::run_with(Switches::default().off(Switch::Identity), input);
        has_none(&off, "ex:v1 owl:sameAs ex:v2\nex:a owl:sameAs ex:b\nex:b ex:q ex:o");
    }
}
