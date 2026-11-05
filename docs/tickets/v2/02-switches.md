# 02: Switches: the seam

Status: todo. Blocked by: None.

The rule families regroup around the switches of the phase 1 spec, as if switches had been there from the start (ADR 0005). `Switch` (the seven groups) and `Switches` (a copyable set: defaults, `complete()`, on and off) join the public interface, with a constructor taking switches, a setter and a getter on `Reasoner`. A change of switches makes the next run full. Each family is registered with its switch (none for the core); the reasoner skips a family whose switch is off and inserts its facts only when it is on. The version 1 contradiction rules move from `rules/datatype.rs` to a `checks` family without changing behaviour. Property chains become the `property-chains` family, and the `prp-spo2` cargo feature goes. Empty families are registered for `identity`, `schema`, `trivial`, `literal-values` and `cls-oo`, so rule tickets fill their own files. The vocabulary gains every IRI the new rules name (the spec's list) in this one change.

## Acceptance
- [ ] With the default switches, every existing suite passes unchanged: core, oracle (minimal, W3C, Brick), chaos, CLI and Python.
- [ ] Unit tests: a changed configuration gives the same closure as a fresh reasoner with it; `equality`, `property-chains` and `checks` off each remove exactly their rules' consequences; the empty groups derive nothing; the defaults match the spec's table.
- [ ] The `prp-spo2` feature is gone, and `scripts/check-wasm.sh` builds a core with property chains.
- [ ] A test lists every IRI the spec names and finds each in the vocabulary.
- [ ] The crate doc describes the switches and why each default is what it is.
