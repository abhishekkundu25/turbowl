# 13: Python bindings

Status: todo. Blocked by: 10.

`crates/python`: pyo3 bindings built with maturin (via `uvx maturin`), a `PyReasoner` with `reasonable`'s methods and return shapes, rdflib terms in and out (plain literals as rdflib makes them, D10), diagnostics exposed (which `reasonable` does not), errors as Python exceptions.

## Acceptance
- [ ] `reasonable`'s 45 Python test cases, ported to this module, pass under `uv run pytest` in a project-local environment.
