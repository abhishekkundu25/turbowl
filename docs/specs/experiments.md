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
  version     changes     a worktree    vs. Baseline    against rules  the kept
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
   chaos, incremental, W3C) and the scoreboard's benchmark on the change and
   on the Baseline (the kept version), on a quiet machine, and writes a
   verification record (`docs/verification/<date>-e<NN>.md`).
5. **Decide.** Keep the change only if every correctness check passes, no
   budget in the current specs regresses, and the rubric score improves on
   the dimension the hypothesis named without a drop of more than 1 point
   elsewhere. Otherwise revert it. A partial win can be kept only if the
   reviewer of the next round agrees.
6. **Record.** Add the row to the table below (also for reverted
   experiments: a negative result is a result) and start the next round
   from the kept version.

## Rubric (each 0 to 10, scored by the reviewer)

Speed and memory are measured on the scoreboard (`CONTEXT.md`): this
reasoner against the competitors on the same scenarios, single-threaded and
on all cores.

| Dimension | 10 means | Evidence the reviewer must cite |
|---|---|---|
| Correctness | every oracle agrees, all W3C cases for implemented rules pass, no open documented difference without an oracle-backed test | the last verification record |
| Coverage | all of OWL 2 RL implemented (rules and diagnostics) | the rule table, the W3C pass list |
| Speed (full run) | first on the scoreboard: lower reasoning time and total time than every competitor on every scenario | the scoreboard record, with ratios |
| Speed (incremental) | first on the scoreboard's time per change, and a one-triple change costs about what the change implies, for additions and removals | the scoreboard's incremental scenarios |
| Memory | first on the scoreboard: lower peak memory per closure triple than every competitor | the scoreboard record |
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
- Numbers come from the scoreboard's benchmark (`crates/bench`) with nothing
  else running, 5 runs or more, median and spread, with what bounds the
  number named (`explain-the-number`).
- A reverted experiment stays in the table with what was learned.
- A change to an oracle, a filter or a budget is its own iteration, with the
  reason, and the reviewer of the next round checks it.
- Features may be added as experiments (a missing OWL 2 RL rule, an output
  format), rated like any other change.

## Iterations

| # | Date | Weakest dimension (from rating) | Change | Hypothesis | Result (numbers) | Rubric before → after | Decision |
|---|---|---|---|---|---|---|---|
| 0 | 2026-11-06 | Coverage: 39 of the 78 W3C rules | version 1 (the first Baseline) | – | Reasoning 3.0 ms on brick-small1, 128.5 ms on brick-buildings, 735 ms on lubm-50 (Nemo 38× to 97× slower); total 4.66 s on lubm-50, 84% of it loading; peak 1,483 MB on lubm-50 (150 B per closure triple); 0.2 ms per added triple, 2.8 ms per removal | – → 53/80 | Baseline |

### Iteration 0: the rating

Version 1 at `f9e3625`, rated from its code (a checkout of that commit)
and its records. There is no scoreboard yet: the only competitors measured
are Nemo (`docs/verification/2026-11-06-v1.md`, 5 runs, load average 1.1
to 2.3) and `reasonable` (`docs/verification/2026-11-05-wave-3.md`, 3 runs,
load average 7 to 11). Soufflé, VLog, RDFox (published results), GraphDB
and Jena are not measured, and no run used all cores. The speed and memory
scores below rate what these two comparisons show.

- **Correctness: 9/10.** All 51 minimal inputs, Brick with small1 and its
  three variants, soda hall and the 108 building models equal Nemo,
  closures and findings (`docs/verification/2026-11-05-wave-4.md`,
  Correctness table). The W3C OWL 2 RL tests pass all 93 cases Nemo
  passes, and the RDF-MT tests pass 37, including the dt-not-type cases
  for the checked datatypes (same table; `data/2026-11-05-wave-4/oracle-w3c.txt`).
  100,000 full and 100,000 incremental chaos runs agree with the reference
  (Chaos table). Every documented difference, F1 to F5, is an OWL-RL fault
  with a test (`crates/oracle/src/differences.rs` at `f9e3625`). Not 10:
  some judges do not run everywhere. OWL-RL does not run on Brick with
  small1, HermiT does not run on the `owl.n3` inputs or the five minimal
  inputs that make a reserved IRI equal to another term (wave 4 record,
  lines 50 to 63), and 41 chaos inputs are too large for the reference.
- **Coverage: 5/10.** Version 1 implements 39 of the 78 rules in the W3C
  OWL 2 RL tables (`docs/specs/v1.md`, "The rule set of version 1"; the
  rule names in `crates/turbowl/src/rules/` at `f9e3625`). It misses
  eq-ref, eq-diff1 to 3, prp-ap, prp-adp, prp-key, prp-npa1 and 2,
  cls-maxc1 and 2, cls-maxqc1 to 4, cls-oo, cax-adc, dt-type1 and 2, dt-eq,
  dt-diff and 18 of the 20 scm-* rules. dt-not-type checks only five
  datatypes (`docs/specs/v1.md`, Implementation Decisions, Datatypes). Of
  the 17 rules that conclude `false`, 7 are reported. TurbOWL passes 93 of
  the 119 runnable W3C OWL 2 RL cases, and OWL-RL passes 100
  (`data/2026-11-05-wave-4/oracle-w3c.txt`, the OWL 2 RL list of failures).
