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

## For the next agent

To continue the loop cold: read `AGENTS.md`, then this file from the top,
then the newest file in `docs/verification/`. Start at step 1 with a fresh
reviewer. The candidates the last reviewer ranked but that were not picked
are listed below; a new reviewer may reuse or replace them.

### Open candidates

Ranked by iteration 3's reviewer, for iterations 3 and later, by expected
gain per effort against the phase 2 budgets still open
(`docs/specs/v2-2-speed.md`, Non-functional budgets). Numbers marked as
estimates are not measured. The research's §10
(`docs/research/sota-algorithms-2026-11.md`) ranks the starting candidates,
P0 and E1 to E9, with hypotheses and how to measure them; §11 lists what
not to do and why.

Where the budgets stand, from iteration 2's evaluation (lubm-50, 3 runs,
load 2 to 3, indicative):

- **Memory:** 57.5 bytes per closure triple on one thread and 65.2 on all
  cores (60.3 to 65.2 over the runs), against at most 50. Held after the
  run: the dictionary 19.8 (text 9.8, hash slots 6.5, entries 3.5), the
  base 7.6, and the store 21.2 on one thread and 12.1 on all cores. Not
  held: 8.8 and 25.6.
- **All cores against one thread:** 3.9 times (416.8 against 107.3 ms),
  against at least 5, which needs about 83 ms on all cores. On one thread
  the rounds take 118 ms, the steps 273 ms (233 ms of it sorting) and the
  rest 26 ms. On all cores they take 36 to 41 ms, 62 to 74 ms and 4 ms. The
  rounds gain 2.9 to 3.3 times, the steps 3.7 to 4.4 and the rest 6. The
  machine's 12 cores are 2 Super, 4 Performance and 6 Efficiency cores
  (`sysctl hw.perflevel*`), and one thread runs on a Super core, so 12
  threads are worth fewer than 12 single threads. At load 2, other jobs
  also held about 2 cores. Measure the ratio on the Baseline on a quiet
  machine before deciding it.
