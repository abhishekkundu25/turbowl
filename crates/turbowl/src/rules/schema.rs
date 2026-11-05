//! The schema family (switch `schema`): the schema rules beyond the
//! class hierarchy (OWL 2 Profiles, Table 9), scm-eqc2, scm-spo, scm-eqp1,
//! scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv, scm-svf1,
//! scm-svf2, scm-avf1, scm-avf2, scm-int and scm-uni, in their W3C forms.
//! The contract is the reference evaluator
//! (`crates/reference/src/rules/schema.rs`), which equals Nemo running
//! `oracles/rules/schema.rls`. scm-sco and scm-eqc1 are in the core's class
//! family; scm-cls, scm-op and scm-dp in the `trivial` family.
//!
//! The rules are semi-naive, as the class family's are (`class.rs`): the
//! hierarchy rules join the recent part of one table with the rest; the
//! restriction rules (scm-hv, scm-svf1/2, scm-avf1/2) and the list rules
//! (scm-int, scm-uni) read schema-sized structures again in each round in
//! which they may have changed, and otherwise join them only with what is
//! recent.
//!
//! The W3C rules have no reflexive premise of their own: scm-svf1 and
//! scm-avf1 on one filler need `y rdfs:subClassOf y`, and scm-hv, scm-svf2
//! and scm-avf2 on one property need `p rdfs:subPropertyOf p`. Those come
//! from a cycle (scm-sco, scm-spo) or from scm-cls and scm-op (the
//! `trivial` switch), never from this family.
//!
//! Constants are read through the round's canon ([`Round::constant`],
//! ADR 0003), and object indexes are lazy, as in `class.rs`.

use super::Context;
use crate::engine::{Pair, Round, Variable, View, gallop, join, join_slices};
use crate::term::{TermId, vocab};
use rustc_hash::FxHashMap;
use std::cmp::Ordering;

/// The schema rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// The constants this file's rules name, through the round's canon.
#[derive(Debug, Clone, Copy)]
struct Vocab {
    sub_class_of: u32,
    sub_property_of: u32,
}

/// One round of the schema rules.
pub(crate) fn rules(round: &mut Round<'_>, _cx: &mut Context<'_>) {
    let v = Vocab {
        sub_class_of: round.constant(vocab::RDFS_SUB_CLASS_OF.0),
        sub_property_of: round.constant(vocab::RDFS_SUB_PROPERTY_OF.0),
    };
    let equivalent_class = round.constant(vocab::OWL_EQUIVALENT_CLASS.0);
    let equivalent_property = round.constant(vocab::OWL_EQUIVALENT_PROPERTY.0);
    mutual(round, v.sub_class_of, equivalent_class);
    mutual(round, v.sub_property_of, equivalent_property);
    scm_spo(round, v);
    scm_eqp1(round, v, equivalent_property);
    for axiom in [vocab::RDFS_DOMAIN, vocab::RDFS_RANGE] {
        let axiom = round.constant(axiom.0);
        scm_dom_rng(round, v, axiom);
    }
    restrictions(round, v);
    lists(round, v);
}

/// scm-eqc2 (`r` is `rdfs:subClassOf`, `to` is `owl:equivalentClass`) and
/// scm-eqp2 (`rdfs:subPropertyOf`, `owl:equivalentProperty`): `a r b`,
/// `b r a` → `a to b`. The recent pairs, mirrored and sorted, are
/// intersected with every part of `r`; a match gives both directions, so
/// the later of the two pairs finds the earlier.
fn mutual(round: &mut Round<'_>, r: u32, to: u32) {
    let Some(pairs) = round.by_subject(r) else {
        return;
    };
    let mut mirrored: Vec<Pair> = pairs.recent().iter().map(|&(a, b)| (b, a)).collect();
    if mirrored.is_empty() {
        return;
    }
    mirrored.sort_unstable();
    for part in pairs.parts() {
        let (mut x, mut y) = (&mirrored[..], part);
        while let (Some(&m), Some(&t)) = (x.first(), y.first()) {
            match m.cmp(&t) {
                Ordering::Less => x = gallop(x, |p| *p < t),
                Ordering::Greater => y = gallop(y, |p| *p < m),
                Ordering::Equal => {
                    let (b, a) = m;
                    round.emit(a, to, b);
                    round.emit(b, to, a);
                    x = &x[1..];
                    y = &y[1..];
                }
            }
        }
    }
}

