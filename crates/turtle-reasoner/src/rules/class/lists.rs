//! cls-int1, cls-int2 and cls-uni: the rules that read the lists of
//! `owl:intersectionOf` and `owl:unionOf`.
//!
//! Lists are read as the reference evaluator reads them (the W3C `LIST[...]`
//! patterns in Datalog form):
//! - the cells of a list are its head and every cell an `rdf:rest` path
//!   reaches from it (`rdf:nil` included, as a cell with no `rdf:rest`
//!   unless the data gives it one);
//! - a cell may have several `rdf:first` and several `rdf:rest`;
//! - a member is an `rdf:first` of a cell from which some `rdf:rest` path
//!   reaches `rdf:nil`, so a cyclic list that never reaches `rdf:nil` has no
//!   member, and `rdf:nil` has none unless the data says otherwise;
//! - shared tails are read once per head (each list is read on its own);
//! - cls-int1 holds for `y` when some path from the head to `rdf:nil` has, at
//!   every cell, an `rdf:first` that `y` is typed with;
//! - every `(class, head)` pair counts, and list triples derived during the
//!   run count from the round after they are derived.
//!
//! Lists are schema-sized, so they are read again each round in which a
//! rule needs them. A list is *changed* in a round when its
//! `intersectionOf`/`unionOf` pair is recent or one of its cells has a
//! recent `rdf:first` or `rdf:rest`; otherwise it reads the same as in the
//! last round. A changed list is joined with all of `rdf:type`, an unchanged
//! one with the recent part only, so per-instance work stays semi-naive.

use super::{TYPE, range};
use crate::engine::{Pair, Round, Variable, View, join_slices};
use crate::term::vocab;
use rustc_hash::FxHashMap;

const FIRST: u32 = vocab::RDF_FIRST.0;
const REST: u32 = vocab::RDF_REST.0;
const NIL: u32 = vocab::RDF_NIL.0;
const INTERSECTION_OF: u32 = vocab::OWL_INTERSECTION_OF.0;
const UNION_OF: u32 = vocab::OWL_UNION_OF.0;

/// Lists up to this many cells are searched for a cell seen before; longer
/// ones use a map.
const SHORT: usize = 16;

/// cls-int1, cls-int2 and cls-uni, for one round. `instances` is `rdf:type`
/// by object; all three rules need it.
pub(super) fn rules(round: &mut Round<'_>, instances: Option<&Variable<Pair>>) {
    let Some(instances) = instances else { return };
    let intersections = round.by_subject(INTERSECTION_OF);
    let unions = round.by_subject(UNION_OF);
    if intersections.is_none() && unions.is_none() {
        return;
    }
    let first = round.by_subject(FIRST);
    let rest = round.by_subject(REST);
    let recent = |v: Option<&Variable<Pair>>| v.is_some_and(|v| !v.recent().is_empty());
    if instances.recent().is_empty() && !recent(intersections) && !recent(unions) && !recent(first) && !recent(rest) {
        return;
    }
    let mut reader = Reader::new(first, rest);
    if let Some(intersections) = intersections {
        let lists = reader.read(intersections);
        let types = round.by_subject(TYPE).map(Variable::all);
        if let Some(types) = &types {
            cls_int1(round, &lists, instances, types);
        }
        cls_int2(round, &lists, instances);
    }
    if let Some(unions) = unions {
        let lists = reader.read(unions);
        cls_uni(round, &lists, instances);
    }
}

