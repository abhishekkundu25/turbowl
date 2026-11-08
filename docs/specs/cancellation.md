# Cancellation: stopping a run or a proof halfway

Status: built 2026-11-08, branch `cancel`. Tickets: `docs/tickets/cancel/`
(01 to 03).

## Problem Statement

Once `Reasoner::reason` or an explanation starts, it runs to the end. An
editor that reasons on every edit (turtle-lsp) can only throw a stale result
away: the stale run keeps a core busy until it finishes, and on a
data-size workspace (claros-l takes seconds) quick edits pile up runs behind
each other. A Python user who presses Ctrl+C during a long `reason()` waits
for it to end. A slow proof (Brick in the complete configuration, ticket 06
of `docs/tickets/explain/`) cannot be stopped either.

## Solution

A **cancel handle**, `Cancel`, that the caller trips from any thread. A run
or a search given one checks it as it goes and stops with
`Error::Cancelled`.

- `Reasoner::reason_cancellable(&mut self, &Cancel) -> Result<Summary, Error>`
  checks the handle before each round. `reason()` runs the same path without
  a handle, and keeps its signature.
- A cancelled run keeps **no partial result**: the reasoner is as loaded and
  not yet reasoned. The base is kept; the closure and the diagnostics are
  empty; the next run is a full one; an explainer gives
  `Error::NotReasoned`. A handle tripped before the call changes nothing.
- `Explainer::explain_cancellable` and
  `Explainer::explain_diagnostic_cancellable` check it between search steps.
  A cancelled search drops its search graph, as a panicked one does, and
  keeps the indexes: the next question is answered as if nothing had been
  cancelled.
- Python gets `turbowl.Cancel`, a `cancel=` argument on `reason`, `explain`
  and `explain_diagnostic`, a `CancelledError`, and Ctrl+C that stops a run
  or a search and raises `KeyboardInterrupt`.

## User Stories

1. As an editor author, I want to stop a reasoning run when the user types again, so that the next run starts at once instead of queueing behind a stale one.
2. As an editor author, I want to trip the stop from another thread than the one reasoning, so that my event loop never blocks on the reasoner.
3. As an editor author, I want a cancelled run to keep no half-finished closure, so that I can never show a partial result as if it were complete.
4. As an editor author, I want the next run after a cancel to give exactly what an uncancelled run gives, so that cancelling is always safe.
5. As an editor author, I want a run to stop within one round of the cancel, so that a cancel frees the core quickly.
6. As an editor author, I want a hover's proof search to stop when the hover moves, so that a slow proof does not delay the next hover.
7. As an editor author, I want the explainer's indexes kept after a cancelled search, so that the next hover does not pay for them again.
8. As an editor author, I want a cancelled call to say it was cancelled, distinct from "no proof", so that I never tell the user a triple has no proof when the search was only stopped.
9. As an editor author, I want one handle per request that stays tripped, so that a late check can never resume work I stopped.
10. As a Python user, I want Ctrl+C to stop a long `reason()`, so that a notebook or a script stays responsive.
11. As a Python user, I want Ctrl+C to stop a long `explain()`, for the same reason.
12. As a Python user, I want a `Cancel` object I can trip from another thread or an async task, so that a Python server can cancel as the Rust one does.
13. As a Python user, I want a cancelled call to raise `CancelledError`, a `TurbowlError`, so that one `except` covers it.
14. As a Rust user who never cancels, I want `reason()` unchanged, so that my code still compiles and runs as fast.
15. As a WebAssembly user, I want cancellation to build for `wasm32`, so that a browser worker can cancel too.
16. As the owner, I want reasoning speed and memory unchanged, so that the scoreboard claim stands.
17. As a maintainer, I want chaos to cancel a run at a random round and then require the next run to equal the reference, so that a wrong recovery is found.
18. As a maintainer, I want chaos to cancel a proof search halfway and then require the next proof to equal the all-cores explainer's, so that a damaged search graph is found.
19. As a maintainer, I want a deterministic way to cancel at the n-th check, so that tests do not depend on timing.

## Implementation Decisions

- **`Cancel`** (public, in the crate root): `Cancel::new()`, `cancel(&self)`,
  `is_cancelled(&self) -> bool`; `Clone` shares one handle; `Send + Sync`;
  `Default`. An `Arc` of an atomic flag, read with relaxed ordering: a run
  stops at the first check after the store is visible, which is all a
  cancel promises. It is never reset.
- **`profile::cancel_after(checks: usize) -> Cancel`**: a handle that trips
  itself at its n-th check (0 is at once). For chaos and the timing tests,
  so that a cancel at a given round needs no thread and no clock. The
  `profile` module is already the interface's place for what only the
  verification crates use.
