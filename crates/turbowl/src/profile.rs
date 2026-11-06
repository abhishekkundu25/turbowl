//! What the last run did and what the reasoner holds: the counters of the
//! improvement loop's experiment P0 (`docs/specs/experiments.md`), read
//! with [`Reasoner::profile`](crate::Reasoner::profile).
//!
//! # Counting costs nothing that matters
//!
//! A run keeps the counts as it goes: one integer add per family and round
//! (its emitted tuples, read from the round buffer's counter before and
//! after the family runs), and per table and step (its distinct pairs and
//! its new tuples). It reads the clock once per family task and round, and
//! a few times per step: in a parallel step, a few times per table. The
//! [`Memory`] breakdown is not kept: [`Reasoner::profile`] computes it by
//! walking the structures, counting each allocation's capacity (what the
//! process holds), not its length. The walk visits each table's batches,
//! never its tuples.
//!
//! # Times
//!
//! Times are wall-clock nanoseconds. A family's time and a step's sort and
//! change times are each task's own: in a parallel round or step, tasks
//! run at once, so their sum can pass the round's or step's wall time
//! ([`Round::ns`], [`Step::ns`]). On WebAssembly, whose standard library has
//! no clock, every time is 0.
//!
//! [`Reasoner::profile`]: crate::Reasoner::profile

use std::mem::size_of;

/// What the last run did ([`Reasoner::reason`](crate::Reasoner::reason)),
/// round by round, and the bytes the reasoner holds now.
///
/// A run that does nothing (no change since the last) has no rounds and no
/// steps. A run that restarts (a merge changed a rule constant) counts the
/// rounds and steps of every attempt, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Profile {
    /// The rule families that ran (those the switches leave on), in table
    /// order: the order of [`Round::emitted`] and [`Round::family_ns`].
    pub families: Vec<&'static str>,
    /// One per round, in order.
    pub rounds: Vec<Round>,
    /// One per step, in order. A step applies what came before it (the
    /// run's input, or a round's output) and ends with the next round's
    /// delta. Each attempt of a run has one step more than rounds: its last
    /// step finds nothing new, or stops the attempt for a restart.
    pub steps: Vec<Step>,
    /// The bytes the reasoner holds, computed when the profile is read.
    pub memory: Memory,
}

/// One round: every family that is on, run once over the store.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Round {
    /// The tuples new in the store when the round starts (each table's
    /// recent part, by subject): the first round's input is the run's, a
    /// later round's is what the step before it found new. A tuple that an
    /// `owl:sameAs` merge rewrote is new again.
    pub delta: usize,
    /// The tuples each family emitted, duplicates and known tuples
    /// included (its rule instances), in [`Profile::families`] order.
    pub emitted: Vec<usize>,
    /// Each family's own time, in [`Profile::families`] order.
    pub family_ns: Vec<u64>,
    /// The round's wall time: counting its delta, and every family.
    pub ns: u64,
}

/// One step: the store sorts each table's new pairs, finds which are new,
/// and advances its tables to the next round.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Step {
    /// The step's wall time: the sorts and changes below, the merges of
    /// `owl:sameAs` and the rewriting they cause, new object indexes, and
    /// the bookkeeping.
    pub ns: u64,
    /// Sorting each table's pairs: the ones the step got, deduplicated,
    /// and their flipped copies for the object indexes, which are distinct.
    pub sort_ns: u64,
    /// Finding what changed: merging each table's last delta into its
    /// stable batches, and dropping the known tuples from its new pairs
    /// (the change of each index).
    pub changed_ns: u64,
    /// The distinct pairs the step got, by subject, summed over the tables:
    /// what the round emitted (or the run's input), duplicates removed.
    pub pairs: usize,
    /// The tuples among them that were new, by subject: the next round's
    /// [`Round::delta`].
    pub new: usize,
}