- **Loading:** 0.81 s on all cores and 1.52 s on one thread, against a
  bar of 1.30 s (a third of version 1's 3.9 s). Met on all cores. The
  spec does not name the thread mode, and the owner decides which reading
  holds. If it is both, "hash a document once" (below) comes first for
  loading.
- **Total time:** 1.93 s on one thread and 0.92 s on all cores, against
  version 1's 4.66 s on lubm-50. Met there. The data-size scenarios have
  not run since phase 1.

**Why the store holds 21.2 bytes per triple on one thread and 12.1 on all
cores.** Both modes hold the same 10,345,697 pairs. The difference is
capacity, 94.4 MB, or 9.1 bytes per closure triple: 76.6 MB in the stable
batches, 8.4 MB in recent parts that are empty and 9.4 MB in the object
indexes. The two modes end a round through different code:

- One thread (`Variable::changed`) keeps every allocation it is given.
  The step joins a table's pairs into the largest batch's vector, which
  grew by doubling as the rules pushed into it, duplicates included, and
  sorts and deduplicates it in place (`Workers::sort_dedup_batches` on one
  thread). `retain_absent` filters it in place into the new recent part.
  `Relation::merge` keeps the larger vector and grows it by doubling when
  full (`merge_into`). So `rdf:type`'s stable batch holds 64 MiB for
  3,775,750 pairs (30.2 MB of them). `memberOf` holds 17.8 MB for 591,723
  pairs (4.7 MB). After the fixpoint every recent part is empty, yet
  `rdf:type`'s still holds 4 MiB and the others another 4 MiB.
- All cores (`Variable::changed_on` with workers) builds a new vector for
  each large merge, sort and filter (`Workers::merge`, `sort_dedup`,
  `absent`), each made by `concat` with exactly its length. So every table
  holds 8 bytes a pair: `rdf:type` 30,206,096 bytes, its recent part
  96 bytes. The price is copies. At each `concat` the inputs, the pieces
  per chunk and the output are all alive, and rayon's `par_sort` adds a
  buffer as long as its input. The tables step side by side. So all
  cores hold less after the run and more at the peak: 25.6 bytes per
  closure triple not held, against 8.8 on one thread.

1. **Size the store's relations to their contents.** Memory, on one
   thread. **Iteration 3.**
   - Hypothesis: on one thread, the store's 21.2 bytes per closure triple
     fall to 12.1 to 13.0, the all-cores figure. The peak falls from 57.5
     to about 49 to 51 (an estimate). The vectors grow through the run,
     so most of the 94 MB of slack is probably held at the peak too, and
     the budget is met if at least 78 MB of it is. Rule: after a sort, a
     filter or a merge, a relation that holds more than an eighth of
     slack gives it back (`Base::merge` already does this with a
     quarter), and an empty recent part holds no allocation. All cores
     do not change: their vectors are exact already.
   - Effect on the rubric: Memory, +1 if one thread passes under 50; it
     closes half of the memory budget. Speed within the spread.
   - Files: `engine/relation.rs` (`from_vec`, `merge`, `merge_into`),
     `engine/variable.rs` (`end_round`, `retain_absent`), and
     `engine/store.rs` if the one-thread sort in `sorted_batches` needs
     it.
   - Risk: low for correctness, since no tuple changes. A shrink is a
     `realloc`, in place for a large block, but a later merge into a
     shrunk vector must grow it again, so a run of small rounds could copy
     a large tier each time. The tiering merges a recent part into a tier
     only when the tier is at most twice its size, so each copy is paid by
     its merge. The `scaling_variable_*` tests check that. One-thread
     reasoning may move by 1 to 2%.
   - Measure: P0's store parts on one thread (stable, recent, object
     index; `rdf:type` and the others) against the all-cores ones. Peak
     and reasoning time on one thread and all cores, lubm-10 and lubm-50,
     5 runs. Brick-buildings and the incremental scenarios, for no
     regression. The engine's `scaling_*` tests. To tell loading's peak
     from reasoning's, read the footprint once after `load` alone (a
     one-off probe).

