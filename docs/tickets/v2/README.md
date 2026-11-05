# Tickets: version 2 (the best OWL 2 RL reasoner)

Specs: `docs/specs/v2-1-scoreboard-and-rules.md` (phase 1, tickets 01 to
15), `docs/specs/v2-2-speed.md` (phase 2, 16 to 21),
`docs/specs/v2-3-removals.md` (phase 3, 22 to 24). Decisions: ADR 0005
(switches), ADR 0006 (the scoreboard). Lock files are in **bold**.

| # | Ticket | Blocked by | Files |
|---|---|---|---|
| 01 | [Remove `reasonable`](01-remove-reasonable.md) | — | `crates/bench/` (`Cargo.toml`, `src/engine.rs`, **`src/main.rs`**, `src/scaling.rs`, `src/compare.rs`, **`README.md`**), **`Cargo.lock`**, `docs/specs/experiments.md`, one doc line in `crates/oracle/src/lib.rs` |
| 02 | [Switches: the seam](02-switches.md) | — | `crates/turbowl/`: `Cargo.toml`, `src/lib.rs`, new `src/switch.rs`, `src/reasoner.rs`, **`src/rules/mod.rs`**, **`src/term.rs`**, new `src/rules/checks.rs`, `src/rules/datatype.rs`, `src/rules/property.rs`, `src/rules/property/chain.rs`, new empty `src/rules/{identity,schema,trivial,literal_values}.rs` and `src/rules/class/enumerations.rs`, `src/rules/class.rs`, `src/rules/testing.rs`; `scripts/check-wasm.sh` |
| 03 | [One rule source for every engine](03-rule-source.md) | 01, 02 | `oracles/nemo/owl2rl.rls` → `oracles/rules/<group>.rls`, new `crates/oracle/src/programs.rs`, `crates/oracle/src/{nemo,lib}.rs`, `crates/oracle/tests/programs/`, `crates/bench/src/nemo.rs`, `crates/bench/Cargo.toml`, **`Cargo.lock`** |
| 04 | [Verification by configuration](04-verification-by-configuration.md) | 03 | `crates/reference/` (`src/evaluate.rs` → `src/rules/<group>.rs`, `src/generate.rs` → `src/generate/<group>.rs`, `src/lib.rs`, `tests/`), `crates/oracle/` (`src/cases.rs` → `src/cases/<group>.rs`, `src/{ours,triangulate,w3c}.rs`, `tests/`) |
| 05 | [Rules: the new checks](05-rules-checks.md) | 04 | the `checks` files: `crates/turbowl/src/rules/checks.rs`, `oracles/rules/checks.rls`, and those in `crates/reference/src/{rules,generate}/` and `crates/oracle/src/cases/` |
| 06 | [Rules: identity](06-rules-identity.md) | 04 | the `identity` files, as for 05 |
| 07 | [Rules: the schema](07-rules-schema.md) | 04 | the `schema` files, as for 05 |
| 08 | [Rules: the trivial group and cls-oo](08-rules-trivial-and-enumerations.md) | 04 | the `trivial` files, and for cls-oo `crates/turbowl/src/rules/class/enumerations.rs` and the `core` files; `crates/turbowl/src/reasoner.rs` (eq-ref on read) |
| 09 | [The OWL 2 datatype map](09-datatype-map.md) | 04 | `crates/turbowl/src/rules/datatype.rs`, new `crates/turbowl/src/rules/datatype/values.rs`, `crates/reference/src/datatype.rs`, the `datatypes` files of the generator and the minimal inputs |
| 10 | [Literal values and typing](10-literal-values-and-typing.md) | 08, 09 | the `literal-values` files; the `trivial` files (dt-type2) |
| 11 | [Switches in the CLI and Python](11-switch-adapters.md) | 02 | `crates/cli/`, `crates/python/` |
| 12 | [Competitors: VLog, Soufflé, Nemo (GraphDB if licensed)](12-competitors.md) | 03, 13 | `crates/bench/src/engine.rs`, new `crates/bench/src/{souffle,vlog,graphdb}.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`**, `scripts/fetch-tools.sh` |
| 13 | [Scoreboard data and the 16 GB cap](13-scoreboard-data.md) | 01 | `scripts/fetch-data.sh`, `crates/bench/src/scenario.rs`, new `crates/bench/src/datasets.rs`, `crates/bench/src/machine.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`** |
| 14 | [Published results and the scoreboard report](14-published-results-and-report.md) | 12, 13 | new `crates/bench/src/{published,report}.rs` and the published-results data file, `crates/bench/src/compare.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`** |
| 25 | [Memory: OWL2Bench RL, UOBM-010 U and OpenCyc](25-memory.md) | 13 | `crates/turbowl/` (where the cause is), `crates/turbowl/src/scaling.rs` |
| 15 | [Close-out: phase 1](15-close-out-phase-1.md) | 01 to 14, 25, version 1's 14 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `CONTEXT.md`, the phase 1 spec, `docs/verification/`, this README |
| 16 | [Fast loading: parsers and the dictionary](16-parsers-and-dictionary.md) | 15 | `crates/turbowl/src/io.rs` and new `src/io/`, **`crates/turbowl/src/term.rs`**, `crates/turbowl/src/scaling.rs` |
| 17 | [All cores: parallel rounds](17-parallel-rounds.md) | 15 | `crates/turbowl/src/engine/`, **`crates/turbowl/src/reasoner.rs`**, **`crates/turbowl/src/rules/mod.rs`**, `crates/turbowl/Cargo.toml`, `scripts/check-wasm.sh` |
| 18 | [All cores: parallel parsing](18-parallel-parsing.md) | 16, 17 | `crates/turbowl/src/io/`, **`crates/turbowl/src/term.rs`** |
| 19 | [Snapshots](19-snapshots.md) | 16, 17 | new `crates/turbowl/src/snapshot.rs`, `crates/turbowl/src/lib.rs`, **`crates/turbowl/src/reasoner.rs`**, `crates/cli/`, `crates/python/` |
| 20 | [The improvement loop](20-improvement-loop.md) | 17, 18, 19 | each experiment's files; **`docs/specs/experiments.md`** (one row per iteration, written by the orchestrator) |
| 21 | [Close-out: phase 2](21-close-out-phase-2.md) | 16 to 20 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `docs/verification/`, this README |
| 22 | [Removals: DRed](22-dred.md) | 21 | `crates/turbowl/src/engine/`, **`crates/turbowl/src/reasoner.rs`**, `crates/turbowl/src/rules/`, `crates/reference/tests/chaos.rs` |
| 23 | [Removals: FBF](23-fbf.md) | 22 | as for 22 |
| 24 | [Close-out: phase 3 and the claim](24-close-out-phase-3.md) | 22, 23 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `docs/verification/`, this README |