/// cls-int1: `c owl:intersectionOf (c1 ... cn)`, `y rdf:type c1`, ...,
/// `y rdf:type cn` → `y rdf:type c`.
///
/// A `y` can newly satisfy a list only if it has a recent type among the
/// list's members (or the list changed), and it must have a type among the
/// members of the list's *pivot*: the cell, of those every path to
/// `rdf:nil` passes through, whose members have the fewest instances. Per
/// list, candidates come from the cheaper side: the recent instances of the
/// members, or the pivot's instances that have a recent type among the
/// members (all of the pivot's instances for a changed list). Recency is
/// `instances`' (by object), the index the joins of the lazy-index argument
/// read (`class.rs`). Candidates are then grouped by `y`, whose types are
/// read once from `types` (by subject) and checked against each list.
fn cls_int1(round: &mut Round<'_>, lists: &Lists, instances: &Variable<Pair>, types: &View<'_, Pair>) {
    let size = |k: u32| instances.parts().map(|part| range(part, k).len()).sum::<usize>();
    // (y, entry)
    let mut candidates: Vec<Pair> = Vec::new();
    for (e, entry) in lists.entries.iter().enumerate() {
        let e = e as u32;
        let Some((pivot, pivot_size)) = lists.pivot(entry, size) else {
            continue;
        };
        if pivot_size == 0 {
            continue;
        }
        let pivot = lists.firsts(pivot);
        let pivot_instances = |candidates: &mut Vec<Pair>, keep: &dyn Fn(u32) -> bool| {
            for &k in pivot {
                for part in instances.parts() {
                    candidates.extend(range(part, k).iter().filter(|t| keep(t.1)).map(|&(_, y)| (y, e)));
                }
            }
        };
        if entry.changed {
            pivot_instances(&mut candidates, &|_| true);
            continue;
        }
        let members = lists.members(entry);
        let recent = instances.recent();
        let recent_size: usize = members.iter().map(|&k| range(recent, k).len()).sum();
        if recent_size == 0 {
            continue;
        }
        if recent_size <= pivot_size.saturating_mul(members.len()) {
            for &k in members {
                candidates.extend(range(recent, k).iter().map(|&(_, y)| (y, e)));
            }
        } else {
            pivot_instances(&mut candidates, &|y| members.iter().any(|&k| recent.contains(&(k, y))));
        }
    }
    candidates.sort_unstable();
    candidates.dedup();
    let mut scratch = Scratch::default();
    let mut classes: Vec<u32> = Vec::new();
    let mut rest = &candidates[..];
    while let Some(&(y, _)) = rest.first() {
        let n = rest.iter().take_while(|t| t.0 == y).count();
        classes.clear();
        types.for_each_value(y, |c| classes.push(c));
        classes.sort_unstable();
        for &(_, e) in &rest[..n] {
            let entry = &lists.entries[e as usize];
            if lists.every_member(entry, |k| classes.binary_search(&k).is_ok(), &mut scratch) {
                round.emit(y, TYPE, entry.class);
            }
        }
        rest = &rest[n..];
    }
}

/// cls-int2: `c owl:intersectionOf (c1 ... cn)`, `y rdf:type c` →
/// `y rdf:type ci` for each i. Join on c: (class, member) pairs with
/// `rdf:type` by object, recent instances for unchanged lists, all for
/// changed ones.
fn cls_int2(round: &mut Round<'_>, lists: &Lists, instances: &Variable<Pair>) {
    let (mut unchanged, mut changed) = (Vec::new(), Vec::new());
    for entry in &lists.entries {
        let pairs = if entry.changed { &mut changed } else { &mut unchanged };
        pairs.extend(lists.members(entry).iter().map(|&k| (entry.class, k)));
    }
    sort(&mut unchanged);
    sort(&mut changed);
    let mut emit = |_, k, y| round.emit(y, TYPE, k);
    join_slices(&unchanged, instances.recent(), &mut emit);
    if !changed.is_empty() {
        for part in instances.parts() {
            join_slices(&changed, part, &mut emit);
        }
    }
}

/// cls-uni: `c owl:unionOf (c1 ... cn)`, `y rdf:type ci` → `y rdf:type c`.
/// Join on ci: (member, class) pairs with `rdf:type` by object, recent
/// instances for unchanged lists, all for changed ones.
fn cls_uni(round: &mut Round<'_>, lists: &Lists, instances: &Variable<Pair>) {
    let (mut unchanged, mut changed) = (Vec::new(), Vec::new());
    for entry in &lists.entries {
        let pairs = if entry.changed { &mut changed } else { &mut unchanged };
        pairs.extend(lists.members(entry).iter().map(|&k| (k, entry.class)));
    }
    sort(&mut unchanged);
    sort(&mut changed);
    let mut emit = |_, c, y| round.emit(y, TYPE, c);
    join_slices(&unchanged, instances.recent(), &mut emit);
    if !changed.is_empty() {
        for part in instances.parts() {
            join_slices(&changed, part, &mut emit);
        }
    }
}

