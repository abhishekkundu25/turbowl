//! The property-chains family (switch `property-chains`): prp-spo2,
//! `p owl:propertyChainAxiom (p1 ... pn)`, `u0 p1 u1 ... pn un` derive
//! `u0 p un`, for chains of any length.
//!
//! # Lists
//!
//! A list is read as the reference evaluator and Nemo's program
//! (`oracles/rules/`) read it: a
//! chain is any path of `rdf:rest` from the axiom's head to `rdf:nil`, with
//! one `rdf:first` of each cell on it. A cell may have several `rdf:first`
//! (each gives its own chain) and several `rdf:rest` (each path is read). A
//! list that never reaches `rdf:nil` gives nothing. List triples are read
//! from the store each round, so those derived during the run count from
//! the round after.
//!
//! # Evaluation
//!
//! Each round, every axiom's list is walked into its chains, the
//! sequences `p1 ... pn`. A chain is an n-way join, evaluated
//! semi-naively without state between rounds:
//!
//! ```text
//! Δ = Σ_i  A1 ⋈ … ⋈ A(i-1) ⋈ Ri ⋈ S(i+1) ⋈ … ⋈ Sn
//! ```
//!
//! where `Ri` is `pi`'s recent part, `A` all of a table and `S` its stable
//! part: a match is found in the round its newest tuple arrived, from the
//! last atom that arrived then, so once over the run. Term `i` starts from
//! `Ri`, extends right on the subject indexes' stable parts, then left on
//! the object indexes (all of them), each step a merge join of a sorted,
//! duplicate-free frontier with the table's parts. The left side reads
//! all of each object index, so how an index splits into stable and recent
//! never matters, only that it exists.
//!
//! A chain is joined in full (`A1 ⋈ … ⋈ An`) instead in the round any of
//! its list triples is new, or in the round after an object index it needs
//! was first built: in the round before, the index was missing and the
//! chain was skipped.
//!
//! Two shapes have no finite set of chains, or too many: a cycle of cells
//! that each still reach `rdf:nil` (a cell with two `rdf:rest`, one back
//! into the list), and a branching list with more than [`MAX_CHAINS`]
//! chains. Their axiom is evaluated naively instead, as the reference does
//! (for each cell, the pairs the list from there joins, to a fixpoint),
//! from scratch in every round where one of its inputs is new. No generated
//! or real input has either shape.

use super::tagged;
use crate::engine::{Pair, Round, Variable, join_slices};
use crate::rules::Context;
use crate::term::{TermId, vocab};
use rustc_hash::FxHashMap;
use std::ops::Range;

/// The most chains one axiom's list is read as before it is evaluated
/// naively.
pub(super) const MAX_CHAINS: usize = 1024;

/// prp-spo2 has no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of prp-spo2.
pub(crate) fn rules(round: &mut Round<'_>, _cx: &mut Context<'_>) {
    let (Some(axioms), Some(first), Some(rest)) = (
        round.by_subject(round.constant(vocab::OWL_PROPERTY_CHAIN_AXIOM.0)),
        round.by_subject(round.constant(vocab::RDF_FIRST.0)),
        round.by_subject(round.constant(vocab::RDF_REST.0)),
    ) else {
        return;
    };
    let nil = round.constant(vocab::RDF_NIL.0);
    let nil_is_a_cell = round.canon().members(nil).len() > 1;
    let mut chains = Chains::new(first, rest, nil, nil_is_a_cell);
    for (part, recent) in tagged(axioms) {
        for &(p, head) in part {
            chains.axiom(round, p, head, recent);
        }
    }
}

/// A cell of a list: where its `rdf:first` and `rdf:rest` values sit in
/// [`Chains::firsts`] and [`Chains::rests`], and whether a chain runs
/// through it (it has a first, and a rest that is `rdf:nil` or a live cell).
#[derive(Debug, Clone)]
struct Cell {
    firsts: Range<usize>,
    rests: Range<usize>,
    live: bool,
}

