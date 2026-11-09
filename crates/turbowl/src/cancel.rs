//! The cancel handle ([`Cancel`]).

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A handle that stops a run ([`Reasoner::reason_cancellable`]) or a proof
/// search ([`Explainer::explain_cancellable`]). Clones share one handle.
/// Once tripped it stays tripped: use one per request.
///
/// Any thread can trip it with [`Cancel::cancel`]. A handle made with
/// [`Cancel::with_check`] also trips itself when its function says so,
/// which needs no second thread: a deadline in a browser, for one.
///
/// A run checks it before each round, and a search between its steps, so a
/// cancel takes effect when the current round or step ends.
///
/// [`Reasoner::reason_cancellable`]: crate::Reasoner::reason_cancellable
/// [`Explainer::explain_cancellable`]: crate::Explainer::explain_cancellable
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<Inner>);

/// The function a handle calls at each check ([`Cancel::with_check`]).
type Check = Box<dyn Fn() -> bool + Send + Sync>;

struct Inner {
    tripped: AtomicBool,
    /// The checks left before the handle trips itself
    /// ([`crate::profile::cancel_after`]); `usize::MAX` when it never does.
    left: AtomicUsize,
    /// Called at each check until the handle trips; true trips it.
    check: Option<Check>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            tripped: AtomicBool::new(false),
            left: AtomicUsize::new(usize::MAX),
            check: None,
        }
    }
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inner")
            .field("tripped", &self.tripped)
            .field("left", &self.left)
            .field("check", &self.check.as_ref().map(|_| "Fn"))
            .finish()
    }
}

impl Cancel {
    /// A handle nobody has tripped.
    pub fn new() -> Self {
        Self::default()
    }

    /// A handle that calls `check` at each of its checks and trips when it
    /// returns true. Once tripped it stays tripped and `check` is not called
    /// again; [`Cancel::cancel`] trips it too. TurbOWL keeps no clock, so a
    /// deadline is the caller's:
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    /// let deadline = Instant::now() + Duration::from_secs(2);
    /// let cancel = turbowl::Cancel::with_check(move || Instant::now() >= deadline);
    /// # assert!(!cancel.is_cancelled());
    /// ```
    ///
    /// The function runs on the thread that reasons or searches, once per
    /// round or step, so it should be cheap.
    pub fn with_check(check: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self(Arc::new(Inner {
            check: Some(Box::new(check)),
            ..Inner::default()
        }))
    }

    /// Trips the handle: every run and search it was given stops at its
    /// next check.
    pub fn cancel(&self) {
        self.0.tripped.store(true, Ordering::Relaxed);
    }

    /// True once [`Cancel::cancel`] was called or the handle tripped itself.
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
        if self.0.check.as_ref().is_some_and(|check| check()) {
            self.cancel();
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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    /// A handle whose function says true at its `at`-th call (0: the first),
    /// and the calls made.
    fn checked(at: usize) -> (Cancel, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let cancel = Cancel::with_check(move || counted.fetch_add(1, Ordering::Relaxed) >= at);
        (cancel, calls)
    }

    #[test]
    fn a_checked_handle_trips_when_its_function_says_so_and_stays_tripped() {
        let (c, calls) = checked(2);
        assert!(!c.check());
        assert!(!c.check());
        assert!(!c.is_cancelled());
        assert!(c.check());
        assert!(c.is_cancelled());
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        assert!(c.check() && c.check());
        assert_eq!(calls.load(Ordering::Relaxed), 3, "not called once tripped");
    }

    #[test]
    fn a_clone_shares_a_checked_handle() {
        let (a, calls) = checked(1);
        let b = a.clone();
        assert!(!a.check());
        assert!(b.check());
        assert!(a.is_cancelled() && a.check());
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn a_checked_handle_can_be_cancelled() {
        let (c, calls) = checked(usize::MAX);
        let clone = c.clone();
        assert!(!c.check());
        clone.cancel();
        assert!(c.is_cancelled() && c.check());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_function_that_never_says_true_never_trips() {
        let (c, calls) = checked(usize::MAX);
        for _ in 0..1000 {
            assert!(!c.check());
        }
        assert!(!c.is_cancelled());
        assert_eq!(calls.load(Ordering::Relaxed), 1000);
        assert!(format!("{c:?}").contains("Cancel"));
    }
}
