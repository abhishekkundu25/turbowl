# 11: Equality by representatives

Status: todo. Blocked by: 03, 10. ADR 0003.

`crates/turtle-reasoner/src/engine/canon.rs`: union-find of term ids, representative choice (larger class wins), rewriting of tuples and re-adding them to deltas. `crates/turtle-reasoner/src/rules/equality.rs`: eq-sym, eq-trans, eq-rep-s/p/o as merges; output expansion to every member and the `sameAs` triples among them. One registration line in **`rules/mod.rs`**.

## Acceptance
- [ ] The eq-* minimal inputs, and generated inputs with sameAs, give the same expanded closure as Nemo and the reference evaluator.
- [ ] A `scaling_*` test: a sameAs clique's run time and stored tuples are linear in its size (sizes spanning 16×), where the eq-* closure would hold n² `sameAs` triples. Expanding the output is n² by definition and is measured per output triple. The `sameas-clique` benchmark (1,000 aliases) records the memory.
