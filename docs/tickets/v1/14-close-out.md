# 14: Close-out: full verification

Status: done on 2026-11-06. Blocked by: all (done). The maintainer chose to
measure without a quiet machine.

Retargeted on 2026-11-05 by version 2's plan (ADR 0006): this ticket
freezes version 1 as the **Baseline** (`CONTEXT.md`) that version 2's
experiments are compared against. `reasonable` is no longer measured.

The wave 4 record (`docs/verification/2026-11-05-wave-4.md`) already ran
every suite and every oracle at `f9e3625`, with no input pending. What is
left:

- Run the benchmark at `f9e3625` (a worktree, not the moving branch) with
  the load average under 1: TurbOWL and Nemo on every default scenario, 5
  runs, each number with its spread and limiter. Run the `scaling_*` tests
  alone. Write `docs/verification/<date>-v1.md` and the raw results.
- Fill the v1 spec's budgets with those numbers. The two budgets that named
  `reasonable` were met in wave 3's trend run (3.65 to 7.30 times faster,
  5.49 to 9.81 times less memory) and are retired by ADR 0006; say so.
- `README.md` (what it is, install, CLI, Rust, Python, the rule set, oracle
  faults, verification, the numbers), `CHANGELOG.md` (0.1.0), `AGENTS.md`
  (final for version 1, pointing at version 2).
- Iteration 0 in `docs/specs/experiments.md`, with a fresh reviewer's
  rubric score.

## Acceptance
- [x] No input is pending in any oracle report (equality landed; wave 4 record).
- [x] The quiet-machine numbers of TurbOWL and Nemo on every default scenario at `f9e3625`, 5 runs, are recorded as the Baseline, each with what limits it. Done differently: the machine was not quiet. The maintainer chose to measure at a 1-minute load average of 1.1 to 2.3. The spread between runs stays under 7% for TurbOWL and under 4% for Nemo, so a claim within a few percent waits for a quiet machine (`docs/verification/2026-11-06-v1.md`).
- [x] Every v1 budget is filled in, or the gap is listed and the maintainer is told (`docs/specs/v1.md`, Results). The two budgets that named `reasonable` are retired by ADR 0006, met in wave 3's trend run.
- [x] README, CHANGELOG (0.1.0) and AGENTS are final for version 1. Done differently: they are written on `v2`, where version 2's phase 1 closed the same day, so the README describes the `v2` branch, the CHANGELOG's 0.1.0 section records version 1, and AGENTS points at version 2. `main` keeps version 1's code without a README.
- [x] Iteration 0 has a rubric score: 53/80 from a fresh reviewer, weakest on coverage (`docs/specs/experiments.md`, Iteration 0: the rating).
