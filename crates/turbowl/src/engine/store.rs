//! The triple store the rules read: one predicate table per predicate term
//! id, and the round loop that runs the rules to a fixpoint.
//!
//! # Equality (ticket 11, ADR 0003)
//!
//! The store holds a [`Canon`] and keeps every tuple in terms of
//! representatives. A rule asks for a merge ([`Round::merge`]); the next
//! step applies the round's merges, then:
//!
//! - every table's tuples that mention an id whose representative changed
//!   are rewritten and queued again ([`Variable::remap`]), so they are a
//!   delta next round and every rule joins them again (as egglog does:
//!   canonicalise, then take the delta);
//! - a table whose predicate lost its place as representative is moved,
//!   rewritten, into the representative's table, as new tuples;
//! - the round's derived triples are rewritten before they are queued.
//!
//! A new tuple that a merge makes possible to join always mentions an id
//! that changed, so it is in a delta: semi-naive evaluation stays complete.
//! That fails only when a rule constant changes ([`Merged::Constant`]): a
//! rule that now reads another table, or tests for identity with another
//! class, has never joined what was stable. The step then stops the run
//! ([`Store::restarted`]), and the caller runs again from its base with
//! this canon ([`Store::into_canon`], [`Store::with_canon`]).
//!
//! # Parallel rounds (ticket 17)
//!
//! A round's output is kept by predicate ([`Pending`]): each predicate's
//! derived pairs in batches, never one buffer of `(p, s, o)` triples. The
//! step sorts each table's own pairs and advances each table on its own,
//! the tables in parallel when the round's output passes
//! [`PARALLEL_FROM`](super::parallel::PARALLEL_FROM). The rules of a round
//! run as tasks ([`Round::tasks`]) and split a large recent part into
//! chunks ([`Round::split`]), each with a buffer of its own; the buffers
//! join this round's in task order, then chunk order. Each table's pairs
//! are sorted, and the merges and the requested indexes keep the order one
//! thread gives them, so the store evolves as it does on one thread,
//! whatever the thread count ([`Store::set_workers`]) or the schedule.
//!
//! A full run's input skips the round buffer: [`Store::fill`] queues each
//! table's tuples as one sorted batch of exactly their size, copied from the
//! base's pairs of that predicate, so the first step neither copies nor
//! sorts them.

use super::canon::{Canon, Merged};
use super::join::join_slices;
use super::parallel::Workers;
use super::relation::Relation;
use super::variable::Variable;
use crate::profile::{self, Clock, Parts, Step};
use rustc_hash::FxHashMap;

mod deletion;

/// A `(subject, object)` pair, or `(object, subject)` in an object index.
pub(crate) type Pair = (u32, u32);

/// One predicate's tuples. Always indexed by subject; indexed by object too
/// once a rule asks for it ([`Round::by_object`]), since most predicates are
/// never joined on their object and a second index doubles their memory.
#[derive(Debug, Default)]
pub(crate) struct Table {
    so: Variable<Pair>,
    os: Option<Variable<Pair>>,
}

impl Table {
    /// The `(subject, object)` pairs.
    pub(crate) fn by_subject(&self) -> &Variable<Pair> {
        &self.so
    }

    /// The `(object, subject)` pairs, when this table is indexed by object.
    pub(crate) fn by_object(&self) -> Option<&Variable<Pair>> {
        self.os.as_ref()
    }

    /// A table of these pairs: a loaded snapshot (`snapshot.rs`), which
    /// checks that `os` holds the pairs of `so` flipped.
    pub(crate) fn from_parts(so: Variable<Pair>, os: Option<Variable<Pair>>) -> Self {
        Self { so, os }
    }

    fn is_active(&self) -> bool {
        self.so.is_active() || self.os.as_ref().is_some_and(Variable::is_active)
    }

    /// The bytes the table holds, by part.
    pub(crate) fn parts(&self) -> Parts {
        let (stable, recent) = self.so.bytes();
        Parts {
            tuples: self.so.len(),
            stable,
            recent,
            object_index: self.os.as_ref().map_or(0, |os| {
                let (stable, recent) = os.bytes();
                stable + recent
            }),
        }
    }
}

/// What the rules of one round produced, applied by the next `step`.
#[derive(Debug, Default)]
pub(crate) struct Pending {
    /// Derived (and inserted) pairs by predicate: a slot per predicate, in
    /// the order the predicates first came.
    slots: Vec<Slot>,
    /// Predicate to slot.
    index: FxHashMap<u32, usize>,
    /// The slot of the last push, where the next push most often goes.
    last: usize,
    /// Pairs pushed, duplicates included.
    pushed: usize,
    /// Predicates to index by object.
    by_object: Vec<u32>,
    /// Pairs of ids to make equal, in the order the rules asked.
    merges: Vec<(u32, u32)>,
    aborted: bool,
}

/// One predicate's pairs in a [`Pending`].
#[derive(Debug)]
struct Slot {
    p: u32,
    /// Where pushes go.
    pairs: Vec<Pair>,
    /// Batches joined from other buffers ([`Pending::absorb`]), moved, not
    /// copied.
    more: Vec<Vec<Pair>>,
}