/// scm-spo: `p1 rdfs:subPropertyOf p2`, `p2 rdfs:subPropertyOf p3` →
/// `p1 rdfs:subPropertyOf p3`. Join on p2: `subPropertyOf` by object with
/// `subPropertyOf` by subject.
fn scm_spo(round: &mut Round<'_>, v: Vocab) {
    let Some(spo) = round.by_subject(v.sub_property_of) else {
        return;
    };
    let Some(spo_by_object) = round.by_object(v.sub_property_of) else {
        return;
    };
    join(spo_by_object, spo, |_, p1, p3| round.emit(p1, v.sub_property_of, p3));
}

/// scm-eqp1: `p1 owl:equivalentProperty p2` → `p1 rdfs:subPropertyOf p2`
/// and `p2 rdfs:subPropertyOf p1`. Reads the recent pairs.
fn scm_eqp1(round: &mut Round<'_>, v: Vocab, equivalent_property: u32) {
    let Some(eqp) = round.by_subject(equivalent_property) else {
        return;
    };
    for &(p1, p2) in eqp.recent().iter() {
        round.emit(p1, v.sub_property_of, p2);
        round.emit(p2, v.sub_property_of, p1);
    }
}

/// scm-dom1 and scm-dom2 (`axiom` is `rdfs:domain`), scm-rng1 and scm-rng2
/// (`rdfs:range`):
/// - `p axiom c1`, `c1 rdfs:subClassOf c2` → `p axiom c2`: join on c1,
///   `axiom` by object with `subClassOf` by subject;
/// - `p2 axiom c`, `p1 rdfs:subPropertyOf p2` → `p1 axiom c`: join on p2,
///   `axiom` by subject with `subPropertyOf` by object.
fn scm_dom_rng(round: &mut Round<'_>, v: Vocab, axiom: u32) {
    let Some(axioms) = round.by_subject(axiom) else {
        return;
    };
    if let (Some(by_class), Some(sco)) = (round.by_object(axiom), round.by_subject(v.sub_class_of)) {
        join(by_class, sco, |_, p, c2| round.emit(p, axiom, c2));
    }
    if let Some(spo_by_object) = round.by_object(v.sub_property_of) {
        join(axioms, spo_by_object, |_, c, p1| round.emit(p1, axiom, c));
    }
}

/// The restriction rules, for one round:
/// - scm-svf1 and scm-avf1: `x1 <filler> y1`, `x1 owl:onProperty p`,
///   `x2 <filler> y2`, `x2 owl:onProperty p`, `y1 rdfs:subClassOf y2` →
///   `x1 rdfs:subClassOf x2`: the restrictions keyed by filler, matched on
///   the property, along `subClassOf`;
/// - scm-hv, scm-svf2 and scm-avf2: `x1 <filler> y`, `x1 owl:onProperty
///   p1`, `x2 <filler> y`, `x2 owl:onProperty p2`, `p1 rdfs:subPropertyOf
///   p2` → `x1 rdfs:subClassOf x2` (scm-avf2: `x2 rdfs:subClassOf x1`):
///   the restrictions keyed by property, matched on the filler, along
///   `subPropertyOf`.
///
/// The restrictions are joined again from their two tables in each round
/// that needs them. In a round where that join has a new pair, every
/// restriction is joined with all of the hierarchy; otherwise only with its
/// recent part.
fn restrictions(round: &mut Round<'_>, v: Vocab) {
    let Some(on_property) = round.by_subject(round.constant(vocab::OWL_ON_PROPERTY.0)) else {
        return;
    };
    let sco = round.by_subject(v.sub_class_of);
    let spo = round.by_subject(v.sub_property_of);
    let recent = |v: Option<&Variable<Pair>>| v.is_some_and(|v| !v.recent().is_empty());
    // (filler, keyed by filler too, the head reversed: scm-avf2)
    for (filler, by_filler, reversed) in [
        (vocab::OWL_SOME_VALUES_FROM, true, false),
        (vocab::OWL_ALL_VALUES_FROM, true, true),
        (vocab::OWL_HAS_VALUE, false, false),
    ] {
        let Some(fillers) = round.by_subject(round.constant(filler.0)) else {
            continue;
        };
        let mut changed = false;
        join(fillers, on_property, |_, _, _| changed = true);
        if !changed && !(by_filler && recent(sco)) && !recent(spo) {
            continue;
        }
        // (x, y, p) for every restriction.
        let mut all = Vec::new();
        for part in fillers.parts() {
            for on in on_property.parts() {
                join_slices(part, on, &mut |x, y, p| all.push((x, y, p)));
            }
        }
        if by_filler && let Some(sco) = sco {
            let keyed = Restrictions::new(all.iter().map(|&(x, y, p)| (y, p, x)).collect());
            keyed.pairs(sco, changed, |x1, x2| round.emit(x1, v.sub_class_of, x2));
        }
        if let Some(spo) = spo {
            let keyed = Restrictions::new(all.iter().map(|&(x, y, p)| (p, y, x)).collect());
            keyed.pairs(spo, changed, |x1, x2| {
                let (sub, sup) = if reversed { (x2, x1) } else { (x1, x2) };
                round.emit(sub, v.sub_class_of, sup);
            });
        }
    }
}

