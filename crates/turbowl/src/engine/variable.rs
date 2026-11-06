//! `Variable`: a relation that grows round by round, split into what earlier
//! rounds saw (`stable`), what the last round added (`recent`, the delta) and
//! what this round derived (`to_add`). `View`: a read-only union of sorted
//! parts, which joins and leapers take.

use super::parallel::Workers;
use super::relation::{Relation, fit, gallop};

/// A relation evaluated semi-naively.
///
/// Invariants: the stable batches and `recent` are pairwise disjoint; each
/// tuple is in `recent` for exactly one round and in `stable` after it. That
/// is what makes "join the recent part of one side with all of the other"
/// derive every result once over the whole run.
#[derive(Debug)]
pub(crate) struct Variable<T> {
    /// Batches seen by earlier rounds, sizes at least doubling towards the
    /// front (datafrog's tiering): O(log n) batches, each tuple merged
    /// O(log n) times over a run.
    stable: Vec<Relation<T>>,
    /// The delta: tuples new in the last `changed`.
    recent: Relation<T>,
    /// Derived this round, not yet visible to joins.
    to_add: Vec<Relation<T>>,
}

impl<T> Default for Variable<T> {
    fn default() -> Self {
        Self {
            stable: Vec::new(),
            recent: Relation::default(),
            to_add: Vec::new(),
        }
    }
}

impl<T: Ord + Copy> Variable<T> {
    /// An empty variable.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A variable whose stable batches are `stable`, with nothing recent or
    /// queued, as a run leaves it: a loaded snapshot (`snapshot.rs`), which
    /// checks the invariants first (each batch non-empty, the batches
    /// disjoint, each more than twice as long as the next).
    pub(crate) fn from_stable(stable: Vec<Relation<T>>) -> Self {
        debug_assert!(stable.iter().all(|b| !b.is_empty()));
        debug_assert!(stable.windows(2).all(|w| w[0].len() > 2 * w[1].len()));
        Self {
            stable,
            ..Self::default()
        }
    }

    /// Queues a batch for the next `changed`. Empty batches are dropped.
    pub(crate) fn insert(&mut self, batch: Relation<T>) {
        if !batch.is_empty() {
            self.to_add.push(batch);
        }
    }

    /// Queues unsorted tuples for the next `changed`.
    pub(crate) fn extend(&mut self, tuples: impl IntoIterator<Item = T>) {
        self.insert(Relation::from_iter(tuples));
    }

    /// Ends a round: `recent` joins `stable`, and what was queued, minus what
    /// is already known, becomes the new `recent`. True when it is non-empty,
    /// so a loop over every variable stops when all return false. Every part
    /// it leaves holds at most an eighth of its length in spare room, and an
    /// empty `recent` none ([`fit`]).
    pub(crate) fn changed(&mut self) -> bool {
        self.end_round(Relation::merge, Relation::from_vec, |mut fresh, known| {
            retain_absent(&mut fresh, known);
            fresh
        })
    }

    /// [`Variable::changed`], with its large merges, sorts and filters on
    /// `workers` when given.
    pub(crate) fn changed_on(&mut self, workers: Option<&Workers>) -> bool
    where
        T: Send + Sync + Default,
    {
        let Some(w) = workers else {
            return self.changed();
        };
        self.end_round(
            |a, b| Relation::from_sorted(w.merge(a.into_vec(), b.into_vec())),
            |all| Relation::from_sorted(w.sort_dedup(all)),
            |fresh, known| w.absent(fresh, known),
        )
    }

