# 21: Close-out: phase 2

Status: done on 2026-11-07, with the gaps the record lists
(`docs/verification/2026-11-07-v2-phase-2.md`). Blocked by: 16 to 20
(done).

Full validation with nothing else running, at every thread count; the scoreboard rerun on a quiet machine, both configurations and both thread modes, nothing over 16 GB; README, CHANGELOG and AGENTS; a verification record.

## Acceptance
- [x] Every budget of the phase 2 spec passes, or the gap is listed and the maintainer is told. The record's Budgets and Gaps list each one. Met: loading lubm-50 on all cores (0.80 s against 1.31 s), every editor-size total below version 1's on both thread modes, one-thread memory (46.6 bytes per closure triple), every oracle at every thread count, 100,000 chaos iterations of each kind, the WebAssembly build, fmt and clippy. The gaps are below.
- [x] The scoreboard record shows, per scenario and axis, where TurbOWL leads and where it does not: `scoreboard.txt` in `docs/verification/data/2026-11-07-v2-phase-2/`, for the short race the maintainer asked for (editor size, both configurations, both thread modes). TurbOWL leads all 22 scenario and axis pairs where an engine finished, in each table. The data-size scenarios ran with TurbOWL alone; the full-size race is ticket 27.

## Gaps

- **The snapshot.** Loading lubm-50's snapshot is 9.2 times as fast as parsing and reasoning it on all cores (CLI wall time: 0.92 against 0.10 s, 5 runs), against 10; about 19 times on one thread. Ticket 19 measured 15 times; parsing and reasoning got faster since. New; for the maintainer.
- **Loading on one thread.** lubm-50 loads in 1.52 s on one thread, against a bar of 1.31 s (a third of version 1's 3.93 s). All cores meet it. For the maintainer.
- **The all-cores speed-up**, accepted by the maintainer on 2026-11-07: lubm-50 reasons 3.9 times as fast on all cores as on one (393 against 102 ms), against 5.
- **The all-cores memory**, accepted by the maintainer on 2026-11-07: about 53 bytes per closure triple on lubm-50, against 50.
- **Not done as written.** The machine was busy (1-minute load average 3 to 5 during the race), and the four chaos shards ran side by side. The race was editor size only, at the maintainer's request. VLog's runs that hit the 16 GB cap passed it by up to 133 MB before the cap stopped them, as in phase 1.
