# 22: Removals: DRed

Status: implemented; performance acceptance and close-out remain open.
Record: `docs/verification/2026-11-09-removals.md`.

A removal overdeletes consequences against the old closure, rederives what
survives, then continues forward. The shared rule program supplies indexed
dependent and survivor queries when no equality classes exist. Native
family replay handles unaffected equality classes. Equality-sensitive,
large and unsupported updates use full reasoning. Nonempty findings are
recomputed. Thresholds are bounded heuristics; workload tuning remains open.

## Acceptance
- [x] A new chaos shard removes random single triples and small batches repeatedly from one closure, and compares each state with a full run; the existing incremental shards pass, now on the maintenance path.
- [ ] Unit tests per family: a removal that leaves another derivation, and one that cascades.
- [ ] Removing one triple from Brick plus a building model costs under 10% of a full run at the close-out.

Regular chaos runs use 400 iterations per kind, split over two shards.
The exhaustive per-family alternate-proof/cascade matrix remains open;
current public regressions cover representative property, class, list,
schema, equality and diagnostic removals and lifecycle behavior.