/// The restrictions of one filler predicate (`owl:someValuesFrom`,
/// `owl:allValuesFrom` or `owl:hasValue`), keyed one way: by filler or by
/// property.
#[derive(Debug, Default)]
struct Restrictions {
    /// The key of each row, sorted.
    keys: Vec<u32>,
    /// `(matched, x)`: the other field and the restriction, sorted within a
    /// key.
    rows: Vec<Pair>,
}

impl Restrictions {
    /// From `(key, matched, x)` triples.
    fn new(mut entries: Vec<(u32, u32, u32)>) -> Self {
        entries.sort_unstable();
        entries.dedup();
        Self {
            keys: entries.iter().map(|e| e.0).collect(),
            rows: entries.iter().map(|e| (e.1, e.2)).collect(),
        }
    }

    /// The rows of `key`.
    fn group(&self, key: u32) -> &[Pair] {
        let start = self.keys.partition_point(|&k| k < key);
        let end = start + self.keys[start..].partition_point(|&k| k <= key);
        &self.rows[start..end]
    }

    /// `f(x1, x2)` for each restriction `x1` keyed `k1` and `x2` keyed `k2`
    /// with the same matched field, for each `(k1, k2)` of `relation` (by
    /// subject): all of it when `changed`, its recent part otherwise.
    fn pairs(&self, relation: &Variable<Pair>, changed: bool, mut f: impl FnMut(u32, u32)) {
        let mut both = |k1: u32, k2: u32| join_slices(self.group(k1), self.group(k2), &mut |_, x1, x2| f(x1, x2));
        if changed {
            let all = relation.all();
            for run in self.keys.chunk_by(|a, b| a == b) {
                all.for_each_value(run[0], |k2| both(run[0], k2));
            }
        } else {
            for &(k1, k2) in relation.recent().iter() {
                both(k1, k2);
            }
        }
    }
}

/// scm-int: `c owl:intersectionOf (c1 ... cn)` → `c rdfs:subClassOf ci`
/// for each i; scm-uni: `c owl:unionOf (c1 ... cn)` → `ci rdfs:subClassOf
/// c` for each i.
///
/// The members are those cls-int2 and cls-uni read (`class/lists.rs`, whose
/// module doc gives the reading): the firsts of the well-formed cells
/// reached from a well-formed head through well-formed cells. Lists are
/// schema-sized: every list is read again in a round where an
/// `intersectionOf`, `unionOf`, `rdf:first` or `rdf:rest` pair is recent,
/// and none in the other rounds.
fn lists(round: &mut Round<'_>, v: Vocab) {
    let intersections = round.by_subject(round.constant(vocab::OWL_INTERSECTION_OF.0));
    let unions = round.by_subject(round.constant(vocab::OWL_UNION_OF.0));
    let first = round.by_subject(round.constant(vocab::RDF_FIRST.0));
    let rest = round.by_subject(round.constant(vocab::RDF_REST.0));
    let recent = |v: Option<&Variable<Pair>>| v.is_some_and(|v| !v.recent().is_empty());
    if !(recent(intersections) || recent(unions) || recent(first) || recent(rest)) {
        return;
    }
    let Some(first) = first else { return };
    let nil = round.constant(vocab::RDF_NIL.0);
    let mut reader = Reader {
        first: first.all(),
        rest: rest.map_or_else(|| View::one(&[]), Variable::all),
        nil,
        nil_is_a_cell: round.canon().members(nil).len() > 1,
        cells: Vec::new(),
        index: FxHashMap::default(),
        members: Vec::new(),
    };
    for (heads, intersection) in [(intersections, true), (unions, false)] {
        let Some(heads) = heads else { continue };
        for part in heads.parts() {
            for &(c, head) in part {
                for &k in reader.members(head) {
                    if intersection {
                        round.emit(c, v.sub_class_of, k);
                    } else {
                        round.emit(k, v.sub_class_of, c);
                    }
                }
            }
        }
    }
}

