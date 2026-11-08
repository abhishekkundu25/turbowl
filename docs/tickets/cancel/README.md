# Tickets: cancellation

Spec: `docs/specs/cancellation.md`. Lock files are in **bold**.

| # | Ticket | Status | Blocked by | Files |
|---|---|---|---|---|
| 01 | [Cancelling a run and a proof search](01-cancel-core.md) | done | — | **`crates/turbowl/src/lib.rs`**, `crates/turbowl/src/reasoner.rs`, `crates/turbowl/src/error.rs`, new `crates/turbowl/src/cancel.rs`, `crates/turbowl/src/profile.rs`, `crates/turbowl/src/engine/store.rs` (additive), `crates/turbowl/src/explain.rs` and `src/explain/search.rs`, their tests; the `error()` arm in `crates/python/src/lib.rs`; `crates/reference/tests/chaos.rs` |
| 02 | [Cancellation in Python](02-cancel-python.md) | done | 01 | `crates/python/` |
| 03 | [Close-out](03-close-out.md) | todo | 01, 02 | `README.md`, `CHANGELOG.md`, the crate doc, `docs/verification/`, this README |

## Waves

One ticket at a time: 01, 02, 03. 02 reads 01's interface; 03 measures both.

## Departures

- 2026-11-08, ticket 01 as built:
  - `Store::run` keeps its signature and behaviour but now calls
    `Store::run_until` with a stop that never trips, so its body changed
    (the acceptance said unchanged). No file in `rules/` changed.
  - The engine checks after each step, before the round's rules: a check
    per round, plus one before the run's end work.
  - After a cancelled incremental run the next run is a full one, so the
    tests compare it with a full run of the same base (as sets: read order
    follows the dictionary's history).
  - Chaos cancels both twins at the same check (a check drawn below 8) and
    only the first explained goal, so the one-thread explainer's graph
    matches the all-cores one. The 400-iteration run cancelled 655 runs.
  - `CancelledError` waits for 02: until then `Error::Cancelled` raises
    `TurbowlError` in Python.
- 2026-11-08, ticket 02 as built:
  - Each `reason`, `explain` and `explain_diagnostic` call runs on a worker
    thread (8 MiB stack), so that the calling thread can look at Python's
    signals. A one-step `explain` from Python takes about 20 µs a call
    (measured on a loaded machine, not compared with `main`).
  - Ctrl+C trips the handle given (or the call's own), waits for the worker,
    keeps the explainer's indexes, and raises `KeyboardInterrupt`.
  - A `cancel` that is not a `turbowl.Cancel` raises `TypeError`.
  - `CancelledError` is a `TurbowlError` and a `RuntimeError`, as
    `NotReasonedError` is.
- 2026-11-08, after the review (two fresh Opus reviewers, standards and
  spec):
  - Fixed: a Ctrl+C during a Python `explain` that failed for another
    reason was dropped; the Python doc called `SnapshotError` a
    `RuntimeError`; the deepening loop now checks the handle itself.
  - Tests added or sharpened: a search cancelled from another thread
    stops within 10 ms; a tripped handle with an incremental or a full run
    to do leaves the reads as they were and the run after is the one there
    was to do; the indexes kept after a cancelled search are measured
    against an explainer that searched nothing.
  - Chaos now draws a run's check below 8 or below 64, and a search's below
    512 or below 4,096, so late cancels happen too; with no inferred triple
    it cancels a diagnostic's search. 400 iterations: 365 runs cancelled.
  - The tests are in `crates/turbowl/tests/cancel.rs`, not beside the
    reasoner's and the explainer's: they use only the public interface.
  - `reason()` runs the same path without a handle (the spec said "with a
    handle nobody trips"); the docs say so.
  - CONTEXT.md gained **Cancel**.