impl Pending {
    /// Queues `(s, o)` for predicate `p`.
    #[inline]
    fn push(&mut self, p: u32, pair: Pair) {
        self.pushed += 1;
        match self.slots.get_mut(self.last) {
            Some(slot) if slot.p == p => slot.pairs.push(pair),
            _ => {
                let i = self.slot(p);
                self.last = i;
                self.slots[i].pairs.push(pair);
            }
        }
    }

    /// The slot of `p`, made when it has none.
    fn slot(&mut self, p: u32) -> usize {
        let slots = &mut self.slots;
        *self.index.entry(p).or_insert_with(|| {
            slots.push(Slot {
                p,
                pairs: Vec::new(),
                more: Vec::new(),
            });
            slots.len() - 1
        })
    }

    /// Takes in another buffer's output after this one's: its pairs (moved
    /// in batches), its merges and its requested indexes.
    fn absorb(&mut self, other: Pending) {
        for slot in other.slots {
            let i = self.slot(slot.p);
            let mine = &mut self.slots[i];
            mine.more
                .extend(std::iter::once(slot.pairs).chain(slot.more).filter(|b| !b.is_empty()));
        }
        self.pushed += other.pushed;
        self.by_object.extend(other.by_object);
        self.merges.extend(other.merges);
        self.aborted |= other.aborted;
    }

    /// Queues `batch`, pairs of predicate `p`, moved, not copied.
    fn push_batch(&mut self, p: u32, batch: Vec<Pair>) {
        if batch.is_empty() {
            return;
        }
        self.pushed += batch.len();
        let i = self.slot(p);
        self.slots[i].more.push(batch);
    }

    /// Sorts and deduplicates each predicate's pairs, in the task that made
    /// them, so that the step has fewer to sort, and runs to merge.
    fn compact(&mut self) {
        for slot in &mut self.slots {
            slot.pairs.sort_unstable();
            slot.pairs.dedup();
        }
    }

    /// Pairs held, duplicates included.
    fn len(&self) -> usize {
        self.slots
            .iter()
            .map(|s| s.pairs.len() + s.more.iter().map(Vec::len).sum::<usize>())
            .sum()
    }

    /// True when no pair and no merge is held.
    fn is_empty(&self) -> bool {
        self.len() == 0 && self.merges.is_empty()
    }

    /// Every predicate's batches, ascending by predicate, leaving no slot.
    fn take(&mut self) -> Vec<(u32, Vec<Vec<Pair>>)> {
        self.index.clear();
        self.last = 0;
        self.pushed = 0;
        let mut groups: Vec<(u32, Vec<Vec<Pair>>)> = self
            .slots
            .drain(..)
            .map(|slot| {
                let batches: Vec<Vec<Pair>> = std::iter::once(slot.pairs)
                    .chain(slot.more)
                    .filter(|b| !b.is_empty())
                    .collect();
                (slot.p, batches)
            })
            .filter(|(_, batches)| !batches.is_empty())
            .collect();
        groups.sort_unstable_by_key(|g| g.0);
        groups
    }

    /// Drops every pair and merge (a restart).
    fn clear(&mut self) {
        self.take();
        self.merges.clear();
        self.aborted = false;
    }

    /// Every pair held, as `(s, p, o)`: slot by slot, in the order the
    /// predicates first came, each slot's pairs in the order they came.
    fn triples(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.slots.iter().flat_map(|slot| {
            std::iter::once(&slot.pairs)
                .chain(&slot.more)
                .flatten()
                .map(move |&(s, o)| (s, slot.p, o))
        })
    }

    /// The pairs the buffer has room for.
    fn capacity(&self) -> usize {
        self.slots
            .iter()
            .map(|s| s.pairs.capacity() + s.more.iter().map(Vec::capacity).sum::<usize>())
            .sum()
    }

    /// The bytes the buffer holds: its pairs, slots and index.
    fn bytes(&self) -> usize {
        let slots: usize = self
            .slots
            .iter()
            .map(|s| {
                profile::vec_bytes(&s.pairs)
                    + profile::vec_bytes(&s.more)
                    + s.more.iter().map(profile::vec_bytes).sum::<usize>()
            })
            .sum();
        slots
            + profile::vec_bytes(&self.slots)
            + profile::hash_bytes(self.index.capacity(), std::mem::size_of::<(u32, usize)>())
            + profile::vec_bytes(&self.by_object)
            + profile::vec_bytes(&self.merges)
    }
}

/// Predicate tables, semi-naively evaluated.
#[derive(Debug, Default)]
pub(crate) struct Store {
    tables: Vec<Table>,
    /// The predicate of each table.
    predicates: Vec<u32>,
    /// Predicate term id to table index. Looked up once per rule and
    /// predicate, never inside a join loop.
    index: FxHashMap<u32, usize>,
    /// Tables whose next `step` has work (queued tuples or a recent part).
    active: Vec<usize>,
    is_active: Vec<bool>,
    pending: Pending,
    rounds: usize,
    /// Which ids are equal; every stored tuple is in its representatives.
    canon: Canon,
    /// The ids whose representative changed in the last step.
    moved: Vec<u32>,
    /// A merge changed a rule constant: the last run stopped there.
    restarted: bool,
    /// Where a round's parallel work runs.
    workers: Workers,
    /// The tuples in the recent parts (both indexes) after the last step:
    /// the next round's delta.
    delta: usize,
    /// The steps of the last run ([`Store::take_steps`]).
    steps: Vec<Step>,
    /// The pairs [`Store::fill`] queued in the tables, for the next step's
    /// count of its pairs ([`Step::pairs`]).
    filled: usize,
    deleting: bool,
    /// Rounds and steps that ran in parallel.
    #[cfg(test)]
    engaged: usize,
}