- **`Error::Cancelled`**, a new variant: "the run was cancelled". The
  Python `error()` arm maps it to `CancelledError`.
- **The engine** gets an additive `Store::run_until(&AtomicBool-like stop,
  rules) -> Option<usize>`: it checks before each round and returns `None`
  when stopped, leaving the store to be dropped. `Store::run` keeps its
  signature and calls it with a stop that never trips. The engine still
  knows no rules and no `Cancel`: it is given a check.
- **The reasoner** checks before the first round, between rounds (through
  the engine), at each restart, and once before its end work (the weights
  of a merged store, `owl:sameAs` of each term to itself, the diagnostics).
  On a stop it replaces the store with a fresh one, clears the inferred
  count, the findings and the diagnostics, and sets the state to `Full`. A
  round is not interrupted: the stop waits for it, which the bench's
  per-round times bound.
- **The explainer** carries the handle in the search's world. The work
  loops (`run`, `run_nested`) and the deepening loop check it; a stopped
  search returns at once. `Explainer` then drops the search graph (and its
  fallback graphs) and keeps the indexes. An index build or a ranking of
  the whole graph is not interrupted.
- **Python**: `reason`, `explain` and `explain_diagnostic` run on a scoped
  worker thread with the GIL released, while the calling thread waits in
  slices of 20 ms and checks Python's signals between them. Ctrl+C trips
  the handle, waits for the worker to stop, and raises `KeyboardInterrupt`;
  a tripped `turbowl.Cancel` raises `CancelledError`. The worker thread is
  given the main thread's stack size. A cancelled `reason` drops the
  explainer's indexes, as any change does.
- **The CLI does not change**: Ctrl+C ends the process, which is the right
  stop for a command.
- **Budgets** (pass/fail):
  - Reasoning times and peak memory on brick-small1, brick-soda-hall, lubm-1
    and lubm-50, default and complete, one thread and all cores: within
    `main`'s spread (the bench, alternating, on a quiet machine).
  - `engine/` changes are additive: `Store::run` and every rule file are
    unchanged.
  - A cancel during a run returns within the longest round of that run plus
    10 ms; during a search, within 10 ms (the timed tests below).
  - After any cancel, the next run's summary, closure in read order and
    diagnostics equal an uncancelled run's (chaos, every iteration).
  - Chaos with the cancels takes at most 10% longer than without.

## Testing Decisions

- Tests use the public interface only: `Cancel`, `reason_cancellable`,
  `explain_cancellable`, `profile::cancel_after`, and the reads. They never
  read the store.
- Unit tests beside the reasoner's (prior art: the incremental-run tests in
  `crates/turbowl/src/reasoner.rs` and `crates/turbowl/tests/`):
  - a tripped handle changes nothing: the state before equals the state
    after, and `reason()` then gives the uncancelled result;
  - a cancel at each check of a multi-round run (k = 1 to rounds) gives
    `Error::Cancelled`, an empty closure and no diagnostics, then a run
    equal to an uncancelled one; the same after an incremental insert and
    after a removal, and with `equality` on through a restart;
  - a cancel from another thread on a long chain returns `Cancelled`
    within the budget.
- Explainer tests in `crates/turbowl/src/explain/tests.rs` (prior art:
  `a_restarted_closure_is_explained_in_time`): a cancel at each check of a
  deep proof gives `Cancelled`; the same question then gives the proof an
  explainer that was never cancelled gives; the cache is still held.
- Chaos (`crates/reference/tests/chaos.rs`): each iteration also runs a
  reasoner with `cancel_after(k)` for a random k from its own `Rng` stream,
  then reasons again and compares with the reference; one explained goal per
  state is first cancelled halfway, then asked again and compared with the
  all-cores explainer. `CHAOS_CANCEL=0` turns this off, to measure it.
- Python (`crates/python/tests/test_cancel.py`, prior art
  `test_explain.py`): `Cancel` tripped before and during a call,
  `CancelledError`'s class, the state after, Ctrl+C through
  `_thread.interrupt_main` from a timer thread.
- WebAssembly: `scripts/check-wasm.sh` builds the core with `Cancel`.

## Out of Scope

- Interrupting a round, an index build or a whole-graph ranking halfway.
- Keeping the previous closure across a cancelled incremental run (it
  would cost a copy of the store on every run).
- Timeouts and deadlines: a caller builds one from a timer that trips the
  handle.
- Cancelling loading, writing or a snapshot.
- Ctrl+C in the CLI (the process ends) and turtle-lsp's own wiring (its
  repository; its handover names the generation counter, which this
  replaces).
- Progress reporting.

## Further Notes

- The owner asked for this on 2026-11-08 and said to assume what fits the
  goal and record it. The decisions taken without the owner are listed in
  the close-out's record.
