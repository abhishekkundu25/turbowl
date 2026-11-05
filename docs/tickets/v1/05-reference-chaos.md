# 05: Reference evaluator and chaos

Status: done. Blocked by: 01.

`crates/reference`: the reference evaluator (the same rules as the `reasonable` and `owl-rl` profiles, written as naive loops over a `HashSet` of `oxrdf` triples, to a fixpoint; no sharing of code with the core), a seeded generator of small ontologies and data that exercises every rule (class and property axioms, lists, chains, sameAs, containers, literals), and `tests/chaos.rs` comparing the core with the reference evaluator (`CHAOS_SEED`, `CHAOS_ITERS`, `CHAOS_ONLY`, failing input saved under `target/chaos/`, a per-iteration time budget), plus incremental chaos (random batch splits and removals equal a full run).

## Acceptance
- [ ] The reference evaluator matches `reasonable` on the parity checklist's minimal inputs (minus D1 to D10), shown in the report.
- [ ] The chaos test compiles against the stub and runs once ticket 10 lands.
