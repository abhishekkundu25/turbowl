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

## For the next agent

To continue the loop cold: read `AGENTS.md`, then this file from the top,
then the newest file in `docs/verification/`. Start at step 1 with a fresh
reviewer. The candidates the last reviewer ranked but that were not picked
are listed below; a new reviewer may reuse or replace them.

### Open candidates

Ranked by iteration 1's reviewer, for iterations 2 and later; iteration 1
is P0, counters and a memory breakdown (ticket 20). They are ranked by
expected gain per effort against the phase 2 budgets that are not met or
not yet measured (`docs/specs/v2-2-speed.md`, Non-functional budgets).
Every number marked as an estimate waits for P0. The research's §10
(`docs/research/sota-algorithms-2026-11.md`) ranks the starting candidates,
P0 and E1 to E9, with hypotheses and how to measure them; §11 lists what
not to do and why.

Where the budgets stand, on the indicative numbers. Memory: 151 bytes per
closure triple on lubm-50, against at most 50. All cores: 3.7 to 4.3
times one thread, against at least 5 (ticket 17). Loading: probably met,
not measured. Ticket 19 took 1.69 s to parse and reason lubm-50 before
ticket 18, and ticket 18 then cut the parse by about 0.64 s, so loading is
now about 0.9 to 1.0 s against a bar of 1.3 s (an estimate). Of that,
`document_key` and the base's sort are about 0.5 s (ticket 16's log).
Total time below the Baseline: not measured since phase 1.

1. **Read the input file in windows, not whole.** Memory.
   - Hypothesis: lubm-50's peak falls from 1,486 MB (151 bytes per
     closure triple) to about 600 MB (about 60 bytes). `Reasoner::load_file`
     reads the whole file into one buffer (`reasoner.rs:276-283`). The
     bench's lubm-50 is one Turtle file of 934,393,037 bytes
     (`target/bench-data/lubm-v1-50.ttl`), 891 of the bench's MB (which
     are MiB): 60% of the peak, held while the dictionary and the parsed
     triples grow. Windows of about 64 MiB, cut at line starts as ticket
     18 cuts its pieces, hold about 7% of that.
   - Effect on the rubric: Memory +1, the lead over Nemo on lubm-50 from
     1.32× to about 3×. Alone it does not reach 50 bytes: candidate 2 and
     P0's breakdown say what else must go. Loading time stays within the
     spread, or improves if reading overlaps parsing.
   - Files: `reasoner.rs` (`load_file`, `load`; `document_key` fed window
     by window), `io.rs` (`parse_on` over a reader), `io/parallel.rs`
     (pieces from each window; the tail and the Turtle state carried to
     the next), `io/cross_check.rs` (cuts at window ends).
   - Risk: an error's line and column across windows; a statement or a
     multi-line string longer than a window; the all-or-nothing rule on a
     parse error (the triples are still collected before `add`); blank
     nodes stay scoped per document. A memory map would avoid the copy,
     but it needs `unsafe` or a new dependency, and a file truncated under
     the map faults the process: not proposed.
   - Measure: P0's memory breakdown at the end of loading and at the
     reasoning peak, to confirm that the peak lies in loading; the bench's
     peak memory and load time on lubm-10 and lubm-50, 5 runs; a
     cross-check test with windows of a few KiB, so that every boundary
     case is hit.