impl Store {
    /// An empty store, with no constants.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// An empty store whose ids are made equal as `canon` says: queued
    /// triples are rewritten into its representatives.
    pub(crate) fn with_canon(canon: Canon) -> Self {
        Self {
            canon,
            ..Self::default()
        }
    }

    /// A store holding `tables` (each predicate once), whose ids are made
    /// equal as `canon` says, after `rounds` rounds: a store as a run leaves
    /// it, restored from a snapshot (`snapshot.rs` checks every invariant
    /// first: the predicates distinct, the tables' batches, the tuples in
    /// representatives).
    pub(crate) fn restore(tables: Vec<(u32, Table)>, canon: Canon, rounds: usize) -> Self {
        let mut store = Self {
            canon,
            rounds,
            ..Self::default()
        };
        for (p, table) in tables {
            debug_assert!(!table.is_active(), "a restored table has no round pending");
            let t = store.table_index(p);
            debug_assert_eq!(t, store.tables.len() - 1, "a predicate is restored once");
            store.tables[t] = table;
        }
        store
    }

    /// True when no round is pending: no table has a recent part or queued
    /// tuples, and nothing is queued but triples ([`Store::queued`]). A
    /// store is settled between runs, which end at a fixpoint.
    pub(crate) fn is_settled(&self) -> bool {
        self.active.is_empty()
            && self.tables.iter().all(|t| !t.is_active())
            && self.moved.is_empty()
            && !self.restarted
            && self.pending.merges.is_empty()
            && self.pending.by_object.is_empty()
    }

    /// The triples queued for the next run ([`Store::insert`],
    /// [`Store::extend`]), predicate by predicate in the order the
    /// predicates first came.
    pub(crate) fn queued(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.pending.triples()
    }

    /// The canon, for a store that is dropped (to run again from the base).
    pub(crate) fn into_canon(self) -> Canon {
        self.canon
    }

    /// Which ids are equal.
    pub(crate) fn canon(&self) -> &Canon {
        &self.canon
    }

    /// True when the last run stopped because a merge changed a rule
    /// constant: the store is incomplete, and the run must start again
    /// from the base with [`Store::into_canon`].
    pub(crate) fn restarted(&self) -> bool {
        self.restarted
    }

    /// Runs the rounds' parallel work on `workers` (one thread unless set).
    pub(crate) fn set_workers(&mut self, workers: Workers) {
        self.workers = workers;
    }

    /// Queues a triple; the next run sees it as new.
    pub(crate) fn insert(&mut self, s: u32, p: u32, o: u32) {
        self.pending.push(p, (s, o));
    }

    /// Queues `(subject, predicate, object)` triples.
    pub(crate) fn extend(&mut self, triples: impl IntoIterator<Item = (u32, u32, u32)>) {
        for (s, p, o) in triples {
            self.pending.push(p, (s, o));
        }
    }

    /// Queues a run's input from the base: each predicate's `[s, o]` pairs
    /// (`base`, as [`Store::fill`] takes it), each copied as one batch, which
    /// the step rewrites into representatives and sorts (a full run after a
    /// restart).
    pub(crate) fn extend_by_predicate(&mut self, base: &[(u32, &[[u32; 2]])]) {
        let parallel = self.workers.engage(pairs_in(base));
        for &(p, pairs) in base {
            let batch = copied(&self.workers, pairs, parallel);
            self.pending.push_batch(p, batch);
        }
    }

    /// Fills a new store with a full run's input: `base`, each predicate's
    /// `[s, o]` pairs (ascending by predicate, each sorted, deduplicated and
    /// not empty), and `facts`, `[s, p, o]` triples. Each predicate's pairs
    /// are queued in its table as one batch of exactly their length, so that
    /// the first step neither copies nor sorts them. A predicate with no
    /// facts gets a copy of the base's pairs, made in chunks on the workers
    /// when the base is large; one with facts gets the union.
    ///
    /// The pairs must be in representatives: the canon must have no class.
    /// The store must have no table and nothing queued, so that its tables
    /// are made in ascending predicate order, as a step makes them.
    pub(crate) fn fill(&mut self, base: &[(u32, &[[u32; 2]])], facts: &[[u32; 3]]) {
        debug_assert!(self.canon.is_empty(), "the pairs are in representatives");
        debug_assert!(self.tables.is_empty() && self.pending.is_empty(), "a new store");
        debug_assert!(base.windows(2).all(|w| w[0].0 < w[1].0), "ascending by predicate");
        let mut facts: Vec<(u32, Pair)> = facts.iter().map(|&[s, p, o]| (p, (s, o))).collect();
        facts.sort_unstable();
        facts.dedup();
        let mut facts = facts.chunk_by(|a, b| a.0 == b.0).peekable();
        let parallel = self.workers.engage(pairs_in(base));
        let mut base = base.iter().peekable();
        loop {
            let next_fact = facts.peek().map(|g| g[0].0);
            let (p, pairs) = match (base.peek(), next_fact) {
                (None, None) => break,
                (Some(&&(p, _)), f) if f.is_none_or(|f| p <= f) => {
                    let &(p, mine) = base.next().expect("peeked");
                    match facts.next_if(|g| g[0].0 == p) {
                        None => (p, copied(&self.workers, mine, parallel)),
                        Some(group) => (p, union(mine, group)),
                    }
                }
                _ => {
                    let group = facts.next().expect("peeked");
                    (group[0].0, group.iter().map(|&(_, pair)| pair).collect())
                }
            };
            let t = self.table_index(p);
            self.filled += pairs.len();
            self.tables[t].so.insert(Relation::from_sorted(pairs));
            self.activate(t);
        }
    }

