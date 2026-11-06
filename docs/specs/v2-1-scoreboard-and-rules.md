# Version 2, phase 1: the scoreboard and all of OWL 2 RL

Status: in progress. Tickets: `docs/tickets/v2/` (01 to 15). Decisions:
ADR 0005 (switches), ADR 0006 (the scoreboard). Words: `CONTEXT.md`
(Competitor, Scoreboard, Published result, Bridge, Baseline, Switch).
Facts about the competitors and datasets: `docs/research/competitors-2026-11.md`.
Starting point: version 1 at `f9e3625` (`docs/specs/v1.md`), whose
quiet-machine numbers version 1's ticket 14 records as the Baseline.

Version 2 makes TurbOWL the best OWL 2 RL reasoner. Phase 1 makes that
claim measurable and fair. Phase 2 (`docs/specs/v2-2-speed.md`) works to
lead the scoreboard. Phase 3 (`docs/specs/v2-3-removals.md`) makes removals
incremental.

## Problem Statement

Version 1 is faster than `reasonable` and Nemo on every scenario measured,
but neither is the strongest engine. The engines that set the state of the
art in OWL 2 RL materialisation (RDFox, VLog, Soufflé) have never run
against it on the datasets their authors used, so nobody can say where
TurbOWL stands.

Version 1 also implements only part of the W3C OWL 2 RL rule tables. A speed
win could come from doing less work. A modeller whose ontology uses keys,
cardinality restrictions, `owl:oneOf`, negative property assertions,
`owl:AllDifferent` or the OWL 2 datatypes gets an incomplete closure and
missed contradictions, without being told. Of the 119 W3C OWL 2 RL test
cases that run here, version 1 passes 93; OWL-RL, a slow Python reasoner,
passes 100.

And there is no way to trade inference for speed. A user who never needs
`owl:sameAs` reasoning, or property chains, or the contradiction checks,
pays for them anyway.

## Solution

- **A scoreboard.** TurbOWL runs against VLog, Soufflé and Nemo (and
  GraphDB, if its licence allows publishing numbers) on Brick, the
  building models, and the public datasets of the RDFox, VLog and Nemo
  papers, up to what fits in 16 GB. Every engine runs the same W3C rules,
  and its closure is checked against TurbOWL's. RDFox, which cannot be run
  here, is compared through its published results, scaled to this machine
  through a bridge and labelled an estimate. `reasonable` leaves the
  project.
- **All of OWL 2 RL.** Every rule of the W3C tables (OWL 2 Profiles §4.3,
  Tables 4 to 9) and every OWL 2 RL datatype (§4.2).
- **Switches.** Every rule belongs to the always-on core or to one switch.
  A caller turns switches on or off. Groups that other reasoners rarely
  implement, or that slow reasoning down, are off by default. The
  complete configuration, every switch on, is the W3C rule set.

## The switches

| Switch | Rules | Default |
|---|---|---|
| none (the core) | cax-sco, cax-eqc1, cax-eqc2, cls-thing, cls-nothing1, cls-int1, cls-int2, cls-uni, cls-svf1, cls-svf2, cls-avf, cls-hv1, cls-hv2, cls-oo, prp-dom, prp-rng, prp-fp, prp-ifp, prp-symp, prp-trp, prp-spo1, prp-eqp1, prp-eqp2, prp-inv1, prp-inv2, scm-sco, scm-eqc1 | always on |
| `equality` | eq-sym, eq-trans, eq-rep-s, eq-rep-p, eq-rep-o | on |
| `property-chains` | prp-spo2 | on |
| `checks` | cax-dw, cax-adc, cls-com, cls-nothing2, cls-maxc1, cls-maxqc1, cls-maxqc2, prp-irp, prp-asyp, prp-pdw, prp-adp, prp-npa1, prp-npa2, eq-diff1, eq-diff2, eq-diff3, dt-not-type | on |
| `identity` | prp-key, cls-maxc2, cls-maxqc3, cls-maxqc4 | off (the close-out: 26% slower on uobm-010-u) |
| `schema` | scm-eqc2, scm-spo, scm-eqp1, scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv, scm-svf1, scm-svf2, scm-avf1, scm-avf2, scm-int, scm-uni | off (the close-out: 22% to 62% slower) |
| `trivial` | eq-ref, scm-cls, scm-op, scm-dp, prp-ap, dt-type1, dt-type2 | off |
| `literal-values` | dt-eq, dt-diff | off |

