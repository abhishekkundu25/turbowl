//! The threads a round may use (ticket 17): [`Workers`], and the few
//! parallel operations the store and the rules need, each with a
//! sequential form.
//!
//! The threads are rayon's, behind the `parallel` feature and only off
//! WebAssembly. Without them every operation here runs on the caller's
//! thread, in the same order, so the results are the same.
//!
//! A round runs in parallel only when its work passes [`PARALLEL_FROM`]
//! tuples ([`Workers::engage`]). Below it, nothing here touches a thread
//! pool: a small incremental change never waits on one, and never starts
//! rayon's global pool.
//!
//! Every operation that returns results returns them in the order of its
//! input, whatever the schedule.

#[cfg(all(feature = "parallel", not(target_family = "wasm")))]
use rayon::prelude::*;
#[cfg(all(feature = "parallel", not(target_family = "wasm")))]
use std::sync::Arc;

/// The smallest round that runs in parallel, in tuples: the round's delta
/// for the rules, its output for the step. Below it, handing work to
/// threads costs more than it saves. Measured by
/// `scaling::parallel::timing_parallel_round_threshold` (its doc has the
/// table): on the 12-core machine of 2026-11-06, a full run with every
/// round parallel was slower than one thread up to 32,768 base triples
/// (0.75 times as fast) and faster from 65,536 (1.2 times), the smallest
/// size from which every measured size gained.
pub(crate) const PARALLEL_FROM: usize = 65_536;

/// The fewest items a chunk gets once a round is parallel.
const GRAIN: usize = 4_096;

/// Where a round's parallel work runs.
#[derive(Debug, Clone, Default)]
pub(crate) struct Workers {
    pool: Pool,
    /// Every round in parallel, however small, and chunks of a few items:
    /// for the tests of thread-count invariance.
    every_round: bool,
    /// Another threshold than [`PARALLEL_FROM`]: for the test that
    /// measures it.
    from: Option<usize>,
}

#[derive(Debug, Clone, Default)]
enum Pool {
    /// The caller's thread only.
    #[default]
    One,
    /// rayon's global pool: every core, or `RAYON_NUM_THREADS`.
    #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
    Global,
    /// A pool of its own, with a set number of threads.
    #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
    Own(Arc<rayon::ThreadPool>),
}

impl Workers {
    /// The caller's thread only.
    pub(crate) fn one() -> Self {
        Self::default()
    }

    /// Every core: rayon's global pool. One thread without the `parallel`
    /// feature.
    pub(crate) fn all() -> Self {
        Self {
            #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
            pool: Pool::Global,
            ..Self::default()
        }
    }

    /// `threads` threads, in a pool of their own (built here). One thread
    /// without the `parallel` feature.
    pub(crate) fn exactly(threads: usize) -> Self {
        #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
        if threads > 1 {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .thread_name(|i| format!("turbowl-{i}"))
                .build()
                .expect("a thread pool");
            return Self {
                pool: Pool::Own(Arc::new(pool)),
                ..Self::default()
            };
        }
        let _ = threads;
        Self::one()
    }

    /// The same workers, for every round however small.
    pub(crate) fn every_round(self) -> Self {
        Self {
            every_round: true,
            ..self
        }
    }

    /// The same workers, with rounds of `size` tuples or more in parallel.
    #[cfg(test)]
    pub(crate) fn from(self, size: usize) -> Self {
        Self {
            from: Some(size),
            ..self
        }
    }

    /// True when work of `size` tuples should run in parallel: it passes
    /// [`PARALLEL_FROM`] and there is more than one thread, or every round
    /// is to. Asks rayon for its thread count only past the threshold, so a
    /// small round never starts the global pool.
    pub(crate) fn engage(&self, size: usize) -> bool {
        self.every_round || (size >= self.from.unwrap_or(PARALLEL_FROM) && self.threads() > 1)
    }

    /// The threads parallel work runs on.
    pub(crate) fn threads(&self) -> usize {
        match &self.pool {
            Pool::One => 1,
            #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
            Pool::Global => rayon::current_num_threads(),
            #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
            Pool::Own(pool) => pool.current_num_threads(),
        }
    }

    /// The size of the chunks `len` items split into: about four chunks a
    /// thread, none under [`GRAIN`] items (one item when every round is
    /// parallel, so that small inputs split too).
    pub(crate) fn chunk(&self, len: usize) -> usize {
        if self.every_round {
            return len.div_ceil(4).max(1);
        }
        len.div_ceil(4 * self.threads()).max(GRAIN)
    }

    /// Runs `op` in this pool.
    fn install<R: Send>(&self, op: impl FnOnce() -> R + Send) -> R {
        match &self.pool {
            #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
            Pool::Own(pool) => pool.install(op),
            _ => op(),
        }
    }

