# Tickets: version 1 (parity with `reasonable`)

Spec: `docs/specs/v1-parity.md`. After these, the improvement loop
(`docs/specs/experiments.md`) takes over. Lock files are in **bold**.

| # | Ticket | Blocked by | Files |
|---|---|---|---|
| 01 | [Scaffold and the interface](01-scaffold.md) | — | workspace `Cargo.toml`, `crates/turtle-reasoner/src/lib.rs`, `crates/turtle-reasoner/src/reasoner.rs` (stub), `crates/turtle-reasoner/src/rules/mod.rs` (table), `AGENTS.md`, `README.md`, scripts |
| 02 | [Terms, dictionary and I/O](02-terms-io.md) | 01 | `crates/turtle-reasoner/src/term.rs`, `crates/turtle-reasoner/src/io.rs` |
| 03 | [The engine](03-engine.md) | 01 | `crates/turtle-reasoner/src/engine/` |
| 04 | [The oracles](04-oracles.md) | 01 | `crates/oracle/`, `oracles/`, `scripts/fetch-data.sh`, `scripts/fetch-tools.sh` |
| 05 | [Reference evaluator and chaos](05-reference-chaos.md) | 01 | `crates/reference/` |
| 06 | [The benchmark](06-bench.md) | 01 | `crates/bench/` |
| 07 | [Rules: class family](07-rules-class.md) | 02, 03 | `crates/turtle-reasoner/src/rules/class.rs`, **`crates/turtle-reasoner/src/rules/mod.rs`** (one line) |
| 08 | [Rules: property family](08-rules-property.md) | 02, 03 | `crates/turtle-reasoner/src/rules/property.rs`, **`crates/turtle-reasoner/src/rules/mod.rs`** (one line) |
| 09 | [Diagnostics and datatypes](09-diagnostics.md) | 02, 03 | `crates/turtle-reasoner/src/diagnostics.rs`, `crates/turtle-reasoner/src/rules/datatype.rs`, **`crates/turtle-reasoner/src/rules/mod.rs`** (one line) |
| 10 | [The reasoner and incremental runs](10-reasoner.md) | 02, 03 | `crates/turtle-reasoner/src/reasoner.rs` |
| 11 | [Equality by representatives](11-equality.md) | 03, 10 | `crates/turtle-reasoner/src/rules/equality.rs`, `crates/turtle-reasoner/src/engine/canon.rs`, **`crates/turtle-reasoner/src/rules/mod.rs`** (one line) |
| 12 | [CLI parity](12-cli.md) | 10 | `crates/cli/` |
| 13 | [Python bindings](13-python.md) | 10 | `crates/python/` |
| 14 | [Close-out: full verification](14-close-out.md) | all | `docs/verification/`, `README.md`, `CHANGELOG.md`, `AGENTS.md`, the specs |

## Waves

1. **01** alone (the orchestrator): every later ticket builds on its interface.
2. **02, 03, 04, 05, 06** together: disjoint folders. 04, 05 and 06 build against the stub interface and are judged once 10 lands.
3. **07, 08, 09, 10** together: each rule family is its own file; the lock file `rules/mod.rs` gets one registration line per family, in separate hunks.
4. **11, 12, 13** together: equality touches the engine's new `canon.rs` and its own rule file; CLI and Python are separate crates.
5. **14** alone: the only ticket that edits README, CHANGELOG, AGENTS and the specs' numbers.

## Log

Departures from the plan, as they happen.