    /// Indexes predicate `p` by object from the next round on.
    pub(crate) fn index_by_object(&mut self, p: u32) {
        self.pending.by_object.push(p);
    }

    /// Runs `rules` round after round until a round derives nothing new.
    /// Returns the rounds in which `rules` ran. Called again after more
    /// inserts, it continues semi-naively from the stored state.
    pub(crate) fn run(&mut self, rules: impl FnMut(&mut Round<'_>)) -> usize {
        self.run_until(|| false, rules).expect("a run nobody stops ends")
    }

    /// [`Store::run`], asking `stop` before each round: `None` when it said
    /// to stop, and the store is then only fit to be dropped.
    pub(crate) fn run_until(
        &mut self,
        mut stop: impl FnMut() -> bool,
        mut rules: impl FnMut(&mut Round<'_>),
    ) -> Option<usize> {
        debug_assert!(!self.restarted, "a store that stopped for a restart is not run again");
        self.steps.clear();
        let mut pending = std::mem::take(&mut self.pending);
        let mut rounds = 0;
        while self.step(&mut pending) {
            if stop() {
                return None;
            }
            rounds += 1;
            self.rounds += 1;
            let parallel = self.workers.engage(self.delta);
            #[cfg(test)]
            {
                self.engaged += usize::from(parallel);
            }
            rules(&mut Round {
                store: self,
                pending: &mut pending,
                parallel,
            });
        }
        // The step took every batch; what is left (the slot table) is
        // given back, so that a stored closure does not keep it.
        debug_assert!(pending.is_empty());
        self.pending = Pending::default();
        Some(rounds)
    }

    /// Applies a round's output and advances every active table. True when
    /// some table has a non-empty recent part or an id moved, so another
    /// round is needed.
    fn step(&mut self, pending: &mut Pending) -> bool {
        let start = Clock::now();
        let mut log = Step::default();
        let changed = self.advance(pending, &mut log);
        log.ns = start.elapsed_ns();
        self.steps.push(log);
        changed
    }

    /// The step's work, its counts and times kept in `log`.
    fn advance(&mut self, pending: &mut Pending, log: &mut Step) -> bool {
        // The pairs `fill` queued, distinct and in tables that were empty.
        log.pairs = std::mem::take(&mut self.filled);
        // Object indexes first, so that the rewriting below covers them.
        pending.by_object.sort_unstable();
        pending.by_object.dedup();
        for p in std::mem::take(&mut pending.by_object) {
            let t = self.table_index(p);
            let table = &mut self.tables[t];
            if table.os.is_none() {
                let start = Clock::now();
                let workers = self.workers.engage(table.so.len()).then_some(&self.workers);
                let parts: Vec<&[Pair]> = table.so.parts().collect();
                let mut os = Variable::new();
                os.insert(flipped(&parts, workers));
                table.os = Some(os);
                log.sort_ns += start.elapsed_ns();
                self.activate(t);
            }
        }

        self.moved.clear();
        let mut constant = false;
        for (a, b) in std::mem::take(&mut pending.merges) {
            constant |= self.canon.merge(a, b, &mut self.moved) == Merged::Constant;
        }
        if constant {
            self.restarted = true;
            pending.clear();
            return false;
        }
        if !self.moved.is_empty() {
            self.rewrite(pending);
        }

        let parallel = self.workers.engage(pending.len());
        #[cfg(test)]
        {
            self.engaged += usize::from(parallel);
        }
        let mut groups = pending.take();
        if !self.canon.is_empty() {
            canonical(&self.canon, &mut groups);
        }
        // Tables are made in ascending predicate order, as the step of one
        // thread makes them.
        let mut work: Vec<(usize, Vec<Vec<Pair>>)> = Vec::with_capacity(groups.len());
        for (p, batches) in groups {
            let t = self.table_index(p);
            self.activate(t);
            work.push((t, batches));
        }
        work.sort_unstable_by_key(|w| w.0);

        // Each active table, with the pairs the round gave it, advances on
        // its own.
        let mut active = std::mem::take(&mut self.active);
        active.sort_unstable();
        let mut work = work.into_iter().peekable();
        let mut jobs: Vec<Job<'_>> = Vec::with_capacity(active.len());
        let mut rest: &mut [Table] = &mut self.tables;
        let mut offset = 0;
        for &t in &active {
            let (_, tail) = std::mem::take(&mut rest).split_at_mut(t - offset);
            let (table, tail) = tail.split_first_mut().expect("an active table exists");
            rest = tail;
            offset = t + 1;
            let batches = work.next_if(|w| w.0 == t).map(|w| w.1);
            jobs.push(Job {
                t,
                table,
                batches,
                new: false,
                recent: 0,
                pairs: 0,
                sort_ns: 0,
                changed_ns: 0,
            });
        }
        debug_assert!(work.next().is_none(), "every table with new pairs is active");
        let workers = &self.workers;
        if parallel {
            // Each table on its own thread, timed on its own.
            workers.for_each(&mut jobs, |job| job.advance(Some(workers)));
            for job in &jobs {
                log.sort_ns += job.sort_ns;
                log.changed_ns += job.changed_ns;
            }
        } else {
            // On this thread, phase by phase, so that a few clock readings
            // time every table.
            let t0 = Clock::now();
            jobs.iter_mut().for_each(|job| job.sort(None));
            let t1 = Clock::now();
            jobs.iter_mut().for_each(|job| job.change(None));
            let t2 = Clock::now();
            jobs.iter_mut().for_each(|job| job.sort_index(None));
            let t3 = Clock::now();
            jobs.iter_mut().for_each(|job| job.change_index(None));
            let t4 = Clock::now();
            log.sort_ns += t0.ns_to(t1) + t2.ns_to(t3);
            log.changed_ns += t1.ns_to(t2) + t3.ns_to(t4);
        }

        let mut changed = false;
        self.delta = 0;
        for job in jobs {
            changed |= job.new;
            self.delta += job.recent;
            log.pairs += job.pairs;
            log.new += job.table.so.recent().len();
            if job.new {
                self.active.push(job.t);
            } else {
                self.is_active[job.t] = false;
            }
        }
        debug_assert!(
            self.tables
                .iter()
                .enumerate()
                .all(|(t, table)| !table.is_active() || self.is_active[t])
        );
        // A merge whose rewritten tuples were all known still changes what
        // the stored ones stand for: a rule that reads the moved ids
        // ([`Round::moved`]) gets a round.
        changed || !self.moved.is_empty()
    }

    /// Rewrites the stored tuples after merges: in place for a table whose
    /// predicate is still a representative, into `pending` (and so into the
    /// representative's table) for one whose predicate is not.
    fn rewrite(&mut self, pending: &mut Pending) {
        let canon = &self.canon;
        let pair = |(a, b): Pair| (canon.find(a), canon.find(b));
        for t in 0..self.tables.len() {
            let p = self.predicates[t];
            let to = canon.find(p);
            let table = &mut self.tables[t];
            if to != p {
                // Between steps a table's `so` has nothing queued; the
                // object index goes with the table (the representative's
                // table builds its own when a rule asks).
                let moved = std::mem::take(table);
                for (s, o) in moved.so.all().iter() {
                    pending.push(to, pair((s, o)));
                }
                continue;
            }
            let mut moved = table.so.remap(pair);
            if let Some(os) = &mut table.os {
                moved += os.remap(pair);
            }
            if moved > 0 && !self.is_active[t] {
                self.is_active[t] = true;
                self.active.push(t);
            }
        }
    }

    fn table_index(&mut self, p: u32) -> usize {
        *self.index.entry(p).or_insert_with(|| {
            self.tables.push(Table::default());
            self.predicates.push(p);
            self.is_active.push(false);
            self.tables.len() - 1
        })
    }

    fn activate(&mut self, t: usize) {
        if !self.is_active[t] {
            self.is_active[t] = true;
            self.active.push(t);
        }
    }

    /// The table of predicate `p`.
    pub(crate) fn table(&self, p: u32) -> Option<&Table> {
        self.index.get(&p).map(|&t| &self.tables[t])
    }

    /// Every predicate and its table.
    pub(crate) fn tables(&self) -> impl Iterator<Item = (u32, &Table)> {
        self.predicates.iter().copied().zip(&self.tables)
    }

    /// True when the store holds `(s, p, o)` (queued triples not counted).
    pub(crate) fn contains(&self, s: u32, p: u32, o: u32) -> bool {
        self.table(p).is_some_and(|t| t.so.contains(&(s, o)))
    }

    /// Triples held (queued triples not counted).
    pub(crate) fn len(&self) -> usize {
        self.tables.iter().map(|t| t.so.len()).sum()
    }

    /// True when the store holds no triple.
    pub(crate) fn is_empty(&self) -> bool {
        self.tables.iter().all(|t| t.so.is_empty())
    }

    /// Every `(subject, predicate, object)`, table by table, in
    /// representatives (see [`Store::canon`]).
    pub(crate) fn triples(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.tables().flat_map(|(p, t)| {
            t.so.parts()
                .flat_map(move |part| part.iter().map(move |&(s, o)| (s, p, o)))
        })
    }

    /// The triples the round buffer has room for: what it holds between
    /// runs, queued triples or not.
    #[cfg(test)]
    pub(crate) fn buffered(&self) -> usize {
        self.pending.capacity()
    }

    /// The rounds and steps that ran in parallel since the store was made.
    #[cfg(test)]
    pub(crate) fn engaged(&self) -> usize {
        self.engaged
    }

    /// Rounds run since the store was made.
    pub(crate) fn rounds(&self) -> usize {
        self.rounds
    }

    /// The steps of the last run, taken: each run starts a new list.
    pub(crate) fn take_steps(&mut self) -> Vec<Step> {
        std::mem::take(&mut self.steps)
    }

    /// The bytes the store's own structures hold, its tables left out: the
    /// list of tables, the index from predicate to table, the active list,
    /// the round buffer (the triples queued for the next run) and the ids
    /// the last step moved.
    pub(crate) fn own_bytes(&self) -> usize {
        use std::mem::size_of;
        profile::vec_bytes(&self.tables)
            + profile::vec_bytes(&self.predicates)
            + profile::hash_bytes(self.index.capacity(), size_of::<(u32, usize)>())
            + profile::vec_bytes(&self.active)
            + profile::vec_bytes(&self.is_active)
            + self.pending.bytes()
            + profile::vec_bytes(&self.moved)
            + profile::vec_bytes(&self.steps)
    }
}

/// Rewrites a round's output into representatives: each pair, and each
/// predicate (whose batches then join its representative's).
fn canonical(canon: &Canon, groups: &mut Vec<(u32, Vec<Vec<Pair>>)>) {
    for (p, batches) in groups.iter_mut() {
        *p = canon.find(*p);
        for pair in batches.iter_mut().flatten() {
            *pair = (canon.find(pair.0), canon.find(pair.1));
        }
    }
    groups.sort_by_key(|g| g.0);
    let mut merged: Vec<(u32, Vec<Vec<Pair>>)> = Vec::with_capacity(groups.len());
    for (p, batches) in groups.drain(..) {
        match merged.last_mut() {
            Some(last) if last.0 == p => last.1.extend(batches),
            _ => merged.push((p, batches)),
        }
    }
    *groups = merged;
}

/// The pairs of `base`, all predicates together.
fn pairs_in(base: &[(u32, &[[u32; 2]])]) -> usize {
    base.iter().map(|(_, pairs)| pairs.len()).sum()
}

/// `pairs` in a vector of exactly their length: in chunks side by side when
/// `parallel` (and there are many), else on this thread.
fn copied(workers: &Workers, pairs: &[[u32; 2]], parallel: bool) -> Vec<Pair> {
    if parallel {
        workers.map_parts(&[pairs], |&[s, o]| (s, o))
    } else {
        pairs.iter().map(|&[s, o]| (s, o)).collect()
    }
}

/// The union of sorted, duplicate-free `base` and a predicate's facts
/// (`(p, pair)`, sorted, duplicate-free), in a vector of exactly its
/// length. The facts are few: each finds its place by binary search.
fn union(base: &[[u32; 2]], facts: &[(u32, Pair)]) -> Vec<Pair> {
    let new = facts
        .iter()
        .filter(|&&(_, (s, o))| base.binary_search(&[s, o]).is_err())
        .count();
    let mut out = Vec::with_capacity(base.len() + new);
    let mut rest = base;
    for &(_, (s, o)) in facts {
        let before = rest.partition_point(|x| *x < [s, o]);
        out.extend(rest[..before].iter().map(|&[s, o]| (s, o)));
        rest = &rest[before..];
        if rest.first() == Some(&[s, o]) {
            rest = &rest[1..];
        }
        out.push((s, o));
    }
    out.extend(rest.iter().map(|&[s, o]| (s, o)));
    out
}

/// One table's part of a step.
struct Job<'a> {
    t: usize,
    table: &'a mut Table,
    /// The pairs the round gave the table, in batches.
    batches: Option<Vec<Vec<Pair>>>,
    /// The table has a non-empty recent part after the step.
    new: bool,
    /// The tuples in its recent parts after the step.
    recent: usize,
    /// The distinct pairs the round gave it.
    pairs: usize,
    /// Its time in sorts and in changes ([`Job::advance`] only).
    sort_ns: u64,
    changed_ns: u64,
}

impl Job<'_> {
    /// Queues the round's pairs in the table, sorted, and ends the round for
    /// it ([`Variable::changed`]): the four phases below, timed. With
    /// `workers`, its large sorts, merges and filters run in parallel.
    fn advance(&mut self, workers: Option<&Workers>) {
        let t0 = Clock::now();
        self.sort(workers);
        let t1 = Clock::now();
        self.change(workers);
        let t2 = Clock::now();
        self.sort_index(workers);
        let t3 = Clock::now();
        self.change_index(workers);
        let t4 = Clock::now();
        self.sort_ns = t0.ns_to(t1) + t2.ns_to(t3);
        self.changed_ns = t1.ns_to(t2) + t3.ns_to(t4);
    }

