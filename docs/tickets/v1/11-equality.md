# 11: Equality by representatives

Status: done. Blocked by: 03, 10. ADR 0003.

`crates/turbowl/src/engine/canon.rs`: union-find of term ids, representative choice (larger class wins), rewriting of tuples and re-adding them to deltas. `crates/turbowl/src/rules/equality.rs`: eq-sym, eq-trans, eq-rep-s/p/o as merges; output expansion to every member and the `sameAs` triples among them. Vocabulary terms follow ADR 0003's amendment: the smallest vocabulary id wins a merge, rule constants are read through the representative map, a merge that moves a vocabulary term restarts the run from the base, and a merged-away predicate's table moves into the representative's. One registration line in **`rules/mod.rs`**.

Left for this ticket by wave 3: the rule families read their constants as bare `vocab` ids (route them through the round); `ours::EQUALITY_PENDING` in the oracle crate and `EQUALITY_PENDING` in the reference crate's `tests/chaos.rs` go to false, and every input is compared; the reasoner's count of generalized triples (`hidden`) assumes each stored triple is in one round's delta once, which `Variable::remap` breaks, so it is recounted after a merge.

As built (ADR 0003, "Amendment: as built"): `rdf:nil` is watched, since the list rules test it for identity; a restart keeps the canon and the findings found so far; and `hidden` became `rdf`, the number of RDF triples in the expanded closure.

## Acceptance
- [x] The eq-* minimal inputs, and generated inputs with sameAs, give the same expanded closure as Nemo and the reference evaluator.
- [x] A `scaling_*` test: a sameAs clique's run time and stored tuples are linear in its size (sizes spanning 16×), where the eq-* closure would hold n² `sameAs` triples. Expanding the output is n² by definition and is measured per output triple. The `sameas-clique` benchmark (1,000 aliases) records the memory.
- [x] Minimal inputs and generated chaos inputs with `sameAs` on vocabulary terms (a property `sameAs` `rdf:type`, two vocabulary terms merged) give the same closure as Nemo and the reference evaluator.
