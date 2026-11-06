# Tickets: version 1 (a fast OWL 2 RL reasoner)

Spec: `docs/specs/v1.md`. After these, the improvement loop
(`docs/specs/experiments.md`) takes over. Lock files are in **bold**.

| # | Ticket | Status | Blocked by | Files |
|---|---|---|---|---|
| 01 | [Scaffold and the interface](01-scaffold.md) | done | — | workspace `Cargo.toml`, `crates/turbowl/src/lib.rs`, `crates/turbowl/src/reasoner.rs` (stub), `crates/turbowl/src/rules/mod.rs` (table), `AGENTS.md`, `README.md`, scripts |
| 02 | [Terms, dictionary and I/O](02-terms-io.md) | done | 01 | `crates/turbowl/src/term.rs`, `crates/turbowl/src/io.rs` |
| 03 | [The engine](03-engine.md) | done | 01 | `crates/turbowl/src/engine/` |
| 04 | [The oracles](04-oracles.md) | done | 01 | `crates/oracle/`, `oracles/`, `scripts/fetch-data.sh`, `scripts/fetch-tools.sh` |
| 05 | [Reference evaluator and chaos](05-reference-chaos.md) | done | 01 | `crates/reference/` |
| 06 | [The benchmark](06-bench.md) | done | 01 | `crates/bench/` |
| 07 | [Rules: class family](07-rules-class.md) | done | 02, 03 | `crates/turbowl/src/rules/class.rs` |
| 08 | [Rules: property family](08-rules-property.md) | done | 02, 03 | `crates/turbowl/src/rules/property.rs` |
| 09 | [Diagnostics and datatypes](09-diagnostics.md) | done | 02, 03 | `crates/turbowl/src/diagnostics.rs`, `crates/turbowl/src/rules/datatype.rs` |
| 10 | [The reasoner and incremental runs](10-reasoner.md) | done | 02, 03 | `crates/turbowl/src/reasoner.rs` |
| 11 | [Equality by representatives](11-equality.md) | done | 03, 10 | `crates/turbowl/src/rules/equality.rs`, `crates/turbowl/src/engine/canon.rs`, **`crates/turbowl/src/rules/mod.rs`** (one line) |
| 12 | [The CLI](12-cli.md) | done | 10 | `crates/cli/` |
| 13 | [Python bindings](13-python.md) | done | 10 | `crates/python/` |
| 14 | [Close-out: full verification](14-close-out.md) | done | all | `docs/verification/`, `README.md`, `CHANGELOG.md`, `AGENTS.md`, the specs |

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
  `crates/turbowl/src/scaling.rs` fit how time grows with size
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
- **07.** Lists and restrictions are schema-sized and are read again from
  the store each round, since a family keeps no state; per-instance work
  stays semi-naive. cls-int1 draws its candidates from a pivot cell every
  path passes through (soda hall: 29.6 ms to 5.1 ms for the family);
  cls-hv2 reads its property by subject, so cls-hv1's large tables need no
  object index. It found that the reference evaluator and Nemo read one
  list shape differently: a cell without `rdf:first` (the reference still
  counted the other members) and `rdf:nil` with triples of its own. Both
  now follow the W3C `LIST[...]` form, as Nemo does.
- **08.** prp-spo2 keeps the `prp-spo2` cargo feature (on by default). A
  chain is an n-way semi-naive join without state, joined in full when a
  list triple is new or an object index it needs was just built; a list
  with a cycle of live cells or over 1,024 chains is read naively, as the
  reference does. prp-fp and prp-ifp also derive `y owl:sameAs y` (the W3C
  rules have no inequality).
- **09.** dt-not-type's ill-typed half reads the terms of each round's new
  tuples (each tuple once over a run; a family has no state to remember
  checked literals). Each round also walks every table and the stable
  schema axioms once: a fixed cost per round that grows with the schema,
  not the data. Every finding is an error; findings are deduplicated by
  the reasoner. An untagged `rdf:langString` literal from an insert hit a
  debug assertion; it is now held as it came and reported.
