# Version 2, phase 3: removals that cost what they change

Status: planned; starts when phase 2 closes (ticket 21). Tickets:
`docs/tickets/v2/` (22 to 24). Background:
`docs/research/sota-algorithms-2026-11.md` §5 (DRed, B/F, FBF and counting,
with their published numbers and fit with this engine).

## Problem Statement

Any removal makes the next run a full run from the remaining base.
On Brick with small1, adding a triple costs 0.2 ms and removing one
2.8 ms; on lubm-50 a removal costs the whole reasoning time. An editor
that deletes a line, or a data pipeline that retracts facts, pays for
everything each time. RDFox maintains its closure on deletion (B/F and
FBF); TurbOWL does not.

## Solution

- **DRed** first. A removal overdeletes every consequence of the removed
  triples, rederives what still has another derivation from what
  survives, and then continues forward as an addition does.
- **FBF** next. Before overdeleting a triple, look for another proof of it
  by a bounded backward search. This keeps a large closure from being torn
  down and rebuilt.
- **Fallback**: a removal that touches an `owl:sameAs` class or a
  vocabulary term, or that removes more than a measured share of the base,
  stays a full run.

## User Stories

1. As an editor author, I want deleting a triple to cost about what the deletion changes, so that deletes are as cheap as inserts.
2. As a data engineer, I want to retract a batch of facts from a large closure without recomputing it, so that updates stay fast.
3. As a user, I want the closure after any sequence of additions and removals to equal a full run over the final base, so that incremental results are always right.
4. As a user, I want diagnostics to disappear when the triples behind them are removed, so that fixed problems stop being reported.
5. As a user, I want a removal that splits an `owl:sameAs` class handled correctly, even if it costs a full run, so that equality stays right.
6. As the maintainer, I want the incremental-remove scenarios on the scoreboard to show the gain, so that the claim covers removals too.
7. As a maintainer, I want chaos to remove random parts of random inputs and compare each state with a full run, as it does today, so that maintenance bugs are found.
8. As a maintainer, I want the full-run fallback chosen by a measured threshold, so that large deletions never cost more than starting over.

## Implementation Decisions

- **DRed in the engine**:
  - Overdeletion runs the rule families forward over a store whose recent
    parts hold the deleted tuples, against the old closure.
  - Rederivation runs one naive round of the families over the survivors
    and keeps only overdeleted tuples that come back.
  - Insertion then continues semi-naively, as an addition does.
- **FBF** adds a per-family check: "does this tuple have a derivation from
  current tuples?", with a set of checked and proved tuples (AIJ 2019 §7).
  It is the second ticket.
- **Findings** are recomputed for the affected part only.
- **Equality**: a removal that touches a class of two or more members, or a
  constant's class, runs in full until a later experiment.

## Testing Decisions

- **Chaos** keeps its incremental shards (insert in batches, remove a random
  part, put it back), now running the maintenance path. A new shard removes
  random single triples and small batches repeatedly from one closure.
- **Removal unit tests** per family (a removal that leaves another
  derivation, a removal that cascades), as in `rules/equality/tests.rs`.
- **Scaling**: removing one triple from closures of growing size costs
  about the same (`scaling_*`, constant growth), except on the fallback
  paths.
- **The scoreboard's** incremental-remove scenarios, plus a data-size one
  that removes 1,000 random facts from lubm-50.

## Non-functional budgets (pass/fail)

- Removing one triple from Brick plus a building model costs under 10% of a
  full run, the same budget additions have.
- Removing 1,000 random facts from lubm-50 costs under 10% of a full run.
- Every chaos shard passes 100,000 iterations, the new removal shard
  included.
- Every oracle still agrees; fmt, clippy and the WebAssembly build pass.

## Out of Scope

- Counting-based maintenance (it conflicts with set semantics:
  `sota-algorithms-2026-11.md` §5.3) and differential dataflow.
- Incremental splitting of `owl:sameAs` classes (B/F≈): a later experiment.
- Publishing numbers without the maintainer's approval.

## Principles

- **Exhaust the design space**: DRed's rederivation has two designs (one
  naive round against a head-directed check per family). Ticket 22 builds
  the first and measures it; FBF's check (ticket 23) is the second.
- **Make operations idempotent**: a removal followed by putting the same
  triples back converges to the same closure as never removing them (a
  chaos check).
- **Explain the number**: each removal number names whether the
  maintenance path or the fallback ran.
