# 18: All cores: parallel parsing

Status: done on 2026-11-06 (`8cee6d4`); the loading budget is measured at
the phase close-out. Blocked by: 16, 17 (done).

Large N-Triples inputs are split at line boundaries and parsed on separate threads into local term tables, then merged into the dictionary under the same feature as parallel rounds. Turtle stays sequential unless a measured split is found.

## Acceptance
- [x] Parallel parsing gives the same dictionary contents and triples as sequential parsing on every test document and on generated large ones: `io/cross_check.rs` compares the parse in pieces with the sequential one on all 16,100 corpus parses and 120,000 generated documents, errors included; disabling any of its four safeguards makes it fail.
- [x] Loading lubm-50 takes at most a third of the Baseline's time at the close-out: 0.80 s on all cores (bar 1.31 s); one thread 1.52 s, a gap (`docs/verification/2026-11-07-v2-phase-2.md`). At load 4, `io::parse` takes 0.194 s on all cores against 0.833 s on one; `document_key` and the base's sort add to the load, and are the improvement loop's first candidates.
- [x] A `scaling_*` test at 1 and 12 threads: `scaling_parse_in_pieces_one_thread` and `scaling_parse_in_pieces_twelve_threads`, each also checking equality with the one-thread parse.
