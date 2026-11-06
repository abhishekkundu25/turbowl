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

## For the next agent

To continue the loop cold: read `AGENTS.md`, then this file from the top,
then the newest file in `docs/verification/`. Start at step 1 with a fresh
reviewer. The candidates the last reviewer ranked but that were not picked
are listed below; a new reviewer may reuse or replace them.

### Open candidates

Ranked by iteration 6's reviewer, for iteration 6 and later, by expected
gain per effort against the phase 2 budgets still open
(`docs/specs/v2-2-speed.md`, Non-functional budgets). Numbers marked as
estimates are not measured. The research's §10
(`docs/research/sota-algorithms-2026-11.md`) ranks the starting candidates,
P0 and E1 to E9, with hypotheses and how to measure them; §11 lists what
not to do and why.

Where the budgets stand, from iteration 5's evaluation (lubm-50 at
`1e74f80`, 3 runs in each thread mode, load 2.3 to 2.5, indicative; bytes
are millions of bytes, from each run's `peak_rss_bytes`):

- **Memory on all cores:** 600 to 605 MB, median 603 (58.3 bytes per
  closure triple), against at most 517.3 MB (50). The median must lose
  86 MB. The largest run, which the bench reports, must lose up to 130 MB
  (647 MB on `d21803b`). One thread peaks at 480 MB (46.4): met.
- **All cores against one thread:** 3.98 times (403.8 against 101.5 ms),
  against at least 5, which needs 80.8 ms on all cores: 20.7 ms less.
  Iteration 5 made one thread 13 ms faster and all cores 1 ms, so the
  ratio fell from 4.07: every one-thread gain raises the bar for all
  cores. By phase, the rounds take 118.3 against 36.3 ms (3.3 times), the
  steps 267.7 against 60.6 ms (4.4 times), the rest 17.8 against 4.6 ms.
  Steps 1 to 4 take 12.1 to 16.2 ms each on all cores. In round 1 the
  property family takes 8.0 ms in both modes; in round 3 it takes 1.5 ms
  on one thread and 3.5 to 3.7 ms on all cores; the class family gains
  1.7 times in round 3. The machine's 12 cores are 2 Super, 4 Performance
  and 6 Efficiency cores. Measure the ratio on the Baseline on a quiet
  machine before deciding it.
- **Loading:** 0.81 s on all cores and 1.52 s on one thread, against a bar
  of 1.30 s. Met on all cores. The owner decides whether the bar holds for
  one thread too; if it does, "hash a document once" (below) comes first
  for loading.
- **Total time:** 1.93 s on one thread and 0.91 s on all cores, against
  version 1's 4.66 s on lubm-50. Met there. The data-size scenarios have
  not run since phase 1.

**How to measure the all-cores peak.** On one thread every run peaks at
the same size, to 1 MB, in every record. On all cores the runs spread.
Per-run peaks of lubm-50 on unchanged code, from the records' JSON:

| Code | Runs | Peaks (MB) | Median (MB) |
|---|---:|---|---:|
| `b1d4b93` (e02 after, e03 before) | 6 | 624, 626, 626, 630, 630, 675 | 628 |
| `eebb962` (e03 after, e04 before) | 6 | 627, 627, 628, 630, 631, 632 | 629 |
| `d21803b` (e04 after, e05 before) | 6 | 598, 599, 602, 608, 632, 647 | 605 |
| `1e74f80` (e05 after) | 3 | 600, 603, 605 | 603 |

18 of the 21 runs sit within 7 MB of their median; 3 land 27 to 47 MB
above it. The bench reports the largest run, so one high run decides the
number in its table. That hid iteration 4's gain on all cores: its record
says the peak did not move (599 to 603 MiB, the largest runs), but the
median fell 24 MB (2.3 bytes per triple), about 70% of the 33.6 MB of
slots it freed. So:

- Run each side at least 10 times, each run its own process (as the bench
  runs it), alternating the two builds run by run (A, B, A, B, ...), with
  nothing else running. A lubm-50 run takes a few seconds.
- Decide on the per-run `peak_rss_bytes` in the JSON, not on the table.
  Report each side's median, quartiles, smallest and largest run, and its
  count of high runs (more than 20 MB over its median).
- A change moves the peak only when the medians differ by more than
  10 MB (1 byte per closure triple) and the two sides' middle halves do
  not overlap. A change that only makes high runs rarer says so, with the
  counts.
- The budget passes only when the largest run is under 517.3 MB, since
  that is what the bench reports.
