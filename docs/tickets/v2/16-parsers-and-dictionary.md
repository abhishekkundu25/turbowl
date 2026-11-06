# 16: Fast loading: parsers and the dictionary

Status: done on 2026-11-06 (`75fae9f`); the loading budget is measured at
the phase close-out. Blocked by: 15 (done).

TurbOWL's own N-Triples and Turtle parsers intern terms straight into term ids, through a faster dictionary: an arena, with each term's hash computed once. Blank-node scoping and error positions stay as they are. N3 and RDF/XML keep Oxigraph's parsers, which also cross-check every document the tests read. The hot path stops using Oxigraph's N-Triples and Turtle parsers.

## Acceptance
- [x] Every document the suites read, plus generated documents, parses to the same triples as Oxigraph's parsers. Malformed documents fail at the same line and column. `io/cross_check.rs`: 16,235 parses of 221 files and 5,193 strings from the sources, and 120,000 generated documents (78,418 rejected), all equal in triples, term ids and errors.
- [x] `scaling_*` tests: parsing and interning are linear in the input size. Fitted exponents 1.01 to 1.10, five runs each.
- [x] Loading lubm-50 takes at most half the Baseline's time on a quiet machine, measured at the phase close-out: 0.80 s on all cores against 3.93 s, on a busy machine (`docs/verification/2026-11-07-v2-phase-2.md`); the spec's budget is a third, met on all cores.