/// Why an axiom's list is evaluated naively.
#[derive(Debug)]
struct Naive;

/// The lists of one round and the buffers every axiom reuses.
#[derive(Debug)]
struct Chains<'a> {
    first: &'a Variable<Pair>,
    rest: &'a Variable<Pair>,
    /// The representative of `rdf:nil`, and whether it is also read on as
    /// a cell: when `owl:sameAs` gave it a class with other members, a cell
    /// equal to `rdf:nil` but not `rdf:nil` itself can be a later cell (the
    /// W3C rules test `rdf:nil` for identity). It becomes true only by a
    /// merge that restarts the run, so it is fixed for a run.
    nil: u32,
    nil_is_a_cell: bool,
    /// The cells reachable from the current head, the head first.
    cells: Vec<u32>,
    index: FxHashMap<u32, usize>,
    info: Vec<Cell>,
    /// `(value, recent)` of each cell's `rdf:first` and `rdf:rest` triples.
    firsts: Vec<(u32, bool)>,
    rests: Vec<(u32, bool)>,
    /// The chains: their properties, concatenated, and for each its range
    /// there and whether one of its list triples is recent.
    properties: Vec<u32>,
    chains: Vec<(Range<usize>, bool)>,
    /// The walk's current path: properties, and cells on it.
    path: Vec<u32>,
    on_path: Vec<bool>,
    /// Join frontiers.
    a: Vec<Pair>,
    b: Vec<Pair>,
}

/// Pushes `(value, recent)` for each `(key, value)` of `v`.
fn values(v: &Variable<Pair>, key: u32, out: &mut Vec<(u32, bool)>) {
    for (part, recent) in tagged(v) {
        let start = part.partition_point(|t| t.0 < key);
        out.extend(part[start..].iter().take_while(|t| t.0 == key).map(|t| (t.1, recent)));
    }
}

impl<'a> Chains<'a> {
    fn new(first: &'a Variable<Pair>, rest: &'a Variable<Pair>, nil: u32, nil_is_a_cell: bool) -> Self {
        Self {
            first,
            rest,
            nil,
            nil_is_a_cell,
            cells: Vec::new(),
            index: FxHashMap::default(),
            info: Vec::new(),
            firsts: Vec::new(),
            rests: Vec::new(),
            properties: Vec::new(),
            chains: Vec::new(),
            path: Vec::new(),
            on_path: Vec::new(),
            a: Vec::new(),
            b: Vec::new(),
        }
    }

    fn axiom(&mut self, round: &mut Round<'a>, p: u32, head: u32, recent: bool) {
        self.read_list(head);
        if !self.info[0].live {
            return;
        }
        self.properties.clear();
        self.chains.clear();
        self.on_path.clear();
        self.on_path.resize(self.cells.len(), false);
        self.path.clear();
        if self.walk(0, recent).is_err() {
            self.naive(round, p, recent);
            return;
        }
        let (properties, chains) = (std::mem::take(&mut self.properties), std::mem::take(&mut self.chains));
        for (range, new) in &chains {
            self.chain(round, p, &properties[range.clone()], *new);
        }
        (self.properties, self.chains) = (properties, chains);
    }

    /// True when a list reads on from `tail` as a cell: every term but
    /// `rdf:nil`, and `rdf:nil` too when it is a cell (see the field).
    fn continues(&self, tail: u32) -> bool {
        tail != self.nil || self.nil_is_a_cell
    }