    /// Queues the round's pairs in the table, sorted and deduplicated.
    fn sort(&mut self, workers: Option<&Workers>) {
        if let Some(batches) = self.batches.take() {
            let pairs = sorted_batches(batches, workers);
            self.pairs = pairs.len();
            self.table.so.insert(pairs);
        }
    }

    /// Ends the round for the pairs by subject: the new ones become recent.
    fn change(&mut self, workers: Option<&Workers>) {
        self.new = self.table.so.changed_on(workers);
        self.recent = self.table.so.recent().len();
    }

    /// The object index gets what is new by subject, flipped: the same
    /// tuples, so it sorts no pair that is not new.
    fn sort_index(&mut self, workers: Option<&Workers>) {
        let Table { so, os } = &mut *self.table;
        if let Some(os) = os
            && !so.recent().is_empty()
        {
            os.insert(flipped(&[&so.recent()[..]], workers));
        }
    }

    /// Ends the round for the object index.
    fn change_index(&mut self, workers: Option<&Workers>) {
        if let Some(os) = &mut self.table.os {
            self.new |= os.changed_on(workers);
            self.recent += os.recent().len();
        }
    }
}

/// The object index of `parts`, pairs by subject: each pair flipped, in one
/// vector of exactly their length, sorted in place, on `workers` when
/// given. The parts are a variable's, disjoint and each duplicate-free, so
/// no two flipped pairs are equal and there is nothing to deduplicate
/// ([`Workers::sort_distinct`] checks it in debug builds).
pub(crate) fn flipped(parts: &[&[Pair]], workers: Option<&Workers>) -> Relation<Pair> {
    let one = Workers::one();
    let workers = workers.unwrap_or(&one);
    let mut pairs = workers.map_parts(parts, |&(s, o)| (o, s));
    workers.sort_distinct(&mut pairs);
    Relation::from_sorted(pairs)
}