The default rule (ADR 0005): a switch is off by default when other
reasoners rarely implement its rules, or when turning it on slows the
reasoning time of a scoreboard scenario by more than 5%. `equality`,
`property-chains` and `checks` keep version 1's behaviour and stay on. The
close-out (ticket 15) applies the rule to `identity` and `schema` with the
research and a quiet-machine measurement.

The close-out turned both off on 2026-11-06
(`docs/verification/2026-11-06-v2-phase-1.md`). The research keeps them
on, since owlrl, GraphDB and the OWL-RS Nemo program implement both groups.
The measurement turns them off:
- `schema` slowed every editor-size scenario by 22% to 39%, claros-l by
  62% and uobm-010-u by 53%.
- `identity` slowed uobm-010-u by 26% and claros-l by 6%. It costs nothing
  measurable on the editor-size scenarios.

The machine was not quiet (load average about 2), by the owner's choice.
The gaps are many times the spread between runs. The defaults therefore
give less than owlrl and GraphDB do (user story 16): no key merges and no
schema subsumptions. Each group comes back with one switch.

## User Stories

The rules

1. As a modeller, I want `owl:hasKey` to make individuals with the same key values the same individual, so that records from two sources merge.
2. As a modeller, I want `owl:maxCardinality 1` and `owl:maxQualifiedCardinality 1` to make the values of a functional-in-context property the same individual, so that cardinality axioms take effect.
3. As a modeller, I want an individual that breaks a `owl:maxCardinality 0` restriction reported, so that impossible data shows up.
4. As a modeller, I want `owl:oneOf` to type each listed individual with the class, so that enumerated classes work.
5. As a modeller, I want `owl:differentFrom`, `owl:AllDifferent` (with `owl:members` or `owl:distinctMembers`) checked against `owl:sameAs`, so that two individuals declared different but derived equal are reported.
6. As a modeller, I want negative property assertions checked, so that a stated "x does not p y" contradicts a derived `x p y`.
7. As a modeller, I want `owl:AllDisjointClasses` and `owl:AllDisjointProperties` checked like their pairwise forms, so that grouped disjointness axioms work.
8. As a modeller, I want the schema rules, so that the class and property hierarchies hold every subsumption the W3C rules derive (`scm-spo` closes `rdfs:subPropertyOf`, `scm-eqc2` makes mutual subclasses equivalent, domains and ranges follow the hierarchies).
9. As a modeller, I want every OWL 2 RL datatype checked (decimals, every integer type, floats, doubles, booleans, strings and their subtypes, dates and times, binary types, URIs), so that a literal outside its datatype's value space is reported whatever its type.
10. As a modeller, I want literals with the same value (`"1"^^xsd:integer` and `"01"^^xsd:integer`, or `"1"^^xsd:integer` and `"1.0"^^xsd:decimal`) to be the same when I turn `literal-values` on, so that value-based equality follows the standard.
11. As a modeller, I want two literals with different values made `owl:sameAs` to be reported when `literal-values` is on, without the reasoner listing every pair of different literals.
12. As a conformance tester, I want the complete configuration to pass every W3C OWL 2 RL test case that rules can pass, so that TurbOWL is the most conformant OWL 2 RL reasoner measured.

The switches

