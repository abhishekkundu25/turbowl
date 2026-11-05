# 23: Removals: FBF

Status: todo. Blocked by: 22.

Before a tuple is overdeleted, a bounded backward search per family looks for another proof of it from current tuples, keeping a set of checked and of proved tuples (AIJ 2019 §7). The full-run threshold is measured again.

## Acceptance
- [ ] The chaos removal shard and the incremental shards pass 100,000 iterations each.
- [ ] Removing 1,000 random facts from lubm-50 costs under 10% of a full run at the close-out.
- [ ] A `scaling_*` test: removing one triple costs about the same whatever the closure size, off the fallback paths.
