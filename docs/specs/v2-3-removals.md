# Version 2, phase 3: removals that cost what they change

Status: implemented on 2026-11-09, acceptance open, on
`feat/incremental-deletions`; phase 2 is closed. Tickets:
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
6. As the owner, I want the incremental-remove scenarios on the scoreboard to show the gain, so that the claim covers removals too.
7. As a maintainer, I want chaos to remove random parts of random inputs and compare each state with a full run, as it does today, so that maintenance bugs are found.
8. As a maintainer, I want the full-run fallback chosen by a measured threshold, so that large deletions never cost more than starting over.

## Implementation Decisions

### Execution contract, 2026-11-09

The owner requested specification and implementation after the small
deletion benchmark in `docs/verification/2026-11-09-deletion-cost.md`.
This specification and tickets 22 to 24 are the local tracker.

- Keep the public signatures of `remove`, `set_base`, `reason`, snapshots,
  cancellation and explanations. Closure and diagnostic reads between a
  mutation and `reason` continue to show the previous result. Removing an absent or inferred-only
  triple is a no-op. Duplicate removals count once.
- Queue actual base deletions until reasoning. Repeated deletions are
  combined. A pure-deletion `set_base` uses the same maintenance path.
  Changes mixed with pending additions, switch changes and large batches
  may use full reasoning; their result remains identical.
- Run DRed against the old closure with deleted tuples as the delta.
  Protect remaining base triples and active families' axiom facts.
  Delete the affected tuples, rederive surviving consequences, then
  continue ordinary forward reasoning. Cycles cannot support themselves.
- Add bounded backward checks before overdeleting a consequence. A proof
  must reach remaining base triples or axiom facts. A failed or exhausted
  check is inconclusive and proceeds through DRed. The search keeps
  checked and proved facts; it never uses the explainer or records proofs
  during ordinary forward reasoning.
- A typing change may conservatively overdelete the subject's inferred
  types together, protecting its remaining base types and axiom facts.
  Static projections of helper-rule heads identify the first ordinary
  triples each helper can conclude. Skip a helper evaluation only when
  every such possible old triple is already affected or protected.
  Projections stop at ordinary triples, whose consequences are processed
  through the deletion delta. Equality conclusions are never skipped by
  this optimization. Head-directed rederivation against the survivors
  restores any extra types still supported by the final base.
- Share the rule-program parser between explanations and maintenance in
  the rules layer. Backward checks initially use ordinary triple rules;
  rules needing helper relations may conservatively use DRed.
- Any affected equality class or vocabulary-class change uses full
  reasoning. Initially, configurations with `trivial` or `literal-values`
  enabled also use full reasoning: their implicit term-presence rules
  require additional deletion handling. This is an explicit coverage gap,
  not an exemption from correctness checks.
- When old findings are empty, positivity proves a deletion cannot create
  one. Otherwise recompute diagnostic findings after a maintained deletion.
  Selective finding maintenance remains a performance gap.
- Cancellation discards partial maintenance just as it discards partial
  forward reasoning. A cancelled result leaves the final base loaded and
  the next run full. A pre-tripped handle changes nothing.
- Snapshots preserve pending deletions explicitly; an incompatible format
  is refused. A pending deletion invalidates an explanation cache just as
  any other mutation does. The next completed run can be explained and
  checked through the existing public interface.
- Choose and document a bounded full-run threshold using measurements;
  check the total pending batch, not only each individual removal.

Additional user stories:

9. As a caller, I want absent, duplicate and inferred-only removals handled safely, so that retries do not change the result.
10. As an editor author, I want replacing the base to handle deletions correctly, so that updating a document uses the same semantics as individual changes.
11. As a user, I want cancellation during a deletion update to recover safely, so that stopping stale work cannot corrupt the next result.
12. As a caller, I want a saved pending update to resume correctly, so that snapshots preserve unprocessed changes.
13. As a user, I want explanations after deletion to use the remaining facts, so that deleted statements do not appear as supporting evidence.
14. As a caller, I want reads to remain stable until reasoning finishes, so that a pending change cannot expose a partial answer.

Implementation proceeds through public-seam regression tests, the engine
operations and DRed, bounded backward checks, then lifecycle integration
and independent verification. Record every departure and unmet budget in
the ticket README and close-out record. No issue tracker is configured;
`/setup-engineering-workflows` can configure an external tracker later.

- **DRed in the engine**:
  - With no equality classes, overdeletion queries the shared rule program
    against the old closure and rederivation checks affected heads against
    the survivors. Helper relations use tabled positive fixpoints.
  - With unaffected equality classes, overdeletion replays the native
    families over deleted recent tuples. Rederivation runs one naive round
    over the survivors and keeps only overdeleted tuples that come back.
  - Insertion then continues semi-naively, as an addition does.
- **FBF** adds a per-family check: "does this tuple have a derivation from
  current tuples?", with a set of checked and proved tuples (AIJ 2019 §7).
  It is the second ticket.
- **Findings** are recomputed when they were nonempty. Selective finding
  maintenance remains open.
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
- Publishing numbers without the owner's approval.

## Principles

- **exhaust-the-design-space**: DRed's rederivation has two designs (one
  naive round against a head-directed check per family). Ticket 22 builds
  the first and measures it; FBF's check (ticket 23) is the second.
- **make-operations-idempotent**: a removal followed by putting the same
  triples back converges to the same closure as never removing them (a
  chaos check).
- **explain-the-number**: each removal number names whether the
  maintenance path or the fallback ran.
