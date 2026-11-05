# 03: The engine

Status: todo. Blocked by: 01. ADR 0001.

`crates/turtle-reasoner/src/engine/`: relations (sorted, duplicate-free `Vec` of term-id tuples, built by sort and dedup), variables (stable batches, recent, to-add), the round loop to a fixpoint, merge join, antijoin, leapjoin (extend by intersecting sorted proposals), and predicate tables. Allocation-aware: merge batches, reuse buffers. No rule knowledge.

## Acceptance
- [ ] Property tests against a naive evaluator: transitive closure, joins of three relations, antijoin, leapjoin, on generated relations (seeded).
- [ ] A micro-benchmark (criterion-free, `cargo test --release -- timing_` style) for the transitive closure of a 100,000-edge chain, with the time recorded in the report.