/// The pairs of `batches`, sorted and deduplicated, on `workers` when given.
fn sorted_batches(batches: Vec<Vec<Pair>>, workers: Option<&Workers>) -> Relation<Pair> {
    let one = Workers::one();
    Relation::from_sorted(workers.unwrap_or(&one).sort_dedup_batches(batches))
}

/// What a rule function sees in one round: the store as the last round left
/// it (read-only), and where to put what it derives.
#[derive(Debug)]
pub(crate) struct Round<'a> {
    store: &'a Store,
    pending: &'a mut Pending,
    /// This round's work runs on the store's workers.
    parallel: bool,
}

impl<'a> Round<'a> {
    pub(crate) fn abort_deletion(&mut self) {
        self.pending.aborted = true;
    }

    pub(crate) fn is_deleting(&self) -> bool {
        self.store.deleting
    }

    /// The store, read-only for this round.
    pub(crate) fn store(&self) -> &'a Store {
        self.store
    }

    /// Predicate `p`'s `(subject, object)` pairs.
    pub(crate) fn by_subject(&self, p: u32) -> Option<&'a Variable<Pair>> {
        self.store.table(p).map(Table::by_subject)
    }

    /// Predicate `p`'s `(object, subject)` pairs. The first time a predicate
    /// is asked for, this returns `None` and the index is built for the next
    /// round, where all its pairs are recent: a rule that skips its join this
    /// round loses nothing.
    pub(crate) fn by_object(&mut self, p: u32) -> Option<&'a Variable<Pair>> {
        let table = self.store.table(p);
        match table.and_then(Table::by_object) {
            Some(os) => Some(os),
            None => {
                if table.is_some_and(|t| !t.so.is_empty()) {
                    self.pending.by_object.push(p);
                }
                None
            }
        }
    }

    /// Derives `(s, p, o)`. Duplicates and known triples are dropped at the
    /// end of the round.
    #[inline]
    pub(crate) fn emit(&mut self, s: u32, p: u32, o: u32) {
        self.pending.push(p, (s, o));
    }

    /// True when this round's work runs in parallel: its delta passed
    /// [`PARALLEL_FROM`](super::parallel::PARALLEL_FROM) and the store has
    /// threads to run it on.
    pub(crate) fn is_parallel(&self) -> bool {
        self.parallel
    }

    /// Runs `f` once for each of `states`, in order. In a parallel round
    /// each call is a task with a round of its own, whose output joins this
    /// round's in the order of `states`: the round's output is the one the
    /// calls give in order on one thread. (The reasoner runs each family as
    /// one task.)
    pub(crate) fn tasks<S: Send>(&mut self, states: &mut [S], f: impl Fn(&mut Round<'_>, &mut S) + Sync + Send) {
        if !self.parallel || states.len() < 2 {
            for state in states {
                f(self, state);
            }
            return;
        }
        let store = self.store;
        let mut jobs: Vec<(&mut S, Pending)> = states.iter_mut().map(|s| (s, Pending::default())).collect();
        store.workers.for_each(&mut jobs, |(state, out)| {
            f(
                &mut Round {
                    store,
                    pending: out,
                    parallel: true,
                },
                state,
            );
        });
        for (_, out) in jobs {
            self.pending.absorb(out);
        }
    }

    /// The semi-naive join of [`join`](super::join()): `f(round, key, a_value,
    /// b_value)` for every pair where at least one side is recent, each pair
    /// once over a run. In a parallel round each recent side is read in
    /// chunks ([`Round::split`]); `f` emits, and never merges, since the
    /// pairs then come in another order.
    pub(crate) fn join<K, A, B>(
        &mut self,
        a: &Variable<(K, A)>,
        b: &Variable<(K, B)>,
        f: impl Fn(&mut Round<'_>, K, A, B) + Sync + Send,
    ) where
        K: Ord + Copy + Sync,
        A: Ord + Copy + Sync,
        B: Ord + Copy + Sync,
    {
        let (ra, rb) = (a.recent(), b.recent());
        if !ra.is_empty() {
            self.split(ra, |round, chunk, _: &mut ()| {
                for sb in b.stable() {
                    join_slices(chunk, sb, &mut |k, x, y| f(round, k, x, y));
                }
            });
        }
        if !rb.is_empty() {
            self.split(rb, |round, chunk, _: &mut ()| {
                for sa in a.stable() {
                    join_slices(sa, chunk, &mut |k, x, y| f(round, k, x, y));
                }
                join_slices(ra, chunk, &mut |k, x, y| f(round, k, x, y));
            });
        }
    }

    /// Calls `f` on consecutive chunks of `items`, each with a state of its
    /// own, and returns the states in order. In a parallel round, a large
    /// `items` (a recent part) splits into chunks that run as tasks, each
    /// with a round of its own whose output joins this round's in chunk
    /// order; otherwise `f` runs once, on all of `items`, with this round.
    ///
    /// The step sorts each table's pairs, so the order of the emits does
    /// not matter, but what is emitted must not depend on the split: `f`'s
    /// work on an item must not depend on the items before it in its
    /// chunk ([`Round::split_runs`] keeps runs of a key whole). The merges
    /// keep their order, which the canon's representatives depend on: `f`
    /// asks for them one item at a time, in item order, or not at all.
    pub(crate) fn split<T: Sync, S: Send + Default>(
        &mut self,
        items: &[T],
        f: impl Fn(&mut Round<'_>, &[T], &mut S) + Sync + Send,
    ) -> Vec<S> {
        self.split_by(items, |_, _| false, f)
    }

    /// [`Round::split`] over pairs sorted by key, with no key's run cut
    /// across two chunks: for work on a run as a whole.
    pub(crate) fn split_runs<K: Eq + Sync, V: Sync, S: Send + Default>(
        &mut self,
        items: &[(K, V)],
        f: impl Fn(&mut Round<'_>, &[(K, V)], &mut S) + Sync + Send,
    ) -> Vec<S> {
        self.split_by(items, |a, b| a.0 == b.0, f)
    }

    /// Sorts and deduplicates `items`, on the workers in a parallel round.
    pub(crate) fn sort_dedup<T: Ord + Copy + Send + Sync + Default>(&self, items: Vec<T>) -> Vec<T> {
        if self.parallel {
            self.store.workers.sort_dedup(items)
        } else {
            let mut items = items;
            items.sort_unstable();
            items.dedup();
            items
        }
    }

    fn split_by<T: Sync, S: Send + Default>(
        &mut self,
        items: &[T],
        same: impl Fn(&T, &T) -> bool,
        f: impl Fn(&mut Round<'_>, &[T], &mut S) + Sync + Send,
    ) -> Vec<S> {
        let workers = &self.store.workers;
        if !self.parallel || workers.chunk(items.len()) >= items.len() {
            let mut state = S::default();
            f(self, items, &mut state);
            return vec![state];
        }
        let store = self.store;
        let outs = workers.chunks_by(items, same, |chunk| {
            let mut out = Pending::default();
            let mut state = S::default();
            f(
                &mut Round {
                    store,
                    pending: &mut out,
                    parallel: true,
                },
                chunk,
                &mut state,
            );
            out.compact();
            (out, state)
        });
        outs.into_iter()
            .map(|(out, state)| {
                self.pending.absorb(out);
                state
            })
            .collect()
    }

    /// The representative of `id`: how a rule reads a constant it names,
    /// so that a constant made equal to another term keeps its meaning.
    #[inline]
    pub(crate) fn constant(&self, id: u32) -> u32 {
        self.store.canon.find(id)
    }

    /// Which ids are equal.
    pub(crate) fn canon(&self) -> &'a Canon {
        &self.store.canon
    }

    /// Makes `a` and `b` equal from the next round on: the next step merges
    /// their classes and rewrites the store. The merges apply in the order
    /// they are asked for, and the canon's representatives depend on it, so
    /// a rule asks for them in an order no split changes (see
    /// [`Round::split`]).
    pub(crate) fn merge(&mut self, a: u32, b: u32) {
        self.pending.merges.push((a, b));
    }

    /// The ids whose representative changed in the step before this round.
    pub(crate) fn moved(&self) -> &'a [u32] {
        &self.store.moved
    }

    /// True when the class of `id` changed in the step before this round:
    /// `id` or a term merged into its class moved ([`Round::moved`]). A
    /// rule constant that is not a vocabulary term (the literals the W3C
    /// tables write, such as `"1"^^xsd:nonNegativeInteger`) can change its
    /// representative, or its class, without a restart, so a rule that
    /// names one joins everything again in such a round.
    pub(crate) fn class_moved(&self, id: u32) -> bool {
        let canon = &self.store.canon;
        let r = canon.find(id);
        self.store.moved.iter().any(|&m| canon.find(m) == r)
    }

    /// This round's number in the store's life, from 1.
    pub(crate) fn number(&self) -> usize {
        self.store.rounds
    }

    /// Triples emitted so far this round, duplicates included (a counter:
    /// read before and after a family's call, it says what the family emitted).
    pub(crate) fn emitted(&self) -> usize {
        self.pending.pushed
    }
}
