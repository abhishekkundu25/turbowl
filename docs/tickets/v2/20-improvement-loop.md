# 20: The improvement loop

Status: done on 2026-11-07, with two gaps the maintainer accepted. Blocked by: 17, 18, 19 (done).

The loop of `docs/specs/experiments.md` runs, one measured change per iteration, each in its own worktree, kept or reverted. The order: P0 (counters and a memory breakdown), then E2, E8, E1, E3, E9, E6, E7 and E10, re-ranked by what the scoreboard shows. Memory comes first while the phase 2 memory budget (50 bytes per closure triple on lubm-50) is not met: P0's memory breakdown, E7, and the dictionary. Each iteration runs the editor-size scenarios on a quiet machine; the data-size ones wait for the close-out.

## Acceptance
- [x] Every iteration has a row in the loop's table, kept or reverted, with its numbers, spread and limiter: iterations 1 to 11 (7 kept, 3 reverted, 1 counted and not built), each with its record in `docs/verification/`.
- [x] Peak memory on lubm-50 is at most 50 bytes per closure triple at the close-out, or the gap is listed with its breakdown: one thread about 46.7; all cores about 52.9, a gap the maintainer accepted, with its breakdown in iterations 6 to 10.
- [x] Every kept change passes the full verification and does not move any budget the wrong way (each record; the phase 2 close-out reruns the slow tier and 100,000 chaos iterations).
- [x] The loop's "For the next contributor" section lists the open candidates when the ticket closes.