- Find where the peak falls before changing it. No probe has run on all
  cores. Read the process's lifetime peak (`ri_lifetime_max_phys_footprint`,
  as `crates/bench/src/machine.rs` does) after loading, after
  `Reasoner::start`, after each round and after each step, on all cores,
  5 runs; inside a step, also after each phase of `rdf:type`'s job (sort,
  change, index sort, index change). Read the allocator's bytes in use
  (`malloc_zone_statistics`) at the same marks. The mark where the
  lifetime peak reaches its final value holds the peak. The gap there
  between the footprint and the bytes in use is freed memory the
  allocator keeps. Keep the output in `docs/verification/data/`.

**Where the all-cores peak goes** (estimates, from the code and P0). The
reasoner holds 376.0 MB after the run (36.3 bytes per closure triple): the
dictionary 171.8 MB (text 101.5, entries 36.7, slots 33.6), the base
78.9 MB, the store 125.3 MB (`rdf:type` 60.4 with its 30.2 MB object
index, `takesCourse` 22.7 with its 11.4 MB index). The median peak's other
227 MB (21.9 bytes per triple) is not held. On one thread that share is
104 MB (10.0), so the parallel paths add about 123 MB. Two kinds of memory
make it up, in shares no one has measured:

- Copies alive together in the steps. Each parallel operation in
  `engine/parallel.rs` keeps its inputs, its pieces and its output alive
  at once. In step 4's change, the run's largest (7.3 ms on all cores),
  `rdf:type`'s subject tiers merge into about 29 MB: inputs 29, pieces 29
  and output 29, so 58 MB beyond the held bytes. Its object index then
  repeats it, and `takesCourse`'s tiers may merge beside it. On one
  thread, `merge_into` grows the larger tier in place: at most 29 MB
  beyond. In step 3's sort, `rdf:type`'s 3.5 million emitted pairs (28 MB)
  stand at 2 times that beyond held at the join (`concat`), the sort
  (`par_sort`'s buffer) and the deduplication (pieces, then `concat`). An
  object index's sort holds 3 times its flip at its last copy.
- Freed memory the allocator keeps: blocks under 8 MB freed on 12 threads.
  The sources are each chunk's round buffer growing by doubling,
  `compact`'s outputs, the merge, filter and deduplication pieces (each
  about a 48th of a table), and the loader's piece tables.

Two results narrow it. Iteration 4's 33.6 MB less held lowered the median
by 24 MB, so the peak falls after the dictionary reaches its full size.
Iteration 5 removed step 0's copies of the base (about 3 times
52.6 MB alive at once), and the median did not move, so the peak is not
at step 0. Iteration 3's probe placed the one-thread peak in the run's
last steps.