    /// [`Variable::changed`] with its three operations given: the union of
    /// two relations, a relation from unsorted tuples, and the tuples of a
    /// sorted vector that a sorted slice does not hold.
    fn end_round(
        &mut self,
        union: impl Fn(Relation<T>, Relation<T>) -> Relation<T>,
        from_vec: impl FnOnce(Vec<T>) -> Relation<T>,
        mut absent: impl FnMut(Vec<T>, &[T]) -> Vec<T>,
    ) -> bool {
        if !self.recent.is_empty() {
            let mut batch = std::mem::take(&mut self.recent);
            while self.stable.last().is_some_and(|last| last.len() <= 2 * batch.len()) {
                let last = self.stable.pop().expect("checked by the loop condition");
                batch = union(last, batch);
            }
            self.stable.push(batch);
        }

        let mut fresh = match self.to_add.len() {
            0 => return false,
            1 => self.to_add.pop().expect("one batch"),
            2 => {
                let b = self.to_add.pop().expect("two batches");
                let a = self.to_add.pop().expect("two batches");
                union(a, b)
            }
            _ => {
                // Many batches: one sort beats a chain of merges.
                let total: usize = self.to_add.iter().map(|b| b.len()).sum();
                let largest = (0..self.to_add.len())
                    .max_by_key(|&i| self.to_add[i].len())
                    .expect("non-empty");
                let mut all = self.to_add.swap_remove(largest).into_vec();
                all.reserve_exact(total - all.len());
                for batch in self.to_add.drain(..) {
                    all.extend_from_slice(&batch);
                }
                from_vec(all)
            }
        }
        .into_vec();

        for batch in &self.stable {
            let known: &[T] = batch;
            // A tier wholly below or above the fresh tuples holds none of
            // them: skip it in O(1). Tuples that grow round by round (a
            // chain's next node) then cost O(tiers) a round, not O(log² n).
            let (Some(first), Some(last)) = (fresh.first(), fresh.last()) else {
                break;
            };
            if known.last().is_some_and(|k| k < first) || known.first().is_some_and(|k| k > last) {
                continue;
            }
            fresh = absent(fresh, known);
            if fresh.is_empty() {
                break;
            }
        }
        // The filter leaves the room of the known tuples it dropped, all of
        // it when nothing is new.
        fit(&mut fresh);
        self.recent = Relation::from_sorted(fresh);
        !self.recent.is_empty()
    }

    /// The tuples new in the last round.
    pub(crate) fn recent(&self) -> &Relation<T> {
        &self.recent
    }

    /// The batches earlier rounds saw.
    pub(crate) fn stable(&self) -> &[Relation<T>] {
        &self.stable
    }

    /// Everything joins see: the stable batches and `recent`.
    pub(crate) fn all(&self) -> View<'_, T> {
        View {
            parts: self.parts().collect(),
        }
    }

    /// The non-empty sorted parts joins see: stable batches, then `recent`.
    pub(crate) fn parts(&self) -> impl Iterator<Item = &[T]> {
        self.stable
            .iter()
            .map(|b| &b[..])
            .chain(Some(&self.recent[..]).filter(|r| !r.is_empty()))
    }

    /// Only the stable batches, as a view (for the "earlier relations"
    /// side of an n-way semi-naive rule).
    pub(crate) fn stable_view(&self) -> View<'_, T> {
        View {
            parts: self.stable.iter().map(|b| &b[..]).collect(),
        }
    }

    /// The bytes the variable holds: its stable batches (and their list),
    /// and its recent part with what is queued. Capacities, not lengths.
    pub(crate) fn bytes(&self) -> (usize, usize) {
        let stable = self.stable.capacity() * std::mem::size_of::<Relation<T>>()
            + self.stable.iter().map(Relation::bytes).sum::<usize>();
        let recent = self.recent.bytes()
            + self.to_add.capacity() * std::mem::size_of::<Relation<T>>()
            + self.to_add.iter().map(Relation::bytes).sum::<usize>();
        (stable, recent)
    }

    /// Tuples joins see (`to_add` not counted).
    pub(crate) fn len(&self) -> usize {
        self.stable.iter().map(|b| b.len()).sum::<usize>() + self.recent.len()
    }

    /// True when joins see no tuple.
    pub(crate) fn is_empty(&self) -> bool {
        self.recent.is_empty() && self.stable.is_empty()
    }

    /// True when joins see `x`.
    pub(crate) fn contains(&self, x: &T) -> bool {
        self.recent.contains(x) || self.stable.iter().any(|b| b.contains(x))
    }

    /// True when a round is pending: queued tuples, or a `recent` part that
    /// the next `changed` must move to `stable`.
    pub(crate) fn is_active(&self) -> bool {
        !self.recent.is_empty() || !self.to_add.is_empty()
    }

    /// Rewrites every tuple through `f` (the canonicalisation step of ticket
    /// 11). A tuple that `f` changes leaves `stable`/`recent` and its image is
    /// queued in `to_add`, so it is a delta again next round and every rule
    /// rejoins it. Returns how many tuples moved. Call between rounds.
    pub(crate) fn remap(&mut self, mut f: impl FnMut(T) -> T) -> usize {
        let mut moved = Vec::new();
        let mut take = |batch: &mut Relation<T>| {
            if batch.iter().all(|&t| f(t) == t) {
                return;
            }
            let mut kept = Vec::with_capacity(batch.len());
            for &t in batch.iter() {
                let image = f(t);
                if image == t {
                    kept.push(t);
                } else {
                    moved.push(image);
                }
            }
            fit(&mut kept);
            *batch = Relation::from_sorted(kept);
        };
        for batch in &mut self.stable {
            take(batch);
        }
        take(&mut self.recent);
        for batch in &mut self.to_add {
            take(batch);
        }
        self.stable.retain(|b| !b.is_empty());
        // Removing tuples can break the size tiering; re-merge from the back.
        let mut tiers: Vec<Relation<T>> = Vec::with_capacity(self.stable.len());
        for batch in self.stable.drain(..) {
            let mut batch = batch;
            while tiers.last().is_some_and(|last| last.len() <= 2 * batch.len()) {
                batch = tiers.pop().expect("checked").merge(batch);
            }
            tiers.push(batch);
        }
        self.stable = tiers;
        let count = moved.len();
        self.extend(moved);
        count
    }

    /// Every tuple, after running to a fixpoint (`to_add` must be empty).
    pub(crate) fn complete(mut self) -> Relation<T> {
        debug_assert!(self.to_add.is_empty(), "complete: tuples still queued");
        let mut result = std::mem::take(&mut self.recent);
        while let Some(batch) = self.stable.pop() {
            result = batch.merge(result);
        }
        result
    }
}