fn sort(pairs: &mut Vec<Pair>) {
    pairs.sort_unstable();
    pairs.dedup();
}

/// The lists of one predicate's `(class, head)` pairs, read this round.
#[derive(Debug, Default)]
struct Lists {
    entries: Vec<Entry>,
    /// Every entry's cells, an entry's in one range, its head first.
    cells: Vec<Cell>,
    /// The `rdf:first` values of the cells, a cell's in one range.
    firsts: Vec<u32>,
    /// The `rdf:rest` values of the cells, as indexes into the entry's cells.
    rests: Vec<u32>,
    /// Each entry's members, sorted and distinct.
    members: Vec<u32>,
}

/// One `(class, head)` pair.
#[derive(Debug, Clone, Copy)]
struct Entry {
    class: u32,
    /// The range of its cells in `Lists::cells`.
    cells: (usize, usize),
    /// The range of its members in `Lists::members`.
    members: (usize, usize),
    /// Whether it may read differently from the last round.
    changed: bool,
}

#[derive(Debug, Clone, Copy)]
struct Cell {
    term: u32,
    firsts: (usize, usize),
    rests: (usize, usize),
    /// An `rdf:rest` of this cell is `rdf:nil`.
    to_nil: bool,
    /// Some `rdf:rest` path from this cell reaches `rdf:nil`.
    reaches_nil: bool,
}

impl Cell {
    fn new(term: u32) -> Self {
        Self {
            term,
            firsts: (0, 0),
            rests: (0, 0),
            to_nil: false,
            reaches_nil: false,
        }
    }
}

/// Buffers for [`Lists::every_member`], reused across calls.
#[derive(Debug, Default)]
struct Scratch {
    holds: Vec<bool>,
    good: Vec<bool>,
}

impl Lists {
    fn members(&self, entry: &Entry) -> &[u32] {
        &self.members[entry.members.0..entry.members.1]
    }

    fn firsts(&self, cell: &Cell) -> &[u32] {
        &self.firsts[cell.firsts.0..cell.firsts.1]
    }

    /// The cell that cls-int1 draws candidates from, and its size: of the
    /// cells every path from the head to `rdf:nil` passes through (the head,
    /// then each cell's only `rdf:rest` while there is one), the one whose
    /// members have the fewest instances (`size` of each, summed). `None`
    /// when the head reaches no `rdf:nil`.
    fn pivot(&self, entry: &Entry, size: impl Fn(u32) -> usize) -> Option<(&Cell, usize)> {
        let cells = &self.cells[entry.cells.0..entry.cells.1];
        if !cells[0].reaches_nil {
            return None;
        }
        let mut best = (0, usize::MAX);
        let mut i = 0;
        // A cycle back to a cell is cut after one lap.
        for _ in 0..cells.len() {
            let c = &cells[i];
            let n = self.firsts(c).iter().map(|&k| size(k)).sum();
            if n < best.1 {
                best = (i, n);
            }
            match self.rests[c.rests.0..c.rests.1] {
                [next] if !c.to_nil => i = next as usize,
                _ => break,
            }
        }
        Some((&cells[best.0], best.1))
    }

    /// Whether some path from the head to `rdf:nil` has, at every cell, an
    /// `rdf:first` for which `holds` is true.
    fn every_member(&self, entry: &Entry, holds: impl Fn(u32) -> bool, scratch: &mut Scratch) -> bool {
        let cells = &self.cells[entry.cells.0..entry.cells.1];
        let Scratch { holds: held, good } = scratch;
        held.clear();
        held.extend(
            cells
                .iter()
                .map(|c| c.reaches_nil && self.firsts[c.firsts.0..c.firsts.1].iter().any(|&k| holds(k))),
        );
        if !held[0] {
            return false;
        }
        good.clear();
        good.resize(cells.len(), false);
        // A cell is good when a member holds and a rest is nil or good. The
        // cells are in breadth-first order, so a backward pass settles a
        // simple list at once; repeat until nothing changes.
        loop {
            let mut grew = false;
            for (i, c) in cells.iter().enumerate().rev() {
                if good[i] || !held[i] {
                    continue;
                }
                if c.to_nil || self.rests[c.rests.0..c.rests.1].iter().any(|&r| good[r as usize]) {
                    good[i] = true;
                    grew = true;
                }
            }
            if !grew || good[0] {
                return good[0];
            }
        }
    }
}

