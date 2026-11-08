# 02: Cancellation in Python

Status: done. Blocked by: 01.

- `turbowl.Cancel` (`cancel()`, `cancelled`), wrapping the Rust handle.
- `reason(cancel=None)`, `explain(s, p, o, cancel=None)` and
  `explain_diagnostic(d, cancel=None)`.
- `CancelledError`, a `TurbowlError`.
- Ctrl+C: the call runs on a scoped worker thread with the GIL released
  (main thread's stack size); the calling thread waits in 20 ms slices and
  checks Python's signals; a signal trips the handle, waits for the worker,
  and raises `KeyboardInterrupt`.
- A cancelled `reason` drops the explainer's indexes.
- Stubs in `_native.pyi`, docs in `python/turbowl/`.

## Acceptance
- [ ] `crates/python/tests/test_cancel.py`: a tripped `Cancel` raises `CancelledError` and changes nothing; a cancel mid-run leaves no closure, then `reason()` equals an uncancelled run; `explain` cancelled, then answered; Ctrl+C through `_thread.interrupt_main` raises `KeyboardInterrupt` and the reasoner reasons again.
- [ ] The whole Python suite passes.
