//! [`Threads`]: how many threads a run may use (ticket 17).

use crate::engine::Workers;

/// How many threads [`Reasoner::reason`](crate::Reasoner::reason) may use,
/// set with [`Reasoner::set_threads`](crate::Reasoner::set_threads).
///
/// The default, [`Threads::ALL`], runs a round on every core when its work
/// passes a measured size (65,536 tuples); a smaller round, such as the
/// rounds after a small change, runs on the caller's thread and never waits
/// on a thread pool. [`Threads::ONE`] runs every round on the caller's
/// thread. The closure, the diagnostics, the [`Summary`](crate::Summary) and
/// the order of every read are the same whatever the setting.
///
/// The threads are rayon's global pool, whose size `RAYON_NUM_THREADS`
/// sets. Without the `parallel` feature, and on WebAssembly, every setting
/// runs on one thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Threads {
    /// `None` for every core.
    count: Option<usize>,
    every_round: bool,
}

impl Threads {
    /// One thread: every round runs on the caller's thread.
    pub const ONE: Threads = Threads {
        count: Some(1),
        every_round: false,
    };

    /// Every core, for the rounds large enough to gain from it (the
    /// default).
    pub const ALL: Threads = Threads {
        count: None,
        every_round: false,
    };

    /// The same threads for every round, however small, with its work
    /// split into small chunks: for tests that check that the thread count
    /// changes nothing, on inputs too small to reach the threshold. Slower
    /// than the default on small rounds.
    pub const fn every_round(self) -> Threads {
        Threads {
            every_round: true,
            ..self
        }
    }

    /// Exactly `threads` threads, in a pool of their own.
    #[cfg(test)]
    pub(crate) const fn exactly(threads: usize) -> Threads {
        Threads {
            count: Some(threads),
            every_round: false,
        }
    }

    /// Where the rounds' parallel work runs.
    pub(crate) fn workers(self) -> Workers {
        let workers = match self.count {
            None => Workers::all(),
            Some(1) => Workers::one(),
            Some(n) => Workers::exactly(n),
        };
        if self.every_round {
            workers.every_round()
        } else {
            workers
        }
    }
}

impl Default for Threads {
    fn default() -> Self {
        Self::ALL
    }
}
