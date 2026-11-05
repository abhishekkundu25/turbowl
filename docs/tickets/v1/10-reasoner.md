# 10: The reasoner and incremental runs

Status: todo. Blocked by: 02, 03.

`crates/turtle-reasoner/src/reasoner.rs`: the public `Reasoner` behind the interface from 01: load documents, insert, remove, set the base (deduplicated; an unchanged base costs nothing), run (full, or semi-naive over additions; any removal re-runs from the base), iterate the closure or only inferred triples, diagnostics, write. `Send`. Runs the rule table.

## Acceptance
- [ ] Interface tests: load twice changes nothing; additions in batches equal one run; a removal removes its inferences; an unchanged base is free (counted, not timed).
- [ ] The oracle suite (04), chaos (05) and the benchmark (06) run end to end with the rules that have landed.
- [ ] The crate doc example in `lib.rs` runs (change its fence from `no_run` back to a plain code block).
- [ ] `scaling_*` tests: loading and a full run are linear in the input size; one incremental addition costs the same at every base size (exponent at most 0.25).
