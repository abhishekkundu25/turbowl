# Tickets: version 1 (a fast OWL 2 RL reasoner)

Spec: `docs/specs/v1.md`. After these, the improvement loop
(`docs/specs/experiments.md`) takes over. Lock files are in **bold**.

| # | Ticket | Blocked by | Files |
|---|---|---|---|
| 01 | [Scaffold and the interface](01-scaffold.md) | — | workspace `Cargo.toml`, `crates/turtle-reasoner/src/lib.rs`, `crates/turtle-reasoner/src/reasoner.rs` (stub), `crates/turtle-reasoner/src/rules/mod.rs` (table), `AGENTS.md`, `README.md`, scripts |
| 02 | [Terms, dictionary and I/O](02-terms-io.md) | 01 | `crates/turtle-reasoner/src/term.rs`, `crates/turtle-reasoner/src/io.rs` |
| 03 | [The engine](03-engine.md) | 01 | `crates/turtle-reasoner/src/engine/` |
| 04 | [The oracles](04-oracles.md) | 01 | `crates/oracle/`, `oracles/`, `scripts/fetch-data.sh`, `scripts/fetch-tools.sh` |
| 05 | [Reference evaluator and chaos](05-reference-chaos.md) | 01 | `crates/reference/` |
| 06 | [The benchmark](06-bench.md) | 01 | `crates/bench/` |
| 07 | [Rules: class family](07-rules-class.md) | 02, 03 | `crates/turtle-reasoner/src/rules/class.rs` |
| 08 | [Rules: property family](08-rules-property.md) | 02, 03 | `crates/turtle-reasoner/src/rules/property.rs` |
| 09 | [Diagnostics and datatypes](09-diagnostics.md) | 02, 03 | `crates/turtle-reasoner/src/diagnostics.rs`, `crates/turtle-reasoner/src/rules/datatype.rs` |
| 10 | [The reasoner and incremental runs](10-reasoner.md) | 02, 03 | `crates/turtle-reasoner/src/reasoner.rs` |
| 11 | [Equality by representatives](11-equality.md) | 03, 10 | `crates/turtle-reasoner/src/rules/equality.rs`, `crates/turtle-reasoner/src/engine/canon.rs`, **`crates/turtle-reasoner/src/rules/mod.rs`** (one line) |
| 12 | [The CLI](12-cli.md) | 10 | `crates/cli/` |
| 13 | [Python bindings](13-python.md) | 10 | `crates/python/` |
| 14 | [Close-out: full verification](14-close-out.md) | all | `docs/verification/`, `README.md`, `CHANGELOG.md`, `AGENTS.md`, the specs |

## Waves

1. **01** alone (the orchestrator): every later ticket builds on its interface.
2. **02, 03, 04, 05, 06** together: disjoint folders. 04, 05 and 06 build against the stub interface and are judged once 10 lands.
3. **07, 08, 09, 10** together: each rule family is its own file, registered in `rules/mod.rs` before the wave, so no two slices share a file.
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
- **04.** The DL reasoner is HermiT 1.3.8 on a fetched Temurin JRE 21:
  Konclude has no arm64 build and reads no Turtle. HermiT does not run on
  inputs that include `owl.n3`. The W3C OWL 2 RL tests come from the
  Internet Archive copy of owl.semanticweb.org (2016-07-03). The default
  run covers the minimal inputs, both W3C suites and Brick with small1;
  the other data inputs run with `ORACLE_SLOW=1`, one test at a time.
  Disagreements outside D1 to D10 are listed in `KNOWN`: the
  `DATATYPE_RANGE` bug (as in 05) and one intersection list per class.
  The owner decides whether they become D11 and D12, and whether the
  `owl-rl` profile keeps the container-membership axioms, which are not
  OWL 2 RL. Ticket 09 emits `Diagnostic::terms` in the order documented
  on `Finding`.
- **Target changed (ADR 0004).** The W3C OWL 2 RL rules are the target,
  judged by Nemo, the W3C tests, HermiT, OWL-RL and the reference
  evaluator; `reasonable` is a speed baseline only, with Nemo beside it.
  One rule set, no profiles, no compatibility with `reasonable`'s
  conventions, CLI or Python API. The spec moved to `docs/specs/v1.md`;
  tickets 07 to 13 were rewritten. D1 to D10 became oracle faults
  (OWL-RL's) only. The open questions above about D11, D12, the
  container axioms, N3 and RDF/XML output, and `reasonable`'s ranges are
  settled by it: no `reasonable` faults are filtered or copied, no
  container axioms, all four output formats stay.
- **Reference evaluator (ADR 0004).** Rules run over generalized triples;
  only the returned closure drops triples that are not RDF (the spec now
  says so). dt-not-type checks five datatypes, so a literal of another
  datatype (`"3.5"^^xsd:decimal` typed `xsd:string`) is not reported,
  though the W3C rule would report it. Ticket 09 matches the reference's
  value-space rules.
- **Oracles and bench (ADR 0004).** Oracle faults F1 to F5 are all
  OWL-RL's; HermiT and the W3C tests never contradicted Nemo. The
  reference evaluator equals Nemo on 1,000 generated inputs. Nemo's
  program binds variable predicates through the properties the schema
  names and joins cax-eqc2 like cax-eqc1 (lubm-1: 245 s to under 2 s,
  same closure). W3C targets: the 93 OWL 2 RL and 30 RDF-MT cases Nemo
  passes, plus the dt-not-type cases within the checked datatypes. The
  bench runs Nemo as a second baseline; `sameas-clique` is opt-in.
- **Research before wave 3.** `docs/research/sota-algorithms-2026-11.md`
  (drafted, then verified against the primary sources) ranks the first
  experiments and changed four tickets: 07 keeps scm-sco, cax-sco and
  cax-eqc as separate functions; 08 copies prp-inv1/2 without a join; 10
  counts tuples per family and round; 11 follows ADR 0003's amendment for
  `sameAs` on vocabulary terms. The spec's 18.7 s sentence was reasonable
  0.2.6's README log from 2023; 0.5.0-a1 takes about 13 ms on that input.
- **Scaling tests (owner's request).** Eighteen `scaling_*` tests in
  `crates/turtle-reasoner/src/scaling.rs` fit how time grows with size
  (dictionary, I/O, relations, joins, variables, store), and the bench fits
  exponents across lubm-1/10/50. They found two super-linear costs in the
  engine. A round's duplicate check walked every stable tier even when a
  tier's range could not hold the new tuples (fixed: skipped, a chain's
  round is flat at about 100 ns). And the check over many small scattered
  batches is N·log²(N/b), datafrog's design (accepted and pinned by
  counted comparisons; E10 would remove it). One heavily loaded run over
  LUBM 1/10/50 fitted time exponents of 1.30 for reasonable (its loading)
  and 0.98 for Nemo; the spec's budget for this reasoner is 1.2.
- **Wave 3, before the slices.** One commit fixed what the four slices
  share, so none of them edits a shared file: the rule interface
  (`RuleFn` takes a `Round` and a `Context`, which holds the dictionary and
  the run's findings; `Finding`; `Family::facts` for the rules with an
  empty body, which the reasoner inserts at the start of a full run), the
  three family files as registered stubs, `Diagnostic::from_finding` as a
  stub, `Round::emitted` for ticket 10's counters, and a shared test
  harness, `rules/testing.rs`. Ticket 08 said no triple with a literal
  subject is emitted, which contradicts the spec's generalized triples: it
  now says they are derived like any other.