/// The bytes the reasoner holds, by part: each allocation's capacity, as
/// the process holds it (the allocator's own overhead is not counted).
/// [`Memory::total`] is the sum of the parts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Memory {
    /// The dictionary's arena: every term's text.
    pub dictionary_text: usize,
    /// The dictionary's entries: one per term, where its text is and what
    /// kind of term it is.
    pub dictionary_entries: usize,
    /// The dictionary's hash slots, from text to term.
    pub dictionary_slots: usize,
    /// The base: the loaded and inserted triples, as sorted pairs by
    /// predicate, and those added since its last merge.
    pub base: usize,
    /// The store's `rdf:type` table (the predicate the canon makes
    /// `rdf:type` stand for).
    pub store_type: Parts,
    /// Every other predicate's table, summed.
    pub store_other: Parts,
    /// The store's own structures: its list of tables, its index from
    /// predicate to table, and the triples queued for the next run.
    pub store_rest: usize,
    /// The five largest tables, largest first: each predicate's term
    /// (`<iri>`, or another term's N-Triples form) and its parts. Part of
    /// [`Memory::store_type`] and [`Memory::store_other`], not counted
    /// again.
    pub largest: Vec<(String, Parts)>,
    /// The canon: which terms `owl:sameAs` made equal.
    pub canon: usize,
    /// The findings, as the rules made them (in term ids).
    pub findings: usize,
    /// Their public form, the diagnostics: messages and terms as text.
    pub diagnostics: usize,
    /// Everything else: the representatives eq-ref adds on read, the keys
    /// of the documents loaded, and the counters.
    pub other: usize,
}

/// The bytes of one table, or of tables summed: its `(subject, object)`
/// pairs, and its `(object, subject)` index when a rule asked for one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Parts {
    /// The tuples the table holds (each once, by subject).
    pub tuples: usize,
    /// The pairs earlier rounds saw, in batches.
    pub stable: usize,
    /// The pairs the last round made new, and any queued for the next.
    pub recent: usize,
    /// The object index: its stable and recent pairs.
    pub object_index: usize,
}

impl Parts {
    /// The table's bytes: its pairs and its object index.
    pub fn bytes(&self) -> usize {
        self.stable + self.recent + self.object_index
    }

    pub(crate) fn add(&mut self, other: &Parts) {
        self.tuples += other.tuples;
        self.stable += other.stable;
        self.recent += other.recent;
        self.object_index += other.object_index;
    }
}

impl Memory {
    /// The dictionary: its text, entries and slots.
    pub fn dictionary(&self) -> usize {
        self.dictionary_text + self.dictionary_entries + self.dictionary_slots
    }

    /// The store: its tables and its own structures.
    pub fn store(&self) -> usize {
        self.store_type.bytes() + self.store_other.bytes() + self.store_rest
    }

    /// Every part: the dictionary, the base, the store, the canon, the
    /// findings, the diagnostics and the rest.
    pub fn total(&self) -> usize {
        self.dictionary() + self.base + self.store() + self.canon + self.findings + self.diagnostics + self.other
    }
}

/// A clock reading, for the counters. On WebAssembly, whose standard
/// library has no clock (reading it panics), every time is 0.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Clock(#[cfg(not(target_family = "wasm"))] std::time::Instant);

impl Clock {
    /// Now.
    #[inline]
    pub(crate) fn now() -> Self {
        #[cfg(not(target_family = "wasm"))]
        return Self(std::time::Instant::now());
        #[cfg(target_family = "wasm")]
        return Self();
    }

    /// Nanoseconds from this reading to `later`.
    #[inline]
    pub(crate) fn ns_to(self, later: Clock) -> u64 {
        #[cfg(not(target_family = "wasm"))]
        return u64::try_from(later.0.saturating_duration_since(self.0).as_nanos()).unwrap_or(u64::MAX);
        #[cfg(target_family = "wasm")]
        {
            let _ = later;
            0
        }
    }

