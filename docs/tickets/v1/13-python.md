# 13: Python bindings

Status: todo. Blocked by: 10.

`crates/python`: pyo3 bindings built with maturin (via `uvx maturin`): a `Reasoner` class that loads files, takes and returns rdflib graphs and terms, runs incrementally, and exposes diagnostics; errors as Python exceptions. A Pythonic API of our own (ADR 0004), documented in the module.

## Acceptance
- [ ] Tests under `uv run pytest` in a project-local environment cover every method, rdflib round trips (language tags, datatypes, blank nodes) and the closure against the Rust library on the minimal inputs.