## How the lock files stay apart

- **`crates/turbowl/src/rules/mod.rs`** and **`crates/turbowl/src/term.rs`**:
  ticket 02 registers every family and adds every vocabulary IRI at once,
  so no rule ticket edits them. In phase 2, 16 then 18 edit `term.rs` in
  sequence, and 17 alone edits `rules/mod.rs`.
- **Each switch group has its own files** in the core, `oracles/rules/`,
  the reference evaluator, the generator and the minimal inputs. Tickets
  02 and 04 create them empty, and one rule ticket fills each group. The
  `trivial` group is filled by 08 and then 10.
- **`crates/bench/src/main.rs`** and **`crates/bench/README.md`**: 01, 13,
  12 and 14 edit them, one per wave.
- **`Cargo.lock`**: 01 removes `reasonable` and 03 adds the bench's
  dependency on the oracle crate, in different waves.
- **`crates/turbowl/src/reasoner.rs`**: 02, then 08 (alone in its wave),
  then 17, 19, 22 and 23 in sequence.

## Waves

Phase 1:

1. **01, 02.** Version 1's ticket 14 runs whenever the owner gives a quiet
   machine; it blocks only 15.
2. **03, 11, 13.**
3. **04, 12, 25.** 25 (added: ticket 13 found the memory blow-up) lands before wave 4, whose rule tickets edit the core.
4. **05, 06, 07, 08, 09, 14.**
5. **10.**
6. **15.**