2. **Merge a parallel step's sorted batches instead of joining and
   sorting them.** All cores: speed and memory.
   - Hypothesis: in a parallel step a table's pairs come as many batches.
     There is one per chunk of each split, already sorted and deduplicated
     (`Pending::compact`), one per chunk of the base in the first step
     (sorted, since the base is sorted by subject, predicate, object),
     and one per family task (unsorted). `sort_dedup_batches` copies them
     into one vector (`concat`). It sorts that with `par_sort`, a merge
     sort with a buffer as long as its input. It deduplicates into a
     vector per chunk and copies those into one vector again. That writes
     each pair about four times and holds 3 to 4 times the input at once.
     Instead, sort each unsorted batch in place, one task each. Then merge
     all batches k ways, in ranges cut by values of the largest batch,
     side by side, dropping duplicates as it goes, into one output. Each
     pair is then written once. P0 puts the all-cores step at 62 to 74 ms
     of 103 to 118 ms, and its sorting at 114 to 125 ms summed over tasks,
     against 17 to 23 ms of changes. Expected (estimates): steps down to
     45 to 55 ms, reasoning to 85 to 90 ms (4.6 to 4.9 times one thread),
     and the all-cores transient from 25.6 to about 17 bytes per closure
     triple. One thread does not change: its round has one buffer, so a
     table gets one batch.
   - Effect on the rubric: Speed (full run) on the all-cores axis, which
     brings the 5-times budget within reach, probably not alone. Memory on
     all cores.
   - Files: `engine/parallel.rs` (`sort_dedup_batches`: a k-way path for
     sorted batches), `engine/store.rs` (`Pending`: say which batches are
     sorted, or sort a task's own batch in its task).
   - Risk: medium. A batch the merge takes for sorted that is not gives a
     wrong table: assert it in debug builds, and compare chaos at one
     thread with all cores. The result is the sorted set, as on one
     thread, so thread-count invariance holds. With 48 chunks and several
     families, k can reach several hundred: a loser tree costs about log k
     comparisons a pair.
   - Measure: P0's step wall time and sort time per step on all cores; the
     bench's reasoning time at one thread and all cores, lubm-10 and
     lubm-50, 5 runs, on a quiet machine; the all-cores peak and its
     spread; chaos at one thread and all cores.

3. **Keep the dictionary's hash slots up to three quarters full.**
   Memory, both modes.
   - Hypothesis: the slots are `u64` and at most half full (`term.rs`:
     `insert`, `index_appended`, `reserve`). lubm-50's terms (the entries
     hold 2,293,760) sit in 8,388,608 slots, 64 MiB, about 27% full. With
     slots up to 3/4 full they fit in 4,194,304 slots, 32 MiB: 3.2 bytes
     per closure triple less in both modes. Over all input sizes a term
     costs 10.7 to 21.3 bytes of slots instead of 16 to 32, a third less.
     With candidate 1, one thread would reach about 46 to 48 (an
     estimate).
   - Effect on the rubric: Memory, in both modes.
   - Files: `term.rs` (the three growth rules and the doc comment on
     `slots`); `snapshot.rs` only if it sizes the slots itself.
   - Risk: linear probing at 3/4 full reads more slots. A miss, which
     every new term is, reads about 8.5 slots at 3/4 against 2.5 at 1/2,
     in one or two cache lines, since each slot holds the term's 32-bit
     hash. The slot comes from FxHash's low bits, and clusters grow with
     the load. Interning bounds loading (the merge takes 110 to 150 ms on
     one thread, ticket 18), so loading may slow 1 to 3% (an estimate).
   - Measure: P0's `dictionary_slots`; the bench's load time on lubm-10
     and lubm-50, one thread and all cores, 5 runs; the interning
     `scaling_*` tests; the snapshot's load time.

4. **Write a parallel merge and a parallel filter into one vector.**
   Memory, all cores.
   - Hypothesis: `Workers::merge` merges each range into a vector of its
     own, and `concat` copies them into one. At the copy, the two inputs,
     the pieces and the output are alive: 3 times the merged size.
     `Workers::absent` does the same. Ending a round merges parts that are
     disjoint (a variable keeps its tiers and its recent part disjoint),
     so the result has exactly a + b pairs, and each range's offset
     follows from its cuts. The ranges can then write into one vector
     directly, which holds 2 times the merged size. The filter can keep
     each chunk's survivors in place and close the gaps after. On
     lubm-50, `rdf:type`'s tiers reach 30 MB. Expected (estimates): 2 to
     5 bytes per closure triple less at the all-cores peak, and a fifth
     off the all-cores change time (17 to 23 ms summed over tasks).
   - Effect on the rubric: Memory on all cores; speed a little.
   - Files: `engine/parallel.rs` (`merge`, `parallel_merge`, `absent`),
     `engine/variable.rs` (`changed_on`: the disjoint merge for the tiers;
     the general merge stays for two queued batches, which may overlap).
   - Risk: the disjoint form on parts that overlap writes duplicates. Use
     it only where the invariant holds, with a debug assertion. Chaos at
     one thread and all cores checks it.
   - Measure: the all-cores peak and its spread, 5 runs; P0's change
     time per step; chaos at one thread and all cores.

5. **Flip the object index's pairs on the workers.** All cores
   (iteration 1's candidate 3, re-ranked).
   - Hypothesis: a newly requested index is built by a sequential flip of
     the whole table at the start of a step, one table after another
     (`Store::advance`). Each step also flips each indexed table's new
     pairs in one loop (`Job::sort_index`). On lubm-50, `rdf:type`
     (3.78 M pairs at the end) and `takesCourse` (1.42 M) have object
     indexes. `rdf:type`'s job is the longest of each step, so its flips
     sit on the serial path. In chunks on the workers they leave it: 3 to
     6 ms off the all-cores reasoning (an estimate), or 3.9 to about 4.1
     times one thread.
   - Effect on the rubric: Speed (full run), all-cores axis. Together with
     candidate 2 it may reach 5 times.
   - Files: `engine/store.rs` (`advance`, `Job::sort_index`).
   - Risk: low. A flip is a map, and the sort after it restores the order.
     Below `PARALLEL_FROM` nothing changes.
   - Measure: P0's step wall times at one thread and all cores (the flips
     are inside the sort time, not apart); the bench's reasoning time,
     lubm-10 and lubm-50, 5 runs, on a quiet machine.