13. As a performance-minded user, I want to turn `equality` off when my data has no meaningful `owl:sameAs`, so that I pay nothing for it.
14. As a performance-minded user, I want to turn `checks` off when I only need inferences, so that validation does not slow my pipeline.
15. As a performance-minded user, I want to turn `property-chains` off, so that ontologies with chain axioms I do not need reason faster.
16. As a user, I want the defaults to give what other good reasoners give, without filler like `x owl:sameAs x`, so that the output is useful as it is.
17. As a conformance tester, I want one call that turns every switch on, so that I get the complete W3C rule set.
18. As a library user, I want the switches as a typed set, so that the compiler stops me from naming a switch that does not exist.
19. As a library user, I want changing the switches between runs to give the same closure as a fresh reasoner with those switches, so that I never see a stale mix.
20. As a command-line user, I want `--switch NAME=on|off`, `--complete` and a list of the switches with their defaults, so that I can set them in scripts.
21. As a Python user, I want the same switches with the same names, so that notebooks and the CLI agree.
22. As a browser developer, I want property chains available in the WebAssembly build, so that it reasons like the native one.

The scoreboard

23. As the owner, I want TurbOWL measured against VLog, Soufflé and Nemo on the same inputs, so that "the best" is a measured claim.
24. As the owner, I want every engine to run the same W3C rules and its closure checked against TurbOWL's, so that no engine wins by doing less work.
25. As the owner, I want a second table where each engine uses its own OWL mode, so that users see the product they would actually run.
26. As the owner, I want the public datasets of the RDFox, VLog and Nemo papers (the official LUBM generator, UOBM, Claros, ChEMBL, Reactome, a DBpedia subset) as scenarios, so that results compare with the literature.
27. As the owner, I want RDFox compared through its published results, only on the same dataset with the same fact counts and scaled through a bridge engine run here, and labelled an estimate, so that the comparison is honest without a licence.
28. As the owner, I want every axis reported: reasoning time, total time, time per incremental change and peak memory, single-threaded and on all cores, in the default and the complete configuration.
29. As the owner, I want the scoreboard to say who leads each axis on each scenario, and where TurbOWL does not, so that the claim is never stretched.
30. As the owner, I want no benchmark run to pass 16 GB of memory, so that my other work keeps running; a run that would is stopped and recorded.
31. As the owner, I want the big scenarios to run only when asked, so that they run rarely.
32. As a maintainer, I want each competitor fetched and pinned by version and checksum into `tools/`, with no system install, so that a run can be reproduced.
33. As a maintainer, I want a competitor that is not installed reported as skipped, as the oracles are, so that a partial machine still runs the rest.
34. As a maintainer, I want `reasonable` gone from the code and the dependency tree, so that nothing measures against it by accident.

Verification

35. As a maintainer, I want every new rule to have a minimal input with its expected inferences, checked against Nemo, OWL-RL and the reference evaluator, so that each rule is pinned to the standard.
36. As a maintainer, I want Nemo's program and the reference evaluator composed from the same switch groups as the reasoner, so that every configuration is checked, not only the default.
37. As a maintainer, I want chaos to draw each iteration's configuration from its seed, so that random configurations meet random inputs.
38. As a maintainer, I want the generator to produce keys, cardinality restrictions, enumerations, negative assertions, all-different and all-disjoint axioms and literals of every datatype, so that chaos reaches every new rule.
39. As a maintainer, I want one source for the rule program that every Datalog engine runs, translated per engine, so that the Nemo, Soufflé and VLog programs cannot drift apart.
40. As a future agent, I want each switch group to live in its own files across the core, the reference evaluator, the generator, the minimal inputs and the rule program, so that rule tickets run in parallel without conflicts.

## Implementation Decisions

- **Subtract first.** `reasonable` leaves the bench (engine, dependency,
  budgets, docs). Results files that hold `reasonable` rows still load: the
  rows are skipped with a note.
- **The switch model.** `Switch` is an enum of the seven groups and
  `Switches` a small copyable set with the defaults as `Default`, a
  `complete()` constructor and on/off setters. The public interface gains
  `Switch`, `Switches`, a constructor taking switches, a setter and a getter.
  Changing the switches marks the next run full. Each rule family is
  registered with the switch it belongs to (none for the core), so the
  reasoner skips a family whose switch is off and inserts a family's facts
  only when it is on. A family maps to one switch, and a switch may hold
  several families.
- **The `prp-spo2` cargo feature goes**: chains become the
  `property-chains` family, registered with that switch, in every build.
