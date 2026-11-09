# 04: The deferred runs

Status: todo. Blocked by: a quiet machine (load average under 1).

The close-out (03) could not run these: the load stayed 4 to 52 from
another project's jobs.

- The bench against `main`, alternating: brick-small1, brick-soda-hall,
  lubm-1 and lubm-50, default and complete, one thread (5 runs per side)
  and all cores (10), as in `docs/verification/2026-11-07-explanations.md`.
- Chaos with and without `CHAOS_CANCEL=0`: `full_runs_match_the_reference_1`
  and `incremental_runs_match_a_full_run_0` at `CHAOS_ITERS=2000`,
  alternating, two runs per side.
- Python's per-call cost of the worker thread: `explain` of a one-step
  proof, 2,000 calls, against `main`'s build.

## Acceptance
- [ ] Reasoning times and peak memory within `main`'s spread, or each one outside named with what limits it.
- [ ] Chaos with cancels at most 10% longer, or the gap recorded for the maintainer.
- [ ] The numbers added to `docs/verification/2026-11-08-cancellation.md` and its data folder.
