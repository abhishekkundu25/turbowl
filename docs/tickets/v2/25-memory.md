# 25: Memory: OWL2Bench RL, UOBM-010 U and OpenCyc

Status: done. Blocked by: 13.

Ticket 13's smoke runs found TurbOWL holding far more memory than its
closure explains:
- `owl2bench-rl-1` peaked at 12.6 GB for a closure of 1.45 M triples, about
  9,300 bytes per triple against about 150 on LUBM;
- `owl2bench-rl-2`, `owl2bench-rl-5`, `uobm-010-u` and `opencyc` passed the
  16 GB cap. RDFox's paper reports 1.1 GB for UOBM-010 U's 35.6 M triples.

Find what holds the memory in each, from measurements, not from reading the
code. The candidates include the store's tuples, the canon's classes,
restarts, the findings and their expansion into diagnostics (a product over
class members), the summary's counts, and the bench's own closure check,
which counts the expanded closure in a hash set. Every run stays under a
memory cap of 8 GB (`--memory-cap 8`), one heavy run at a time, on cut-down
inputs where they show the same growth.

Fix the root cause in the core, with a test that pins it, such as a
`scaling_*` test where that growth fails. If the fix needs an interface
change (for example, diagnostics read lazily instead of all at once), stop
and report the options. A cause in the bench's closure check goes to ticket
12, which owns `engine.rs`.

## Acceptance
- [x] For each scenario, a measured breakdown of the peak (what holds how much) and the method.
- [x] The root cause is fixed, with a test that fails before the fix; or, if an interface change is needed, the options are reported.
- [x] `owl2bench-rl-1` and `uobm-010-u` run under the cap, with their peak bytes per closure triple recorded (smoke runs).
- [x] Every suite passes as before.
