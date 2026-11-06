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

## For the next agent

To continue the loop cold: read `AGENTS.md`, then this file from the top,
then the newest file in `docs/verification/`. Start at step 1 with a fresh
reviewer. The candidates the last reviewer ranked but that were not picked
are listed below; a new reviewer may reuse or replace them.

### Open candidates

Ranked by iteration 7's reviewer, for iteration 7 and later, by expected
gain per effort against the two phase 2 budgets still open
(`docs/specs/v2-2-speed.md`, Non-functional budgets): the all-cores peak
memory and the all-cores speed-up. Numbers marked as estimates are not
measured. The research's §10 (`docs/research/sota-algorithms-2026-11.md`)
ranks the starting candidates, P0 and E1 to E9, with hypotheses and how to
measure them; §11 lists what not to do and why.

Where the budgets stand on lubm-50 (bytes are millions of bytes, from each
run's `peak_rss_bytes`):

- **Memory on all cores:** iteration 6's 10 Baseline runs (load 3.1 to
  5.6) peak at 599.7 to 606.6 MB, median 601.9 MB (58.2 bytes per closure
  triple), against at most 517.3 MB (50). The median must lose 84.6 MB,
  the largest run 89.3 MB. One thread peaks at 480 MB (46.4): met. If the
  owner reads the budget as the bytes in use (the question after
  iteration 7's rating), it is met in both modes.
- **All cores against one thread:** 3.94 times (403.8 ms in iteration 5
  against 102.4 ms in iteration 6, medians), against at least 5, which
  needs 80.8 ms on all cores: 21.6 ms less. Every one-thread gain raises
  the bar. By phase (iteration 5, medians of 3), the rounds take 118.3
  against 36.3 ms (3.3 times), the steps 267.7 against 60.6 ms (4.4
  times), the rest 17.8 against 4.6 ms. The machine's 12 cores are 2
  Super, 4 Performance and 6 Efficiency cores. Measure the ratio on the
  Baseline on a quiet machine before deciding it.
- **Loading:** 0.81 s on all cores and 1.52 s on one thread, against a bar
  of 1.30 s. Met on all cores. The owner decides whether the bar holds for
  one thread too; if it does, "hash a document once" (below) comes first
  for loading.
- **Total time:** 1.93 s on one thread and 0.91 s on all cores, against
  version 1's 4.66 s on lubm-50. Met there. The data-size scenarios have
  not run since phase 1.

**What the probe found.** Iteration 6's probe read the footprint, its
lifetime peak and the allocator's bytes in use at 782 marks of a run, on
all cores (8 runs on the Baseline, `data/2026-11-06-e06/probe-base-all-*.txt`).
Only its output is in the repository, not its code.

- The footprint ratchets. It grows from 265 to 274 MB after loading to
  about 600 MB, and the run's last marks hold its peak. Every increase adds
  to the final peak, unless a later allocation reuses what an earlier one
  freed.
- Where it grows (the Baseline runs): `start` +34 MB; round 1 +82 to
  84 MB, while the bytes in use grow by 46 MB; step 1 +72 to 73 MB;
  step 2 +79 to 83 MB; step 3 +28 MB; step 4 +33 to 34 MB; steps 5 and 6
  about 1 MB.
  Inside the steps, the largest increases are:
  - the first object-index build of `rdf:type` in step 1: +27.8 MB, for an
    index of 1.14 million pairs (9.1 MB);
  - the first object-index build of `takesCourse` in step 2: +25 to
    28 MB, for an index of about 1.4 million pairs (about 11 MB);
  - one table's change (`t21` in the probe) in steps 1 and 3: up to
    +24 MB each;
  - `rdf:type`'s index sorts: +5.6 MB in step 1 and +9.2 MB in step 2;
  - step 4's index change of `rdf:type`: +28.5 MB, a fresh 29 MB vector.
- An index build adds about 3 times its index. `sorted` deduplicates the
  flip on all cores: `par_sort` allocates a buffer as long as the flip,
  then the flip, the deduplicated pieces and their joined copy are alive
  together (3 × 9.1 = 27.4 MB for `rdf:type`).
- The bytes in use never pass 460.6 MB. The footprint passes that in
  step 2 and ends about 140 MB above it.
- After the run the reasoner holds 376.0 MB: the dictionary 171.8 MB
  (text 101.5, entries 36.7, slots 33.6), the base 78.9 MB and the store
  125.3 MB (`rdf:type` 60.4 MB with its 30.2 MB object index,
  `takesCourse` 22.7 MB with its 11.4 MB index).

**What converts into the footprint.** Three iterations cut memory on all
cores, and the footprint followed them unequally:

| Iteration | What it removed | Nominal | All-cores median | Share |
|---|---|---:|---:|---:|
| 4 | the dictionary's spare slots, held for the whole run | 33.6 MB | −24 MB | about 70% |
| 5 | step 0's copies of the base, short-lived | about 3 × 52.6 MB at once | −2 MB (605 → 603) | about 0 |
| 6 | the merge's and the filter's pieces, short-lived | 29 MB in step 4's largest merge | −7.5 MB | about a quarter |

Held bytes lower every mark after they go, so the footprint follows them.
A short-lived allocation that a change removes also frees nothing for
later allocations to reuse, and those then take fresh pages. At these
shares, closing 84.6 MB needs about 120 MB of held bytes cut, or about
340 MB of short-lived allocations. Each memory estimate below is a range
between those shares.

**The three kinds of change** the probe opens:

- **(a) Allocate less churn.** Ranked below (candidates 1, 4 and 5), with
  the shares above in mind: no single cut of short-lived memory closes
  the gap, and each must clear 10 MB at the median. The sites the probe
  names that have a cheap fix come first: the index builds and sorts (1),
  the steps' sorts (4), the round buffers (5). Step 4's fresh 29 MB merge
  has no cheap fix on all cores ("Not ranked", below).
- **(b) Return free memory to the system between steps**
  (`malloc_zone_pressure_relief` on macOS, `malloc_trim` on glibc). Not in
  the library:
  - The call needs `extern "C"` and `unsafe` in a core that has no
    `unsafe` code. The spec allows `unsafe` only in a module that holds
    one hot spot, with a measured speed gain (`v2-2-speed.md`,
    Implementation Decisions), and this is not a hot spot.
  - It acts on the whole process. It trims every zone, including the
    embedder's allocations, so a library that calls it changes global
    state. That goes against the rubric's embeddability.
  - It does nothing under another global allocator, on wasm32 or on musl.
  - Its cost grows with the embedder's heap. Each call walks every zone,
    and the pages it returns fault in again in the next step, against the
    speed budget.
  - The bench would then report what one allocator does after a call that
    an embedder does not make.

  Use it once as a diagnostic in a probe that is not landed: one relief
  call after each step, 3 runs on the Baseline. If the peak then falls
  under 517.3 MB, the gap is memory the allocator keeps, and cuts of
  short-lived memory can close it. If it does not, held bytes (candidate
  2) come first. A CLI option that calls it could come later as an
  operability feature, if the owner asks for one, and it would not count
  toward the budget.
- **(c) The measure.** The question after iteration 7's rating. The pick
  below serves both open budgets, so it does not wait on the answer. If
  the owner reads the budget as the bytes in use, candidates 2, 4 and 5
  lose their reason, and the speed candidates (1, 3 and 6) come first.

**How to measure the all-cores peak.** On one thread every run peaks at
the same size, to 1 MB, in every record. On all cores the runs spread.
Per-run peaks of lubm-50 on unchanged code, from the records' JSON:

| Code | Runs | Peaks (MB) | Median (MB) |
|---|---:|---|---:|
| `b1d4b93` (e02 after, e03 before) | 6 | 624, 626, 626, 630, 630, 675 | 628 |
| `eebb962` (e03 after, e04 before) | 6 | 627, 627, 628, 630, 631, 632 | 629 |
| `d21803b` (e04 after, e05 before) | 6 | 598, 599, 602, 608, 632, 647 | 605 |
| `1e74f80` (e05 after) | 3 | 600, 603, 605 | 603 |
| `1e74f80` (e06 before) | 10 | 599.7 to 606.6, middle half 600.9 to 602.5 | 601.9 |

28 of the 31 runs sit within 7 MB of their median; 3 land 27 to 47 MB
above it. The bench's table reports the largest run, so one high run
decides the number in its table. So:

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
- The budget passes only when the largest run is under 517.3 MB, since
  that is what the bench reports.
- For a change aimed at one phase, also run the probe on both sides
  (3 runs each) and compare the footprint's increase at that phase. Keep
  the probe's code this time, in the bench or as an example outside the
  library, so that the next reviewer can run it again.

1. **Sort a flipped index in place on the workers, without deduplicating
   it.** All cores: speed, then memory. Iteration 6's candidate 2, now
   with the probe's numbers. **Iteration 7.**
   - Hypothesis: an object index is built from a flip of its table. When a
     rule first asks for the index, `Store::advance` flips the whole table
     on the caller's thread, one table after another, before the step's
     jobs start. In each step, `Job::sort_index` flips the table's new
     recent part in one loop. Then `sorted` treats the flip as unsorted
     pairs that may repeat. On all cores it sorts them with `par_sort`,
     which allocates a buffer as long as its input, deduplicates them into
     pieces and copies the pieces into one vector (`concat`). A flip of a
     duplicate-free relation has no duplicate, so the pieces and the copy
     do nothing. The probe measured these sites: `rdf:type`'s first build
     adds 27.8 MB of footprint for a 9.1 MB index, `takesCourse`'s 25 to
     28 MB for about 11 MB, and `rdf:type`'s index sorts 5.6 and 9.2 MB.
     Instead, flip in chunks on the workers into one vector of exact size,
     and sort it in place with `par_sort_unstable`, which needs no buffer
     and gives the same order when no two pairs are equal. The build then
     allocates the flip and nothing more. Expected (estimates): the four
     sites add about 69 MB, of which about 44 MB is above their flips; at
     the shares above, 10 to 30 MB off the all-cores median. Also 3 to
     6 ms off all-cores reasoning (to 96 to 99 ms, 4.1 to 4.2 times one
     thread): the builds' flips run on one thread before the step's jobs
     start, and `rdf:type`'s job, the longest of each step, flips on one
     thread and then sorts with a buffer.
   - Effect on the rubric: Speed (full run) on the all-cores axis, the
     dimension the hypothesis names first; Memory on all cores second.
     One thread keeps its path.
   - Files: `engine/store.rs` (`advance`'s first build,
     `Job::sort_index`), `engine/parallel.rs` (a sort for a
     duplicate-free vector beside `sort_dedup_batches`).
   - Risk: low. A flip is a map, and the sort restores the order.
     `Relation::from_sorted` asserts strict order in debug builds, so a
     repeated pair trips it. Below `PARALLEL_FROM` nothing changes. Chaos
     at one thread and all cores, and the tests that run every round in
     parallel (`Workers::every_round`), check it.
   - Measure: before the change, the relief diagnostic of (b) on the
     Baseline, 3 runs, probe only. Then the all-cores peak by the protocol
     above (10 runs per side, alternating, by medians and middle halves);
     the probe's increases at the two builds and at `rdf:type`'s index
     sorts, which should fall to about the size of each flip; P0's step
     times in both modes; reasoning time in both modes on lubm-10 and
     lubm-50, 10 runs alternating, on a quiet machine; chaos at one thread
     and all cores.
   - Why first: it serves both open budgets, the probe measured its sites,
     it is small and low in risk, and it holds under either answer to the
     owner's question.

2. **Keep the base once: each table's first tier shares the base's
   pairs.** Memory, both modes. Iteration 6's candidate 5, moved up
   because held bytes convert best.
   - Hypothesis: the reasoner holds the base twice. The base is 78.9 MB of
     `(s, p, o)` triples, 12 bytes each (7.6 bytes per closure triple). The
     store holds the same 6.57 million triples again as pairs in its
     tables' tiers: the tables other than `rdf:type` hold 52.6 MB of
     subject tiers, most of them base pairs (an estimate). Keep the base
     by predicate as sorted `(s, o)` pairs (52.6 MB, 26 MB less), and let
     a full run give each table its base pairs as its first tier by
     sharing them (an `Arc<[Pair]>`), not by copying. A shared tier is
     copied only when a merge takes it, which happens to tables that
     derive about half their size or more, such as `rdf:type` and
     `memberOf`. Expected (estimates): 55 to 70 MB less held (5.5 to 7
     bytes per closure triple); at iteration 4's share, 40 to 50 MB off
     the all-cores median and about as much off one thread. With
     candidate 1, the median would be about 520 to 550 MB: near the
     budget, not under it.
   - Effect on the rubric: Memory, both modes. Full runs no longer need
     `fill`'s count and copy.
   - Files: `reasoner.rs` (`Base`), `engine/relation.rs` (a shared form),
     `engine/variable.rs`, `engine/store.rs` (`fill`), `snapshot.rs` (the
     format reads the base).
   - Risk: medium to high, and the largest change here. Removals,
     `inferred()`, the base's lookups and its merge of new triples, and
     the snapshot format all read the base. Restarts rewrite the pairs,
     so they keep the copying path. Chaos, the incremental chaos and the
     snapshot round trip check it.
   - Measure: P0's held bytes (base and store) in both modes; the peak in
     both modes by the protocol; load time and incremental times on the
     editor-size scenarios.

3. **Run each property-family rule as one split over all its axioms.**
   All cores, speed. Iteration 6's candidate 4.
   - Hypothesis: `copies` (`rules/property.rs`) walks the schema tuples
     and calls `Round::split` once per axiom and part. LUBM's ontology has
     25 `rdfs:domain`, 18 `rdfs:range`, 5 `rdfs:subPropertyOf` and 3
     `owl:inverseOf` tuples (`univ-bench.ttl`), so a round can run about
     50 splits one after another, each a fork and join with a `Pending`
     per chunk. In round 1 the family takes 8.0 ms on one thread and 8.0
     to 8.4 ms on all cores; in round 3, 1.5 ms against 3.5 to 3.7 ms.
     Instead, gather each rule's work for the round (each axiom's parts,
     with its target) into one list and cut it into even chunks across
     axioms: one fork and join per rule per round. The emitted set does
     not change, and copies ask for no merges, so their order does not
     matter (`Round::split`'s doc). Expected (estimates): the family at 3
     to 4 ms in round 1 and about 1 ms in round 3 on all cores. Round 1 is
     then bound by the datatype family (6.3 to 7.9 ms), so 3 to 5 ms off
     all-cores reasoning.
   - Effect on the rubric: Speed (full run) on the all-cores axis.
   - Files: `rules/property.rs` (`copies`, `copy_each`, `each_axiom`), and
     `engine/store.rs` if `Round` needs a split over a list of slices.
   - Risk: low. First count, with a counter, the splits each round runs on
     lubm-50 and their sizes: if they are few or large, the time is
     elsewhere (writing fresh pages, or `compact`'s sorts, which on all
     cores run inside the family). Chaos at one thread and all cores.
   - Measure: P0's family times per round in both modes, 10 runs
     alternating; reasoning time in both modes on lubm-10 and lubm-50; the
     all-cores peak by the protocol (fewer chunks may also mean fewer
     round buffers).

4. **Sort and deduplicate a parallel step's batches in place.** All
   cores: memory, and speed either way. New: it takes the memory half of
   iteration 6's candidate 3 (now candidate 6).
   - Hypothesis: on all cores, `sort_dedup_batches` copies a table's
     batches into a fresh vector (`concat`), sorts it with `par_sort` and
     its buffer as long as the input, deduplicates it into pieces and
     copies the pieces into one vector again. At that last copy, 3 times
     the input is alive. One thread sorts in place in the largest batch's
     allocation. Instead, join the batches into the largest batch's
     allocation as one thread does, sort with `par_sort_unstable`, and
     deduplicate in place: each chunk keeps its survivors at its start,
     then one pass closes the gaps from left to right (`copy_within`, the
     helper on `exp/e06-one-vector-merge`), and `fit` gives back the room.
     The step sorts run for every active table at once, which is where the
     bytes in use reach their highest (460.6 MB in step 3). The probe shows
     increases of 1 to 11 MB at the ends of step sorts in steps 1 to 3.
     Expected (estimates): 5 to 20 MB off the all-cores median. Speed may
     go either way: `par_sort` gains from the sorted runs that `compact`
     gives each chunk, and `par_sort_unstable` does not.
   - Effect on the rubric: Memory on all cores; Speed (full run) on the
     all-cores axis, either way.
   - Files: `engine/parallel.rs` (`sort_dedup_batches`, `joined`, a helper
     that closes gaps).
   - Risk: low to medium. A chunk must drop a pair equal to the last pair
     of the chunk before it, read before any pair moves (today's code
     reads `items[from - 1]`). Chaos at one thread and all cores, and
     `Workers::every_round`, check it.
   - Measure: as candidate 1. If the steps get slower, try candidate 6
     instead, which removes the sort.

5. **Give each chunk of a copy the room it will use.** Memory on all
   cores. Iteration 6's candidate 6, now with the probe's number.
   - Hypothesis: round 1 adds 82 to 84 MB of footprint while the bytes in
     use grow by 46 MB. In a parallel round each chunk of a split and each
     family task fills a `Pending` of its own, whose vectors grow by
     doubling: a chunk holds up to twice its pairs, and the smaller blocks
     it outgrew are freed on its thread. A copy (prp-dom, prp-rng,
     prp-spo1, prp-eqp1/2, prp-inv1/2, prp-symp) emits one pair per pair
     it reads, to one predicate, so its chunk can reserve its length
     before its loop and never double. Expected (estimates): round 1's
     increase falls toward the 46 MB its bytes in use grow by; 5 to 20 MB
     off the all-cores median.
   - Effect on the rubric: Memory on all cores; a little speed, with no
     regrowth copies.
   - Files: `engine/store.rs` (a way for a chunk's round to reserve room
     for one predicate), `rules/property.rs` (`copy_each`).
   - Risk: low. A reservation changes no tuple. First count which
     families emit round 1's pairs (P0's per-family counts): if the copies
     are a small share, the surplus is elsewhere.
   - Measure: the probe's increase in round 1; the all-cores peak by the
     protocol; reasoning time in both modes.