    /// Nanoseconds since this reading.
    #[inline]
    pub(crate) fn elapsed_ns(self) -> u64 {
        self.ns_to(Clock::now())
    }
}

/// The bytes a vector of `T` holds: its capacity.
pub(crate) fn vec_bytes<T>(v: &Vec<T>) -> usize {
    v.capacity() * size_of::<T>()
}

/// The bytes a hash table (the standard one, or `rustc-hash`'s, both
/// hashbrown's tables) of `capacity` entries of `entry` bytes holds: its
/// buckets, a control byte per bucket and one group of control bytes more.
/// The buckets are a power of two, at most 7/8 full past 8 of them.
pub(crate) fn hash_bytes(capacity: usize, entry: usize) -> usize {
    if capacity == 0 {
        return 0;
    }
    let buckets = if capacity < 8 {
        (capacity + 1).next_power_of_two()
    } else {
        (capacity * 8 / 7).next_power_of_two()
    };
    buckets * (entry + 1) + 16
}

/// The bytes the diagnostics hold: the list, each message, each list of
/// terms, and each term's text (its length: `oxrdf` does not give the
/// capacity).
pub(crate) fn diagnostic_bytes(diagnostics: &Vec<crate::Diagnostic>) -> usize {
    use oxrdf::Term;
    let term = |t: &Term| match t {
        Term::NamedNode(n) => n.as_str().len(),
        Term::BlankNode(b) => b.as_str().len(),
        Term::Literal(l) => {
            let tagged = l.language().map_or(0, str::len);
            let typed = if tagged == 0 && l.datatype() != oxrdf::vocab::xsd::STRING {
                l.datatype().as_str().len()
            } else {
                0
            };
            l.value().len() + tagged + typed
        }
        #[allow(unreachable_patterns)]
        _ => 0,
    };
    vec_bytes(diagnostics)
        + diagnostics
            .iter()
            .map(|d| d.message.capacity() + vec_bytes(&d.terms) + d.terms.iter().map(term).sum::<usize>())
            .sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The estimate matches the buckets the tables allocate: the capacity
    /// they report is the buckets' 7/8 (or one less than the buckets, below
    /// 8), so the estimate gives the power of two back.
    #[test]
    fn hash_bytes_counts_the_buckets() {
        for n in [1, 3, 4, 7, 8, 13, 14, 15, 100, 1000, 100_000] {
            let set: rustc_hash::FxHashSet<u64> = (0..n).collect();
            let cap = set.capacity();
            let buckets = (hash_bytes(cap, 8) - 16) / 9;
            assert!(buckets.is_power_of_two(), "{n}: {buckets}");
            assert!(
                buckets > cap && cap >= buckets * 7 / 8 - 1,
                "{n}: capacity {cap}, buckets {buckets}"
            );
        }
        assert_eq!(hash_bytes(0, 8), 0);
    }

    #[test]
    fn the_total_is_the_sum_of_the_parts() {
        let parts = |n| Parts {
            tuples: n,
            stable: 8 * n,
            recent: n,
            object_index: 2 * n,
        };
        let m = Memory {
            dictionary_text: 1,
            dictionary_entries: 2,
            dictionary_slots: 4,
            base: 8,
            store_type: parts(16),
            store_other: parts(32),
            store_rest: 64,
            largest: vec![("<http://example.org/p>".into(), parts(32))],
            canon: 128,
            findings: 256,
            diagnostics: 512,
            other: 1024,
        };
        assert_eq!(m.dictionary(), 7);
        assert_eq!(m.store(), 11 * 16 + 11 * 32 + 64);
        assert_eq!(m.total(), 7 + 8 + m.store() + 128 + 256 + 512 + 1024);
    }

    #[test]
    fn the_clock_runs_forward() {
        let start = Clock::now();
        let later = Clock::now();
        assert_eq!(later.ns_to(start), 0, "a reading before is no time");
        assert!(start.elapsed_ns() >= start.ns_to(later));
    }
}