- **10.** The base is a sorted vector of term-id triples plus a hash set of
  recent additions (12 to 16 bytes a triple). Loading the same document
  again (same bytes, format and base IRI, keyed by a 128-bit hash) changes
  nothing until a removal. A blank node inserted by a caller keeps its
  label on output (one shaped like a document label gets `_0`). The
  closure's generalized triples are counted round by round, not found by a
  pass. Counters per family and round stay internal for P0. Until ticket
  11, the oracle crate and chaos leave out any input that mentions
  `owl:sameAs`, `owl:FunctionalProperty` or `owl:InverseFunctionalProperty`
  (the eq-* rules cannot fire on any other input; the argument is on
  `ours::needs_equality`); the reports count them. The bench's
  `incremental-remove` picks removals without a blank node.
- **Wave 3, integrated.** The doc example runs. On the merged tree: the 35
  minimal inputs that do not need equality equal Nemo (closure and
  findings), 5 are pending; the W3C OWL 2 RL tests pass every case Nemo
  passes (84, plus 9 pending), the RDF-MT tests 37 (Nemo 30), including
  `datatypes-range-clash` and `xmlsch-02-whitespace-facet-2`, and no
  consistent case is reported inconsistent; Brick with small1 equals Nemo
  (26,381 triples) and HermiT entails its subsumptions; 100,000 full and
  100,000 incremental chaos iterations pass (48,107 compared, the rest
  need equality).
  The bench runs all three engines end to end; a first one-run look (load
  average near 60, not a measurement) put this reasoner 5 to 8 times
  faster than `reasonable` on brick-small1 and lubm-1, with closures equal
  to Nemo's, and an incremental addition at about 30% of a full run against
  the 10% budget (fixed per-round costs: lists, restrictions and tables
  read again each round). The quiet-machine numbers belong to ticket 14.
- **Renamed: TurbOWL.** The owner chose the name TurbOWL (`turbowl`) on
  2026-11-05, before wave 4. The core crate is `crates/turbowl`
  (`use turbowl::…`), the binary `turbowl`, the packages `turbowl-cli`,
  `turbowl-python`, `turbowl-oracle`, `turbowl-reference` and
  `turbowl-bench`, and the repository `github.com/abhishekkundu25/turbowl`
  (private and empty until the owner pushes). The bench's results files
  are schema 4; `--compare` reads an older file's `turtle-reasoner`
  engine as `turbowl`.
- **Wave 3, recorded.** `docs/verification/2026-11-05-wave-3.md` holds
  the exact numbers at `3252f68`, with its raw results. The benchmark ran
  3 times per engine with the load average at 7 to 11 (a trend, not a
  claim). Full runs are 3.65 to 7.30 times faster than `reasonable`, with
  5.49 to 9.81 times less memory. An incremental addition costs 6.5% of a
  full run, inside the 10% budget; the 30% above came from a one-run look
  under a load average near 60. The fitted LUBM time exponent is 1.056
  (`reasonable` 1.213, Nemo 1.059), and all 24 `scaling_*` tests pass.
- **12.** The ticket left some behaviour open; the slice decided it.
  - `--fail-on` defaults to `error`, so an inconsistent model exits 2
    unless `--fail-on never` is given. The closure and report are still
    written.
  - The report is the diagnostics, then the summary. It goes to stdout,
    or to stderr when the closure takes stdout.
  - The `json` and `ndjson` field names are stable, with terms as
    N-Triples strings. The summary carries the load and reasoning times.
  - `--summary-only` refuses `-o`, `--format` and `--inferred-only`.
  - An input extension that names no format is a usage error (64).
  - Arguments are parsed by hand. The only new dependency is
    `serde_json`, which was already in the lock.

  Left for later: reading stdin, with `--input-format`; reporting the N3
  statements dropped on load, which the interface does not expose; and
  `Severity::as_str` and `Ord` on `Severity`, so that the CLI and Python
  need no mappings of their own.