**For the speed budget alone**, the order is candidate 1 (3 to 6 ms),
candidate 3 (3 to 5 ms), then candidate 6 (a k-way merge, 4 to 8 ms, with
iteration 5's caution) or candidate 4 if it gains, then the object-index
merge beside the subject merge (1 to 3 ms, not while memory is open), then
the families that gain little (count page faults first). Candidates 1 and
3 together give an estimated 6 to 11 ms of the 21.6 ms needed. The rest
must come from the steps (60.6 ms on all cores) or from better use of the
6 Efficiency cores.

Still open, ranked after these five:

6. **Merge a parallel step's sorted batches k ways instead of joining and
   sorting them.** All cores, speed (iteration 4's candidate 3). Sort
   each family task's batch in its task (call `compact` in
   `Round::tasks`, as `Round::split` does), then merge all batches k ways,
   in ranges cut by values of the largest batch, side by side, dropping
   duplicates, each range into its own slice of one vector, with the gaps
   closed after. Expected (estimates): the steps from 60.6 to 52 to 56 ms
   on all cores, and reasoning to 93 to 97 ms. Iteration 5 is a caution:
   it removed 34 to 37 ms of sorting summed over tasks and gained about
   1 ms of wall time, since only `rdf:type`'s sort is on the critical
   path. Risk: medium; k can reach several hundred, and a loser tree
   costs about 8 comparisons a pair. Files: `engine/parallel.rs`,
   `engine/store.rs` (`Round::tasks`). It conflicts with candidate 4:
   run one of them, not both.
7. **Hash a document once.** Loading and total time (iteration 1's
   candidate 4). `DocumentKey` hashes all 934 MB of lubm-50 twice on one
   thread, with FxHash and SipHash (`reasoner.rs`). XXH64, already in
   `snapshot.rs`, run with two seeds in blocks, would cut about 0.3 s from
   both modes (an estimate). On one thread it is the change that would
   bring loading (1.52 s) under the 1.30 s bar, if the owner reads the
   budget for both modes. Older snapshots keep no document keys. Measure
   with P0's load breakdown and load time on lubm-10 and lubm-50.
8. **Sort the parsed base on the workers** (iteration 1's candidate 5).
   `Base::fresh` sorts 6.6 million triples on one thread. `par_sort` adds
   a 79 MB buffer while it runs, against the memory budget. Loading is met
   on all cores, and this does nothing on one thread, so it comes last.

Not ranked here, and why:

- **Return free memory to the system from the library**, or **choose
  another global allocator** (mimalloc, jemalloc) in the bench or the CLI.
  See (b) above. TurbOWL is a library, and a library can neither choose
  its embedder's allocator nor trim the embedder's heap. Only the bench and
  the CLI would gain, so the scoreboard would measure what an embedder
  does not get. Never report either as the library's number.
- **A merge of `rdf:type`'s object-index tiers without a fresh vector**
  (step 4's +28.5 MB, the probe's last large increase). One thread grows
  the larger tier in place and merges from the back (`merge_into`). On all
  cores there is no cheap form: each range's output overlaps the next
  range's input, so the ranges cannot run side by side. A sequential merge
  of the largest tier would put about 5 ms (an estimate) on the critical
  path, against the speed budget. Iteration 6's one-vector merge took
  7.5 MB off the median.
- **A pool that hands a step's freed vectors to the next round.** The
  allocator already does this on one thread. Across 12 threads it needs a
  shared pool, against `separate-before-serializing-shared-state`. Not
  before candidates 4 and 5 measure what reuse gains.
- **Run a table's object-index merge beside its subject merge.** 1 to 3 ms
  off all cores (an estimate), but it puts two 29 MB merges of `rdf:type`
  at the same time, at the end of the run, against the memory budget. Not
  while the all-cores peak is over 50.
- **The other families that gain little from threads.** The class family
  gains 1.7 times in round 3, the datatype family 2.8 to 3.5 times in
  round 1. Count page faults per round (`getrusage` around a round in a
  probe) before changing a family.
- **Drop the dictionary's slots during a run.** The slots (33.6 MB, held)
  sit idle through a run, and held bytes convert well (about 24 MB off the
  median, by iteration 4's share). But the first change after a full run
  would pay the rebuild, about 30 to 60 ms for 2.2 million terms (an
  estimate), against an editor's 0.2 ms per change. Not unless the owner
  accepts that cost.
- **E2, E8 and E1** (the owner's first picks, research §10) act on
  Brick's class hierarchy. No phase 2 budget is open on Brick, and LUBM's
  hierarchy is shallow. They come after the memory and all-cores budgets,
  or first if the owner keeps that order.
- **E7** (CSR for `rdf:type`'s object index): P0 puts that index at
  30.2 MB, 2.9 bytes per closure triple. A CSR of 4-byte subjects with
  offsets per class would save about 1.5 bytes held (an estimate, about
  11 MB off the median at iteration 4's share), but joins read an index
  as sorted pairs, so every join over it changes. Worth it after
  candidates 1 to 5 if memory is still above 50.
- **E10** (a blocked Bloom filter on each large stable tier): for
  many-round inputs. It costs 12 to 15% more memory, against the budget
  now.
- **Split IRIs into a namespace and a local name in the dictionary.** The
  text is 9.8 bytes per closure triple (101.5 MB, about 44 bytes a term),
  and LUBM's IRIs share a few thousand namespaces. A namespace id fits in
  the entry's 32-bit field, which an IRI leaves unused. It might save 4 to
  5 bytes per triple held (about 30 to 35 MB off the median at iteration
  4's share), but every reader of a borrowed `Term::Iri(&str)` changes,
  and so does the snapshot format: with candidate 2, the largest change
  here.
- **The dictionary merge** runs on one thread, 110 to 150 ms on lubm-50
  (ticket 18's log). A sharded dictionary that keeps first-seen ids would
  run it in parallel, for more effort than candidate 7 and a similar gain.
