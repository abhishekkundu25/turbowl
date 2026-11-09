# 05: The deferred validation runs

Status: todo. Blocked by: none (04 closed without them, at the maintainer's
request: the machine was not quiet, load 2.7 to 4 from other applications).

Run these with nothing else running, then add their results to
`docs/verification/2026-11-07-explanations.md` and its data folder.

- **The explain latencies at load under 1**: `turbowl-bench --explain
  --only <scenario>`, one scenario at a time, default and `--complete`,
  for brick-small1, brick-soda-hall, lubm-1, lubm-50, sameas-clique and
  claros-l. The numbers in the record were taken at load 1.5 to 2.3.
- **lubm-50, default, one thread, 10 runs per side** alternating `main`
  and `explain`: the one time outside `main`'s spread in the record
  (reasoning +0.3%, 381.1 against 382.1 ms) is confirmed or ruled noise.
- **100,000 chaos iterations per kind**: the four shards alone (`--exact`)
  with `CHAOS_ITERS=100000`, in parallel from one built test binary.
  Record each shard's time against phase 2's (full about 35 min,
  incremental about 42) and against `CHAOS_EXPLAINED=0`. Expect about 2.7
  times as long with proofs (see the gap in the record).
- **The slow oracle tier**: `ORACLE_SLOW=1`, the Brick variants and soda
  hall, one test at a time (`--exact`).

## Acceptance
- [ ] Each run's raw output in `docs/verification/data/2026-11-07-explanations/`.
- [ ] The record's tables updated, and any new gap stated with its numbers.
- [ ] AGENTS.md's chaos shard times updated to the measured ones.