/// Reads the members of lists from the `rdf:first` and `rdf:rest` tables.
struct Reader<'a> {
    first: View<'a, Pair>,
    rest: View<'a, Pair>,
    /// The representative of `rdf:nil`, and whether it is read on as a
    /// cell: when `owl:sameAs` gives `rdf:nil` other members, a cell equal
    /// to it but not `rdf:nil` itself is a later cell (`class/lists.rs`).
    nil: u32,
    nil_is_a_cell: bool,
    /// The cells of the list being read, its head first.
    cells: Vec<Cell>,
    /// A cell's term to its index in `cells`.
    index: FxHashMap<u32, usize>,
    members: Vec<u32>,
}

/// A list cell.
struct Cell {
    term: u32,
    /// Its rests that are cells, as indexes.
    rests: Vec<usize>,
    /// A rest is `rdf:nil`.
    to_nil: bool,
    has_first: bool,
    /// A first, and a rest that is `rdf:nil` or a well-formed cell.
    well_formed: bool,
}

impl Reader<'_> {
    /// The members of the list at `head`, sorted and distinct.
    fn members(&mut self, head: u32) -> &[u32] {
        self.cells.clear();
        self.index.clear();
        self.members.clear();
        self.cell(head);
        // The cells, breadth first along rdf:rest, stopping at rdf:nil.
        let mut tails = Vec::new();
        let mut i = 0;
        while i < self.cells.len() {
            tails.clear();
            self.rest.for_each_value(self.cells[i].term, |tail| tails.push(tail));
            for &tail in &tails {
                if tail == self.nil {
                    self.cells[i].to_nil = true;
                    if !self.nil_is_a_cell {
                        continue;
                    }
                }
                let next = self.cell(tail);
                self.cells[i].rests.push(next);
            }
            i += 1;
        }
        // Which cells are well formed: a backward pass settles a simple
        // list; repeat until nothing changes.
        let mut grew = true;
        while grew {
            grew = false;
            for i in (0..self.cells.len()).rev() {
                let c = &self.cells[i];
                if !c.well_formed && c.has_first && (c.to_nil || c.rests.iter().any(|&r| self.cells[r].well_formed)) {
                    self.cells[i].well_formed = true;
                    grew = true;
                }
            }
        }
        // The firsts of the well-formed cells reached from a well-formed
        // head through well-formed cells.
        if self.cells[0].well_formed {
            let mut seen = vec![false; self.cells.len()];
            seen[0] = true;
            let mut reached = vec![0];
            while let Some(i) = reached.pop() {
                let members = &mut self.members;
                self.first.for_each_value(self.cells[i].term, |k| members.push(k));
                for &r in &self.cells[i].rests {
                    if !seen[r] && self.cells[r].well_formed {
                        seen[r] = true;
                        reached.push(r);
                    }
                }
            }
        }
        self.members.sort_unstable();
        self.members.dedup();
        &self.members
    }

    /// The index of the cell `term`, added when new.
    fn cell(&mut self, term: u32) -> usize {
        if let Some(&i) = self.index.get(&term) {
            return i;
        }
        let mut has_first = false;
        self.first.for_each_value(term, |_| has_first = true);
        self.cells.push(Cell {
            term,
            rests: Vec::new(),
            to_nil: false,
            has_first,
            well_formed: false,
        });
        self.index.insert(term, self.cells.len() - 1);
        self.cells.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::testing::{self, expand};

    /// Asserts that the schema family alone infers exactly `expected` from
    /// `turtle`, in one run and in two (the first half of the statements,
    /// then the rest). Inputs name no blank node: each run's document has
    /// its own.
    #[track_caller]
    fn infers(turtle: &str, expected: &str) {
        let run = testing::run(&["schema"], turtle);
        assert_eq!(run.inferred(), expand(expected), "input:\n{turtle}");
        let statements: Vec<&str> = turtle.split(" .").filter(|s| !s.trim().is_empty()).collect();
        let (a, b) = statements.split_at(statements.len() / 2);
        if a.is_empty() {
            return;
        }
        let mut twice = testing::run(&["schema"], &format!("{} .", a.join(" .")));
        twice.add(&format!("{} .", b.join(" .")));
        assert_eq!(twice.closure(), run.closure(), "incremental, input:\n{turtle}");
    }

    #[test]
    fn scm_eqc2() {
        infers(
            "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
            "ex:A owl:equivalentClass ex:B
             ex:B owl:equivalentClass ex:A",
        );
    }

    #[test]
    fn scm_spo() {
        infers(
            "ex:p rdfs:subPropertyOf ex:q . ex:q rdfs:subPropertyOf ex:r . ex:r rdfs:subPropertyOf ex:s .",
            "ex:p rdfs:subPropertyOf ex:r
             ex:p rdfs:subPropertyOf ex:s
             ex:q rdfs:subPropertyOf ex:s",
        );
    }

    /// scm-eqp1 gives the two subproperty pairs, scm-spo closes the cycle
    /// reflexively, and scm-eqp2 makes each pair equivalent.
    #[test]
    fn scm_eqp1_and_scm_eqp2() {
        infers(
            "ex:p owl:equivalentProperty ex:q .",
            "ex:p rdfs:subPropertyOf ex:q
             ex:q rdfs:subPropertyOf ex:p
             ex:p rdfs:subPropertyOf ex:p
             ex:q rdfs:subPropertyOf ex:q
             ex:q owl:equivalentProperty ex:p
             ex:p owl:equivalentProperty ex:p
             ex:q owl:equivalentProperty ex:q",
        );
    }

    #[test]
    fn scm_dom1_and_scm_dom2() {
        infers(
            "ex:p rdfs:domain ex:A . ex:A rdfs:subClassOf ex:B . ex:q rdfs:subPropertyOf ex:p .",
            "ex:p rdfs:domain ex:B
             ex:q rdfs:domain ex:A
             ex:q rdfs:domain ex:B",
        );
    }

    #[test]
    fn scm_rng1_and_scm_rng2() {
        infers(
            "ex:p rdfs:range ex:A . ex:A rdfs:subClassOf ex:B . ex:q rdfs:subPropertyOf ex:p .",
            "ex:p rdfs:range ex:B
             ex:q rdfs:range ex:A
             ex:q rdfs:range ex:B",
        );
    }

    #[test]
    fn scm_hv() {
        infers(
            "ex:r1 owl:hasValue ex:v . ex:r1 owl:onProperty ex:p . ex:r2 owl:hasValue ex:v .
             ex:r2 owl:onProperty ex:q . ex:p rdfs:subPropertyOf ex:q . ex:r3 owl:hasValue ex:w .
             ex:r3 owl:onProperty ex:q .",
            "ex:r1 rdfs:subClassOf ex:r2",
        );
    }

    #[test]
    fn scm_svf1_and_scm_avf1() {
        infers(
            "ex:r1 owl:someValuesFrom ex:A . ex:r1 owl:onProperty ex:p . ex:r2 owl:someValuesFrom ex:B .
             ex:r2 owl:onProperty ex:p . ex:A rdfs:subClassOf ex:B . ex:r3 owl:allValuesFrom ex:A .
             ex:r3 owl:onProperty ex:p . ex:r4 owl:allValuesFrom ex:B . ex:r4 owl:onProperty ex:p .
             ex:r5 owl:someValuesFrom ex:B . ex:r5 owl:onProperty ex:q .",
            "ex:r1 rdfs:subClassOf ex:r2
             ex:r3 rdfs:subClassOf ex:r4",
        );
    }

    /// scm-svf2 and scm-avf2 run in opposite directions.
    #[test]
    fn scm_svf2_and_scm_avf2() {
        infers(
            "ex:r1 owl:someValuesFrom ex:A . ex:r1 owl:onProperty ex:p . ex:r2 owl:someValuesFrom ex:A .
             ex:r2 owl:onProperty ex:q . ex:p rdfs:subPropertyOf ex:q . ex:r3 owl:allValuesFrom ex:A .
             ex:r3 owl:onProperty ex:p . ex:r4 owl:allValuesFrom ex:A . ex:r4 owl:onProperty ex:q .",
            "ex:r1 rdfs:subClassOf ex:r2
             ex:r4 rdfs:subClassOf ex:r3",
        );
    }

    /// Two restrictions on one property and one filler: the W3C rules need
    /// `A rdfs:subClassOf A` (scm-cls) or `p rdfs:subPropertyOf p`
    /// (scm-op), which this family never adds.
    #[test]
    fn one_filler_and_one_property_need_a_reflexive_premise() {
        infers(
            "ex:r1 owl:someValuesFrom ex:A . ex:r1 owl:onProperty ex:p .
             ex:r2 owl:someValuesFrom ex:A . ex:r2 owl:onProperty ex:p .",
            "",
        );
        infers(
            "ex:r1 owl:someValuesFrom ex:A . ex:r1 owl:onProperty ex:p .
             ex:r2 owl:someValuesFrom ex:A . ex:r2 owl:onProperty ex:p . ex:A rdfs:subClassOf ex:A .",
            "ex:r1 rdfs:subClassOf ex:r2
             ex:r2 rdfs:subClassOf ex:r1
             ex:r1 rdfs:subClassOf ex:r1
             ex:r2 rdfs:subClassOf ex:r2
             ex:A owl:equivalentClass ex:A
             ex:r1 owl:equivalentClass ex:r2
             ex:r2 owl:equivalentClass ex:r1
             ex:r1 owl:equivalentClass ex:r1
             ex:r2 owl:equivalentClass ex:r2",
        );
    }

    #[test]
    fn scm_int_and_scm_uni() {
        infers(
            "ex:I owl:intersectionOf ex:i1 . ex:i1 rdf:first ex:A . ex:i1 rdf:rest ex:i2 .
             ex:i2 rdf:first ex:B . ex:i2 rdf:rest rdf:nil . ex:U owl:unionOf ex:u1 .
             ex:u1 rdf:first ex:C . ex:u1 rdf:rest rdf:nil .",
            "ex:I rdfs:subClassOf ex:A
             ex:I rdfs:subClassOf ex:B
             ex:C rdfs:subClassOf ex:U",
        );
    }

    /// A list that never reaches `rdf:nil`, or has a cell without a first,
    /// has no members.
    #[test]
    fn ill_formed_lists_have_no_members() {
        infers(
            "ex:I owl:intersectionOf ex:c1 . ex:c1 rdf:first ex:A . ex:c1 rdf:rest ex:c2 .
             ex:c2 rdf:first ex:B . ex:c2 rdf:rest ex:c1 . ex:U owl:unionOf ex:u1 .
             ex:u1 rdf:first ex:C . ex:u1 rdf:rest ex:u2 . ex:u2 rdf:rest rdf:nil .",
            "",
        );
    }

    /// With every switch on, the schema rules feed the class rules and each
    /// other: a property's domain reaches its subproperty's instances
    /// through the hierarchy the restrictions give.
    #[test]
    fn the_rules_feed_the_core() {
        let run = testing::run_with(
            crate::Switches::default(),
            "ex:r1 owl:someValuesFrom ex:A ; owl:onProperty ex:p .
             ex:r2 owl:someValuesFrom ex:B ; owl:onProperty ex:p .
             ex:A rdfs:subClassOf ex:B . ex:x a ex:r1 .
             ex:q rdfs:subPropertyOf ex:p . ex:p rdfs:domain ex:r1 .",
        );
        for t in [
            "ex:x a ex:r2",
            "ex:q rdfs:domain ex:r1",
            "ex:q rdfs:domain ex:r2",
            "ex:p rdfs:domain ex:r2",
        ] {
            assert!(run.has(t), "{t}");
        }
    }
}