1. **Write a parallel merge and a parallel filter into one vector.**
   Memory, all cores (iteration 4's candidate 4). **Iteration 6.**
   - Hypothesis: `parallel_merge` (two tiers joining when a recent part
     moves to `stable`, or two queued batches) merges each range into a
     vector of its own, and `concat` copies the pieces into one vector,
     which it first fills with placeholders. At the copy, the inputs, the
     pieces and the output are alive: 3 times the merged size, where one
     thread's `merge_into` needs at most 2. `Workers::absent` (the new
     pairs filtered against each tier) does the same: the fresh pairs,
     the kept pieces and the output. Instead, give each merge range a
     slice of one output vector, as long as its two input ranges, and
     merge into it directly, as `by_predicate` already fills its vectors.
     The tiers are disjoint (`Variable`'s invariant), so their slices fill
     exactly. Two queued batches may overlap: one pass then closes the
     gaps from left to right (`copy_within`), and `fit` gives back the
     rest. The filter needs no new vector: each chunk of the fresh pairs
     keeps its survivors at the start of its own range, in place, then
     the same pass closes the gaps. In step 4, `rdf:type`'s 29 MB merge
     then needs 29 MB beyond the held bytes, not 58, and so does its
     object index's. Expected (estimates): the all-cores median peak 20 to
     30 MB lower (2 to 3 bytes per closure triple), if the peak falls in
     a step's change, as on one thread. Each merge writes its pairs twice,
     not three times, and the filter allocates nothing: up to 2 ms off
     all-cores reasoning.
   - Effect on the rubric: Memory on all cores, a quarter to a third of
     the 86 MB gap. Speed a little. One thread does not change: it takes
     neither path.
   - Files: `engine/parallel.rs` only (`merge`, `parallel_merge`,
     `absent`, and a helper that closes gaps; `concat` stays for
     `joined`).
   - Risk: low to medium. A slice sized wrong writes past its range or
     leaves a gap uncounted: `Relation::from_sorted` asserts strict order
     in debug builds, so a duplicate or a hole trips it. Chaos at one
     thread and all cores, and the tests that run every round in
     parallel (`Workers::every_round`), check it. Placeholders written by
     the workers, not a zeroed allocation: `by_predicate`'s doc measured
     a zeroed one 13 MB higher.
   - Measure: first the probe above, on the Baseline. If it puts the peak
     in a step's change, run this. If it puts it in an index sort, run
     candidate 2 first; if in a step's sort, candidate 3. Then the
     all-cores peak by the protocol above (10 runs each side,
     alternating); P0's change time per step on all cores; reasoning time
     in both modes on lubm-10 and lubm-50; chaos at one thread and all
     cores.

2. **Build and extend object indexes on the workers, sorting a flip
   without deduplicating it.** All cores: speed, and memory in the index
   sorts (iteration 4's candidate 6, widened).
   - Hypothesis: when a rule first asks for an index, `Store::advance`
     flips the whole table on the caller's thread, one table after
     another, before the step's jobs start. Each step, `Job::sort_index`
     flips the table's new recent part in one loop. `rdf:type` (3.78
     million pairs at the end) and `takesCourse` (1.42 million) have
     object indexes, and `rdf:type`'s job is the longest of each step, so
     these flips sit on the critical path. Then `sorted` deduplicates the
     flip. A flip of a duplicate-free relation has no duplicate, so on
     all cores that is a deduplication pass into pieces and a `concat`
     copy for nothing, after `par_sort`, which allocates a buffer as long
     as its input. Instead, flip in chunks on the workers and sort in
     place with `par_sort_unstable`, which needs no buffer and gives the
     same order when no two pairs are equal. Expected (estimates): 3 to
     6 ms off all-cores reasoning (to 95 to 98 ms, 4.1 to 4.2 times one
     thread). An index sort's transient falls from 3 times its flip to
     none beyond the flip: about 16 MB less in step 3, for `rdf:type`'s
     1 million new pairs.
   - Effect on the rubric: Speed (full run) on the all-cores axis. Memory
     on all cores if the peak falls in an index sort.
   - Files: `engine/store.rs` (`advance`, `Job::sort_index`, a sort for
     flips beside `sorted`).
   - Risk: low. A flip is a map, and the sort after it restores the
     order. Below `PARALLEL_FROM` nothing changes. A debug assertion
     checks that the sorted flip is strictly increasing.
   - Measure: P0's step wall times on all cores and one thread (the flips
     are inside the sort time); reasoning time in both modes on lubm-10
     and lubm-50, 10 runs alternating, on a quiet machine; the all-cores
     peak by the protocol.

