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
| 1 | 2026-11-06 | Incremental speed (a removal costs a full run, phase 3's); within phase 2, memory: 151 B per closure triple on lubm-50 against 50 | P0: counters and a memory breakdown (`fafa7b6`), `Reasoner::profile` and the bench's schema 8 | Makes every later claim measurable; costs no time beyond the spread | Editor-size A/B, TurbOWL, 5 runs, before and after interleaved twice, load 2.7 to 3.4: every scenario within the spread (lubm-50 reasoning 105 to 108 ms before, 102 to 104 after; brick-small1 2.4 to 2.7, 2.4). lubm-50 breakdown: peak 161.6 B per closure triple, of which the reasoner holds 39.5 after the run (dictionary 19.8, base 7.6, store 12.1); the other 122.0 is transient, and the 934 MB input text alone is about 90 | 58 → 58 (instrumentation: no dimension moves) | Kept |
| 2 | 2026-11-06 | (iteration 1's rating, memory first) | Read N-Triples and Turtle files in 32 MiB windows instead of whole (`b1d4b93`) | The whole input text is about 90 of lubm-50's 161.6 bytes per closure triple at the peak; windows should bring the peak to about 70 | lubm-50, 3 runs each, before and after, load 2.1 to 2.3: peak on one thread 1,422 → 567 MB (144.1 → 57.5 B per closure triple), on all cores 1,616 → 643 MB (163.8 → 65.2). Held after the run, unchanged: dictionary 19.8, base 7.6, store 21.2 on one thread and 12.1 on all cores. Transient: 8.8 on one thread, 25.6 on all cores. Total time 1.87 → 1.93 s on one thread (loading +4%), 0.88 → 0.92 s on all cores. Editor-size A/B, one thread, 5 runs twice interleaved: reasoning within the spread everywhere; lubm-10's peak 269 → 134 MB, its total 339 → 361 ms | Memory up, no other dimension moves a point | Kept: the memory budget gains 87 to 99 B per triple; loading stays under its bar |
| 3 | 2026-11-06 | Incremental speed (phase 3's); within phase 2, memory: 57.5 and 65.2 B per closure triple against 50 | Relations sized to their contents: spare room above an eighth given back after a sort, a merge or a filter, exact growth in merges, empty recent parts freed (`eebb962`) | On one thread the store holds 21.2 B per triple against 12.1 on all cores for the same pairs; fitting it should bring the one-thread peak from 57.5 to about 49 to 51 | lubm-50, 3 runs each, load 1.5 to 1.7: one thread 567 → 525 MB (57.5 → 53.2 B per closure triple; store 21.2 → 12.2), all cores 601 → 603 MB (60.9 → 61.1, unchanged). Editor-size A/B, one thread, 5 runs twice interleaved: times within the spread everywhere; peaks lubm-10 134 → 123 MB, brick-buildings 52 → 45 MB. Half of the store's 93 MB had never been resident; the peak is now the held 39.6, about 100 MB the allocator keeps after the run frees it, and 17 to 25 MB allocated after the last step (`docs/verification/2026-11-06-e03.md`) | 58 → 58 (memory moves, not by a point) | Kept |
| 4 | 2026-11-06 | Incremental speed (phase 3's); within phase 2, memory: 53.2 and 61.1 B per closure triple against 50 | Dictionary hash slots up to three quarters full instead of half (`d21803b`) | The slots are 67.1 MB on lubm-50, all resident; three quarters halves them, 3.2 B per triple in both modes | lubm-50, 3 runs each, load 1.9 to 2.8: one thread 524 → 463 MB (53.1 → 47.0 B per closure triple: under the budget), all cores 599 → 603 MB by the largest of 3 runs, but 629 → 605 MB by the median of the per-run peaks (corrected by iteration 6's reviewer: the largest run is noise on all cores). Editor-size A/B, one thread, 5 runs twice interleaved: within the spread everywhere. A miss up to 1.6 times slower; loading unchanged (`docs/verification/2026-11-06-e04.md`) | 58 → 58 | Kept: the one-thread memory budget is met |
| 5 | 2026-11-06 | (iteration 4's rating) | A full run gives each table its base as one exact sorted batch, skipping step 0's sort and copy (`1e74f80`) | The base pushed through doubling round buffers leaves freed blocks the allocator keeps: 2 to 5 B per closure triple less on one thread, 30 to 100 MB less on all cores | lubm-50, 3 runs each, load 2.3 to 2.5: one thread 463 → 458 MB (47.0 → 46.4), all cores 605 → 603 MB by the median of the per-run peaks (617 → 577 by the largest run, which is noise). Reasoning on one thread 416 to 418 → 403 to 404 ms on lubm-50, 75.5 → 72.8 ms on lubm-10; all-cores sort time 108 → 75 ms. The memory hypothesis failed: the round buffer reused memory the loader had freed (`docs/verification/2026-11-06-e05.md`) | 58 → 58 | A partial win (speed, not the memory it named): kept, iteration 6's reviewer agreeing |
| 6 | 2026-11-06 | Incremental speed (phase 3's); within phase 2, all-cores memory: 58.3 B per closure triple at the median against 50 | The parallel merge and filter write into one vector by disjoint ranges, not pieces joined by `concat` (branch `exp/e06-one-vector-merge`) | The all-cores peak falls in a step's change phase, where the pieces and their joined copy are alive together: 20 to 30 MB less | lubm-50, all cores, 10 single runs per side alternating, load 3.1 to 5.6: median peak 601.9 → 594.4 MB, middle halves 600.9 to 602.5 and 591.0 to 596.4; reasoning unchanged. The probe: live bytes never pass 461 MB (the budget is 517.3); the footprint ends about 200 MB above them, memory the allocator keeps after large short-lived allocations (`docs/verification/2026-11-06-e06.md`) | 58 → 58 | Reverted: 7.5 MB, under the 10 MB bar |
| 7 | 2026-11-07 | Incremental speed (phase 3's); within phase 2, the all-cores footprint (58.2 B per closure triple at the median against 50) and speed-up (3.94 against 5) | Object indexes flipped into one exact vector and sorted in place on the workers, with no buffer and no deduplication (`f4fc0dd`) | The first index builds grow the footprint 27.8 MB for a 9.1 MB index: building in place should save 10 to 30 MB of peak and 3 to 6 ms | lubm-50, all cores, 10 alternating single runs per side, load 2.2 to 3.2: median peak 601.0 → 580.7 MB, middle halves 600.5 to 602.1 and 580.3 to 582.0; largest 602.9 → 588.2 MB. Implementer's pairs: reasoning 102.1 → 98.2 ms (4.1 times one thread). One thread unchanged at 46.4 B per triple. Editor-size A/B within the spread (`docs/verification/2026-11-07-e07.md`) | 58 → 58 | Kept |
| 8 | 2026-11-07 | Incremental speed (phase 3's); within phase 2, the all-cores footprint: 580.7 MB at the median against 517.3 | The base as sorted pairs by predicate in one flat vector, built in the parse buffer (`b654de6`); snapshots at format version 2 | The base holds 12 bytes a triple; pairs by predicate hold 8: 26.3 MB less held, 18 to 26 MB off the peak | lubm-50, all cores, 10 alternating single runs per side, load about 3: median peak 581.5 → 544.2 MB, middle halves 580.5 to 582.3 and 542.9 to 547.2, largest 583.5 → 548.3. One thread 46.9 → 46.6 B per triple. Reasoning on one thread 407 → 395 ms on lubm-50 (`docs/verification/2026-11-07-e08.md`) | 58 → 58 | Kept |
| 9 | 2026-11-07 | (iteration 8's rating) | During a large full run on several threads, the dictionary drops its lookup table and rebuilds it in id order at the next change (`fb500d8`, reverted by `38df2c3`; branch `exp/e09-dict-drop`); the owner accepted the first change's wait | The table is 33.6 MB, all resident: 20 to 30 MB off the peak | On top of iteration 8, lubm-50 on all cores, 10 alternating single runs per side: median peak 543.8 → 536.3 MB, middle halves apart, largest 547.1 → 539.5. Alone, before iteration 8, it gave 45 MB: the two overlap, since both free memory the parallel steps reuse. One thread unchanged. The first change after a large parallel run waits about 28 ms (`docs/verification/2026-11-07-e09.md`) | 58 → 58 | Reverted: 7.5 MB on top of iteration 8, under the 10 MB bar, for a wait and a rebuild path |
| 10 | 2026-11-07 | Incremental speed (phase 3's); within phase 2, the all-cores speed-up: 3.6 to 4.1 against 5 (the memory gap now accepted by the owner) | A first object index built inside its table's job, beside its sort (branch `exp/e10-index-in-job`) | 9.5 ms of first index builds run on the caller's thread before the jobs start: 3 to 5 ms off all-cores reasoning | lubm-50, 10 alternating single runs per side on all cores, load 1.7 to 2.0: reasoning median 94.4 → 92.7 ms, middle halves overlapping; ratio 4.10 → 4.17. `rdf:type`'s job, its index now built after its sort, sets the step (`docs/verification/2026-11-07-e10.md`) | 58 → 58 | Reverted: within the noise |
| 11 | 2026-11-07 | (iteration 10's rating: the code had not changed) | One split per property rule over all its axioms, instead of one per axiom: counted first, not built | Round 1's property family gains nothing on all cores because its splits are many and small: 3 to 4 ms | The count refuted it: 24 large splits in 604 chunks. On all cores the family does about 34 ms of CPU against 8.3 on one thread: writing pairs costs twice as much, and each chunk sorts its output. Round 1 nearly saturates 12 cores (6 of them Efficiency) (`docs/verification/2026-11-07-e11.md`) | 58 → 58 | Not built: the reading was wrong |

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

### Iteration 1: the rating

The `v2` branch at `d6c42f5`, rated from its code and its records. It has
phase 1's rules and switches, and phase 2's tickets 16 to 19: TurbOWL's own
parsers, parallel rounds, parallel parsing and snapshots. The last full
record (`docs/verification/2026-11-06-v2-phase-1.md`) ran on `660e3bd`,
before phase 2, so its correctness runs and its scoreboard do not cover
the code rated here. The scoreboard ran at load 1.8 to 2.9, on one thread,
in the old default (`schema` and `identity` on), and stopped early. Phase
2's numbers come from the tickets' log (`docs/tickets/v2/README.md`,
entries 16 to 19): medians of 3 at load 2 to 4, indicative, not a record.
No engine has run on all cores on the scoreboard, and no peak memory has
been measured since phase 1.

- **Correctness: 8/10.** On `660e3bd` every judge agrees in both
  configurations: the minimal inputs, Brick with its variants, soda hall
  and the 108 building models equal Nemo; the W3C OWL 2 RL tests pass 101
  of 119 runnable cases (Nemo 101, OWL-RL 100) and RDF-MT 35; 100,000 full
  and 100,000 incremental chaos runs agree with the reference (phase 1
  record, Final code). Phase 2 kept its own checks: the parse in pieces
  equals the sequential parse on 16,100 corpus parses and 120,000
  generated documents (ticket 18), chaos compares one thread with all
  cores after every `reason()` (ticket 17), and a snapshot round trip
  gives the same closure (ticket 19). Not higher: no oracle suite and no
  100,000-iteration chaos run has checked the new parsers, the parallel
  rounds and the snapshots together. The judges' gaps of iteration 0
  remain: OWL-RL does not run on Brick with small1, HermiT does not run
  on the `owl.n3` inputs, and 848 full and 824 incremental chaos inputs
  are too large for the reference.
- **Coverage: 9/10.** All 78 rules of the W3C tables are in
  (`crates/turbowl/src/rules/mod.rs`, the family table; `CHANGELOG.md`),
  with the 32 datatypes of the OWL 2 RL map (ticket 10's log). TurbOWL
  passes every W3C OWL 2 RL case that Nemo passes, 101 of 119, and 15 of
  the 18 failures expect a conclusion that no rule derives (phase 1
  record, "The W3C cases beyond the rules"). Not 10: three failures come
  from the runner, which loads no imported ontology (`WebOnt-imports-011`)
  and keeps annotations in Direct-only conclusions (the two `-Direct`
  cases). Until the runner reads them as the W3C does, the pass list
  cannot show every applicable case passing.
- **Speed (full run): 8/10.** On the phase 1 scoreboard TurbOWL leads all
  46 scenario-axis pairs in the old default and all 22 in the complete
  configuration where some engine finished. Its total time is 2.5×
  (lubm-1) to 516× (owl2bench-rl-2) lower than the fastest other engine,
  and it alone finished uobm-010-u, owl2bench-rl-5 and claros-l. Phase 2
  cut lubm-50's reasoning on one thread from 739 to 803 ms to 423 to
  438 ms, and to 103 to 114 ms on all cores (ticket 17). It cut the parse
  from 0.833 s on one thread to 0.194 s on all cores (ticket 18). Not
  higher: the margins are smallest on LUBM (2.5× to 3.2× against
  Soufflé), where loading was 79% of the total. The scoreboard has no
  all-cores axis for any engine and no RDFox estimate. GraphDB and Jena
  are not measured. The new default has not been raced, and the machine
  was busy.
- **Speed (incremental): 5/10.** Unchanged since iteration 0. One added
  triple costs 0.1 ms on brick-small1 (old default). One removal costs
  3.9 ms, or 2.8 ms with `schema` off, about a full run, because any
  removal runs again from the base (phase 1 record, the scoreboard and
  the speed table). No competitor supports changes. Phase 3's DRed
  (ticket 22) has not started, and the incremental scenarios cover one
  small input.
- **Memory: 6/10.** On the phase 1 scoreboard TurbOWL has the lowest peak
  on every scenario where some engine finished. On lubm-50 that is
  1,486 MB, against 1,957 MB for Nemo, 2,770 MB for VLog and 3,005 MB for
  Soufflé: 151 bytes per closure triple, 1.32× under Nemo
  (`data/2026-11-06-v2-phase-1/B-default-editor.txt`). At rest the state
  is smaller: lubm-50's snapshot is 30 bytes per closure triple, with
  107 MB of terms, 79 MB of base and 125 MB of stored pairs
  (`crates/turbowl/src/snapshot.rs`, module doc). Not higher: the margin
  is narrowest on lubm-50. The papers put RDFox at 51 and VLog at about
  12 bytes per triple (`docs/specs/v2-2-speed.md`), and phase 2's budget
  of 50 is a third of today's figure. No peak has been measured since
  phase 1, while ticket 17 says a large table's peak can briefly double on
  all cores and ticket 18 holds about 80 MB of pieces during a parse.
- **Architecture: 8/10.** The public interface is the `Reasoner`, now 22
  methods with the switches, the threads and four snapshot methods, plus
  `Format`, `Summary`, `Diagnostic`, `Severity`, `Error`, `Switch`,
  `Switches` and `Threads` (`crates/turbowl/src/lib.rs:124-128`). Every
  other item is `pub(crate)`. The rule table registers each family in one
  line with its switch (`rules/mod.rs`). Each parallel operation has a
  one-thread form that gives the same result (`engine/parallel.rs`), so
  one thread and many share one code path. Not higher: the engine still
  keeps leapjoin, antijoin and `join_static`, which no rule calls, behind
  `#![allow(dead_code, unused_imports)]` (`engine/mod.rs:68`), and
  `Dictionary::reserve` is unused (`term.rs:186`). The snapshot reads the
  dictionary's private fields from an `impl` block inside `term.rs`
  (ticket 19's log). `reasoner.rs` has 1,656 lines, `rules/checks.rs`
  1,624 and `snapshot.rs` 1,472; splitting `reasoner.rs` is an open
  follow-up.
- **Embeddability: 7/10.** The core builds for wasm32 without `io` and
  `parallel` (`scripts/check-wasm.sh`; ticket 17's acceptance). The direct
  dependencies are `oxrdf`, `oxttl`, `oxrdfxml`, `oxiri`, `rustc-hash`,
  and `rayon` off WebAssembly (`crates/turbowl/Cargo.toml`). The Python
  package has the snapshots (`crates/python/src/lib.rs:90-101`). Not
  higher: Python has no thread setting while the Rust interface has
  `Reasoner::set_threads` (ticket 17's log: only `RAYON_NUM_THREADS`).
  `Threads::ALL` runs on rayon's global pool, which an embedder's own
  rayon work shares (`threads.rs`). The wasm32 build still has no parser,
  and `oxrdf` still needs a `getrandom` backend there.
- **Operability: 7/10.** `README.md` and `CHANGELOG.md` exist now (ticket
  15). A parse error names its file, line and column. A refused snapshot
  is an error with exit code 1, and the CLI documents `--save-snapshot`,
  `--load-snapshot` and the exit codes 0, 1, 2 and 64
  (`crates/cli/src/main.rs`, crate doc). Each diagnostic carries its rule
  as `code`, a severity, a message and its terms in a documented order.
  Not higher: neither `README.md` nor `CHANGELOG.md` has changed since
  `529a5d5`, so snapshots, threads and parallel loading are missing from
  both. The terms of a diagnostic are positional, not named fields. The
  CLI reads `-` as a file name, not stdin (`main.rs:501`), and has no
  thread option. The N3 statements dropped on load are not reported, and
  `Error` gained a variant without being `#[non_exhaustive]`.

Total: 58/80.

The weakest dimension is incremental speed, because a one-triple removal
still costs a full run until phase 3's DRed; inside phase 2's scope it is
memory, at 151 bytes per closure triple on lubm-50 against a budget of 50,
and the candidates below start there.

### Iteration 3: the rating

The `v2` branch at `21a789c`: the code iteration 1 rated, plus P0
(`fafa7b6`) and the windowed reading (`b1d4b93`). Rated from the code, the
newest record (`docs/verification/2026-11-06-v2-phase-1.md`, on
`660e3bd`) and iteration 2's evaluation: an editor-size A/B on one thread
(5 runs, twice interleaved) and the lubm-50 breakdown in both thread modes
(3 runs), at load 2.1 to 3.2. Iterations 1 and 2 have no verification
record: `docs/verification/` has nothing after phase 1, although step 4
asks for one. The evaluation script ran the default suites (the minimal
inputs, W3C, Brick, the reference, chaos at its default count, pytest),
but their output is not kept in the repository. The scoreboard is still
phase 1's. P0 reads the bytes held after the run, not at the peak, so
"not counted" mixes loading's transients, the steps' transients, the
allocator's caches and the binary.

- **Correctness: 8/10.** Unchanged in substance since iteration 1. The
  phase 1 record has every judge agreeing on `660e3bd`. The windowed
  reader has its own judge: the cross-check reads every document the tests
  read in windows of every small size and compares it with the whole
  parse (`io/cross_check.rs`, `b1d4b93`). The closure of lubm-50 is
  10,345,697 triples before and after iteration 2, on one thread and on
  all cores. Not higher: no oracle suite, slow tier or 100,000-iteration
  chaos run has checked phase 2's parsers, parallel rounds, snapshots or
  windows (1,068 changed lines in the parse path alone since the last
  record). The judges' gaps of iteration 0 remain. The evidence is weaker
  than iteration 1's: two kept iterations have no record to cite.
- **Coverage: 9/10.** Unchanged: no rule changed since iteration 1. All
  78 rules are in, and TurbOWL passes the 101 of 119 runnable W3C OWL 2 RL
  cases Nemo passes. Not 10 for iteration 1's reason: three failures come
  from how the runner reads a case.
- **Speed (full run): 8/10.** On lubm-50, reasoning takes 416.8 ms on one
  thread and 107.3 ms on all cores (medians of 3, load 2 to 3). The total
  is 1.93 s and 0.92 s, against 15.6 s for Soufflé, the fastest other
  engine on the phase 1 scoreboard (8.1 and 17 times). That comparison
  crosses records, configurations and machine loads. The editor-size
  scenarios stayed within the spread through iteration 2. Not higher: the
  scoreboard predates phase 2, no competitor has run on all cores, and
  RDFox has no estimate. All cores give 3.9 times one thread (3.5 to 4.1
  over the runs) against the budget's 5. Iteration 2 made loading 2 to 4%
  slower. Loading is 0.81 s on all cores and 1.52 s on one thread,
  against a bar of 1.30 s (a third of version 1's 3.9 s). The spec does
  not name the thread mode, so the bar is met on all cores only.
- **Speed (incremental): 5/10.** Unchanged. One added triple costs
  0.2 ms and one removal 1.9 to 2.0 ms, against a full run of 2.3 to
  5.0 ms on brick-small1 (`incremental-*` in iteration 2's A/B). A removal
  still runs again from the base.
- **Memory: 7/10.** Up 1. Iteration 2 cut lubm-50's peak from 1,422 MB to
  567 MB on one thread (57.5 bytes per closure triple) and from 1,616 MB
  to 643 MB on all cores (65.2). That is 3.0 to 3.5 times under Nemo's
  1,957 MB and 4.7 to 5.3 times under Soufflé's 3,005 MB (phase 1
  scoreboard). Not higher: the budget of 50 is missed in both modes. The
  papers put VLog at about 12 bytes per triple. The comparison crosses
  records and configurations. The all-cores peak varies by 51 MB over 3
  runs (624 to 675 MB), so the scheduling decides part of it.
- **Architecture: 7/10.** Down 1. The interface gained `Reasoner::profile`
  and the public `profile` module, for one adapter (the bench), and the
  crate doc says so (`lib.rs`). That follows `AGENTS.md`. The rest went
  the wrong way. `reasoner.rs` grew from 1,656 to 2,157 lines (tests
  included), and the document key (`DocumentKey`, `Documents`, about
  160 lines) lives there, not in `io`. `io::parse_reader_on` takes 9
  arguments, and `io::read` passes 11 to `parallel::parse`, two of them
  bare booleans, under `#[allow(clippy::too_many_arguments)]`. The engine
  still keeps unused code behind `#![allow(dead_code, unused_imports)]`
  (`engine/mod.rs:68`), and `Dictionary::reserve` is still unused
  (`term.rs:193`). The one-thread and parallel forms of each step give the
  same tuples but not the same memory: 21.2 against 12.1 bytes per closure
  triple of store (see the open candidates).
- **Embeddability: 7/10.** Unchanged. The core builds for wasm32 without
  `io` and `parallel` (the evaluation's `check-wasm.sh`). Python still has
  no thread setting (`crates/python/src/lib.rs` has none), and the wasm32
  build still has no parser.
- **Operability: 7/10.** Unchanged. A parse error still names its file,
  line and column, and the windowed reader counts the lines before each
  window (`b1d4b93`). Not higher: `README.md` and `CHANGELOG.md` have not
  changed since `529a5d5`, so snapshots, threads, parallel loading and
  windowed reading are missing from both. The CLI reads `-` as a file name
  (`main.rs:501`) and has no thread option. The terms of a diagnostic are
  positional, and `Error` is not `#[non_exhaustive]`.

Total: 58/80.

The weakest dimension is incremental speed, because a one-triple removal
still costs a full run until phase 3's DRed. Inside phase 2 it is memory:
57.5 bytes per closure triple on one thread and 65.2 on all cores, against
a budget of 50. The candidates below start there, and the all-cores
candidates also serve the 5-times budget.

### Iteration 4: the rating

The `v2` branch at `a7710d5`: the code iteration 3 rated, plus iteration
3's change (`eebb962`, relations sized to their contents). Rated from the
code, the three loop records (`docs/verification/2026-11-06-e01.md` to
`-e03.md`, raw results in `data/2026-11-06-e01/` to `-e03/`) and the
phase 1 record. Iteration 3's lubm-50 breakdown ran 3 times in each thread
mode at load 1.5 to 1.7. Its editor-size A/B ran 5 times, twice
interleaved, on one thread at load 1.9 to 2.5. No loop record ran on a
quiet machine, and the scoreboard is still phase 1's. The bench's "MB" are
MiB: lubm-50's 525 MB on one thread is 550.7 million bytes, and the budget
of 50 bytes per closure triple is 517.3 million bytes (493 MiB).

- **Correctness: 8/10.** Every suite passed on each kept tree (each
  record's Correctness section): the core library (313 tests at
  `eebb962`) and its doc tests, the CLI 29, the oracle library 37 with
  `minimal`, `programs`, `reference`, `w3c`, `data_brick` and `snapshot`,
  the reference evaluator's 15 and 25 and `chaos` at its default count,
  the bench 42, pytest 787, fmt, clippy and the wasm32 core
  (`data/2026-11-06-e03/suites-and-runs.txt`). Iterations 1 and 2 now have
  their records, which iteration 3's rating missed. Iteration 3 changes no
  tuple: lubm-50's closure stays 10,345,697 triples in both thread modes.
  Not higher: the slow oracle tier and the 100,000-iteration chaos runs
  have not run since the phase 1 record (`660e3bd`). Phase 2's parsers,
  parallel rounds, snapshots and windows, and iteration 3's sizing, are
  checked by the default suites only. The judges' gaps of iteration 0
  remain.
- **Coverage: 9/10.** Unchanged: no rule changed since iteration 1. All
  78 rules are in, and TurbOWL passes the 101 of 119 runnable W3C OWL 2 RL
  cases that Nemo passes. Not 10 for iteration 1's reason: three failures
  come from how the runner reads a case.
- **Speed (full run): 8/10.** On lubm-50, reasoning takes 416.9 to
  417.1 ms on one thread and 101.7 to 102.6 ms on all cores (e03, 3 runs
  each). The total is 1.94 s and 0.91 s, 8.0 and 17 times under Soufflé's
  15.6 s on the phase 1 scoreboard, a comparison across records and
  configurations. Every editor-size scenario stayed within the spread
  (e03's A/B). Not higher: all cores give 4.1 times one thread, against
  the budget's 5. By phase, the rounds take 118.2 against 35.8 ms (3.3
  times), the steps 272.3 against 62.2 ms (4.4 times), and the rest 26.6
  against 3.9 ms. Loading takes 1.52 s on one thread, against a bar of
  1.30 s. No competitor has run on all cores, RDFox has no estimate, and
  the machine was busy.
- **Speed (incremental): 5/10.** Unchanged. One added triple costs
  0.2 ms and one removal 1.9 to 2.0 ms, against a full run of 2.3 to
  2.8 ms on brick-small1 (e03's A/B). A removal still runs again from the
  base.
- **Memory: 7/10.** lubm-50's peak is 53.2 bytes per closure triple on
  one thread (525 MB, down from 57.5) and 61.1 on all cores (603 MB,
  unchanged) (e03). That is 3.2 to 3.7 times under Nemo's 1,957 MB and 5.0
  to 5.7 times under Soufflé's 3,005 MB (phase 1 scoreboard). The store
  now holds 12.2 bytes per triple on one thread, as on all cores, and
  brick-buildings' peak fell from 52 to 45 MB. Not higher: the budget of
  50 is missed in both modes, by 3.2 bytes per triple on one thread and
  11.1 on all cores. The reasoner holds 39.6 bytes per triple after the
  run. On one thread most of the other 13.6 is memory freed during the
  run that stays resident (the open candidates have the account). The
  papers put VLog at about 12.
- **Architecture: 7/10.** Better in substance, not by a point. Iteration 3
  removed a fault the last rating named: the one-thread and parallel forms
  of a step now leave the same memory (12.2 and 12.1 bytes per triple),
  through one rule, `fit` (`engine/relation.rs`), documented where it
  applies. The interface did not change. Not higher: `reasoner.rs` still
  has 2,157 lines, with the document key inside. Four functions in `io`
  and one in `snapshot.rs` take too many arguments under
  `#[allow(clippy::too_many_arguments)]` (`io.rs:222`, `io.rs:284`,
  `io/parallel.rs:206`, `io/parallel.rs:496`, `snapshot.rs:1200`). The
  engine still keeps unused code behind `#![allow(dead_code,
  unused_imports)]` (`engine/mod.rs:68`), and `Dictionary::reserve` is
  still unused (`term.rs:193`).
- **Embeddability: 7/10.** Unchanged. The core builds for wasm32 without
  `io` and `parallel` (e03's suites). Python still has no thread setting
  (`crates/python/src/lib.rs` has none), and the wasm32 build still has no
  parser.
- **Operability: 7/10.** Unchanged. A parse error names its file, line and
  column. Not higher: `README.md` and `CHANGELOG.md` have not changed
  since `529a5d5`, so snapshots, threads, parallel loading and windowed
  reading are missing from both. The CLI reads `-` as a file name
  (`main.rs:501`) and has no thread option. The terms of a diagnostic are
  positional, and `Error` is not `#[non_exhaustive]`.

Total: 58/80.

The weakest dimension is incremental speed, because a one-triple removal
still runs again from the base until phase 3's DRed; inside phase 2 it is
memory, 3.2 bytes per closure triple over the budget on one thread and
11.1 on all cores, and the candidates below start there.

### Iteration 6: the rating

The `v2` branch at `cf0848c`: the code iteration 4 rated, plus iteration
4's change (`d21803b`, dictionary slots to three quarters) and iteration
5's (`1e74f80`, a full run from exact sorted batches). Rated from the code
and the five loop records (`docs/verification/2026-11-06-e01.md` to
`-e05.md`), with their raw results read run by run. Iteration 5's
evaluation ran at load 1.9 to 2.9: the editor-size A/B on one thread (5
runs, twice interleaved) and the lubm-50 breakdown (3 runs in each thread
mode). No loop record ran on a quiet machine, and the scoreboard is still
phase 1's. Bytes below are millions of bytes, from each run's
`peak_rss_bytes`. The bench's tables print MiB, and for the peak they print
the largest run.

- **Correctness: 8/10.** Unchanged. Every suite passed on `1e74f80`
  (`data/2026-11-06-e05/suites-and-runs.txt`): the core library's 319
  tests and 3 doc tests, the CLI's 29, the oracle library's 37 with
  `minimal`, `programs`, `reference`, `w3c`, `data_brick` and `snapshot`,
  the reference evaluator's 15, 25 and `chaos`, the bench's 42, pytest's
  787, fmt, clippy and the wasm32 core. Iteration 5 adds three tests; one
  fills a store in chunks and on one thread and compares the tables.
  lubm-50's closure is 10,345,697 triples in every run of both modes. Not
  higher: the slow oracle tier and the 100,000-iteration chaos runs have
  not run since the phase 1 record (`660e3bd`). Phase 2's parsers,
  parallel rounds, snapshots and windows, and iterations 3 to 5, are
  checked by the default suites only. The judges' gaps of iteration 0
  remain.
- **Coverage: 9/10.** Unchanged: no rule changed since iteration 1. Not 10
  for iteration 1's reason: three failures come from how the runner reads
  a case.
- **Speed (full run): 8/10.** On lubm-50, reasoning takes 402.8 to
  404.4 ms on one thread, down from 416.6 to 417.2, and 100.0 to 101.6 ms
  on all cores (e05, 3 runs each). The totals are 1.93 s and 0.91 s, 8.1
  and 17 times under Soufflé's 15.6 s on the phase 1 scoreboard, a
  comparison across records. In the A/B, lubm-1, lubm-10 and lubm-50 got
  3 to 4% faster and every other scenario stayed within the spread. Not
  higher: all cores give 3.98 times one thread against the budget's 5,
  which now needs 80.8 ms, 20.7 ms less. By phase (medians of 3), the
  rounds take 118.3 against 36.3 ms (3.3 times), the steps 267.7 against
  60.6 ms (4.4 times), and the rest 17.8 against 4.6 ms. Round 3 gains
  2.1 times and rounds 5 and 6 gain 1.6 to 1.7 times. The property family
  takes 8.0 ms in round 1 in both modes, and in round 3 it takes 1.5 ms on
  one thread against 3.5 to 3.7 ms on all cores. Loading takes 1.52 s on
  one thread, against a bar of 1.30 s. No competitor has run on all cores,
  RDFox has no estimate, and the machine was busy.
- **Speed (incremental): 5/10.** Unchanged. One added triple costs 0.2 ms
  and one removal 1.9 ms, against a full run of 2.3 to 2.7 ms on
  brick-small1 (e05's A/B). A removal still runs again from the base.
- **Memory: 7/10.** On one thread lubm-50 peaks at 480 MB in every run,
  46.4 bytes per closure triple: under the budget of 50 (517.3 MB) since
  iteration 4. On all cores it peaks at 600 to 605 MB, median 603 (58.0
  to 58.5; e05). In the bench's MiB that is 3.4 to 4.3 times under Nemo's
  1,957 on the phase 1 scoreboard. The store holds 12.1 bytes per triple
  in both modes. Not higher: all cores is the default thread mode, and it
  misses the budget by 8.3 bytes per triple (86 MB at the median). On
  unchanged code its runs spread from 598 to 647 MB (the open candidates
  have the table). The papers put VLog at about 12. It moves to 8 when the
  all-cores median of 10 interleaved runs is under 50.
- **Architecture: 7/10.** Better in substance, not by a point. Iteration
  4 sizes the dictionary's slots alike however it is filled, and
  `Dictionary::reserve`, unused at the last two ratings, now has a caller
  (`snapshot.rs:827`). Iteration 5 adds a second way to start a store:
  `Store::fill` and `by_predicate` (`engine/store.rs`, about 150 lines),
  for a full run whose canon has no class, beside the round-buffer path
  that a restart still takes. The module doc and `fill`'s doc say why,
  and a test checks that the chunked fill equals the one-thread fill. Not
  higher: `reasoner.rs` has 2,203 lines, with the document key inside.
  Five functions take too many arguments under
  `#[allow(clippy::too_many_arguments)]` (two in `io.rs`, two in
  `io/parallel.rs`, one in `snapshot.rs`). The engine still keeps unused
  code behind `#![allow(dead_code, unused_imports)]` (`engine/mod.rs:68`).
- **Embeddability: 7/10.** Unchanged. The core builds for wasm32 without
  `io` and `parallel` (e05's suites). Python still has no thread setting
  (`crates/python/src/lib.rs` has not changed since `628edad`), and the
  wasm32 build still has no parser.
- **Operability: 7/10.** Unchanged. A parse error names its file, line and
  column. Not higher: `README.md` and `CHANGELOG.md` have not changed
  since `529a5d5`, so snapshots, threads, parallel loading and windowed
  reading are missing from both. The CLI reads `-` as a file name and has
  no thread option. The terms of a diagnostic are positional, and `Error`
  is not `#[non_exhaustive]`.

Total: 58/80.

The weakest dimension is incremental speed, because a one-triple removal
still runs again from the base until phase 3's DRed. Inside phase 2 it is
memory on all cores, the default thread mode: 58.3 bytes per closure
triple at the median against 50. After it comes all-cores reasoning at
3.98 times one thread against 5. The candidates below start there.

**Iteration 5's partial win.** Keep it. The decide rule asks for a gain on
the dimension the hypothesis named, and memory did not move. One thread
fell from 486 to 480 MB. All cores went from 598 to 647 MB on
`d21803b` (6 runs, median 605) to 600 to 605 MB (3 runs, median 603). The
record's "617 → 577" compares the two largest runs, and the first is a
high outlier (647 MB). So it is a partial win, and the speed gain carries
it. One-thread reasoning on lubm-50 fell from 416.1 to 417.6 ms to 402.8
to 404.4 ms
across the two interleaved sets and the breakdown, with no overlap. That
is 13 ms (3%), and lubm-10 fell from 75.5 to 72.8 ms. Every suite passed,
with debug assertions on as well. No budget regressed in substance. The
all-cores ratio fell from 4.07 to 3.98 only because one thread got
faster: all-cores reasoning went from 102.0 to 106.2 ms to 100.0 to
101.6 ms. The cost is about 150 tested lines, used only when the canon
has no class. Reverting would give back a measured 3% for no gain. From
now on the 5-times budget is measured against the faster one-thread time,
so all cores need 80.8 ms instead of 83.

### Iteration 7: the rating

The `v2` branch at `0ce0818`. Its code is the code iteration 6 rated:
iteration 6 was reverted, and no file under `crates/` changed after
`1e74f80`. Rated from the code and the six loop records
(`docs/verification/2026-11-06-e01.md` to `-e06.md`), with iteration 6's
raw results read run by run: the 10 alternating bench pairs and the 8 probe
runs on the Baseline (`data/2026-11-06-e06/`). Iteration 6 ran at load 3.1
to 5.6. No loop record ran on a quiet machine, and the scoreboard is still
phase 1's. Bytes below are millions of bytes.

- **Correctness: 8/10.** Unchanged. The kept code's suites last ran in
  iteration 5, and all passed (`data/2026-11-06-e05/suites-and-runs.txt`).
  Iteration 6 ran every suite on its change, also with debug assertions
  on. lubm-50's closure was 10,345,697 triples in all 20 bench runs and in
  every probe run (e06 record). Not higher: the slow oracle tier and the
  100,000-iteration chaos runs have not run since the phase 1 record
  (`660e3bd`). The judges' gaps of iteration 0 remain.
- **Coverage: 9/10.** Unchanged: no rule changed since iteration 1. Not 10
  for iteration 1's reason: three failures come from how the runner reads
  a case.
- **Speed (full run): 8/10.** Unchanged. On lubm-50, iteration 6's 10
  Baseline runs on all cores take a median of 102.4 ms of reasoning (e06
  record). One thread takes 402.8 to 404.4 ms (e05). That is 3.94 times,
  against the budget's 5, which needs 80.8 ms: 21.6 ms less. The totals
  are 1.93 s and 0.91 s, 8.1 and 17 times under Soufflé's 15.6 s on the
  phase 1 scoreboard, a comparison across records. Not higher: the
  all-cores ratio, loading on one thread (1.52 s against a bar of
  1.30 s), no competitor run on all cores, no RDFox estimate, and a busy
  machine.
- **Speed (incremental): 5/10.** Unchanged. One added triple costs 0.2 ms
  and one removal 1.9 ms, against a full run of 2.3 to 2.7 ms on
  brick-small1 (e05's A/B). A removal still runs again from the base.
- **Memory: 7/10.** The score is unchanged, and the evidence is better.
  On all cores the 10 Baseline runs peak at 599.7 to 606.6 MB, median
  601.9 MB, middle half 600.9 to 602.5 MB (58.2 bytes per closure triple;
  e06). None of the 10 was a high run. One thread peaks at 480 MB (46.4
  bytes; e05), under the budget of 517.3 MB. The probe splits the
  all-cores peak in two. The program's bytes in use reach at most 455.8 to
  460.6 MB at the probe's 782 marks (44.1 to 44.5 bytes per closure
  triple), in step 3. The footprint, which the bench records, grows at
  nearly every phase and almost never falls. It ends 199 to 205 MB above
  the bytes in use: memory the run freed and the allocator keeps. Not
  higher: all cores is the default thread mode, and its footprint misses
  the budget by 84.6 MB at the median. The papers put VLog at about 12.
  It moves to 8 when the all-cores median of 10 alternating runs is under
  50 bytes per closure triple by the measure the owner names (the question
  below).
- **Architecture: 7/10.** Unchanged: the code is the one iteration 6
  rated. `reasoner.rs` has 2,203 lines, with the document key inside. Five
  functions take too many arguments under
  `#[allow(clippy::too_many_arguments)]`. The engine still keeps unused
  code behind `#![allow(dead_code, unused_imports)]` (`engine/mod.rs:68`).
  Iteration 6's revert left nothing behind in the code; its gap-closing
  helper waits on `exp/e06-one-vector-merge`.
- **Embeddability: 7/10.** Unchanged. The core builds for wasm32 without
  `io` and `parallel` (e05's suites). It has no `unsafe` code: the
  workspace denies `unsafe_code` (`Cargo.toml:21`), and only the bench
  allows it, for its system calls (`crates/bench/src/machine.rs`,
  `main.rs`). Python still has no thread setting (`crates/python/src/lib.rs`
  has not changed since `628edad`), and the wasm32 build still has no
  parser.
- **Operability: 7/10.** Unchanged. A parse error names its file, line and
  column. Not higher: `README.md` and `CHANGELOG.md` have not changed
  since `529a5d5`, so snapshots, threads, parallel loading and windowed
  reading are missing from both. The CLI reads `-` as a file name and has
  no thread option. The terms of a diagnostic are positional, and `Error`
  is not `#[non_exhaustive]`.

Total: 58/80.

The weakest dimension is incremental speed, because a one-triple removal
still runs again from the base until phase 3's DRed. Inside phase 2 it is
memory on all cores: the footprint misses the budget by 84.6 MB at the
median, while the program's own bytes stay 56.7 MB under it, so what the
budget measures decides the next step. After it comes all-cores reasoning
at 3.94 times one thread against 5.

**A question for the owner.** The memory budget allows 50 bytes per
closure triple on lubm-50 (517.3 MB). Which peak does it limit? One choice
is the process's peak footprint. The bench records it for every engine,
and it counts freed memory that the allocator keeps: 601.9 MB on all cores
at the median of 10 runs. The other is the most that the reasoner's own
allocations hold at once: at most 460.6 MB at the probe's marks. The
reviewer recommends the footprint. It is what the system must supply, so
it decides whether LUBM-1K fits in 9 GB and every run stays under the
16 GB cap. It is also what the scoreboard reads for every competitor; a
live-bytes peak for TurbOWL alone would compare unlike numbers. With it,
three additions: record the bytes in use beside the footprint in every
memory record, as the program's own demand; name the platform in the
budget (macOS, its system allocator, all cores), since the footprint
depends on the allocator's policy; and count only what the library does,
so that no allocator choice and no call that returns memory, in the bench
or the CLI, counts toward the budget. If the owner picks the bytes in use
instead, the budget is met in both thread modes, the bench needs a
counting allocator for TurbOWL's runs to report it, and the all-cores
speed-up becomes the one open budget.

### Iteration 8: the rating

The `v2` branch at `b08bb40`, whose code is `f4fc0dd`: the code iteration
7 rated, plus iteration 7's change (object indexes sorted in place on the
workers) and the owner's answer on the memory budget (`8a904fa`). Rated
from the code, the seven loop records (`docs/verification/2026-11-06-e01.md`
to `-e06.md` and `2026-11-07-e07.md`) and iteration 7's runs, read run by
run: the orchestrator's 10 alternating all-cores runs per side on lubm-50
(`data/2026-11-07-e07/e07-allpeak-*.json`), the implementer's 10 pairs at
load 1.7 to 2.2, and the implementer's probe, 3 runs per side. No loop
record ran on a quiet machine, and the scoreboard is still phase 1's.
Bytes below are millions of bytes.

- **Correctness: 8/10.** Unchanged. Every suite passed on `f4fc0dd`
  (`data/2026-11-07-e07/suites-and-runs.txt`): the core library's 323
  tests and 3 doc tests, the CLI's 29, the oracle library's 37 with
  `minimal`, `programs`, `reference`, `w3c`, `data_brick` and `snapshot`,
  the reference evaluator's 15, 25 and `chaos`, the bench, pytest's 787,
  fmt, clippy and the wasm32 core. Iteration 7 changes how an object index is sorted, not
  what it holds. A flip of a duplicate-free relation has no two equal
  pairs, so an unstable sort gives the one order, and
  `Workers::sort_distinct` checks it in debug builds
  (`engine/parallel.rs`). The implementer also ran the core library and
  chaos with debug assertions on. lubm-50's closure was 10,345,697 triples
  in every run. Not higher: the slow oracle tier and the
  100,000-iteration chaos runs have not run since the phase 1 record
  (`660e3bd`). The judges' gaps of iteration 0 remain.
- **Coverage: 9/10.** Unchanged: no rule changed since iteration 1. Not 10
  for iteration 1's reason: three failures come from how the runner reads
  a case.
- **Speed (full run): 8/10.** On lubm-50 on all cores, reasoning fell from
  a median of 102.1 ms (middle half 101.6 to 106.4) to 98.2 ms (97.2 to
  99.0) over the implementer's 10 alternating pairs. One thread takes
  403.8 ms at its fastest (e05) and 405.6 to 408.9 ms in e07's A/B, which
  stayed within the spread on every editor-size scenario. All cores give
  4.11 times one thread, against the budget's 5, which needs 80.8 ms:
  17.4 ms less. The totals are about 1.93 s and 0.91 s, 8.1 and
  17 times under Soufflé's 15.6 s on the phase 1 scoreboard, a comparison
  across records. The probe shows what bounds a step on all cores:
  `rdf:type`'s job. In a probe run (132 ms of reasoning, against the
  bench's 98), it takes 20.0 ms of step 1's 24.0 ms, 16.0 ms of them in its
  sort. The first object-index builds run on the caller's thread before
  any job starts: 3.6 ms in step 1 and 4.5 ms in step 2 (means of 3 probe
  runs). Not higher: the all-cores ratio, loading on one thread (1.52 s
  against a bar of 1.30 s), no competitor run on all cores, no RDFox
  estimate, and a busy machine.
- **Speed (incremental): 5/10.** Unchanged. One added triple costs 0.2 ms
  and one removal 1.9 ms, against a full run of 2.3 to 2.7 ms on
  brick-small1 (e05's A/B). A removal still runs again from the base.
- **Memory: 7/10.** Better by about 20 MB at the median, not by a point.
  On all cores, the orchestrator's 10 runs on `f4fc0dd` peak at 580 to
  588.2 MB, median 580.7 MB, middle half 580.3 to 582.0 MB (56.1 bytes per
  closure triple), against a median of 601.0 MB on the Baseline (e07). The
  implementer's 10 give a median of 581.3 MB, range 579.6 to 583.1. The
  budget now names its measure (`v2-2-speed.md`, the owner, 2026-11-06):
  the process's footprint, and the largest of 10 alternating all-cores
  runs must stay under 517.3 MB. The median misses it by 63.4 MB and the
  largest run by 70.9 MB. One thread peaks at 480 MB (46.4 bytes; e07),
  under it. The bytes in use
  reach at most 459.2 to 459.7 MB at the probe's marks, and 398.0 to
  398.3 MB at the end, where the footprint is 581.8 to 585.9 MB. So 184 to
  188 MB of the peak is memory the run freed and the allocator keeps. In
  the bench's MiB, the median is about 3.5 times under Nemo's 1,957 on the
  phase 1 scoreboard. Not higher: all cores is the default thread mode,
  and it misses the budget. It moves to 8 when the largest of 10
  alternating all-cores runs is under 517.3 MB.
- **Architecture: 7/10.** Better in substance, not by a point. Iteration 7
  replaced the store's `sorted` with `flipped` (`engine/store.rs`), on two
  new operations of `Workers`, `map_parts` and `sort_distinct`. Each has
  its one-thread form, and the doc gives the reason: a flip of distinct
  pairs is distinct. The first build no longer copies the table through
  `all()` before it sorts. Not higher: `reasoner.rs` has 2,203 lines, with
  the document key inside. Five functions take too many arguments under
  `#[allow(clippy::too_many_arguments)]`. The engine still keeps unused
  code behind `#![allow(dead_code, unused_imports)]` (`engine/mod.rs:68`).
- **Embeddability: 7/10.** Unchanged. The core builds for wasm32 without
  `io` and `parallel` (e05's suites), and iteration 7's parallel paths sit
  behind the same `cfg`. The core has no `unsafe` code. Python still has
  no thread setting (`crates/python/src/lib.rs` has not changed since
  `628edad`), and the wasm32 build still has no parser.
- **Operability: 7/10.** Unchanged. A parse error names its file, line and
  column. Not higher: `README.md` and `CHANGELOG.md` have not changed
  since `529a5d5`, so snapshots, threads, parallel loading and windowed
  reading are missing from both. The CLI reads `-` as a file name and has
  no thread option. The terms of a diagnostic are positional, and `Error`
  is not `#[non_exhaustive]`.

Total: 58/80.

The weakest dimension is incremental speed, since a one-triple removal
still runs again from the base until phase 3's DRed; inside phase 2 it is
memory on all cores, 70.9 MB over the budget by the largest run, and the
candidates below cut held bytes first because they convert best.

**The owner's budget change** (`8a904fa`), checked as the rules ask for a
change to a budget. It takes the measure the last rating recommended, the
process's footprint, with the three additions: the bytes in use recorded
beside it, the platform named, and no allocator choice or memory-return
call of the bench or the CLI counted. It also sets the decision: the
largest of 10 alternating all-cores runs under 517.3 MB. That is stricter
than a median, and right, since the bench's table reports the largest
run. It names no run count for one thread. One thread's runs agree to
1 MB in every record, so 3 runs are enough there.

### Iteration 10: the rating

The `v2` branch at `d78fbd3`, whose code is `b654de6`: the code iteration 8
rated, plus iteration 8's change (the base as sorted pairs by predicate,
snapshots at format version 2). Iteration 9 was reverted, and no file under
`crates/` differs from `b654de6`. Rated from the code and the nine loop
records (`docs/verification/2026-11-06-e01.md` to `2026-11-07-e09.md`),
with iterations 8 and 9 read run by run: 20 alternating all-cores runs on
lubm-50 in each (`data/2026-11-07-e08/` and `-e09/`, `*-allpeak-*.json`),
and the P0 breakdowns, 3 runs per thread mode, whose per-round and per-step
times are used below. Iteration 7's probe (`data/2026-11-07-e07/probe/`)
has not run since iteration 8. The runs were at load 2.8 to 4.9. No loop
record ran on a quiet machine, and the scoreboard is still phase 1's. Bytes
below are millions of bytes.

- **Correctness: 8/10.** Unchanged. Every suite passed on `b654de6`
  (`data/2026-11-07-e08/suites-and-runs.txt`): the core library's 328
  tests and 3 doc tests, the CLI's 29, the oracle library's 37 with
  `minimal`, `programs`, `reference`, `w3c`, `data_brick` and `snapshot`,
  the reference evaluator's 15, 25 and `chaos`, the bench's 42, pytest's
  787, fmt, clippy and the wasm32 core. Iteration 8 checks the base against
  a sorted-set model over random edits, and a version 1 snapshot is refused
  with a clear error. lubm-50's closure was 10,345,697 triples in all 64
  lubm-50 runs of e08 and e09. Not higher: the slow oracle tier and the
  100,000-iteration chaos runs have not run since the phase 1 record
  (`660e3bd`), so phase 2's parsers, parallel rounds, snapshots and the
  kept iterations are checked by the default suites only. The judges' gaps
  of iteration 0 remain.
- **Coverage: 9/10.** Unchanged: no rule changed since iteration 1. Not 10
  for iteration 1's reason: three failures come from how the runner reads
  a case.
- **Speed (full run): 8/10.** Unchanged. On lubm-50, one thread takes
  391.6 to 394.1 ms (e08's breakdown) and 387.8 to 396.0 ms in the A/Bs of
  e08 and e09. All cores take a median of 96.6 ms over e08's 10 runs
  (middle half 96.4 to 102.5) and 106.8 ms over e09's 10 Baseline runs
  (97.2 to 107.6), at load 2.8 to 3.9. That is 3.6 to 4.1 times, against
  the budget's 5, which needs about 78 ms: 18 to 19 ms less than the better
  median. P0 (3 runs per mode in e08 and in e09) says where the time goes.
  The rounds take 35.8 to 37.7 ms against 118.0 to 126.6 ms on one thread
  (3.2 times). The steps take 56.1 to 59.5 ms against 265.9 to 286.2 ms
  (4.8 times). The rest takes 2.7 to 3.2 ms against 2.2 to 2.5 ms. At 5
  times, the rounds would take about 24 ms and the steps about 54 ms, so
  the rounds hold about 13 ms of the 18 ms, the steps 2 to 3 ms and the
  rest about 2 ms. In round 1 the property family takes 7.9 to 8.6 ms on
  all cores and 8.1 to 8.4 ms on one thread: it gains nothing. In round 3
  the class family gains 1.7 times (7.4 to 7.7 ms against 12.9 to
  14.4 ms), and the property family takes 3.5 to 4.1 ms against 1.5 to
  1.6 ms: it is slower on all cores. Loading takes 0.80 s on all cores and
  1.51 s on one thread, against a bar of 1.30 s. The totals are 0.90 s and
  about 1.9 s, 17 and 8 times under Soufflé's 15.6 s on the phase 1
  scoreboard, a comparison across records. Not higher: the all-cores
  ratio, loading on one thread, no competitor run on all cores, no RDFox
  estimate, and a busy machine.
- **Speed (incremental): 5/10.** Unchanged. One added triple costs 0.2 ms
  and one removal 1.9 ms, against a full run of 2.3 to 3.0 ms on
  brick-small1 (the A/Bs of e08 and e09). A removal still runs again from
  the base.
- **Memory: 7/10.** Better by about 37 MB at the median, not by a point.
  On all cores, e08's 10 runs on `b654de6` peak at 542.7 to 548.3 MB,
  median 544.2 MB, middle half 542.9 to 547.2 MB (52.6 bytes per closure
  triple), against a median of 581.5 MB on `f4fc0dd`. e09's 10 runs on the
  same code give a median of 543.8 MB and a largest run of 547.1 MB. None
  of the 30 runs on `f4fc0dd` and `b654de6` was a high run. The budget
  asks for the largest of 10 alternating runs under 517.3 MB, and the
  largest run misses it by 29.8 to 31.0 MB. One thread peaks at 481.8 to
  485.9 MB (46.6 to 47.0 bytes; e08 and e09), under the budget. After the
  run the reasoner holds 349.7 MB (33.8 bytes per closure triple): the
  dictionary 171.8 MB (text 101.5, entries 36.7, slots 33.6), the base
  52.6 MB and the store 125.3 MB (P0). So the reasoner does not hold about
  195 MB of the all-cores peak: mostly memory the run freed and the
  allocator keeps. The probe's most
  in use (459.7 MB before iteration 8) has not been measured since. In the
  bench's MiB, the median is about 3.8 times under Nemo's 1,957 on the
  phase 1 scoreboard. Not higher: all cores is the default thread mode,
  and it misses the budget. It moves to 8 when the largest of 10
  alternating all-cores runs is under 517.3 MB.
- **Architecture: 7/10.** The score is unchanged; the substance is mixed.
  Iteration 8 removed the store's own scatter of the base by predicate:
  `fill` now copies each predicate's pairs as the base holds them
  (`engine/store.rs`, 270 lines changed, 1,132 lines now). But the base's
  new structure, `ByPredicate`, with its in-place conversion, its merge
  from the back and its removal (about 180 lines), lives in `reasoner.rs`.
  That file grew from 2,203 to 2,638 lines and still holds the document
  key. `Dictionary::reserve` still carries `#[allow(dead_code)]` and a
  comment that calls it an experiment, although the snapshot loader calls
  it (`term.rs:208`, `snapshot.rs:836`). Five functions take too many
  arguments under `#[allow(clippy::too_many_arguments)]`. The engine still
  keeps unused code behind `#![allow(dead_code, unused_imports)]`
  (`engine/mod.rs:68`).
- **Embeddability: 7/10.** Unchanged. The core builds for wasm32 without
  `io` and `parallel` (e08's suites). It still has no `unsafe` code:
  iteration 8 turns the parse buffer into pairs in place with
  `into_flattened`. Python still has no thread setting
  (`crates/python/src/lib.rs` has not changed since `628edad`), and the
  wasm32 build still has no parser.
- **Operability: 7/10.** Unchanged. A parse error names its file, line and
  column, and a version 1 snapshot is refused with a clear error. Not
  higher: `README.md` and `CHANGELOG.md` have not changed since `529a5d5`,
  so snapshots and their format version 2, threads, parallel loading and
  windowed reading are missing from both. The CLI reads `-` as a file name
  and has no thread option. The terms of a diagnostic are positional, and
  `Error` is not `#[non_exhaustive]`.

Total: 58/80.

The weakest dimension is incremental speed, since a one-triple removal
still runs again from the base until phase 3's DRed; inside phase 2 it is
memory on all cores, 30 MB over the budget by the largest run, which only a
held cut of about 40 MB can close in one change (see the open candidates).

## For the next agent

To continue the loop cold: read `AGENTS.md`, then this file from the top,
then the newest file in `docs/verification/`. Start at step 1 with a fresh
reviewer. The candidates the last reviewer ranked but that were not picked
are listed below; a new reviewer may reuse or replace them.

### Open candidates

Added after iteration 11 (the orchestrator, from its counts; not yet rated):
- Skip `Pending::compact`'s per-chunk sort for outputs that come out unsorted (prp-rng, prp-inv), leaving them to the step's sort: 10 to 19 ms of CPU out of round 1's saturated pool on all cores, part of it moved to the step. Measure the round and the step together.
- Count the datatype family's splits before changing it: in round 1 it spends 45 to 50 ms of CPU on all cores against 22 on one thread, with 13 of its 29 splits inline.
- Read a family's all-cores time with care: it includes other families' chunks that its waiting thread ran.


Ranked by iteration 10's reviewer, for iteration 10 and later, by expected
gain per effort against the two phase 2 budgets still open
(`docs/specs/v2-2-speed.md`, Non-functional budgets): the all-cores
footprint and the all-cores speed-up. Numbers marked as estimates are not
measured. The research's §10 (`docs/research/sota-algorithms-2026-11.md`)
ranks the starting candidates, P0 and E1 to E9, with hypotheses and how to
measure them; §11 lists what not to do and why.

Where the budgets stand on lubm-50 (bytes are millions of bytes, from each
run's `peak_rss_bytes`):

- **Memory on all cores:** the largest of 10 alternating runs must stay
  under 517.3 MB (50 bytes per closure triple; the owner, 2026-11-06). On
  `b654de6`, e08's 10 runs have a median of 544.2 MB, a middle half of
  542.9 to 547.2 MB and a largest run of 548.3 MB. e09's 10 Baseline runs
  have a median of 543.8 MB and a largest run of 547.1 MB (load 2.8 to
  3.9). The largest run sits 3 to 4 MB above the median, so the median
  must reach about 513 MB: about 31 MB less. One thread peaks at 482 to
  486 MB (46.6 to 47.0 bytes per closure triple): met.
- **All cores against one thread:** 3.6 to 4.1 times (387.8 to 396.0 ms
  on one thread; medians of 96.6 and 106.8 ms on all cores, e08 and e09),
  against at least 5, which needs about 78 ms: 18 to 19 ms less than the
  better median. Every one-thread gain raises the bar. The machine's 12
  cores are 2 Super, 4 Performance and 6 Efficiency cores.
- **Loading:** 0.80 s on all cores and 1.51 s on one thread, against a bar
  of 1.30 s. Met on all cores. The owner decides whether the bar holds for
  one thread too.
- **Total time:** 0.90 s on all cores and about 1.9 s on one thread,
  against version 1's 4.66 s on lubm-50. Met there. The data-size
  scenarios have not run since phase 1.

**Can the open candidates close the 30 MB? The reviewer's answer, for the
owner.** Not the small ones. The churn cuts (candidates 3 and 6, and
iteration 6's one-vector merge) are worth 5 to 20 MB each by the measured
conversion. They overlap with each other and with the freed room that
iteration 8 left, and each one risks the 10 MB bar. Three or four of them
in a row might reach 30 MB, or might not. A held cut of about 40 MB can
reach it in one change, and the list has two. Candidate 4 shares the
base's pairs with the tables: 43 MB held, with 20 to 35 MB expected,
because part of today's copy sits in memory that the load freed, and that
part becomes a hole. Candidate 5 splits the dictionary's IRIs: 41 to 52 MB
held, with 30 to 45 MB expected, because the text is allocated while
loading and so lowers the footprint the run starts from. Both are large
changes. Candidate 4 changes the store's pair type in 18 files and gives
relations a shared form. Candidate 5 changes every reader of an IRI's text
and the snapshot format. The reviewer's view: the 30 MB, 5.8% of the
budget, is reachable only through one of these two, and candidate 4 may
fall short. The owner should hear this before the next memory iteration
runs, and decide whether a 5.8% miss is worth a large structural change
now. The speed candidates do not depend on that answer, so iteration 10
runs one of them.

**Three kinds of memory cut, after iteration 9.** The footprint ratchets:
it grows at nearly every phase of a run and almost never falls. So a cut
helps only as far as it removes memory that the run would otherwise have
to take fresh from the system.

- **(a) Held through the peak.** Memory the reasoner holds from before the
  run, or from early in it, to the run's end, where the peak is. Cutting it
  lowers the bytes in use under the whole curve, so the peak falls whatever
  else the run frees. The exception is the part of it that sat in memory
  freed earlier: that part becomes a hole, and a hole is worth only what
  (b) says. Iteration 4 (33.6 MB of slots, made while loading) converted
  at about 70%. Iteration 8 (26.3 MB of base) converted at about 140%,
  because its in-place conversion also left no freed parse buffer.
  Candidates 4 and 5, E7, and 4-byte slots ("Not ranked").
- **(b) Freed during the run, for later allocations to reuse.** Worth only
  what the run still has to place after the free. Iteration 9 freed 33.6 MB
  of slots at the run's start. Alone, before iteration 8, it gave 45 MB.
  After iteration 8 it gave 7.5 MB, because iteration 8 had already left
  the run room to reuse. Every cut of this kind now competes with that
  room: iteration 9 again, a pool of freed vectors.
- **(c) Less churn.** A site that allocates less hands part of its growth
  to the next fresh allocation. The share was 25 to 45% before iteration 8
  (iterations 6 and 7), and is likely less now, since the run has more
  freed room to draw on. Candidates 3 and 6, iteration 6's merge, and the
  k-way merge (candidate 7). The exception is churn at the peak: the run's
  last large allocations, which nothing after them would have reused.
  Step 4's merge of `rdf:type`'s object index is the one known site
  ("Not ranked").

**What converts into the footprint.** Six iterations cut memory on all
cores, and the footprint followed them unequally:

| Iteration | What it removed | Kind | Nominal | All-cores median | Share |
|---|---|---|---:|---:|---:|
| 4 | the dictionary's spare slots, made while loading | (a) | 33.6 MB | −24 MB | about 70% |
| 5 | step 0's copies of the base | (c) | about 3 × 52.6 MB at once | −2 MB | about 0 |
| 6 | the merge's and the filter's pieces (reverted) | (c) | 29 MB in step 4's largest merge | −7.5 MB | about a quarter |
| 7 | the index builds' and index sorts' buffers, pieces and copies | (c) | 46.6 MB of growth at those sites | −20.3 MB | about 40% |
| 8 | 4 of the base's 12 bytes a triple, in the parse buffer | (a) | 26.3 MB | −37.3 MB | about 140% |
| 9 | the dictionary's slots, freed at the run's start (reverted) | (b) | 33.6 MB | −45 MB alone, −7.5 MB after 8 | 134%, then 22% |

**Where the footprint comes from.** Iteration 7's probe reads the
footprint, its lifetime peak and the allocator's bytes in use at about 770
marks of a run, on all cores. Its numbers predate iteration 8, which made
the base 26.3 MB smaller and changed what the run's start copies (52.6 MB
of pairs now). Growth by phase, in MB, means of 3 runs, after iteration 7:

| Where | Growth | What it is |
|---|---:|---|
| The run's start (`fill`) | 34.0 | held: the tables get the base's 6.57 million pairs; the bytes in use grow by 74 MB, so about 40 MB of the copy sat in memory the load had freed |
| Round 1 (the rules) | 86.9 | the round's output buffers; the bytes in use grow by 46 MB. The property family emits 4,157,053 of the round's 4,157,065 pairs |
| First object-index builds | 20.7 | held: `rdf:type`'s 9.1 MB and `takesCourse`'s 11.4 MB indexes |
| Steps' subject sorts and changes | 119.4 | `sort_dedup_batches`, then the merge and the filter, each with pieces and a joined copy |
| Steps' object-index sorts and changes | 55.6 | step 4's merge of `rdf:type`'s index alone: 28.5 to 30.0 MB, a fresh 27.7 MB vector, after which the run grows only 4.4 MB more |
| Rounds 2 to 6 | −0.5 | |
| Total | 316.1 | |

**Before the next memory iteration: measure the Baseline.** Rerun
iteration 7's probe on `b654de6` (3 runs, all cores), with its marks
adjusted where iteration 8 moved an anchor line in `engine/store.rs`. Add
one mark after `fill`. It says where the footprint grows now that
iteration 8 changed the start, and whether step 4's index merge still sets
the peak. As a diagnostic only, in a probe that is not landed, add one
`malloc_zone_pressure_relief` call after each step (3 runs): it shows how
much of the about 195 MB that the reasoner does not hold is reachable
at all. Neither is an iteration, and
neither changes the library.

**Where the speed-up is lost.** The rounds hold about 13 ms of the 18 ms
the budget needs, and the steps 2 to 3 ms (the Speed rating above has the
P0 numbers). In a step, the critical path is the first object-index builds
on the caller's thread, then `rdf:type`'s job. In iteration 7's probe
(after side, run 1: 132 ms of reasoning against the bench's 97 ms; the
step code has not changed since), in ms:

| Step | Step | Builds before the jobs | `rdf:type`'s job | its sort | its change | its index sort | its index change |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | 24.0 | 3.6 (`rdf:type`'s index) | 20.1 | 16.0 | 0.9 | 1.7 | 1.2 |
| 2 | 18.4 | 5.9 (`takesCourse`'s index, 4.5) | 12.4 | 7.7 | 2.3 | 1.6 | 0.5 |
| 3 | 16.1 | 0.1 | 15.9 | 10.9 | 1.2 | 3.0 | 0.6 |
| 4 | 14.1 | 0.1 | 14.0 | 6.2 | 2.4 | 0.5 | 4.7 |

The builds and `rdf:type`'s job are 72 of the 73 ms of steps 1 to 4, and
the sorts are 40.8 of the job's 62.4 ms. In the bench (P0, e08 and e09,
all cores), steps 1 to 4 take 13.5 to 16.7, 14.0 to 16.2, 11.7 to 13.0
and 11.4 to 12.2 ms: 4.9, 4.0, 6.0 and 4.5 times one thread's 65.4 to
67.3, 58.3 to 59.5, 71.2 to 74.9 and 51.3 to 61.8 ms. Step 2, whose jobs
wait for `takesCourse`'s build, is the furthest from 5. What would shorten
the path, in order:

1. Take the first builds off the caller's thread (candidate 1): step 2's
   5.9 ms wait leaves the path, and `rdf:type`'s 3.6 ms build in step 1
   runs beside its sort.
2. Shorten the sort, 65% of the job. Each sort makes four allocations as
   long as its input, and step 1's sort window grows the footprint by
   43 MB, so fresh pages are part of its time (candidate 3; iteration 7
   gained 4 ms from a similar cut).
3. Step 4's index change (4.7 ms): a parallel merge into pieces, then a
   fresh 27.7 MB vector ("Not ranked").
4. If the job still decides after these, split it by ranges of
   `rdf:type`'s pairs.

Candidates 1 and 3 give 4 to 9 ms by the estimates below. The 5-times
budget also needs the rounds: round 1's property family (candidate 2), and
then round 1's datatype family (2.9 times) and round 3's class family (1.7
times), which need counting before a change ("Not ranked").

**How to measure the all-cores peak.** On one thread every run peaks at
the same size, to 1 MB in most records and 4 MB in one. On all cores the
runs spread. Per-run peaks of lubm-50 on unchanged code, from the records'
JSON:

| Code | Runs | Peaks (MB) | Median (MB) |
|---|---:|---|---:|
| `b1d4b93` (e02 after, e03 before) | 6 | 624, 626, 626, 630, 630, 675 | 628 |
| `eebb962` (e03 after, e04 before) | 6 | 627, 627, 628, 630, 631, 632 | 629 |
| `d21803b` (e04 after, e05 before) | 6 | 598, 599, 602, 608, 632, 647 | 605 |
| `1e74f80` (e05 after) | 3 | 600, 603, 605 | 603 |
| `1e74f80` (e06 before) | 10 | 599.7 to 606.6, middle half 600.9 to 602.5 | 601.9 |
| `1e74f80` (e07 before) | 10 | 600 to 602.9, middle half 600.5 to 602.1 | 601.0 |
| `f4fc0dd` (e07 after) | 10 | 580 to 588.2, middle half 580.3 to 582.0 | 580.7 |
| `f4fc0dd` (e08 before) | 10 | 580.1 to 583.5, middle half 580.5 to 582.3 | 581.5 |
| `b654de6` (e08 after) | 10 | 542.7 to 548.3, middle half 542.9 to 547.2 | 544.2 |
| `b654de6` (e09 before) | 10 | 540.3 to 547.1, middle half 543.0 to 545.0 | 543.8 |

78 of the 81 runs sit within 8 MB of their median, and 3 land 27 to 47 MB
above it. None of the last 30 runs in the table was a high run, and the
largest of 10 sat 2 to 8 MB above its median. So:

- Run each side at least 10 times, each run its own process (as the bench
  runs it), alternating the two builds run by run (A, B, A, B, ...), with
  nothing else running.
- Decide on the per-run `peak_rss_bytes` in the JSON, not on the table.
  Report each side's median, quartiles, smallest and largest run, and its
  count of high runs (more than 20 MB over its median).
- A change moves the peak only when the medians differ by more than
  10 MB (1 byte per closure triple) and the two sides' middle halves do
  not overlap. A change that only makes high runs rarer says so, with the
  counts.
- The budget passes only when the largest of 10 runs is under 517.3 MB
  (`v2-2-speed.md`).
- Record the bytes in use beside the footprint (the budget asks for it):
  P0's held bytes after the run, and the probe's most in use.
- For a change aimed at one phase, also run iteration 7's probe on both
  sides (3 runs each) and compare that phase's growth.
- For a speed change, decide on reasoning time over the same 10
  alternating runs per side: medians and middle halves, with P0's round
  and step times beside them. The machine has been busy in every record,
  and all-cores medians of the same code have differed by 10 ms between
  records (e08 and e09), so only a comparison within one alternating set
  counts.

1. **Build a first object index inside its table's job.** Speed on all
   cores. Iteration 8's candidate 5, with the probe's numbers.
   **Iteration 10.**
   - Hypothesis: `Store::advance` builds each first object index on the
     caller's thread before any job starts (`engine/store.rs`). In the
     probe, step 1's jobs wait 3.6 ms for `rdf:type`'s build, and step 2's
     jobs wait 5.9 ms, 4.5 ms of it for `takesCourse`'s. Step 2 is the step
     furthest from 5 times: 14.0 to 16.2 ms against 58.3 to 59.5 ms on one
     thread (4.0 times; P0, e08 and e09). Build the index in its table's
     own job instead, beside the job's sort: the build reads the table's
     parts as they stand, the sort makes a relation of the round's new
     batches, and the job inserts that relation once both finish. Do this
     when the step has no merges. A step with merges keeps today's order,
     so that the rewrite covers the new index. Then `takesCourse`'s build
     runs beside `rdf:type`'s job, and `rdf:type`'s build beside its own
     sort. Expected (estimates): 3 to 5 ms off all-cores reasoning, from
     the probe's 9.5 ms of waiting scaled by the bench's 0.72, with the
     builds competing for cores. That is about a quarter of the 18 ms the
     budget needs. One thread does the same work in the same order.
   - Effect on the rubric: Speed (full run), on the all-cores axis. No
     memory change is intended: the indexes' 9.1 and 11.4 MB are allocated
     a little later.
   - Files: `engine/store.rs` (`advance`, `Job`); `engine/parallel.rs` if
     `Workers` needs a join of two closures, with its one-thread form.
   - Risk: low. A job owns its table, and the index holds the same pairs.
     Chaos at one thread and all cores, and `Workers::every_round`, check
     it. A build beside the sort holds the index and the sort's buffers at
     once in step 1. That may raise step 1's footprint by a few MB, far
     below the peak at the run's end, and the peak protocol checks it.
   - Measure: P0's times for steps 1 and 2 in both modes; reasoning time on
     lubm-10 and lubm-50 on all cores, 10 alternating single runs per side,
     by medians and middle halves; one thread, 3 runs per side (no change
     expected); the all-cores peak by the protocol above, which must not
     rise by more than its spread.
   - Why first: it is the smallest change with a measured target, it has
     low risk, and it needs no decision from the owner while the owner
     weighs the memory answer above.

2. **Run each property-family rule as one split over all its axioms.**
   Speed on all cores. Iteration 8's candidate 7, now with the round times
   it asked for.
   - Hypothesis: in round 1 the property family takes 7.9 to 8.6 ms on all
     cores and 8.1 to 8.4 ms on one thread (P0, 6 runs per mode in e08 and
     e09). It gains nothing, and it is round 1's slowest family. In round 3
     it takes 3.5 to 4.1 ms against 1.5 to 1.6 ms on one thread. `copies`
     (`rules/property.rs`) calls `Round::split` or `split_runs` once per
     axiom and part, one after another inside the family's task. LUBM's 25
     `rdfs:domain`, 18 `rdfs:range`, 5 `rdfs:subPropertyOf` and 3
     `owl:inverseOf` tuples give about 50 fork-joins a round, each with up
     to 48 chunk buffers. Gather each rule's (axiom, part) work for the
     round into one list and cut it into even chunks across axioms: one
     fork-join per rule. Expected (estimates): the family falls to 2 to
     3 ms in round 1, where the datatype family (7.7 to 8.2 ms) then
     decides. That takes 2 to 3 ms off round 1 and about 1 ms off rounds 2
     and 3: 3 to 4 ms in all.
   - Doubt: first count the splits and their sizes with a counter (not
     landed). If they are few and large, the time is in the output
     buffers' growth (candidate 6), and this proposal stops.
   - Effect on the rubric: Speed (full run), on the all-cores axis. Memory:
     fewer chunk buffers, a little.
   - Files: `rules/property.rs`; `engine/store.rs` if `Round` needs a split
     over a list of slices.
   - Risk: low. The step sorts each table's pairs, so the order of the
     emits does not matter, and the copies ask for no merges. Chaos at
     both thread counts and `Workers::every_round` check it.
   - Measure: P0's family times per round in both modes; reasoning time as
     in candidate 1; the all-cores peak by the protocol.

3. **Sort and deduplicate a step's batches in place.** Memory on all cores
   (churn) and speed. Iteration 8's candidate 3.
   - Hypothesis: `rdf:type`'s sort takes 16.0 ms of its 20.1 ms job in
     step 1 of a probe run, and 40.8 ms of the job's 62.4 ms over steps 1
     to 4. Step 1's sort window grows the footprint by 43 MB. On all cores
     `sort_dedup_batches` (`engine/parallel.rs`) makes four allocations as
     long as its input: `concat` fills one with zeros and copies the
     batches in, `par_sort` takes a buffer as long, deduplication writes
     pieces with a whole chunk's capacity, and a second `concat` joins
     them. Instead, join the batches into the largest batch's allocation
     as one thread does, sort there, and deduplicate each chunk in place.
     Then close the gaps from left to right (`copy_within`, the helper on
     `exp/e06-one-vector-merge`) and `fit`: one allocation instead of four.
     Expected (estimates): 10 to 20 MB off the all-cores median, iteration
     7's share for a similar cut, and maybe less, since iteration 8 left
     more freed room for this site to reuse. Also 1 to 4 ms off all-cores
     reasoning, from fewer fresh pages on the critical path.
   - Doubt: `par_sort` is a merge sort and gains from the sorted runs that
     `compact` leaves in each batch; `par_sort_unstable` does not. If the
     in-place sort is slower, keep `par_sort` and its buffer, and remove
     only the pieces and the second copy.
   - Effect on the rubric: Memory on all cores; Speed (full run) on the
     all-cores axis.
   - Files: `engine/parallel.rs` (`sort_dedup_batches`, `joined`, a helper
     that closes gaps).
   - Risk: low to medium. A chunk must drop a pair equal to the last pair
     of the chunk before it, read before any pair moves. Chaos at both
     thread counts and `Workers::every_round` check it.
   - Measure: the all-cores peak by the protocol; reasoning time as in
     candidate 1; P0's step sort times; the probe's growth in the steps'
     subject sorts, 3 runs per side.

4. **Share the base's pairs as each table's first tier.** Memory in both
   modes, a held cut. Iteration 8's candidate 6.
   - Hypothesis: a full run copies each predicate's base pairs into its
     table (`Store::fill`): 52.6 MB. A table keeps that copy as its first
     tier to the run's end unless its derived pairs reach half its base,
     which merges the tier. On lubm-50 that leaves every base table but
     `rdf:type` and a few small ones: about 5.4 million of the 6.57 million
     base pairs, 43 MB (an estimate). Share them instead: the table's first
     tier points into the base's vector, and a merge that takes the tier
     copies it, as today. This is memory held through the peak, so it
     lowers the peak whatever else the run frees. The exception is the
     part of today's copy that sits in memory the load freed (about 40 MB
     of the start's 74 MB before iteration 8): that part becomes a hole,
     worth what iteration 9's hole was worth. Expected (estimates): 20 to
     35 MB off the all-cores median and 30 to 43 MB off one thread. The
     start's copy goes too, a few ms on one thread, which raises the
     5-times bar by about 1 ms.
   - Cost: the base holds `[u32; 2]` and the store holds `(u32, u32)`, and
     a shared view without `unsafe` needs one type. So `Pair` becomes
     `[u32; 2]` across the crate: about 200 references in 18 files, a
     mechanical change that the compiler checks. `Relation` gains a shared
     form, an `Arc` of the base's vector and a range, and `merge`, `remap`
     and `into_vec` copy a shared tier. Hold the base's vector itself in
     the `Arc`; never copy it into a new `Arc<[_]>`, since a freed 53 to
     79 MB buffer is memory the allocator keeps (iteration 8's first try).
     An addition large enough to merge into the base copies the base while
     a store shares it, about 53 MB once. A removal drops the store first.
   - Effect on the rubric: Memory, both modes. It moves Memory to 8 if the
     largest of 10 all-cores runs passes under 517.3 MB. Speed: one thread
     a little faster at the start.
   - Files: `engine/store.rs` (`Pair`, `fill`), `engine/relation.rs`,
     `engine/variable.rs`, `reasoner.rs` (`ByPredicate`, `Base`),
     `snapshot.rs`, `profile.rs` (count the shared bytes once), and the
     rules, for the pair type.
   - Risk: medium to high. Every operation that takes or rewrites a
     relation's vector, and the snapshot, meet the shared form. Chaos, full
     and incremental, at both thread counts, the snapshot round trip and
     the base's model test check it.
   - Measure: the all-cores peak by the protocol; one thread, 3 runs per
     side; P0's store bytes, which should fall by about 43 MB; the
     incremental scenarios; the probe's growth at the run's start.
   - Why fourth: by gain per effort it comes after candidate 3. But it is
     one of the two listed changes that can close the memory budget alone.
     Run it after the owner has heard the answer above.

5. **Split IRIs into a namespace and a local name in the dictionary.**
   Memory in both modes, a held cut made while loading. Moved up from "Not
   ranked".
   - Hypothesis: the text is 101.5 MB on lubm-50, about 44 bytes a term,
     and LUBM's IRIs share a few thousand namespaces. A namespace id fits
     in the entry's 32-bit `aux` field, which an IRI leaves unused. That
     might hold 41 to 52 MB less (an estimate). The text is allocated
     while loading, before the run, so the cut lowers the footprint the
     run starts from; it is not a copy the run had placed in freed memory.
     Expected (estimates): 30 to 45 MB off the all-cores median, and about
     as much off one thread and off the load's own peak.
   - Effect on the rubric: Memory, both modes; it could close the budget
     alone. Loading may get slower: each IRI is split, and its hash covers
     two parts.
   - Files: `term.rs` (entries, interning, `Term::Iri`), every reader of a
     borrowed IRI (the writers in `io`, the diagnostics, the rules that
     read an IRI's text), `io/terms.rs`, `snapshot.rs` (format version 3).
   - Risk: high, the largest change here. Every writer borrows
     `Term::Iri(&str)`, and the snapshot format changes.
   - Measure: P0's dictionary text bytes; the all-cores peak by the
     protocol; one thread, 3 runs per side; load time on lubm-10 and
     lubm-50, 10 alternating runs per side; the snapshot's size and load
     time.

Still open, ranked after these five:

6. **Give each chunk of a copy the room it will use.** Memory on all cores
   (churn). Iteration 8's candidate 4. Round 1 grows the footprint by
   86.9 MB while the bytes in use grow by 46 MB (the probe, before
   iteration 8). The property family emits almost all of the round's
   4,157,065 pairs as copies, into chunk buffers that grow by doubling. A
   copy emits at most one pair per pair it reads, so a fresh chunk buffer
   can reserve exactly what it will emit. Expected (estimates): 5 to
   15 MB, less after iteration 8 for the reason in (c). First add marks
   around each family task and split of round 1 to the probe; if the
   property family's chunks do not hold the surplus, stop. Run it after
   candidate 2, which changes how the chunks are cut. Files:
   `engine/store.rs`, `rules/property.rs`. Risk: low.
7. **Merge a step's sorted batches k ways instead of joining and sorting
   them.** Speed on all cores (iteration 4's candidate 3). Sort each family
   task's batch in its task, then merge all batches k ways, in ranges
   side by side, into one vector. Expected (estimates): 4 to 8 ms, and one
   fresh vector per table instead of four. Iteration 5 is a caution: it
   removed 34 to 37 ms of sorting summed over tasks and gained about 1 ms
   of wall time. Risk: medium. Files: `engine/parallel.rs`,
   `engine/store.rs` (`Round::tasks`). It conflicts with candidate 3: run
   one of them, not both.
8. **Hash a document once.** Loading and total time (iteration 1's
   candidate 4). `DocumentKey` hashes all 934 MB of lubm-50 twice on one
   thread, with FxHash and SipHash (`reasoner.rs`). XXH64, already in
   `snapshot.rs`, run with two seeds in blocks, would cut about 0.3 s from
   both modes (an estimate). On one thread it would bring loading (1.51 s)
   under the 1.30 s bar, if the owner reads the budget for both modes.
   Measure with P0's load breakdown and load time on lubm-10 and lubm-50.
9. **Sort the parsed base on the workers** (iteration 1's candidate 5).
   Since iteration 8, `ByPredicate::from_triples` sorts the 6.6 million
   parsed triples by predicate on one thread, in the parse buffer
   (`reasoner.rs`). `par_sort_unstable` would sort them in place with no
   buffer. Loading is met on all cores, and this does nothing on one
   thread.

Not ranked here, and why:

- **Return free memory to the system from the library**, or **choose
  another global allocator** (mimalloc, jemalloc) in the bench or the CLI.
  The owner's budget excludes both: neither counts toward it. A library
  can neither choose its embedder's allocator nor trim the embedder's
  heap; the call needs `unsafe` in a core that has none, acts on the whole
  process, and does nothing under another allocator, on wasm32 or on musl.
- **Iteration 9 again** (the dictionary's slots dropped during a large
  parallel run, `exp/e09-dict-drop`). A cut of kind (b): after iteration 8
  it gave 7.5 MB, and after any later cut it would give no more.
- **A merge of `rdf:type`'s object-index tiers without a fresh vector.**
  Step 4's merge was the run's last large growth before iteration 8:
  +28.5 to 30.0 MB for a fresh 27.7 MB vector of 3.46 million pairs, with
  4.4 MB of growth after it. If it still sets the peak, a cut here is churn
  at the peak and would convert like a held cut; the probe on the Baseline
  decides. One thread grows the larger tier in place and merges from the
  back (`merge_into`). On all cores there is no cheap form: each range's
  output overlaps the next range's input. A sequential merge would put
  about 5 ms on the critical path (an estimate), and its `realloc` may move
  the tier anyway.
- **Iteration 6's one-vector merge again** (`exp/e06-one-vector-merge`):
  7.5 MB, under the 10 MB bar. A repeat needs a reason: after candidate 3,
  the merges and filters may be where the steps' growth goes next. Run the
  branch on the probe then, before proposing it.
- **A pool that hands a step's freed vectors to the next round.** A cut
  of kind (b). Across 12 threads it needs a shared pool, against
  `separate-before-serializing-shared-state`.
- **Run a table's object-index merge beside its subject merge.** 1 to 3 ms
  off all cores (an estimate), but it puts two 29 MB merges of `rdf:type`
  at the same time, at the end of the run, against the memory budget. Not
  while the all-cores peak is over 50 bytes per closure triple.
- **The families that gain little from threads.** The rounds hold about
  13 ms of the speed-up's 18 ms. Round 1's datatype family gains 2.9 times
  (7.7 to 8.2 ms against 22.0 to 23.7 ms), and round 3's class family 1.7
  times (7.4 to 7.7 ms against 12.9 to 14.4 ms). Before a change, mark
  each rule of those rounds in the probe and count page faults per round
  (`getrusage` around a round).
- **4-byte dictionary slots.** A held cut: the slots are 2^22 of 8 bytes,
  33.6 MB, each the term's 32-bit hash beside its id. Ids alone would hold
  16.8 MB less (about 12 MB off the median at 70%). But every probe would
  then read an entry and the arena to compare a term, so the dictionary
  merge, 110 to 150 ms on lubm-50, would get slower, and the gain sits near
  the 10 MB bar.
- **E2, E8 and E1** (the owner's first picks, research §10) act on Brick's
  class hierarchy. No phase 2 budget is open on Brick, and LUBM's
  hierarchy is shallow. They come after the memory and all-cores budgets,
  or first if the owner keeps that order.
- **E7** (CSR for `rdf:type`'s object index): P0 puts that index at
  30.2 MB, 2.9 bytes per closure triple. A CSR of 4-byte subjects with
  offsets per class would hold about 15 MB less (an estimate, about 11 MB
  off the median at 70%), but joins read an index as sorted pairs, so
  every join over it changes, and so would step 4's merge.
- **E10** (a blocked Bloom filter on each large stable tier): for
  many-round inputs. It costs 12 to 15% more memory, against the budget
  now.
- **The dictionary merge** runs on one thread, 110 to 150 ms on lubm-50
  (ticket 18's log). A sharded dictionary that keeps first-seen ids would
  run it in parallel, for more effort than candidate 8 and a similar gain.


## Phase 3 deletion experiments, 2026-11-09

These are the implementation probes for tickets 22 and 23, based on
`83b356e`, on `feat/incremental-deletions`. Raw files are in
`docs/verification/data/2026-11-09-removals/`. The maintainer authorized the
small comparison at the current load, so timings are approximate. No
benchmark ran beside our builds or tests. These probes do not replace the
phase 3 scoreboard or acceptance runs.

| Probe | Change and question | Evidence and decision |
|---|---|---|
| D0 | Measure the existing removal path against unchanged, insertion and fresh full reasoning. | The separate `2026-11-09-deletion-cost.md` record has 11 checked samples per case. Deletion median 1.996 ms, full 2.047 ms. It confirms that the old deletion path recomputes the closure. |
| D1 | Replay native families over deleted deltas, then make a naive rederivation pass. | `first-maintenance.json`: deletion median 3.839 ms, full 2.157 ms. The class family dominates. Rejected as the empty-canon fast path; retained as conservative maintenance for unaffected equality classes. |
| D2 | Query the shared rule program for dependent heads and survivor support. | `indexed-maintenance.json` and its stderr show bounded helper evaluation falling back. Retain the architecture, revise helper evaluation before judging speed. |
| D3 | Estimate indexed atom cardinalities to choose joins. | `indexed-estimates.json` and `indexed-fallback-probe.json` still reach the query limit. Estimates remain, but they do not resolve the helper work alone. |
| D4 | Table recursive helper queries to their positive fixpoint. | `tabled-helpers.json` still reaches the work bound on the Brick deletion. Retain for sound recursive evaluation; performance remains open. |
| D5 | Specialize list helper queries with native list code. | `native-list-queries.json` still falls back. Reverted the specialization and deleted its temporary module. |
| D6 | Build missing object indexes locally for maintenance queries. | `lazy-object-indexes.json` still falls back. Retain bounded indexes to avoid repeated object scans. |
| D7 | Reuse completed helper calls rather than recomputing previous tables. | `completed-helper-calls.json` still falls back. The diagnostic trace records 49,263 steps in allTypes and 28,367 in listCell, out of the 100,000-step limit. Retain reuse and investigate unnecessary helper conclusions. |
| D8 | Batch a changed subject's inferred types and prune helper calls whose first ordinary-triple conclusions are all affected or protected. | `projected-typing-batches.json`: deletion median 1.029 ms, full 2.071 ms; all closures checked, no fallback. Retained. This combined candidate was measured together, so the separate contribution of batching and projections is unmeasured. |
| D9 | Give backward goals shallow and deep search budgets within a run-wide bound, with bounded proof-only object indexes. | The clean `proof-budget-probe.json` measured 1.101 ms for deletion and 2.148 ms for full reasoning. It still overdeleted 71 facts and queued 13 repair seeds, exactly as D8. No work reduction established; reverted the added tuning and proof indexes for simplicity. The small timing difference across loads does not establish a regression. |

D2 through D7 include temporary profiling probes. Their elapsed times are
diagnostic only and must not be cited as clean performance comparisons.
The trace identified charged logical work; it did not measure isolated
CPU time per rule. The final source contains no tracing instrumentation.

The current bounds are safety heuristics, not tuned crossover estimates:
pending removals at most `clamp(base / 8, 16, 4096)`, affected facts at most
`max(closure / 4, 64)`, 100,000 query steps per phase, and 1,024 backward
steps per run with depth 64 and at most 16 candidates per atom. Exceeding a
maintenance bound uses full reasoning. Backward exhaustion merely declines
to preserve a fact; DRed still establishes the correct result.
