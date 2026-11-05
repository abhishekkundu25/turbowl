# 22: Removals: DRed

Status: todo. Blocked by: 21.

A removal overdeletes every consequence of the removed triples, by running the families forward over the deleted tuples against the old closure. It rederives what survives with one naive round of the families, then continues forward as an addition does. A removal that touches an `owl:sameAs` class or a constant's class, or that removes more than a measured share of the base, stays a full run. Findings are recomputed for the affected part.

## Acceptance
- [ ] A new chaos shard removes random single triples and small batches repeatedly from one closure, and compares each state with a full run; the existing incremental shards pass, now on the maintenance path.
- [ ] Unit tests per family: a removal that leaves another derivation, and one that cascades.
- [ ] Removing one triple from Brick plus a building model costs under 10% of a full run at the close-out.