    /// True when the operations run on rayon's threads.
    fn threaded(&self) -> bool {
        !matches!(self.pool, Pool::One)
    }

    /// `items` sorted and deduplicated ([`Workers::sort_dedup_batches`]).
    pub(crate) fn sort_dedup<T: Ord + Copy + Send + Sync + Default>(&self, items: Vec<T>) -> Vec<T> {
        self.sort_dedup_batches(vec![items])
    }

    /// The items of `batches`, sorted and deduplicated. With threads and
    /// many items: joined side by side, sorted by rayon's parallel merge
    /// sort (which gains from sorted runs: the chunks of a split loop come
    /// sorted), and deduplicated in chunks side by side.
    pub(crate) fn sort_dedup_batches<T: Ord + Copy + Send + Sync + Default>(&self, batches: Vec<Vec<T>>) -> Vec<T> {
        let total: usize = batches.iter().map(Vec::len).sum();
        let mut items = self.joined(batches, total);
        #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
        if self.threaded() && total >= 4 * GRAIN {
            return self.install(|| {
                items.par_sort();
                let items = &items;
                let size = items.len().div_ceil(4 * self.threads()).max(GRAIN);
                let kept: Vec<Vec<T>> = (0..items.len().div_ceil(size))
                    .into_par_iter()
                    .map(|c| {
                        let (from, to) = (c * size, ((c + 1) * size).min(items.len()));
                        let mut kept = Vec::with_capacity(to - from);
                        let mut last = from.checked_sub(1).map(|i| items[i]);
                        for &x in &items[from..to] {
                            if last != Some(x) {
                                kept.push(x);
                                last = Some(x);
                            }
                        }
                        kept
                    })
                    .collect();
                concat(kept)
            });
        }
        items.sort_unstable();
        items.dedup();
        items
    }

    /// The batches, `total` items, one after another in one vector.
    fn joined<T: Copy + Send + Sync + Default>(&self, mut batches: Vec<Vec<T>>, total: usize) -> Vec<T> {
        if batches.len() == 1 {
            return batches.pop().expect("one batch");
        }
        #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
        if self.threaded() && total >= 4 * GRAIN {
            return self.install(|| concat(batches));
        }
        // On one thread, into the largest batch's allocation.
        let largest = (0..batches.len()).max_by_key(|&i| batches[i].len()).unwrap_or(0);
        let mut all = if batches.is_empty() {
            Vec::new()
        } else {
            batches.swap_remove(largest)
        };
        all.reserve(total - all.len());
        for batch in batches {
            all.extend_from_slice(&batch);
        }
        all
    }

    /// The union of two sorted, duplicate-free vectors. With threads and
    /// many items, cut into ranges merged side by side.
    pub(crate) fn merge<T: Ord + Copy + Send + Sync + Default>(&self, a: Vec<T>, b: Vec<T>) -> Vec<T> {
        #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
        if self.threaded() && a.len() + b.len() >= 4 * GRAIN {
            return self.install(|| parallel_merge(&a, &b, self.threads()));
        }
        super::relation::Relation::from_sorted(a)
            .merge(super::relation::Relation::from_sorted(b))
            .into_vec()
    }

    /// The items of sorted `fresh` that sorted `known` does not hold. With
    /// threads and many items, `fresh` is filtered in chunks side by side.
    pub(crate) fn absent<T: Ord + Copy + Send + Sync + Default>(&self, fresh: Vec<T>, known: &[T]) -> Vec<T> {
        #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
        if self.threaded() && fresh.len() >= 4 * GRAIN {
            return self.install(|| {
                let size = fresh.len().div_ceil(4 * self.threads()).max(GRAIN);
                let kept: Vec<Vec<T>> = fresh.par_chunks(size).map(|chunk| absent_from(chunk, known)).collect();
                concat(kept)
            });
        }
        absent_from(&fresh, known)
    }

    /// Calls `f` on every item, the items spread over the threads.
    pub(crate) fn for_each<T: Send>(&self, items: &mut [T], f: impl Fn(&mut T) + Sync + Send) {
        #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
        if self.threaded() {
            return self.install(|| items.par_iter_mut().with_max_len(1).for_each(f));
        }
        items.iter_mut().for_each(f);
    }

    /// `f` of each chunk of `items` ([`Workers::chunk`]), in order.
    pub(crate) fn chunks<T: Sync, R: Send>(&self, items: &[T], f: impl Fn(&[T]) -> R + Sync + Send) -> Vec<R> {
        self.chunks_by(items, |_, _| false, f)
    }

