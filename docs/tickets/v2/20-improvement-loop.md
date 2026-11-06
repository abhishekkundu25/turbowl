# 20: The improvement loop

Status: todo. Blocked by: 17, 18, 19.

The loop of `docs/specs/experiments.md` runs, one measured change per iteration, each in its own worktree, kept or reverted. The order: P0 (counters and a memory breakdown), then E2, E8, E1, E3, E9, E6, E7 and E10, re-ranked by what the scoreboard shows. Memory comes first while the phase 2 memory budget (50 bytes per closure triple on lubm-50) is not met: P0's memory breakdown, E7, and the dictionary. Each iteration runs the editor-size scenarios on a quiet machine; the data-size ones wait for the close-out.

## Acceptance
- [ ] Every iteration has a row in the loop's table, kept or reverted, with its numbers, spread and limiter.
- [ ] Peak memory on lubm-50 is at most 50 bytes per closure triple at the close-out, or the gap is listed with its breakdown.
- [ ] Every kept change passes the full verification and does not move any budget the wrong way.
- [ ] The loop's "For the next agent" section lists the open candidates when the ticket closes.
