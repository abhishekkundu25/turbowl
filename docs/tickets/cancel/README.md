# Tickets: cancellation

Spec: `docs/specs/cancellation.md`. Lock files are in **bold**.

| # | Ticket | Status | Blocked by | Files |
|---|---|---|---|---|
| 01 | [Cancelling a run and a proof search](01-cancel-core.md) | todo | — | **`crates/turbowl/src/lib.rs`**, `crates/turbowl/src/reasoner.rs`, `crates/turbowl/src/error.rs`, new `crates/turbowl/src/cancel.rs`, `crates/turbowl/src/profile.rs`, `crates/turbowl/src/engine/store.rs` (additive), `crates/turbowl/src/explain.rs` and `src/explain/search.rs`, their tests; the `error()` arm in `crates/python/src/lib.rs`; `crates/reference/tests/chaos.rs` |
| 02 | [Cancellation in Python](02-cancel-python.md) | todo | 01 | `crates/python/` |
| 03 | [Close-out](03-close-out.md) | todo | 01, 02 | `README.md`, `CHANGELOG.md`, the crate doc, `docs/verification/`, this README |

## Waves

One ticket at a time: 01, 02, 03. 02 reads 01's interface; 03 measures both.

## Departures