/// Reads lists from the `rdf:first` and `rdf:rest` tables.
#[derive(Debug)]
struct Reader<'a> {
    first: View<'a, Pair>,
    rest: View<'a, Pair>,
    /// Cells with a recent `rdf:first` or `rdf:rest`, sorted.
    touched: Vec<u32>,
    /// A cell's term to its index among the cells of the list being read,
    /// once the list is longer than [`SHORT`].
    index: FxHashMap<u32, u32>,
    scratch: Vec<u32>,
}

impl<'a> Reader<'a> {
    fn new(first: Option<&'a Variable<Pair>>, rest: Option<&'a Variable<Pair>>) -> Self {
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
            touched,
            index: FxHashMap::default(),
            scratch: Vec::new(),
        }
    }

    /// The lists of every `(class, head)` pair of `heads`.
    fn read(&mut self, heads: &Variable<Pair>) -> Lists {
        let mut lists = Lists::default();
        for part in heads.stable() {
            for &(class, head) in part.iter() {
                self.entry(&mut lists, class, head, false);
            }
        }
        for &(class, head) in heads.recent().iter() {
            self.entry(&mut lists, class, head, true);
        }
        lists
    }

    fn entry(&mut self, lists: &mut Lists, class: u32, head: u32, recent: bool) {
        let start = lists.cells.len();
        let mut changed = recent;
        self.index.clear();
        lists.cells.push(Cell::new(head));
        // Breadth first along rdf:rest, each cell once.
        let mut i = start;
        while i < lists.cells.len() {
            let term = lists.cells[i].term;
            changed |= self.touched.binary_search(&term).is_ok();
            let firsts = lists.firsts.len();
            self.first.for_each_value(term, |k| lists.firsts.push(k));
            let rests = lists.rests.len();
            let mut to_nil = false;
            let Lists {
                cells, rests: tails, ..
            } = &mut *lists;
            let index = &mut self.index;
            self.rest.for_each_value(term, |tail| {
                to_nil |= tail == NIL;
                let next = (cells.len() - start) as u32;
                // A short list is searched; a long one gets a map.
                let seen = if next as usize <= SHORT {
                    cells[start..].iter().position(|c| c.term == tail).map(|i| i as u32)
                } else {
                    if index.is_empty() {
                        index.extend(cells[start..].iter().enumerate().map(|(i, c)| (c.term, i as u32)));
                    }
                    index.get(&tail).copied()
                };
                let local = seen.unwrap_or_else(|| {
                    cells.push(Cell::new(tail));
                    if !index.is_empty() {
                        index.insert(tail, next);
                    }
                    next
                });
                tails.push(local);
            });
            let cell = &mut lists.cells[i];
            cell.firsts = (firsts, lists.firsts.len());
            cell.rests = (rests, lists.rests.len());
            cell.to_nil = to_nil;
            i += 1;
        }
        let end = lists.cells.len();
        // Which cells reach rdf:nil: a backward pass settles a simple list;
        // repeat until nothing changes.
        loop {
            let mut grew = false;
            for i in (start..end).rev() {
                let c = lists.cells[i];
                if c.reaches_nil {
                    continue;
                }
                if c.to_nil
                    || lists.rests[c.rests.0..c.rests.1]
                        .iter()
                        .any(|&r| lists.cells[start + r as usize].reaches_nil)
                {
                    lists.cells[i].reaches_nil = true;
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        self.scratch.clear();
        for c in &lists.cells[start..end] {
            if c.reaches_nil {
                self.scratch.extend_from_slice(&lists.firsts[c.firsts.0..c.firsts.1]);
            }
        }
        self.scratch.sort_unstable();
        self.scratch.dedup();
        let members = lists.members.len();
        lists.members.extend_from_slice(&self.scratch);
        lists.entries.push(Entry {
            class,
            cells: (start, end),
            members: (members, lists.members.len()),
            changed,
        });
    }
}
