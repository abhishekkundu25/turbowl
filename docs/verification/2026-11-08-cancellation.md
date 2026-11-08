# Verification: cancellation

Date: 2026-11-08. Branch `cancel`, spec `docs/specs/cancellation.md`,
tickets in `docs/tickets/cancel/`. Apple M6, 12 cores, 32 GB, macOS 27.0.
Raw data: `docs/verification/data/2026-11-08-cancellation/`.

The machine was never quiet: the load average stayed between 4 and 52, from
another project's Python jobs. The runs that need a quiet machine (load
under 1) moved to ticket 04 (`docs/tickets/cancel/04-deferred-runs.md`):
the bench against `main`, the chaos time with and without cancels, and
Python's per-call cost.

## Results against the budgets

| budget | result |
|---|---|
| Reasoning times and peak memory within `main`'s spread | **not measured** (ticket 04). The run path adds one check of an `Option` per round and one at the end of a run; `reason()` passes `None`. |
| `engine/` changes additive; `Store::run` and `rules/` unchanged | **met**, with one departure: `Store::run` keeps its signature and behaviour but its body now calls `Store::run_until`. No file in `rules/` changed. |
| A cancel during a run returns within its longest round and step plus 10 ms | **met** (`a_cancel_from_another_thread_stops_the_run_within_a_round`) |
| A cancel during a search returns within 10 ms | **met** (`a_cancel_from_another_thread_stops_a_search_within_10_ms`) |
| After any cancel, the next run equals an uncancelled one (chaos) | **met**: 400 iterations per kind, 365 runs cancelled, every run after compared with the reference and between one thread and all cores |
| Chaos with the cancels at most 10% longer | **not measured**: under load, inconclusive (below); ticket 04 |
| fmt, workspace clippy, WebAssembly | **met** |

## Tests

- `crates/turbowl/tests/cancel.rs`, 7 tests, the public interface only:
  - a tripped handle changes nothing, with nothing to do, an incremental
    run to do and a full run to do;
  - a cancel at every check (the first 64, then a growing step) of a full
    run, of incremental runs (one through an equality restart) and of a run
    after a removal, default and complete: `Cancelled`, no closure, no
    diagnostics, `NotReasoned` from the explainer, then a run equal to a
    full run of the same base;
  - a cancel at every check of a proof search and of a diagnostic's, default
    and complete: the indexes kept, then the proof an explainer that was
    never cancelled gives;
  - the two timed tests above, a cancel from another thread.
- `crates/turbowl/src/cancel.rs`: 2 unit tests of the handle.
- `crates/python/tests/test_cancel.py`, 6 tests: the handle,
  `CancelledError`, a tripped handle, a cancel from a timer thread, Ctrl+C
  through `_thread.interrupt_main` during a run and during proofs, and a
  `cancel` of the wrong type.
- Chaos: every iteration cancels one of its runs on both reasoners at the
  same check (below 8 or below 64) and its first explained goal (below 512
  or below 4,096). All seven chaos tests pass.
- `-p turbowl`: 376 tests. Oracle: 52. CLI: 35. Python: 815. fmt, workspace
  clippy and `check-wasm.sh` pass.

## Chaos time (under load, inconclusive)

`full_runs_match_the_reference_1` at `CHAOS_ITERS=1000`, alternating, load
7.7 to 13.5:

| run | `CHAOS_CANCEL=0`, s | cancels on, s |
|---|---|---|
| 1 | 75.7 | 74.7 |
| 2 | 44.3 | 55.7 |

The spread between the two pairs (75 s against 44 s) is larger than any
difference between the sides. The cancels add at most one partial run per
iteration, and a run is a small part of an iteration's time next to the
reference evaluation and the proofs.

## Review

Two fresh Opus reviewers read the diff, one against the repo's standards
and one against the spec. Fixed after it:

- In Python, a Ctrl+C during an `explain` that failed for another reason
  was dropped.
- The Python doc called `SnapshotError` a `RuntimeError`.
- The deepening loop now checks the handle itself, not only through the
  work loops.
- Tests were added or sharpened: the timed search, a tripped handle with
  work to do, and the indexes measured after a cancel.
- Chaos now cancels late as well as early, and cancels a diagnostic's
  search when a state has no inferred triple.

Left as they are (judgement calls):

- The `Option<&Cancel>` threading and its `is_some_and` checks, against a
  handle nobody trips in `reason()`. `None` costs no atomic load and no
  allocation.
- `profile::cancel_after` in `profile`, which AGENTS.md calls the bench's
  module. It is for chaos and the tests, and the crate doc says so.

## Decisions taken without the owner

The owner said to assume what fits the goal and record it.

1. **A cancelled run keeps no partial result.** The reasoner is as loaded
   and not yet reasoned, and the next run is a full one, even after a
   cancelled incremental run. Keeping the previous closure would cost a copy
   of the store on every run.
2. **A handle tripped before the call changes nothing**, and it stays
   tripped: one per request.
3. **Checks sit between rounds, not inside them.** A round, an index build
   and a whole-graph ranking are not interrupted.
4. **New methods.** `reason_cancellable`, `explain_cancellable` and
   `explain_diagnostic_cancellable` are new, and `reason()` and `explain()`
   keep their signatures.
5. **The test hook lives in `profile`.** `profile::cancel_after` is a
   handle that trips itself at its n-th check, so that tests need no clock.
6. **The Python worker thread.** Each Python `reason`, `explain` and
   `explain_diagnostic` runs on a worker thread with an 8 MiB stack. It
   costs about 20 µs a call, measured under load and not yet compared with
   `main` (ticket 04).
7. **Ctrl+C in Python.** It trips the handle given, or the call's own, keeps
   the explainer's indexes, and raises `KeyboardInterrupt`.
8. **`CancelledError`** is a `TurbowlError` and a `RuntimeError`, as
   `NotReasonedError` is.
9. **The CLI does not change.** Ctrl+C ends the process.

## Open for the owner

- Ticket 04: the runs that need a quiet machine.
- Merging `cancel` into `main`, and turtle-lsp's use of it. Its handover on
  `feat/turbowl-reasoning` names a generation counter; with `Cancel`, the
  editor can stop the stale run instead.
