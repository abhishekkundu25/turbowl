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
| 12 | [Competitors: VLog, Soufflé, GraphDB](12-competitors.md) | 03, 13 | `crates/bench/src/engine.rs`, new `crates/bench/src/{souffle,vlog,graphdb}.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`**, `scripts/fetch-tools.sh` |
| 13 | [Scoreboard data and the 16 GB cap](13-scoreboard-data.md) | 01 | `scripts/fetch-data.sh`, `crates/bench/src/scenario.rs`, new `crates/bench/src/datasets.rs`, `crates/bench/src/machine.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`** |
| 14 | [Published results and the scoreboard report](14-published-results-and-report.md) | 12, 13 | new `crates/bench/src/{published,report}.rs` and the published-results data file, `crates/bench/src/compare.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`** |
| 15 | [Close-out: phase 1](15-close-out-phase-1.md) | 01 to 14, version 1's 14 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `CONTEXT.md`, the phase 1 spec, `docs/verification/`, this README |
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
3. **04, 12.**
4. **05, 06, 07, 08, 09, 14.**
5. **10.**
6. **15.**

Phase 2: **16, 17**, then **18, 19**, then **20**, then **21**.

Phase 3: **22**, then **23**, then **24**.

## Log

Departures from the plan, as they happen.