    /// Reads the cells reachable from `head` and marks the live ones.
    fn read_list(&mut self, head: u32) {
        self.cells.clear();
        self.index.clear();
        self.info.clear();
        self.firsts.clear();
        self.rests.clear();
        self.cells.push(head);
        self.index.insert(head, 0);
        let mut i = 0;
        while i < self.cells.len() {
            let cell = self.cells[i];
            let f = self.firsts.len();
            values(self.first, cell, &mut self.firsts);
            let r = self.rests.len();
            values(self.rest, cell, &mut self.rests);
            for k in r..self.rests.len() {
                let tail = self.rests[k].0;
                if self.continues(tail) && !self.index.contains_key(&tail) {
                    self.index.insert(tail, self.cells.len());
                    self.cells.push(tail);
                }
            }
            self.info.push(Cell {
                firsts: f..self.firsts.len(),
                rests: r..self.rests.len(),
                live: false,
            });
            i += 1;
        }
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..self.cells.len() {
                let cell = &self.info[i];
                if cell.live || cell.firsts.is_empty() {
                    continue;
                }
                let live = self.rests[cell.rests.clone()]
                    .iter()
                    .any(|&(t, _)| t == self.nil || self.info[self.index[&t]].live);
                if live {
                    self.info[i].live = true;
                    changed = true;
                }
            }
        }
    }

    /// Walks the live cells from `cell`, pushing every chain to `rdf:nil`.
    /// Fails on a cycle of live cells or past [`MAX_CHAINS`] chains.
    fn walk(&mut self, cell: usize, recent: bool) -> Result<(), Naive> {
        if self.on_path[cell] {
            return Err(Naive);
        }
        self.on_path[cell] = true;
        let info = self.info[cell].clone();
        for f in info.firsts {
            let (q, q_recent) = self.firsts[f];
            self.path.push(q);
            for r in info.rests.clone() {
                let (tail, tail_recent) = self.rests[r];
                let new = recent || q_recent || tail_recent;
                if tail == self.nil {
                    if self.chains.len() == MAX_CHAINS {
                        return Err(Naive);
                    }
                    let start = self.properties.len();
                    self.properties.extend_from_slice(&self.path);
                    self.chains.push((start..self.properties.len(), new));
                }
                if self.continues(tail) {
                    let next = self.index[&tail];
                    if self.info[next].live {
                        self.walk(next, new)?;
                    }
                }
            }
            self.path.pop();
        }
        self.on_path[cell] = false;
        Ok(())
    }

    /// Derives `u p w` for the chain `qs`, semi-naively unless `full`.
    fn chain(&mut self, round: &mut Round<'a>, p: u32, qs: &[u32], full: bool) {
        let n = qs.len();
        // The subject index of each property, and the object index of all
        // but the last (asked for every round, so that each is built).
        let mut so: Vec<&'a Variable<Pair>> = Vec::with_capacity(n);
        let mut os: Vec<Option<&'a Variable<Pair>>> = Vec::with_capacity(n);
        let mut missing = false;
        for (j, &q) in qs.iter().enumerate() {
            match round.by_subject(q) {
                Some(table) => so.push(table),
                None => missing = true,
            }
            if j + 1 < n {
                os.push(round.by_object(q));
            }
        }
        if missing {
            // A property with no pair: the chain joins nothing.
            return;
        }
        // An object index built this step holds every pair as recent, while
        // its subject index has a stable part: the chain was skipped last
        // round for want of it.
        let built = os
            .iter()
            .zip(&so)
            .any(|(os, so)| os.is_some_and(|os| os.stable().is_empty() && !so.stable().is_empty()));
        if full || built {
            self.full(round, p, &so);
            return;
        }
        if os.iter().any(Option::is_none) {
            return;
        }
        for i in 0..n {
            let recent = so[i].recent();
            if recent.is_empty() {
                continue;
            }
            // Right: (x_i, x_(i-1)) keyed on the object, extended on the
            // stable subject indexes to (x_n, x_(i-1)).
            self.a.clear();
            self.a.extend(recent.iter().map(|&(s, o)| (o, s)));
            self.a.sort_unstable();
            for table in &so[i + 1..] {
                if self.a.is_empty() {
                    break;
                }
                extend(&self.a, table.stable().iter().map(|b| &b[..]), &mut self.b);
                std::mem::swap(&mut self.a, &mut self.b);
            }
            if i == 0 {
                for &(w, u) in &self.a {
                    round.emit(u, p, w);
                }
                continue;
            }
            // Left: (x_(i-1), x_n), extended on the object indexes (all of
            // each) to (x_0, x_n).
            for pair in &mut self.a {
                *pair = (pair.1, pair.0);
            }
            self.a.sort_unstable();
            for table in os[..i].iter().rev() {
                if self.a.is_empty() {
                    break;
                }
                let table = table.expect("checked above");
                extend(&self.a, table.parts(), &mut self.b);
                std::mem::swap(&mut self.a, &mut self.b);
            }
            for &(u, w) in &self.a {
                round.emit(u, p, w);
            }
        }
    }

    /// Derives `u p w` for the whole chain whose subject indexes are `so`.
    fn full(&mut self, round: &mut Round<'a>, p: u32, so: &[&'a Variable<Pair>]) {
        self.a.clear();
        for part in so[0].parts() {
            self.a.extend(part.iter().map(|&(s, o)| (o, s)));
        }
        self.a.sort_unstable();
        for table in &so[1..] {
            if self.a.is_empty() {
                return;
            }
            extend(&self.a, table.parts(), &mut self.b);
            std::mem::swap(&mut self.a, &mut self.b);
        }
        for &(w, u) in &self.a {
            round.emit(u, p, w);
        }
    }

    /// The reference's reading of the list at the current head, for the
    /// shapes that have no finite set of chains: for each live cell, the
    /// pairs `(u, w)` the list from that cell to `rdf:nil` joins, to a
    /// fixpoint. Runs only in a round where one of its inputs is new.
    fn naive(&mut self, round: &mut Round<'a>, p: u32, recent: bool) {
        let store = round.store();
        let table = |q: u32| store.table(q).map(|t| t.by_subject());
        let new = recent
            || self.firsts.iter().chain(&self.rests).any(|&(_, r)| r)
            || self
                .firsts
                .iter()
                .any(|&(q, _)| table(q).is_some_and(|t| !t.recent().is_empty()));
        if !new {
            return;
        }
        // joined[c]: (u, w) pairs, sorted by u.
        let mut joined: Vec<Vec<Pair>> = vec![Vec::new(); self.cells.len()];
        let mut found: Vec<Pair> = Vec::new();
        let mut changed = true;
        while changed {
            changed = false;
            for c in 0..self.cells.len() {
                let info = self.info[c].clone();
                if !info.live {
                    continue;
                }
                found.clear();
                for f in info.firsts.clone() {
                    let Some(q) = table(self.firsts[f].0) else {
                        continue;
                    };
                    for r in info.rests.clone() {
                        let tail = self.rests[r].0;
                        if tail == self.nil {
                            for part in q.parts() {
                                found.extend_from_slice(part);
                            }
                        }
                        if !self.continues(tail) {
                            continue;
                        }
                        let next = self.index[&tail];
                        if !self.info[next].live || joined[next].is_empty() {
                            continue;
                        }
                        // (x, u) of q joined with (x, w) of the tail.
                        self.a.clear();
                        for part in q.parts() {
                            self.a.extend(part.iter().map(|&(u, x)| (x, u)));
                        }
                        self.a.sort_unstable();
                        join_slices(&self.a, &joined[next], &mut |_, u, w| found.push((u, w)));
                    }
                }
                found.sort_unstable();
                found.dedup();
                if found.len() > joined[c].len() {
                    std::mem::swap(&mut joined[c], &mut found);
                    changed = true;
                }
            }
        }
        for &(u, w) in &joined[0] {
            round.emit(u, p, w);
        }
    }
}

/// Joins the sorted, duplicate-free frontier `from` (`(key, carried)`)
/// with each sorted part of a relation (`(key, next)`) into `into`, as
/// sorted, duplicate-free `(next, carried)` pairs.
fn extend<'p>(from: &[Pair], parts: impl IntoIterator<Item = &'p [Pair]>, into: &mut Vec<Pair>) {
    into.clear();
    for part in parts {
        join_slices(from, part, &mut |_, carried, next| into.push((next, carried)));
    }
    into.sort_unstable();
    into.dedup();
}
