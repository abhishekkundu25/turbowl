# ADR 0001: an own semi-naive engine over sorted term-id relations

Status: accepted, 2026-11-05.

## Context

The reasoner must be fast with no compromise, run in single-threaded
WebAssembly, and leave room for experiments (columnar storage, worst-case
optimal joins, parallel rounds, incremental maintenance). The research
(`docs/research/reasonable-parity.md`, and turtle-lsp's
`docs/research/reasoner-sota-2026-11.md`) weighed three bases.

## Options

1. **datafrog as a dependency** (what `reasonable` uses). Small, proven,
   pure Rust, builds for wasm. Its relations are `Vec`s of tuples; it has no
   deletion, no parallelism and no columnar layout, and changing those means
   forking it.
2. **Nemo's physical layer** (`nemo-physical`). Columnar tries, leapfrog
   triejoin, interval compression. Large, research code, "unstable", built
   for batch runs of user programs; adopting it ties the design to its plan
   language and release cadence.
3. **An own engine with datafrog's semantics.** Sorted, duplicate-free
   relations of term ids; variables with stable, recent and to-add parts;
   merge joins, antijoins and leapjoins; a round loop to the fixpoint. About
   the size of datafrog, owned here, so each later experiment changes one
   module.

## Decision

Option 3. Rules are Rust code over the engine's relations (not an
interpreted rule language), one module per rule family, as `reasonable`
does: the rule set is fixed, and compiled joins are the fastest form of it.
The engine is an internal module of the core crate; nothing outside the
crate sees a relation.

## Consequences

- The engine's own tests compare it with a naive evaluator on generated
  relations.
- Experiments that change storage or joins replace the engine module behind
  the same internal interface, and the oracle and chaos suites judge them.
- Leapfrog triejoin is patented by LogicBlox/Infor (CA2885244C, US9177026B2,
  US10120906B2; see the research). The engine uses datafrog-style leapjoin
  (sorted extension of one variable by intersecting proposals), not the
  patented trie iterator design; an experiment that wants a full worst-case
  optimal join reads the claims first.
