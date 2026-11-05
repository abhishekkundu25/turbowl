# ADR 0006: the scoreboard replaces `reasonable`; RDFox counts through its papers

Status: accepted, 2026-11-05. Amends ADR 0004 ("`reasonable` is a speed
baseline").

## Context

The owner's goal for version 2 is the best OWL 2 RL reasoner in the world.
Version 1 is faster than `reasonable` on every scenario measured (3.6 to
7.3 times, with 5.5 to 9.8 times less memory: `docs/verification/`), and
much faster than Nemo, so beating them says little. The state of the art
in OWL 2 RL materialisation is RDFox (commercial), VLog, Soufflé and Nemo
(`docs/research/sota-algorithms-2026-11.md` §7). The owner cannot obtain an
RDFox licence. The machine has 32 GiB and runs the owner's other jobs.

## Decision

- **`reasonable` leaves the project**: its engine, its dependency and its
  budgets. Its repository stays a data source (Brick and the building
  models).
- **The scoreboard** (`CONTEXT.md`) is what the claim is measured on:
  - Competitors run here: VLog, Soufflé, Nemo, and GraphDB if its licence
    allows publishing numbers.
  - Correctness and coverage are gates. Then four axes: reasoning time,
    total time (loading plus reasoning), time per incremental change, and
    peak memory, single-threaded and on all cores.
  - The headline race runs the same W3C rules on every engine, with each
    closure checked against this reasoner's. A second table lets an engine
    use its own OWL mode.
  - The claim is "first on every axis on every scenario"; where that does
    not hold, the table says so.
- **RDFox counts through published results only**: the same public dataset,
  the same fact counts before and after reasoning, scaled to this machine
  through a bridge (an engine that ran next to RDFox in the paper and runs
  here), and always labelled an estimate.
- **Big runs stay small and rare**: no run of any engine passes 16 GB of
  memory, and data-size scenarios run only at a phase close-out, on a quiet
  machine.

## Consequences

- The bench drops the `reasonable` crate and gains subprocess engines,
  public datasets and a memory cap.
- "Baseline" now means this project's own kept version; the other engines
  are competitors.
- A published RDFox number is a floor: RDFox has improved since its
  papers, so beating an old number proves less than beating a run.
