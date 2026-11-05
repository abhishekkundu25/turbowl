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

- **Wave 2, run.** A first run of wave 2 stopped partway; tickets 02, 03,
  05 and 06 were finished from its uncommitted work, and 04 was started
  again.
- **02.** `io::write` also writes N3 (as Turtle) and RDF/XML, since the
  public `Reasoner::write` takes any `Format`. Ticket 10 decides whether
  to reject them. N3 statements that are not RDF (formulas, variables,
  literal subjects) are dropped and counted.
- **03.** The timing test cannot close one 100,000-edge chain (5 billion
  pairs); it measures the head's closure row of that chain and the full
  closure of 1,000 chains of 100 edges. `Family` in `rules/mod.rs` still
  has no `rules: RuleFn` field: the first rule family (07 to 09) adds it.
  A rule's first `by_object` request returns `None` and builds the index
  for the next round.
- **05.** `reasonable` reports `RDFS.DATATYPE_RANGE` for any IRI or blank
  object of a property with an IRI range (`term_in_datatype_space`,
  `lib/src/reasoner.rs:2397`). No D covers it, so the reference copies
  it; the owner decides whether it becomes D11. D6 is wider than its row
  says (literal subjects have visible effects through sameAs and
  diagnostics), and D5 also covers one list per class and ignoring
  derived list triples. Chaos against the stub passes and prints
  `NOT COMPARED`.
- **06.** The incremental stream is two scenarios, `incremental-add` and
  `incremental-remove`; `sameas-clique` is added for ticket 11. Each run
  is a child process of the bench binary, so each has its own peak
  memory. `machine.rs` allows `unsafe_code` locally for `getloadavg` and
  `getrusage`. Under load, `reasonable` times out on `sameas-clique`.