Phase 2: **16, 17**, then **18, 19**, then **20**, then **21**.

Phase 3: **22**, then **23**, then **24**.

## Log

Departures from the plan, as they happen.

- **01.** The bench's `Engine` trait went with `reasonable`, since TurbOWL
  was its only other adapter; the competitors of ticket 12 run as
  subprocesses, as Nemo does. `--list` also prints the engines. Results
  files keep schema 4 until ticket 14; `--compare` skips the rows of any
  engine the bench no longer runs, and says so.
- **02.** The vocabulary already named every IRI the new rules need, so
  `term.rs` gained only docs and a test, and the canon's rule constants are
  unchanged. The internal counters list only the families that ran. The
  checks of version 1 moved verbatim to `rules/checks.rs`, and
  `scaling_datatype_rules_linear_in_instances` became
  `scaling_checks_linear_in_instances`.
- **11.** `--complete` is a starting point wherever it comes; a later
  `--switch` for the same name wins. Python's `Reasoner` takes
  keyword-only `switches` and `complete`, and has no setter yet. The CLI's
  help and the Python docs each describe the switches' costs (the CLI owns
  its help text). On review, the `literal-values` text was corrected: dt-diff
  only checks, and lists no `owl:differentFrom` triples.
- **03.** The group files are compiled into the oracle crate
  (`include_str!`), so editing a rule needs a rebuild, which cargo does by
  itself. Nemo's runner reads whatever outputs the program exports, so a
  configuration without `checks` exports no diagnostic files (ticket 04
  replaced `nemo::CODES` with `nemo::codes(switches)`). The Soufflé and VLog programs
  are checked against each engine's grammar but have not run: ticket 12
  runs them. Rulewerk has no `!=`, so the VLog program writes
  `?x != c` as `~sameTerm(?x, c)` over facts; Soufflé reads tab-separated
  terms, not N-Triples. A test reads the spec's switch table and checks
  that every rule sits in exactly one group file.
- **04.** The W3C suites run in the complete configuration only (the
  spec's choice; the ticket's acceptance said both). A case's written
  expectations count only where its group is on, and an expected finding
  only where its code's group is on. Version 1's cases went where their
  rules live: `scm-sco-cycle` and `not-scm-spo` in `schema`, `not-eq-ref` in
  `trivial`, the dt-not-type cases in `datatypes`. The core cases `empty` and
  `no-thing-typing` state the exact closure, so ticket 08 must exempt the
  `trivial` group from them when it fills it. Chaos draws its configuration
  from a stream of its own, so every input is as before. Then, as a
  prefactor on `v2`, the generator gave each file's new shapes their own
  stream, drawn after version 1's: the fingerprint of version 1's part
  stays, and no two rule tickets edit the same value.
- **13.** The memory cap first read resident memory, which macOS
  compresses under pressure: one smoke run held 69 GB for about 2.5
  minutes while showing 2 GB, until the agent stopped it. The cap now
  reads the physical footprint (compressed and swapped pages included),
  and later runs stopped near 16.4 GB. Oxford's `Claros.ttl` has 84 lines
  with illegal IRI characters; the fetch script percent-encodes them as it
  unpacks, so every engine reads the same repaired file. The results file
  moved to schema 5 here (the `over-memory` status), not in ticket 14. The
  large zips are pinned by the CRC-32 their zips record until their first
  approved download. The smoke runs found the memory blow-up that ticket
  25, added to phase 1, diagnoses. `claros-le` derives 226 M triples
  against the paper's 555 M: `Claros_LE.owl` uses `owl:hasSelf`, which is
  outside OWL 2 RL, so only the L variants can be the same job.
- **Worktrees.** A copied `crates/python/.venv` runs pytest against the
  main checkout's build. A slice runs `uv sync --reinstall-package turbowl`
  in `crates/python` before pytest, and the briefs say so.