- **Speed (full run): 6/10.** TurbOWL is first on every scenario against
  both engines that were measured. Against Nemo, its reasoning time is 38×
  to 97× lower and its total time 8.7× to 55× lower
  (`docs/verification/2026-11-06-v1.md`, Speed and memory table;
  `data/2026-11-06-v1/v1-baseline.txt`). Against `reasonable`, its full run
  is 3.65× to 7.30× faster and its total 2.92× to 6.31× faster on a loaded
  machine (`docs/verification/2026-11-05-wave-3.md`, Speed and memory
  table). Not higher: the strongest competitors are not measured, nothing
  ran on all cores, both machines were loaded, and the comparison with
  Nemo uses version 1's rule set, half of OWL 2 RL. Loading is 84% of the
  total on lubm-50 and 86% on lubm-1 (v1 record, "What limits each
  number").
- **Speed (incremental): 5/10.** One added triple costs 0.2 ms against a
  3.0 ms full run on brick-small1 (v1 record, Speed and memory table,
  `incremental-add`), inside version 1's 10% budget. One removal costs
  2.8 ms, about a full run, because any removal runs again from the base
  (`incremental-remove`; `docs/specs/v1.md`, Implementation Decisions,
  Incremental runs). The rubric asks that a one-triple removal cost about
  what the change implies, and it does not. Nemo has no change stream.
  Against `reasonable`, an addition is 20.9× faster and a removal 4.46×
  faster (wave 3 record, Speed and memory table), on a loaded machine. The
  incremental scenarios cover one small input only (27,261 closure
  triples).
- **Memory: 6/10.** TurbOWL's peak is lower than Nemo's on every scenario,
  by 1.33× (lubm-50) to 4.21× (brick-buildings) (`data/2026-11-06-v1/v1-baseline.txt`,
  the `x turbowl mem` column). It is 147 to 166 bytes per closure triple on
  LUBM and 51 on brick-buildings (v1 record, "What limits each number").
  Against `reasonable` it is 5.49× to 9.81× lower (wave 3 record, Budgets
  table). Not higher: the scoreboard's other competitors are not measured,
  and the margin over Nemo shrinks as LUBM grows (2.01× on lubm-10, 1.33× on
  lubm-50).
- **Architecture: 8/10.** The public interface is the `Reasoner` with 12
  methods plus `Format`, `Summary`, `Diagnostic`, `Severity` and `Error`;
  everything else is `pub(crate)` (`crates/turbowl/src/lib.rs` at
  `f9e3625`). The rule families register one line each in a static table
  (`rules/mod.rs`, `FAMILIES`), which has a second adapter in tests
  (`Reasoner::with_families`, `reasoner.rs`). The engine knows no rules.
  The `io` feature is a seam with two adapters (native and wasm32). Not
  higher: the engine keeps leapjoin, antijoin and `join_static` that no
  rule calls, behind `#![allow(dead_code, unused_imports)]`
  (`engine/mod.rs:57`), so `engine/leapjoin.rs` (315 lines) fails the
  deletion test for the reasoner itself.
- **Embeddability: 8/10.** The core builds for `wasm32-unknown-unknown`
  (wave 4 record, The test suites; `scripts/check-wasm.sh`). The interface
  is `Send`, checked at compile time (`reasoner.rs:157`). The core has no
  global mutable state; the one `Mutex` is in the test-only `scaling.rs`.
  It has four direct dependencies, `oxrdf`, `oxttl`, `oxrdfxml` and
  `rustc-hash` (`cargo tree -p turbowl -e normal`). The Python API covers
  every `Reasoner` method, and pytest checks all 51 minimal inputs against
  the Rust library (wave 4 record, The test suites, 287 passed). Not
  higher: the wasm32 build turns `io` off, so a browser embedder cannot
  parse a document and must insert triples, and `oxrdf` pulls in
  `rand` and `getrandom`, which need a backend chosen on wasm32.
- **Operability: 6/10.** `Error` gives the file, line and column of a
  parse error (`crates/turbowl/src/error.rs`). Each `Diagnostic` carries
  its W3C rule as `code`, a severity, a message and its terms, in an order
  documented per code (`diagnostics.rs`). The CLI documents its report,
  stable JSON and NDJSON fields and exit codes 0, 1, 2 and 64
  (`crates/cli/src/main.rs`, crate doc), and its 20 tests pass (wave 4
  record, The test suites). Not higher: `f9e3625` has no `README.md` and
  no `CHANGELOG.md` (ticket 14 left them open), the terms of a diagnostic
  are positional, not named fields, the CLI does not read stdin, and the N3
  statements dropped on load are not reported (`docs/tickets/v1/README.md`,
  log entry 12).

Total: 53/80.

The weakest dimension is coverage: version 1 implements half of the W3C
OWL 2 RL rules and 7 of the 17 rules that find inconsistencies, and
coverage is a gate on the scoreboard, so no speed result counts as a claim
about OWL 2 RL until the missing rules are in.

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
