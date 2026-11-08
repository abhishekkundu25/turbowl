# Tickets: cancellation

Spec: `docs/specs/cancellation.md`. Lock files are in **bold**.

| # | Ticket | Status | Blocked by | Files |
|---|---|---|---|---|
| 01 | [Cancelling a run and a proof search](01-cancel-core.md) | done | — | **`crates/turbowl/src/lib.rs`**, `crates/turbowl/src/reasoner.rs`, `crates/turbowl/src/error.rs`, new `crates/turbowl/src/cancel.rs`, `crates/turbowl/src/profile.rs`, `crates/turbowl/src/engine/store.rs` (additive), `crates/turbowl/src/explain.rs` and `src/explain/search.rs`, their tests; the `error()` arm in `crates/python/src/lib.rs`; `crates/reference/tests/chaos.rs` |
| 02 | [Cancellation in Python](02-cancel-python.md) | todo | 01 | `crates/python/` |
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
