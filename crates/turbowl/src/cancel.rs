//! The cancel handle ([`Cancel`]).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A handle that stops a run ([`Reasoner::reason_cancellable`]) or a proof
/// search ([`Explainer::explain_cancellable`]) from any thread. Clones share
/// one handle. Once tripped it stays tripped: use one per request.
///
/// A run checks it before each round, and a search between its steps, so a
/// cancel takes effect when the current round or step ends.
///
/// [`Reasoner::reason_cancellable`]: crate::Reasoner::reason_cancellable
/// [`Explainer::explain_cancellable`]: crate::Explainer::explain_cancellable
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    tripped: AtomicBool,
    /// The checks left before the handle trips itself
    /// ([`crate::profile::cancel_after`]); `usize::MAX` when it never does.
    left: AtomicUsize,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            tripped: AtomicBool::new(false),
            left: AtomicUsize::new(usize::MAX),
        }
    }
}

impl Cancel {
    /// A handle nobody has tripped.
    pub fn new() -> Self {
        Self::default()
    }

    /// Trips the handle: every run and search it was given stops at its
    /// next check.
    pub fn cancel(&self) {
        self.0.tripped.store(true, Ordering::Relaxed);
    }

    /// True once [`Cancel::cancel`] was called (or the handle tripped itself).
    pub fn is_cancelled(&self) -> bool {
        self.0.tripped.load(Ordering::Relaxed)
    }

    /// A handle that trips itself at its `checks`-th check (0: the first).
    pub(crate) fn after(checks: usize) -> Self {
        let cancel = Self::new();
        cancel.0.left.store(checks, Ordering::Relaxed);
        cancel
    }

    /// The check a run or a search makes: true when it must stop.
    pub(crate) fn check(&self) -> bool {
        if self.is_cancelled() {
            return true;
        }
        if self.0.left.load(Ordering::Relaxed) == usize::MAX {
            return false;
        }
        let counted = self
            .0
            .left
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |left| left.checked_sub(1));
        if counted.is_err() {
            self.cancel();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::Cancel;

    #[test]
    fn a_clone_shares_the_handle() {
        let a = Cancel::new();
        let b = a.clone();
        assert!(!b.check());
        a.cancel();
        assert!(b.is_cancelled() && b.check());
    }

    #[test]
    fn a_counted_handle_trips_at_its_check_and_stays_tripped() {
        let c = Cancel::after(2);
        assert!(!c.check());
        assert!(!c.check());
        assert!(!c.is_cancelled());
        assert!(c.check());
        assert!(c.is_cancelled() && c.check());
        assert!(Cancel::after(0).check());
    }
}
