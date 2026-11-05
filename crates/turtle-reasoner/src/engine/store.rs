//! The triple store the rules read: one predicate table per predicate term
//! id, and the round loop that runs the rules to a fixpoint.

use super::relation::Relation;
use super::variable::Variable;
use rustc_hash::FxHashMap;

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

    fn is_active(&self) -> bool {
        self.so.is_active() || self.os.as_ref().is_some_and(Variable::is_active)
    }
}

/// What the rules of one round produced, applied by the next `step`.
#[derive(Debug, Default)]
pub(crate) struct Pending {
    /// Derived (and inserted) triples, as `(predicate, subject, object)` so
    /// that sorting groups them by table.
    triples: Vec<(u32, u32, u32)>,
    /// Predicates to index by object.
    by_object: Vec<u32>,
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
}

impl Store {
    /// An empty store.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Queues a triple; the next run sees it as new.
    pub(crate) fn insert(&mut self, s: u32, p: u32, o: u32) {
        self.pending.triples.push((p, s, o));
    }

    /// Queues `(subject, predicate, object)` triples.
    pub(crate) fn extend(&mut self, triples: impl IntoIterator<Item = (u32, u32, u32)>) {
        self.pending
            .triples
            .extend(triples.into_iter().map(|(s, p, o)| (p, s, o)));
    }

    /// Indexes predicate `p` by object from the next round on.
    pub(crate) fn index_by_object(&mut self, p: u32) {
        self.pending.by_object.push(p);
    }

    /// Runs `rules` round after round until a round derives nothing new.
    /// Returns the rounds in which `rules` ran. Called again after more
    /// inserts, it continues semi-naively from the stored state.
    pub(crate) fn run(&mut self, mut rules: impl FnMut(&mut Round<'_>)) -> usize {
        let mut pending = std::mem::take(&mut self.pending);
        let mut rounds = 0;
        while self.step(&mut pending) {
            rounds += 1;
            self.rounds += 1;
            rules(&mut Round {
                store: self,
                pending: &mut pending,
            });
        }
        self.pending = pending;
        rounds
    }

    /// Applies a round's output and advances every active table. True when
    /// some table has a non-empty recent part, so another round is needed.
    fn step(&mut self, pending: &mut Pending) -> bool {
        // Ticket 11 (canon.rs): rewrite `pending.triples` through the canon
        // map here, before sorting, and call `remap` on the tables whose
        // tuples mention a merged term.

        pending.by_object.sort_unstable();
        pending.by_object.dedup();
        for p in std::mem::take(&mut pending.by_object) {
            let t = self.table_index(p);
            let table = &mut self.tables[t];
            if table.os.is_none() {
                let mut os = Variable::new();
                os.insert(Relation::from_vec(table.so.all().iter().map(|(s, o)| (o, s)).collect()));
                table.os = Some(os);
                self.activate(t);
            }
        }

        let triples = &mut pending.triples;
        triples.sort_unstable();
        triples.dedup();
        let mut rest = &triples[..];
        while let Some(&(p, _, _)) = rest.first() {
            let n = rest.partition_point(|t| t.0 == p);
            let (group, tail) = rest.split_at(n);
            rest = tail;
            let t = self.table_index(p);
            let table = &mut self.tables[t];
            table
                .so
                .insert(Relation::from_sorted(group.iter().map(|&(_, s, o)| (s, o)).collect()));
            if let Some(os) = &mut table.os {
                os.insert(Relation::from_vec(group.iter().map(|&(_, s, o)| (o, s)).collect()));
            }
            self.activate(t);
        }
        triples.clear();

        let mut changed = false;
        let active = std::mem::take(&mut self.active);
        for &t in &active {
            let table = &mut self.tables[t];
            let mut new = table.so.changed();
            if let Some(os) = &mut table.os {
                new |= os.changed();
            }
            changed |= new;
            if new {
                self.active.push(t);
            } else {
                self.is_active[t] = false;
            }
        }
        debug_assert!(
            self.tables
                .iter()
                .enumerate()
                .all(|(t, table)| !table.is_active() || self.is_active[t])
        );
        changed
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

    /// Every `(subject, predicate, object)`, table by table.
    pub(crate) fn triples(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.tables().flat_map(|(p, t)| {
            t.so.parts()
                .flat_map(move |part| part.iter().map(move |&(s, o)| (s, p, o)))
        })
    }

    /// Rounds run since the store was made.
    pub(crate) fn rounds(&self) -> usize {
        self.rounds
    }
}

/// What a rule function sees in one round: the store as the last round left
/// it (read-only), and where to put what it derives.
#[derive(Debug)]
pub(crate) struct Round<'a> {
    store: &'a Store,
    pending: &'a mut Pending,
}

impl<'a> Round<'a> {
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
    pub(crate) fn emit(&mut self, s: u32, p: u32, o: u32) {
        self.pending.triples.push((p, s, o));
    }

    /// This round's number in the store's life, from 1.
    pub(crate) fn number(&self) -> usize {
        self.store.rounds
    }
}