- **Families follow the switches.** The version 1 contradiction rules move
  out of `datatype.rs` into a `checks` family with no change in behaviour,
  so `datatype.rs` holds the datatypes and dt-not-type. New families:
  `identity`, `schema`, `trivial`, `literal-values`, and `cls-oo` inside the
  class family. The seam ticket registers each as an empty stub, so the
  rule tickets fill their own files.
- **The vocabulary** gains every IRI the new rules name (`owl:hasKey`,
  `owl:maxCardinality`, `owl:maxQualifiedCardinality`, `owl:onClass`,
  `owl:onDataRange`, `owl:oneOf`, `owl:differentFrom`, `owl:AllDifferent`,
  `owl:members`, `owl:distinctMembers`, `owl:AllDisjointClasses`,
  `owl:AllDisjointProperties`, `owl:NegativePropertyAssertion`,
  `owl:sourceIndividual`, `owl:assertionProperty`, `owl:targetIndividual`,
  `owl:targetValue`, `owl:ObjectProperty`, `owl:DatatypeProperty`,
  `owl:AnnotationProperty`, `rdfs:Datatype`, the annotation properties of
  prp-ap, and every OWL 2 RL datatype IRI) in one change, before the rule
  tickets.
- **eq-ref is an output rule.** With representatives, `x owl:sameAs x` for
  every term of the closure is produced when the closure is read, and is
  stored only once a rule could read it: when `owl:sameAs` is the subject
  or object of a stored triple (ADR 0003's eq-ref amendment).
- **dt-diff is never listed.** With `literal-values` on, two literals with
  different values that become `owl:sameAs` give the eq-diff1 finding; the
  `owl:differentFrom` triples between all literals are not materialised,
  nor is what eq-rep-s and eq-rep-o derive from them (every two terms
  equal to such literals are reported through eq-diff1 instead). This is a
  departure in output only, and the W3C runner lists it
  (`w3c::DEPARTURES`).
- **dt-eq merges literal classes.** With `literal-values` on, literals with
  equal values (the OWL 2 value spaces, where `xsd:integer` values are
  `xsd:decimal` values) join one class in the canon. The value rules range
  over the input's literals and the two literals the cardinality rules
  name (`"0"` and `"1"` of `xsd:nonNegativeInteger`), so that
  `"1"^^xsd:integer` reaches cls-maxc2 as the W3C rules say; a merge that
  moves one of those two makes the cardinality rules join every
  restriction again (ADR 0003's literal-constants amendment).
- **dt-type2 is stored**, since rules read `rdf:type` with a datatype as
  the class (`owl:someValuesFrom xsd:integer`, a datatype in a hierarchy).
- **The datatype map** holds the value space and lexical mapping of the 32
  datatypes OWL 2 RL supports (OWL 2 Profiles, Section 4.2.1), shared by
  dt-type1, dt-type2, dt-not-type, dt-eq and dt-diff. `owl:real`,
  `owl:rational` and `rdf:langString` are outside it: their literals have
  no known value. A language-tagged literal reads as an `rdf:PlainLiteral`
  value.
- **One rule source.** The W3C rule forms move from one Nemo file into one
  file per switch group, in Nemo's syntax. The oracle crate composes a
  program for a configuration and emits it for Nemo, Soufflé and VLog. The
  bench uses the same programs.
- **The scoreboard in the bench.** Competitors are subprocess engines,
  like Nemo today, each fetched and pinned into `tools/`. Each runs the
  composed program, and its closure is checked against TurbOWL's. Each
  engine has single-threaded and all-cores rows where it can run in
  parallel. A competitor's own OWL mode, where it has one, gets its own
  rows.
- **Datasets.** The official LUBM generator (run on the fetched JRE),
  UOBM, Claros, ChEMBL, Reactome and a DBpedia subset are fetched and
  pinned into `data/`, at sizes where every engine stays under 16 GB. They
  are opt-in scenarios. The bench's own LUBM-style data stays for the
  scaling fits and quick runs.
- **The memory cap.** The bench watches each run's resident memory and
  stops the run, by its PID, at 16 GB, recording `over-memory`. That is a
  result, not a problem.
- **Published results** are a data file in the bench with sources: paper,
  RDFox version, hardware, threads, dataset, program variant, fact counts
  before and after, time and memory, and the bridge engines' numbers from
  the same table. An estimate is computed only when the dataset's fact
  counts here match the paper's and the bridge ran here on the same data.
- **The scoreboard report** gives, per scenario and axis, each engine's
  value and the leader, in both configurations and both thread modes, with
  the published estimates in their own section.

## Testing Decisions

- **The interface is the test surface**, as in version 1: every suite drives
  the `Reasoner`, the CLI or Python, and compares closures and diagnostics
  as sets.
- **Minimal inputs per rule**: one or more for each new rule, with the
  expected inferences or findings written out, as in
  `crates/oracle/src/cases/` (one file per group). They run in the default and the complete
  configuration against Nemo, OWL-RL and the reference evaluator
  (`crates/oracle/tests/minimal.rs`).
- **Configurations**: the oracle suite runs every input in both
  configurations (`crates/oracle/tests/`); the W3C suites in the complete
  one (`tests/w3c.rs`); chaos draws a configuration per iteration
  (`crates/reference/tests/chaos.rs`), with the reference evaluator
  honouring the same switches.
- **Switch behaviour**: a changed configuration gives the same closure as a
  fresh reasoner with it; each switch off removes exactly its rules'
  consequences, checked against the reference evaluator; the defaults are
  as the table says (unit tests next to `reasoner.rs`, like
  `rules/equality/tests.rs`).
- **The rule source**: the default configuration's composed Nemo program
  gives version 1's results on every minimal input, the W3C suites and
  Brick; the Soufflé and VLog emitters are checked against golden programs;
  each competitor's closure is checked against TurbOWL's on every scenario
  it runs.
- **Datatypes**: the value spaces are tested against the W3C datatype test
  cases (the RDF-MT `datatypes-*` and `xmlsch-*` cases) and a table of
  edge cases per datatype; the reference evaluator's datatype module is
  extended the same way.
- **Adapters**: the CLI's switches in `crates/cli/tests/cli.rs`; Python's in
  `crates/python/tests/`, compared with the Rust library on the minimal
  inputs in both configurations.
- **The bench**: unit tests for the memory cap (a child that allocates past
  a small cap is stopped), the published-result arithmetic (an estimate is
  refused when fact counts differ), the results-file reader (old files with
  `reasonable` rows), and the scoreboard table.
- **Scaling**: `scaling_*` tests for each new rule family with a quadratic
  risk: prp-key (keys over many individuals), cls-maxc2 (many values), the
  schema rules over a deep hierarchy, dt-eq over many literals.

## Non-functional budgets (pass/fail)

Correctness (gates):
- Every oracle agrees on every input in the default and the complete
  configuration: Nemo exactly (dt-* aside), OWL-RL by containment after its
  listed faults, HermiT on every inferred subsumption it can judge, the
  reference evaluator on chaos.
- The W3C OWL 2 RL suite passes at least 101 of its 119 runnable cases in
  the complete configuration, every case Nemo passes among them. Each case
  that fails is listed with the reason it is beyond the rules. (The first
  draft said 113; the owner set 101 on 2026-11-06, once every rule had
  landed: Nemo and OWL-RL fail the other 18 too. The close-out found that
  15 of them expect a conclusion no OWL 2 RL rule derives, and 3 fail on
  how the test runner reads a case: an import it does not load, and
  annotations under the Direct Semantics. The runner's fix is a
  follow-up.)
- The RDF-MT suite passes at least 35 cases in the complete
  configuration, and no consistent case is reported inconsistent. Two of
  version 1's 37 (`rdfs-domain-and-range-intensionality-domain` and
  `-range`) test that RDFS does not carry a domain or range up the class
  hierarchy; OWL 2 RL's scm-dom1 and scm-rng1 do, so TurbOWL, Nemo and
  OWL-RL all entail them once `schema` is on, as the W3C rules say. A
  third, `rdfs-entailment-test002`, expects `"flargh"^^xsd:string` in an
  `rdf:langString` range to be inconsistent; `rdf:langString` is no
  datatype of OWL 2 RL, so dt-not-type does not check it.
- 100,000 full and 100,000 incremental chaos iterations pass, each drawing
  its configuration from the seed.
- On every scoreboard scenario, every competitor's closure in the same
  configuration has the same size as TurbOWL's, or the difference is a
  listed competitor fault with a test.

Performance:
- In the default configuration, the reasoning time of every editor-size
  scenario stays within 5% of the version 1 Baseline (median of 5 runs, a
  quiet machine, outside the spread). New rules cost nothing on inputs that
  do not use their vocabulary.
- No benchmark run of any engine passes 16 GB of resident memory.

Build:
- The core builds for `wasm32-unknown-unknown` with default features off,
  property chains included.
- `cargo fmt --check` and `cargo clippy --release --locked --workspace
  --all-targets -- -D warnings` are clean.

## Out of Scope

- Speed work: the parser, the snapshot format, parallel rounds and the
  engine experiments (phase 2).
- Incremental removal (phase 3).
- Running RDFox; publishing any number (crates.io, PyPI, GitHub, a report)
  without the owner's approval.
- A Protégé plugin, explanations, OWL 2 EL, QL or full OWL 2.
- Web-size data (more than 16 GB for any engine).
- Rules outside the W3C OWL 2 RL tables (SWRL, user rules, SHACL).

## Further Notes

- Six W3C OWL 2 RL cases fail in OWL-RL too and name no rule
  (`DisjointClasses-001`, `New-Feature-DisjointObjectProperties-001`,
  `New-Feature-ReflexiveProperty-001`, `WebOnt-I5.5-005`,
  `WebOnt-imports-011`, `chain2trans1`). With every rule in, 18 fail on
  every oracle of the rule set, these six among them; ticket 15 lists each
  with its reason.
- Nemo encodes literals as IRIs. It judges dt-type2, dt-eq and dt-diff
  through facts the oracle computes from the input's literals with the
  reference evaluator's datatype map (`Codec::literal_facts`); dt-not-type
  stays outside Nemo, judged by OWL-RL, the W3C tests and the reference
  evaluator, as in version 1.

## Principles

- **subtract-before-you-add**: `reasonable` goes before any competitor
  arrives (ticket 01); the contradiction rules leave `datatype.rs` before
  new checks are added (ticket 02).
- **migrate-callers-then-delete-legacy-apis**: the `prp-spo2` cargo feature
  is deleted when `property-chains` replaces it (ticket 02).
- **redesign-from-first-principles**: families are regrouped around the
  switches, as if switches had been there from the start (ticket 02).
- **foundational-thinking**, **model-the-domain**, **type-system-discipline**:
  `Switch` and `Switches` are types, each rule belongs to exactly one group,
  and an unknown switch name cannot compile (ticket 02); the datatype map
  is one table that every datatype rule reads (ticket 09).
- **boundary-discipline**: switch names are parsed once, at the CLI and
  Python boundaries (ticket 11); dataset and tool downloads are checked by
  checksum at fetch time (tickets 12, 13).
- **separate-before-serializing-shared-state**: every switch group has its
  own files in each crate, created by tickets 02 and 04, so rule tickets
  never edit the same file (tickets 05 to 10).
- **build-the-lever**: one rule source emitted for Nemo, Soufflé and VLog
  (ticket 03).
- **encode-lessons-in-structure**: each configuration is verified by
  construction, since Nemo and the reference are composed from the same
  groups (tickets 03, 04).
- **make-operations-idempotent**: the fetch scripts and the dataset
  generator converge to the same files when rerun, and resume after a
  crash (tickets 12, 13).
- **explain-the-number**: every scoreboard number carries its run count,
  spread, configuration, thread count and limiter; every published estimate
  its source, bridge and fact counts (tickets 14, 15).
- **outcome-oriented-execution**: three phases with a close-out each; a
  rule group may be empty between the seam ticket and its rule ticket.
