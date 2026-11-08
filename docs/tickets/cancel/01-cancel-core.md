# 01: Cancelling a run and a proof search

Status: todo. Blocked by: none.

- `Cancel` (`new`, `cancel`, `is_cancelled`, `Clone` shares it, `Send + Sync`,
  `Default`) and `profile::cancel_after(checks)`, a handle that trips itself
  at its n-th check.
- `Error::Cancelled`, with its Python `error()` arm (`CancelledError`, defined
  in 02; until then the arm raises `TurbowlError`), so the workspace builds.
- `Reasoner::reason_cancellable(&Cancel) -> Result<Summary, Error>`; `reason()`
  runs the same path with a handle nobody trips.
- The engine's additive `Store::run_until`, checking before each round.
- A stopped run leaves the reasoner as loaded and not yet reasoned: base kept,
  closure and diagnostics empty, state `Full`. A handle tripped before the
  call changes nothing.
- `Explainer::explain_cancellable` and `explain_diagnostic_cancellable`: the
  work loops and the deepening check the handle; a stopped search drops the
  search graph and keeps the indexes.

## Acceptance
- [ ] Reasoner tests: a tripped handle changes nothing; a cancel at every check of a multi-round run, of an incremental run, of a run after a removal and of a run through an equality restart gives `Cancelled`, an empty closure and no diagnostics, then a run equal to an uncancelled one.
- [ ] A cancel from another thread on a long chain returns within the longest round plus 10 ms.
- [ ] Explainer tests: a cancel at every check of a deep proof gives `Cancelled`; the same question then gives the uncancelled proof; the cache is still held.
- [ ] Chaos: each iteration cancels one run at a random check and one proof search halfway, then compares (`CHAOS_CANCEL=0` turns it off); all seven chaos tests pass.
- [ ] `Store::run` and every file in `rules/` unchanged.
- [ ] fmt, workspace clippy, `check-wasm.sh`, `-p turbowl`, `-p turbowl-oracle`, chaos and the CLI suite pass.
