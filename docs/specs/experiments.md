# The improvement loop: rate, change, evaluate, decide

Status: starts when version 1 (`docs/specs/v1.md`) is verified.
This is the main way this project improves after version 1. Every change
after version 1 goes through it, and every iteration is recorded here.

## The loop

Each iteration runs these six steps, in order, with different agents for
rating and for changing, so that nobody grades their own work.

```
        ┌──────────────────────────────────────────────────────────┐
        ▼                                                          │
  1. RATE ──► 2. FIND ──► 3. CHANGE ──► 4. EVALUATE ──► 5. DECIDE ──► 6. RECORD
  reviewer    reviewer    implementer   verification    orchestrator   table row,
  scores the  proposes    applies one   suite + bench   keeps or       next round
  current     ranked      proposal in   on the change   reverts        starts from
  version     changes     a worktree    vs. baseline    against rules  the kept
                                                                       version
```

1. **Rate.** A fresh reviewer agent reads the code, the last verification
   record and this table, and scores the current version on the rubric
   below. It does not see the implementer's notes.
2. **Find.** The same reviewer proposes up to five changes, each with a
   hypothesis ("leapjoin for prp-spo2 cuts Brick time by 30% because ..."),
   the expected effect on the rubric, the files it touches, and the risk.
   It ranks them by expected gain per effort. The orchestrator picks the top
   one that is not a repeat of a reverted experiment (unless the reviewer
   says what is different this time).
3. **Change.** An implementer agent, in its own worktree, makes only that
   change, with tests. It does not tune the benchmark or the oracles.
4. **Evaluate.** The orchestrator runs the full verification (every oracle,
   chaos, incremental, W3C) and the benchmark against the kept baseline, on
   a quiet machine, and writes a verification record
   (`docs/verification/<date>-e<NN>.md`).
5. **Decide.** Keep the change only if every correctness check passes, no
   budget in the v1 spec regresses, and the rubric score improves on the
   dimension the hypothesis named without a drop of more than 1 point
   elsewhere. Otherwise revert it. A partial win can be kept only if the
   reviewer of the next round agrees.
6. **Record.** Add the row to the table below (also for reverted
   experiments: a negative result is a result) and start the next round
   from the kept version.

## Rubric (each 0 to 10, scored by the reviewer)

| Dimension | 10 means | Evidence the reviewer must cite |
|---|---|---|
| Correctness | every oracle agrees, all W3C cases for implemented rules pass, no open documented difference without an oracle-backed test | the last verification record |
| Coverage | all of OWL 2 RL implemented (rules and diagnostics) | the rule table, the W3C pass list |
| Speed (full run) | faster than every open reasoner on every benchmark scenario | the benchmark record, with ratios |
| Speed (incremental) | a one-triple change costs about what the change implies, for additions and removals | the incremental scenarios |
| Memory | lowest peak per closure triple of the compared reasoners | the benchmark record |
| Architecture | one deep module, small interface, no shallow layers, every seam has two adapters or a test reason | a walk of the crate with the deletion test |
| Embeddability | builds for wasm32, no global state, `Send`, small dependency set, Python bindings current | build logs, `cargo tree` |
| Operability | clear errors, diagnostics with structured fields, CLI behaviour, docs current | CLI tests, README |

The total (out of 80) goes in the table. The reviewer also writes one
sentence on the weakest dimension, and that guides the next find step.

## Rules

- One change per iteration. If a proposal needs two changes, it is two
  iterations.
- Correctness never trades for speed: a failing oracle is an automatic
  revert.
- Numbers come from the benchmark with nothing else running, 5 runs or more,
  median and spread, with what bounds the number named
  (`explain-the-number`).
- A reverted experiment stays in the table with what was learned.
- A change to an oracle, a filter or a budget is its own iteration, with the
  reason, and the reviewer of the next round checks it.
- Features may be added as experiments (a missing OWL 2 RL rule, an output
  format), rated like any other change.

## Iterations

| # | Date | Weakest dimension (from rating) | Change | Hypothesis | Result (numbers) | Rubric before → after | Decision |
|---|---|---|---|---|---|---|---|
| 0 | | | version 1 (baseline) | | | – → | baseline |

## For the next agent

To continue the loop cold: read `AGENTS.md`, then this file from the top,
then the newest file in `docs/verification/`. Start at step 1 with a fresh
reviewer. The candidates the last reviewer ranked but that were not picked
are listed below; a new reviewer may reuse or replace them.

### Open candidates

(filled by each round's reviewer)

Before the first round: `docs/research/sota-algorithms-2026-11.md` §10 ranks
the starting candidates with hypotheses, expected effects and how to measure
them on `crates/bench`. In order: P0 (counters and a memory breakdown, before
any claim), E2 (a closure module for scm-sco), E8 (drop cax-eqc1/2), E1
(cax-sco only from types it did not produce), E3 (schema first), E9 (inverse
tables as views), E4 (DRed for removals), E5 (parallel rounds, native only),
E6 (a faster sort, if the profile says so) and E7 (a compact `rdf:type`
object index). §11 lists what not to do and why.

Found by the scaling tests: E10, a blocked Bloom filter on each large stable
tier of a variable, so a round's duplicate check costs O(1) per tier instead
of a gallop. Hypothesis: many-round inputs (long chains, incremental edits)
go from N·log²(N/b) to N log N in the duplicate check, for about 12 to 15%
more memory. Measure: `scaling_variable_small_batches_comparisons`, the
chain and incremental scenarios, and peak memory on brick-buildings and
lubm-50. Few-round inputs (Brick, LUBM) should not change.
