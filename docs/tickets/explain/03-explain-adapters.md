# 03: Explanations in the CLI and Python

Status: todo. Blocked by: 01.

- **CLI** (`crates/cli/`):
  - `--explain '<s> <p> <o>'` takes one N-Triples triple and can be
    repeated. The triple is parsed with `oxttl`, a new dependency of the
    CLI that is already in the lock file through `turbowl`. Each proof is
    printed after the report as the `Display` tree.
  - `--explain-diagnostics` prints the proof of every listed diagnostic.
  - In the JSON and NDJSON reports, proofs appear as objects: steps with
    statement, rule and premise indexes, the goal last.
  - A triple not in the closure is reported as such and does not change
    the exit code. A malformed value exits 64.
  - `--help` and the crate doc describe both flags.
- **Python** (`crates/python/`):
  - `Reasoner.explain(s, p, o)` and `Reasoner.explain_diagnostic(d)`
    return a `Proof` (`steps`, `leaves()`, `depth`, and `__str__` as the
    tree), or `None`.
  - `NativeReasoner` keeps an `Option<ExplainCache>` and clears it in every
    `&mut` method.
  - Stubs go in `_native.pyi`, docs in `python/turbowl/`.

## Acceptance
- [ ] `crates/cli/tests/cli.rs`: text and JSON proofs, a file with blank nodes, a triple not in the closure, `--explain-diagnostics`, a malformed value exiting 64.
- [ ] `crates/python/tests/test_explain.py`: a proof's steps and leaves, `None` outside the closure, a diagnostic's proof, the cache dropped after `insert`.
- [ ] fmt, clippy, the CLI suite and the Python suite pass.