    /// [`Workers::chunks`], with no cut between two neighbours that `same`
    /// holds of: a cut moves forward past them.
    pub(crate) fn chunks_by<T: Sync, R: Send>(
        &self,
        items: &[T],
        same: impl Fn(&T, &T) -> bool,
        f: impl Fn(&[T]) -> R + Sync + Send,
    ) -> Vec<R> {
        let size = self.chunk(items.len());
        let mut cuts = vec![0];
        let mut at = size;
        while at < items.len() {
            while at < items.len() && same(&items[at - 1], &items[at]) {
                at += 1;
            }
            if at < items.len() {
                cuts.push(at);
            }
            at += size;
        }
        cuts.push(items.len());
        #[cfg(all(feature = "parallel", not(target_family = "wasm")))]
        if self.threaded() {
            return self.install(|| {
                cuts.par_windows(2)
                    .with_max_len(1)
                    .map(|w| f(&items[w[0]..w[1]]))
                    .collect()
            });
        }
        cuts.windows(2).map(|w| f(&items[w[0]..w[1]])).collect()
    }
}

/// The items of sorted `fresh` not in sorted `known`: galloping when
/// `known` is much longer, stepping otherwise.
pub(crate) fn absent_from<T: Ord + Copy>(fresh: &[T], known: &[T]) -> Vec<T> {
    let (Some(first), Some(last)) = (fresh.first(), fresh.last()) else {
        return Vec::new();
    };
    let start = known.partition_point(|k| k < first);
    let end = known.partition_point(|k| k <= last);
    let mut known = &known[start..end.max(start)];
    let mut kept = Vec::with_capacity(fresh.len());
    if known.len() > 4 * fresh.len() {
        for x in fresh {
            known = super::relation::gallop(known, |y| y < x);
            if known.first() != Some(x) {
                kept.push(*x);
            }
        }
    } else {
        for x in fresh {
            while known.first().is_some_and(|y| y < x) {
                known = &known[1..];
            }
            if known.first() != Some(x) {
                kept.push(*x);
            }
        }
    }
    kept
}

/// The vectors one after another, copied side by side into one.
#[cfg(all(feature = "parallel", not(target_family = "wasm")))]
fn concat<T: Copy + Send + Sync + Default>(parts: Vec<Vec<T>>) -> Vec<T> {
    let total = parts.iter().map(Vec::len).sum();
    let mut out: Vec<T> = Vec::with_capacity(total);
    out.par_extend(rayon::iter::repeat_n(T::default(), total));
    let mut rest = &mut out[..];
    let mut slots = Vec::with_capacity(parts.len());
    for part in &parts {
        let (slot, tail) = std::mem::take(&mut rest).split_at_mut(part.len());
        slots.push(slot);
        rest = tail;
    }
    slots
        .into_par_iter()
        .zip(parts.par_iter())
        .for_each(|(slot, part)| slot.copy_from_slice(part));
    out
}

/// The union of sorted, duplicate-free `a` and `b`, in pieces merged side
/// by side: `a` is cut evenly, and `b` where each cut's value falls, so an
/// item in both lands in one piece.
#[cfg(all(feature = "parallel", not(target_family = "wasm")))]
fn parallel_merge<T: Ord + Copy + Send + Sync + Default>(a: &[T], b: &[T], threads: usize) -> Vec<T> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let pieces = (4 * threads).clamp(1, long.len().div_ceil(GRAIN));
    let mut cuts: Vec<(usize, usize)> = (0..=pieces)
        .map(|i| {
            let at = i * long.len() / pieces;
            let other = if at == 0 {
                0
            } else if at == long.len() {
                short.len()
            } else {
                short.partition_point(|y| *y < long[at])
            };
            (at, other)
        })
        .collect();
    cuts.dedup();
    let merged: Vec<Vec<T>> = cuts
        .par_windows(2)
        .map(|w| merge_slices(&long[w[0].0..w[1].0], &short[w[0].1..w[1].1]))
        .collect();
    concat(merged)
}

/// The union of two sorted, duplicate-free slices.
#[cfg(all(feature = "parallel", not(target_family = "wasm")))]
fn merge_slices<T: Ord + Copy>(mut a: &[T], mut b: &[T]) -> Vec<T> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    while let (Some(&x), Some(&y)) = (a.first(), b.first()) {
        match x.cmp(&y) {
            std::cmp::Ordering::Less => {
                out.push(x);
                a = &a[1..];
            }
            std::cmp::Ordering::Greater => {
                out.push(y);
                b = &b[1..];
            }
            std::cmp::Ordering::Equal => {
                out.push(x);
                a = &a[1..];
                b = &b[1..];
            }
        }
    }
    out.extend_from_slice(a);
    out.extend_from_slice(b);
    out
}