/// Keeps the items of sorted `fresh` that sorted `known` does not hold, in
/// place: galloping when `known` is much longer, stepping otherwise.
fn retain_absent<T: Ord + Copy>(fresh: &mut Vec<T>, mut known: &[T]) {
    if known.len() > 4 * fresh.len() {
        fresh.retain(|x| {
            known = gallop(known, |y| y < x);
            known.first() != Some(x)
        });
    } else {
        fresh.retain(|x| {
            while known.first().is_some_and(|y| y < x) {
                known = &known[1..];
            }
            known.first() != Some(x)
        });
    }
}

/// A read-only union of sorted parts: a variable's batches, or one relation.
/// Parts of a variable are disjoint; a view made of overlapping parts is
/// allowed but counts a shared tuple once per part.
#[derive(Debug, Clone)]
pub(crate) struct View<'a, T> {
    parts: Vec<&'a [T]>,
}

impl<'a, T: Ord + Copy> View<'a, T> {
    /// A view of one sorted slice.
    pub(crate) fn one(part: &'a [T]) -> Self {
        Self { parts: vec![part] }
    }

    /// The sorted parts.
    pub(crate) fn parts(&self) -> &[&'a [T]] {
        &self.parts
    }

    /// Tuples in all parts.
    pub(crate) fn len(&self) -> usize {
        self.parts.iter().map(|p| p.len()).sum()
    }

    /// True when no part has a tuple.
    pub(crate) fn is_empty(&self) -> bool {
        self.parts.iter().all(|p| p.is_empty())
    }

    /// True when a part holds `x`.
    pub(crate) fn contains(&self, x: &T) -> bool {
        self.parts.iter().any(|p| p.binary_search(x).is_ok())
    }

    /// Every tuple, part by part (sorted within a part only).
    pub(crate) fn iter(&self) -> impl Iterator<Item = T> + '_ {
        self.parts.iter().flat_map(|p| p.iter().copied())
    }
}

impl<'a, T: Ord + Copy> From<&'a Relation<T>> for View<'a, T> {
    fn from(relation: &'a Relation<T>) -> Self {
        Self::one(relation)
    }
}

impl<'a, K: Ord + Copy, V: Ord + Copy> View<'a, (K, V)> {
    /// Calls `f` with each value whose key is `key`, part by part.
    pub(crate) fn for_each_value(&self, key: K, mut f: impl FnMut(V)) {
        for part in &self.parts {
            let start = part.partition_point(|t| t.0 < key);
            for t in part[start..].iter().take_while(|t| t.0 == key) {
                f(t.1);
            }
        }
    }
}
