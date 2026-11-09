# 23: Removals: FBF

Status: implemented; scaling, large-input performance and extended chaos
acceptance remain open. Record: `docs/verification/2026-11-09-removals.md`.

Before overdeleting a tuple, bounded backward search over the shared
ordinary-triple rules looks for another proof grounded in the remaining
base or axioms. It keeps checked and proved tuples. Helper rules and
exhausted searches proceed conservatively through DRed. The whole run has
a 1,024-step work bound. Full-run threshold tuning is still open.

## Acceptance
- [ ] The chaos removal shard and the incremental shards pass 100,000 iterations each.
- [ ] Removing 1,000 random facts from lubm-50 costs under 10% of a full run at the close-out.
- [ ] A `scaling_*` test: removing one triple costs about the same whatever the closure size, off the fallback paths.