3. **Merge a parallel step's sorted batches k ways instead of joining and
   sorting them.** All cores: speed and memory (iteration 4's candidate
   3). Iteration 5 removed the base's batches from step 0; the rest
   stands.
   - Hypothesis: in a parallel step a table's pairs come as many batches:
     one per chunk of each split, already sorted and deduplicated
     (`Pending::compact`), and one per family task, unsorted.
     `sort_dedup_batches` joins them (`concat`), sorts them with
     `par_sort`, deduplicates them into pieces and copies those again. It
     writes each pair about four times and holds 2 times the input at
     each stage. Instead, sort each family task's batch in its task (call
     `compact` in `Round::tasks`, as `Round::split` does), then merge all
     batches k ways, in ranges cut by values of the largest batch, side
     by side, dropping duplicates. Each range writes into its own slice
     of one vector, then the gaps close, as in candidate 1. Pages of a
     slice that a range leaves unwritten need not become resident. Expected
     (estimates): the steps from 60.6 to 52 to 56 ms on all cores, and
     reasoning to 93 to 97 ms (4.2 to 4.3 times one thread). The sort's
     transient falls from 2 times its input to its input plus its
     output: about 14 MB less in step 3 for `rdf:type`. Iteration 5 is a
     caution: it removed 34 to 37 ms of sorting summed over tasks and
     gained about 1 ms of wall time. Only the longest job's sort, which
     is `rdf:type`'s, is on the critical path, so watch the step's wall
     time, not the summed sort time.
   - Effect on the rubric: Speed (full run) on the all-cores axis, and
     Memory on all cores if the peak falls in a step's sort.
   - Files: `engine/parallel.rs` (`sort_dedup_batches`: a k-way path when
     every batch is sorted, reusing candidate 1's gap closing),
     `engine/store.rs` (`Round::tasks`: compact each task's output).
   - Risk: medium. A batch taken for sorted that is not gives a wrong
     table: assert it in debug builds, and compare chaos at one thread
     with all cores. k can reach several hundred: a loser tree costs
     about 8 comparisons a pair, and hundreds of input streams may miss
     the cache more than `par_sort` does. One thread does not change.
   - Measure: P0's step wall time and sort time per step on all cores;
     reasoning time in both modes on lubm-10 and lubm-50, 10 runs
     alternating, on a quiet machine; the all-cores peak by the protocol;
     chaos at one thread and all cores.

4. **Run each property-family rule as one split over all its axioms.**
   All cores, speed. New.
   - Hypothesis: `copies` (`rules/property.rs`) walks the schema tuples
     and calls `Round::split` once per axiom and part. LUBM's ontology
     has 25 `rdfs:domain`, 18 `rdfs:range`, 5 `rdfs:subPropertyOf` and 3
     `owl:inverseOf` tuples (`univ-bench.ttl`), so a round can run about
     50 splits one after another. Each split of a part over 4,096 pairs is
     a fork and join, with a `Pending` per chunk. In round 1 the
     family takes 8.0 ms on one thread and 8.0 to 8.4 ms on all cores; in
     round 3, 1.5 ms against 3.5 to 3.7 ms. Instead, gather each rule's
     work for the round (each axiom's parts, with its target) into one
     list and cut it into even chunks across axioms: one fork and join
     per rule per round. The emitted set does not change, and copies ask
     for no merges, so their order does not matter (`Round::split`'s
     doc). Expected (estimates): the family at 3 to 4 ms in round 1 and
     about 1 ms in round 3 on all cores. Round 1's wall time is then
     bound by the datatype family (6.3 to 7.9 ms), so 3 to 5 ms off
     all-cores reasoning.
   - Effect on the rubric: Speed (full run) on the all-cores axis.
   - Files: `rules/property.rs` (`copies`, `copy_each`, `each_axiom`), and
     `engine/store.rs` if `Round` needs a split over a list of slices.
   - Risk: low. Chaos at one thread and all cores. First count, with a
     counter, the splits each round runs on lubm-50 and their sizes: if
     they are few or large, the time is elsewhere (writing fresh pages,
     or `compact`'s sorts, which on all cores run inside the family).
   - Measure: P0's family times per round in both modes, 10 runs
     alternating; reasoning time in both modes on lubm-10 and lubm-50.

5. **Keep the base once: each table's first tier shares the base's
   pairs.** Memory, both modes. New; it widens iteration 1's candidate 2
   ("keep the base by predicate"), which it replaces.
   - Hypothesis: the reasoner holds the base twice. The base is 78.9 MB
     of `(s, p, o)` triples, 12 bytes each (7.6 bytes per closure
     triple). The store holds the same 6.57 million triples again as
     pairs in its tables' tiers: the tables other than `rdf:type` hold
     52.6 MB of subject tiers for 6.57 million pairs, most of them base
     pairs (an estimate). If the base is kept by predicate as sorted `(s, o)`
     pairs (52.6 MB, 26 MB less), and a full run gives each table its
     base pairs as its first tier by sharing them (an `Arc<[Pair]>`),
     not by copying, the store holds only what the run derives plus the
     tiers that merges rewrote. A shared tier is copied only when a merge
     takes it, which happens to tables that derive about half their size
     or more, such as `rdf:type` and `memberOf`. Expected (estimates):
     26 MB from the base's form and 30 to 45 MB from sharing, 5.5 to 7
     bytes per closure triple in both modes. That would bring the
     all-cores median to about 51 to 53, the largest held saving left.
   - Effect on the rubric: Memory, both modes. Full runs no longer need
     `fill`'s count and copy.
   - Files: `reasoner.rs` (`Base`), `engine/relation.rs` (a shared form),
     `engine/variable.rs`, `engine/store.rs` (`fill`), `snapshot.rs`
     (the format reads the base).
   - Risk: medium to high, and the largest change here. Removals,
     `inferred()`, the base's lookups and its merge of new triples, and
     the snapshot format all read the base. Restarts rewrite the pairs,
     so they keep the copying path. Chaos, the incremental chaos and the
     snapshot round trip check it.
   - Measure: P0's held bytes (base and store) in both modes; the peak in
     both modes by the protocol; load time and incremental times on the
     editor-size scenarios.

Still open, ranked after these five:

6. **Give each predicate's round buffer the room it used last round**
   (iteration 4's candidate 5). It was aimed at one thread, which now
   meets the budget. On all cores the same idea applies to each chunk's
   buffer: a copy emits one pair per pair it reads, so a chunk of a copy
   can reserve its length before its loop and never double. Run it only
   if the probe says that freed doubling blocks are a large part of the
   all-cores peak.
7. **Hash a document once.** Loading and total time (iteration 1's
   candidate 4). `DocumentKey` hashes all 934 MB of lubm-50 twice on one
   thread, with FxHash and SipHash (`reasoner.rs`). XXH64, already in
   `snapshot.rs`, run with two seeds in blocks, would cut about 0.3 s from
   both modes (an estimate). On one thread it is the change that would
   bring loading (1.52 s) under the 1.30 s bar, if the owner reads the
   budget for both modes. Risk and measurement as iteration 1 wrote them:
   older snapshots keep no document keys; P0's load breakdown; load time
   on lubm-10 and lubm-50.
8. **Sort the parsed base on the workers** (iteration 1's candidate 5).
   `Base::fresh` sorts 6.6 million triples on one thread. `par_sort` adds
   a 79 MB buffer while it runs, against the memory budget. Loading is met
   on all cores, and this does nothing on one thread, so it comes last.

Not ranked here, and why:

- **Run a table's object-index merge beside its subject merge.** The
  index's tiers do not depend on the step's new pairs, so their merge
  could run while the subject side sorts: 1 to 3 ms off all cores (an
  estimate). But it puts two 29 MB merges of `rdf:type` at the same time,
  at the end of the run, against the memory budget. Not while the
  all-cores peak is over 50.
- **The other families that gain little from threads.** The class family
  gains 1.7 times in round 3, the datatype family 2.8 to 3.5 times in
  round 1. Count page faults per round (`/usr/bin/time -l`, or
  `getrusage` around a round in a probe) before changing a family.
  Candidate 4 covers the property family.
- **Allocator tricks**: another global allocator (mimalloc, jemalloc), or
  asking macOS's allocator to give memory back
  (`malloc_zone_pressure_relief`). TurbOWL is a library, and a library
  cannot choose its embedder's allocator: only the bench and the CLI
  would gain, so the scoreboard would measure what an embedder does not
  get. Giving memory back at the end does not lower a peak that falls
  during the run. Even if the probe shows that freed memory kept per
  thread is most of the all-cores gap, cut what the program frees first
  (candidates 1 to 3 and 6), and never report a trick as the library's
  number.
- **Drop the dictionary's slots during a run.** Reasoning reads terms by
  id and never interns (`Context` borrows the dictionary), so the slots
  (33.6 MB) sit idle through a run. Dropping them and rebuilding at the
  next intern would cut 3.2 bytes per closure triple. But the first change
  after a full run would pay the rebuild, about 30 to 60 ms for 2.2
  million terms (an estimate), against an editor's 0.2 ms per change. Not
  unless the owner accepts that cost.
- **E2, E8 and E1** (the owner's first picks, research §10) act on
  Brick's class hierarchy. No phase 2 budget is open on Brick, and LUBM's
  hierarchy is shallow. They come after the memory and all-cores budgets,
  or first if the owner keeps that order.
- **E7** (CSR for `rdf:type`'s object index): P0 puts that index at
  30.2 MB, 2.9 bytes per closure triple. A CSR of 4-byte subjects with
  offsets per class would save about 1.5 (an estimate), but joins read an
  index as sorted pairs, so every join over it changes. Worth it after
  candidates 1 to 5 if memory is still above 50.
- **E10** (a blocked Bloom filter on each large stable tier): for
  many-round inputs. It costs 12 to 15% more memory, against the budget
  now.
- **Split IRIs into a namespace and a local name in the dictionary.** The
  text is 9.8 bytes per closure triple (101.5 MB, about 44 bytes a term),
  and LUBM's IRIs share a few thousand namespaces. A namespace id fits in
  the entry's 32-bit field, which an IRI leaves unused. It might save 4
  to 5 bytes per triple, but every reader of a borrowed `Term::Iri(&str)`
  changes, and so does the snapshot format: with candidate 5, the largest
  change here.
- **The dictionary merge** runs on one thread, 110 to 150 ms on lubm-50
  (ticket 18's log). A sharded dictionary that keeps first-seen ids would
  run it in parallel, for more effort than candidate 7 and a similar gain.
