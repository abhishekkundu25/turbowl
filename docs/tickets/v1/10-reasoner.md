# 10: The reasoner and incremental runs

Status: done. Blocked by: 02, 03.

`crates/turbowl/src/reasoner.rs`: the public `Reasoner` behind the interface from 01: load documents, insert, remove, set the base (deduplicated; an unchanged base costs nothing), run (full, or semi-naive over additions; any removal re-runs from the base), iterate the closure or only inferred triples, diagnostics, write. `Send`. Inserts every family's `facts` at the start of a full run (inferred, never base). Runs the rule table, counting the tuples each family emits and the new ones per round (a `len()` before and after each call; the counters the improvement loop's first iteration needs).

## Acceptance
- [x] Interface tests: load twice changes nothing; additions in batches equal one run; a removal removes its inferences; an unchanged base is free (counted, not timed).
- [x] The oracle suite (04), chaos (05) and the benchmark (06) run end to end with the rules that have landed.
- [x] The crate doc example in `lib.rs` runs (change its fence from `no_run` back to a plain code block).
- [x] `scaling_*` tests: loading and a full run are linear in the input size; one incremental addition costs the same at every base size (exponent at most 0.25).