Still open from iteration 1, ranked after these five:

6. **Hash a document once.** Loading and total time (iteration 1's
   candidate 4). `DocumentKey` hashes all 934 MB of lubm-50 twice on one
   thread, with FxHash and SipHash (`reasoner.rs`). XXH64, already in
   `snapshot.rs`, run with two seeds in blocks, would cut about 0.3 s from
   both modes (an estimate). On one thread it is the change that would
   bring loading (1.52 s) under the 1.30 s bar, if the owner reads the
   budget for both modes. Risk and measurement as iteration 1 wrote them:
   older snapshots keep no document keys; P0's load breakdown; load time
   on lubm-10 and lubm-50.
7. **Keep the base by predicate, as sorted (s, o) pairs.** Memory and
   full runs (iteration 1's candidate 2). The base is 7.6 bytes per
   closure triple (78.9 MB, 12 bytes a triple). Pairs per predicate take
   8 bytes, 2.5 bytes per closure triple less in both modes, and a full
   run with an empty canon could hand each predicate's pairs to its table
   as one sorted batch. More effort than candidates 1 to 4 for a similar
   gain. Take it if memory is still above 50 after them.
8. **Sort the parsed base on the workers.** Loading on all cores
   (iteration 1's candidate 5). `Base::fresh` sorts 6.6 M triples on one
   thread. `par_sort` adds a 79 MB buffer while it runs, against the
   memory budget. Loading is met on all cores, and this does nothing on
   one thread, so it comes last.

Not ranked here, and why:

- **The families that gain little from threads.** In round 1 the
  property family takes 8.1 ms on one thread and 7.8 to 8.7 ms on all
  cores, and the datatype family 22.0 against 7.5 to 8.0 ms. In round 3
  the class family takes 12.7 against 7.4 to 8.7 ms. Together they may
  hold 5 to 10 ms of the 5-times gap (an estimate). P0 gives each
  family's time, not why it does not scale. The property family runs a
  split, and so a barrier, per axiom and part. Count them before changing
  anything.
- **E2, E8 and E1** (the owner's first picks, research §10) act on
  Brick's class hierarchy. No phase 2 budget is open on Brick, and LUBM's
  hierarchy is shallow. They come after the memory and all-cores budgets,
  or first if the owner keeps that order.
- **E7** (CSR for `rdf:type`'s object index): P0 now puts that index at
  30.2 MB, 2.9 bytes per closure triple. A CSR of 4-byte subjects with
  offsets per class would save about 1.5 (an estimate). Worth it after
  candidates 1 to 4 if memory is still above 50.
- **E10** (a blocked Bloom filter on each large stable tier): for
  many-round inputs. It costs 12 to 15% more memory, against the budget
  now.
- **Split IRIs into a namespace and a local name in the dictionary.** The
  text is 9.8 bytes per closure triple (101.5 MB, about 44 bytes a term),
  and LUBM's IRIs share a few thousand namespaces. A namespace id fits in
  the entry's 32-bit field, which an IRI leaves unused. It might save 4
  to 5 bytes per triple, but every reader of a borrowed `Term::Iri(&str)`
  changes, and so does the snapshot format: the largest change here.
- **The dictionary merge** runs on one thread, 110 to 150 ms on lubm-50
  (ticket 18's log). A sharded dictionary that keeps first-seen ids would
  run it in parallel, for more effort than candidate 6 and a similar gain.