2. **Keep the base by predicate, as sorted (s, o) pairs.** Memory, and
   full runs.
   - Hypothesis: the base takes 79 MB on lubm-50, as `[s, p, o]` triples
     of 12 bytes (`Base` in `reasoner.rs`; the snapshot's breakdown in
     `snapshot.rs`). One sorted relation of (s, o) pairs per predicate
     takes 53 MB, 2.5 bytes less per closure triple. A full run with an
     empty canon then inserts each predicate's base relation into its
     table as one sorted batch, instead of pushing 6.6 M triples through
     the round buffer (`Reasoner::start`, `Store::extend_from_slice`) and
     sorting them again per table. The reasoning peak falls by 26 to
     80 MiB (2.5 to 8 bytes per closure triple: the base's 4 bytes a
     triple, and the round buffer's slack), and the first step sorts
     6.6 M pairs fewer.
   - Effect on the rubric: Memory, with candidate 1, toward 50 bytes;
     Speed (full run), a few percent on lubm-50 and on every full run
     after a removal (incremental-remove).
   - Files: `reasoner.rs` (`Base`, `start`, `set_base`, `remove`, the
     inferred-only reads), `engine/store.rs` (a table takes a sorted
     batch), `snapshot.rs` (the `BASE` section, in a new format version).
   - Risk: every reader of the base changes (`contains`, `added`,
     `sorted_all`, `remove`). After a restart the canon is not empty, so
     the base must still be rewritten into representatives and sorted:
     the fast path is for an empty canon only. Older snapshots are
     refused, as the format intends.
   - Measure: P0's memory breakdown (base, round buffer, tables) at the
     reasoning peak; the bench's peak and reasoning time on lubm-10,
     lubm-50, brick-buildings and incremental-remove; chaos for the
     restart and equality paths.

3. **Flip the object index's pairs on the workers.** All cores.
   - Hypothesis: lubm-50 reasoning on all cores goes from 3.7 to 4.3
     times one thread to at least 5 times. By Amdahl's law on 12 threads,
     4 times leaves 16 to 20% of the one-thread time on one thread, about
     70 to 88 ms of 430 ms, and 5 times allows at most 13%, about 55 ms.
     Each table's step copies its new (s, o) pairs into the (o, s) index
     in one sequential loop (`store.rs:646`), and a requested index is
     built by a sequential flip of the whole table, one table after
     another (`store.rs:404`). `rdf:type`, the largest table, pays both.
     Done in chunks on the workers, the flips leave the serial path.
   - Effect on the rubric: Speed (full run), on the all-cores axis. It
     meets the budget only if P0 shows the flips at 15 ms or more of the
     serial path. If P0 names a larger serial part (cls-int1's candidate
     building, `rdf:type`'s single job, a family that dominates a round:
     ticket 17's "Still sequential"), take that part instead, alone.
   - Files: `engine/store.rs` (`step`, `Job::advance`), and
     `engine/parallel.rs` if a flip needs a helper there.
   - Risk: low for correctness. A flip is a map, and the sort after it
     restores the order. Chaos at one thread and at all cores checks that
     the result does not change; below `PARALLEL_FROM` nothing changes.
   - Measure: P0's time per round at 1 and 12 threads, split into the
     families and the step, and the step into sorts, `changed()`, flips
     and index builds; the bench's reasoning time on lubm-10 and lubm-50
     at one thread and all cores, 5 runs. Measure the 5-times budget on
     the Baseline on a quiet machine first: at load 2 to 4, other jobs
     held 2 to 4 of the 12 cores.

4. **Hash a document once, on the workers.** Loading and total time.
   - Hypothesis: `document_key` hashes the whole document twice on one
     thread, with FxHash and then SipHash (`reasoner.rs:902-914`): two
     passes over lubm-50's 934 MB. XXH64, already in `snapshot.rs`, run
     with two seeds over 1 MiB blocks on the workers, the block hashes
     then hashed in order, takes under 0.05 s (an estimate), and cuts
     loading by about 0.3 s of about 0.9 s.
   - Effect on the rubric: Speed (full run). On all cores loading is
     about 90% of lubm-50's total time, so the total falls by about a
     third (an estimate), on the scenarios where Soufflé comes closest.
     It serves total time, not the loading budget, which is probably met.
   - Files: `reasoner.rs` (`document_key`, `probe_key`), `snapshot.rs`
     (`xxh64` shared with the crate).
   - Risk: the key is not cryptographic, as today. A snapshot saved
     before the change keeps no document keys on load, since its probe
     key differs (the format intends this), so loading the same document
     again after such a snapshot parses it again. If candidate 1 lands
     first, each window's blocks are hashed as they come, which this
     design allows.
   - Measure: P0's load breakdown (read, key, parse, merge, base sort);
     the bench's load time on lubm-10 and lubm-50, 5 runs; brick-small1
     for no regression (a small document stays on one thread).

5. **Sort the parsed base on the workers.** Loading and total time.
   - Hypothesis: `Base::fresh` sorts and deduplicates the parsed batch on
     one thread (`reasoner.rs:958-962`): 6.6 M triples of 12 bytes on
     lubm-50. `Workers::sort_dedup` (`engine/parallel.rs:156`) does it on
     every core, which cuts the sort to a fifth or less and loading by
     about 0.1 s (an estimate: ticket 16 gives 0.5 s for the key and the
     sort together, and P0 splits them).
   - Effect on the rubric: Speed (full run), as candidate 4. If candidate
     2 lands first, the same change applies to its sort per predicate.
   - Files: `reasoner.rs` (`Base::fresh`, `Base::flush`, and `add`, which
     then needs the workers).
   - Risk: rayon's `par_sort` is a merge sort with a buffer as large as
     its input, so the sort holds about 79 MB more while it runs, against
     candidate 1's gain. Measure the peak too. Small batches (an insert)
     stay on one thread.
   - Measure: P0's load breakdown; the bench's load time and peak memory
     on lubm-10 and lubm-50.

Not ranked here, and why:

- **E2, E8 and E1** (the owner's first picks, research §10) act on
  Brick's class hierarchy. No phase 2 budget is open on Brick, where
  TurbOWL's total time leads by 11× to 17×, and LUBM's hierarchy is
  shallow, so their gain on lubm-50 is small. They come next once the
  memory and all-cores budgets are met, or first if the owner keeps that
  order.
- **E7** (CSR for `rdf:type`'s object index): stored pairs are 125 MB of
  lubm-50's 312 MB state, and the object indexes are part of that. Worth
  it only if P0 shows that relations, not loading or transient buffers,
  hold the peak.
- **E10** (found by the scaling tests): a blocked Bloom filter on each
  large stable tier of a variable, so a round's duplicate check costs
  O(1) per tier instead of a gallop. Hypothesis: many-round inputs (long
  chains, incremental edits) go from N·log²(N/b) to N log N in the
  duplicate check, for about 12 to 15% more memory, which works against
  the memory budget now. Measure:
  `scaling_variable_small_batches_comparisons`, the chain and
  incremental scenarios, and peak memory on brick-buildings and lubm-50.
  Few-round inputs (Brick, LUBM) should not change.
- **The dictionary merge** runs on one thread, 110 to 150 ms on lubm-50
  (ticket 18's log). A sharded dictionary that keeps first-seen ids
  would run it in parallel, for more effort than candidates 4 and 5 and a
  similar gain.