- **13.** The slice decided the API's shape.
  - `triples()` and `inferred()` return lists of rdflib tuples, and
    `graph()` returns an rdflib `Graph`.
  - `serialize` and `write` stand for `Reasoner::write`.
  - A literal of `xsd:string` comes back with no datatype, as rdflib's
    parsers read `"x"`. Typed literals keep their lexical form
    (`normalize=False`). Language tags come back lowercase, as oxrdf holds
    them.
  - pyo3 0.30 has no `extension-module` feature: maturin sets
    `PYO3_BUILD_EXTENSION_MODULE`. So `cargo --workspace` links libpython
    and needs a Python on PATH; AGENTS.md's clippy line is now
    `--workspace`.
  - The minimal-input comparison runs `examples/minimal_cases.rs`, which
    writes each case and the Rust library's results as N-Triples. pytest
    compares them exactly; rdflib's parser would rename blank nodes and
    normalise literals.

  Left for later: abi3 wheels (only for publishing), and `write_file` and
  `Severity::as_str` in the interface.
- **11.** Equality by representatives, built as ADR 0003 and its first
  amendment say. A second amendment, "as built", records the details.
  - The canon (`engine/canon.rs`) is a union-find in which every member
    points at its representative. A class that holds a rule constant is
    represented by its smallest constant. Otherwise the larger class wins,
    and a tie goes to the smaller id.
  - Every family reads its constants through the round. A merge that moves
    a constant restarts the run from the base, keeping the canon. So does
    the first merge into `rdf:nil`'s class: the W3C list rules test
    `rdf:nil` for identity, so a term equal to it can be a later list cell.
  - Findings are kept in representatives and expanded to every choice of
    members. dt-not-type names the literal it checked. `hidden` became
    `rdf`, the number of RDF triples in the expanded closure.
  - The pending machinery is gone (`needs_equality`, `Outcome::Pending`,
    chaos's `NeedsEquality`), so every input is compared.
  - Departures in the verification crates:
    - F1 also covers the aliases of `rdfs:subClassOf`: eq-rep-p copies
      the `c rdfs:subClassOf c` that OWL-RL misses.
    - HermiT does not judge an input that makes a reserved IRI (`rdf:`,
      `rdfs:`, `owl:`, `xsd:`) `owl:sameAs` another term. Such an input is
      not OWL 2 DL, as with `owl.n3`. Nemo and the reference still judge
      it.
    - The generator makes vocabulary terms equal, so every seed now draws
      different inputs.
    - The reference evaluator's `chain()` is semi-naive, and
      `every_member` walks a list's cells from the end. The results are
      the same, checked against Nemo on 1,500 seeds. An input with a
      property `owl:sameAs rdf:rest` had taken the old `chain()` 38 s.

  Left for later: each step that merges scans the whole store to rewrite
  it (occurrence lists would avoid that, as an experiment), and a finding
  expands as a product over its classes, as the W3C closure does.
- **Wave 4, integrated.** On the merged tree (`f8fc3b9`):
  - The 51 minimal inputs equal Nemo, closures and findings.
  - The W3C OWL 2 RL tests pass every case Nemo passes (93), and the RDF-MT
    tests 37 (Nemo 30).
  - Brick with small1 and its three variants, soda hall, and the 108
    building models equal Nemo. This was the first full run of the
    building models and of the `owl.n3` variants.
  - 100,000 full and 100,000 incremental chaos iterations pass. Every input
    is compared except 41 that are too large for the reference.
  - The CLI's 20 tests and Python's 287 pass.

  `docs/verification/2026-11-05-wave-4.md` has the numbers.
- **14.** Version 1 closed on 2026-11-06, without a quiet machine.
  - The Baseline (`docs/verification/2026-11-06-v1.md`) ran at `f9e3625`
    with TurbOWL and Nemo, 5 runs on every default scenario, at a 1-minute
    load average of 1.1 to 2.3, by the owner's choice. The spread between
    runs stays under 7% for TurbOWL and under 4% for Nemo. Nemo's
    reasoning takes 38 to 97 times as long. All 26 `scaling_*` tests pass,
    the highest at 1.18, and the LUBM time exponent is 1.01.
  - Every budget of the spec is met (`docs/specs/v1.md`, Results). ADR
    0006 retires the two that named `reasonable`; wave 3's trend run met
    them.
  - README, CHANGELOG and AGENTS were written on `v2`, the same day
    version 2's phase 1 closed. So the README describes the `v2` branch,
    the CHANGELOG's 0.1.0 section records version 1, and `main` keeps
    version 1's code without a README.
  - Iteration 0's rubric score is written in `docs/specs/experiments.md`
    separately; until then the ticket's last box stays open.
