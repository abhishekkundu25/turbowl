# Changelog

What changed in each version of TurbOWL. Nothing has been published yet:
the versions here are the repository's. The numbers come from the records
in `docs/verification/`.

## Unreleased: version 2, phase 1

Every rule of the W3C OWL 2 RL tables, the switches, and the scoreboard
(`docs/specs/v2-1-scoreboard-and-rules.md`, record
`docs/verification/2026-11-06-v2-phase-1.md`).

### Added

- **Every rule of OWL 2 Profiles §4.3, Tables 4 to 9**: 78 rules, against
  version 1's 39. New are prp-key, cls-maxc2, cls-maxqc3 and cls-maxqc4
  (`identity`); scm-eqc2, scm-spo, scm-eqp1, scm-eqp2, scm-dom1, scm-dom2,
  scm-rng1, scm-rng2, scm-hv, scm-svf1, scm-svf2, scm-avf1, scm-avf2,
  scm-int and scm-uni (`schema`); eq-ref, scm-cls, scm-op, scm-dp, prp-ap,
  dt-type1 and dt-type2 (`trivial`); dt-eq and dt-diff (`literal-values`);
  cls-oo (the core); and the checks cax-adc, cls-maxc1, cls-maxqc1,
  cls-maxqc2, prp-adp, prp-npa1, prp-npa2, eq-diff1, eq-diff2 and eq-diff3.
- **The OWL 2 RL datatype map**: the 32 datatypes of OWL 2 Profiles §4.2.1,
  shared by dt-not-type, dt-type1, dt-type2, dt-eq and dt-diff.
- **Switches**: seven named groups of rules, `Switch` and `Switches` in the
  library, with `Reasoner::with_switches`, `Reasoner::switches` and
  `Reasoner::set_switches`. `equality`, `property-chains` and `checks` are
  on by default; `identity`, `schema`, `trivial` and `literal-values` are
  off. `Switches::complete()` turns every switch on: the W3C rule set.
  Changing the switches makes the next run a full one.
- **The CLI**: `--switch NAME=on|off`, `--complete` and `--switches` (each
  switch, its default and its cost). The JSON summary gains `switches`.
- **Python**: `Reasoner(switches={...}, complete=...)` and
  `Reasoner.switches`, with the CLI's names.
- **Verification by configuration**: the rule program is one file per
  switch group (`oracles/rules/`), composed for any configuration and
  emitted for Nemo, Soufflé and VLog. The reference evaluator honours the
  switches, chaos draws a configuration per iteration, and the oracle suite
  runs every input in the default and the complete configuration.
- **Oracle faults F6 to F15**, all OWL-RL's, each shown against the W3C
  text with a test (`crates/oracle/src/differences.rs`).
- **The scoreboard** in the bench: Soufflé 2.5 and VLog 1.3.7 as
  competitors beside Nemo, the public datasets (LUBM from UBA, UOBM,
  Claros, OWL2Bench), a 16 GB memory cap that stops a run, RDFox's
  published results with the bridge's conditions, and `--scoreboard`.

### Changed

- **dt-not-type** checks the 32 datatypes of OWL 2 RL, not 5. An untagged
  `rdf:langString` literal is no longer reported: `rdf:langString` is not a
  datatype of OWL 2 RL.
- **Transitive properties** close a large table in one step: owl2bench-rl-1
  went from over 8 GB to 81 MB.
- **prp-key** reads a key list whose cells branch by pairs along its paths,
  not by every path: a chaos input went from 32 s to 0.16 s.
- **The defaults** of `identity` and `schema` are off, by ADR 0005's speed
  rule. `schema` slowed 12 of the 15 measured scenarios by more than 5%
  (22% to 39% at editor size, 62% on claros-l), `identity` uobm-010-u by
  26% and claros-l by 6%.

### Removed

- **`reasonable`**, from the bench and the dependency tree (ADR 0006).
  Results files that hold its rows still load, and the rows are skipped.
- **The `prp-spo2` cargo feature**: property chains are the
  `property-chains` switch, in every build, WebAssembly included.

## 0.1.0: version 1 (2026-11-05)

A fast OWL 2 RL reasoner, `f9e3625` on `main` (`docs/specs/v1.md`,
records `docs/verification/2026-11-05-wave-4.md` and
`docs/verification/2026-11-06-v1.md`).

### Added

- **The library** (`crates/turbowl`): `Reasoner` (load a file or a string,
  insert, remove, set the base, clear, reason, read the closure, the
  inferred triples and the diagnostics, write), `Format`, `Summary`,
  `Diagnostic`, `Severity` and `Error`. Terms in and out are `oxrdf` types.
- **Input and output** in Turtle, N-Triples, N3 and RDF/XML. Blank nodes
  are scoped per document; N3 statements that are not RDF are dropped.
- **39 rules of the W3C OWL 2 RL tables**: eq-sym, eq-trans, eq-rep-s,
  eq-rep-p, eq-rep-o; prp-dom, prp-rng, prp-fp, prp-ifp, prp-symp,
  prp-trp, prp-spo1, prp-spo2, prp-eqp1, prp-eqp2, prp-inv1, prp-inv2;
  cls-thing, cls-nothing1, cls-int1, cls-int2, cls-uni, cls-svf1,
  cls-svf2, cls-avf, cls-hv1, cls-hv2, cax-sco, cax-eqc1, cax-eqc2;
  scm-sco, scm-eqc1; and, reported as diagnostics, cax-dw, prp-pdw,
  cls-nothing2, prp-asyp, prp-irp, cls-com and dt-not-type (for
  `xsd:string`, `xsd:integer`, `xsd:int`, `rdf:langString` and
  `rdf:XMLLiteral`).
- **`owl:sameAs` by representatives**: a union-find of equal terms inside;
  reads expand each representative to every member.
- **Incremental runs**: additions are joined against the stored closure; a
  removal runs again from the remaining base; setting an unchanged base
  costs nothing.
- **The `turbowl` command**: the closure to a file or stdout, the
  diagnostics as text, JSON or NDJSON, `--fail-on`, `--max-diagnostics`,
  `--summary-only`, and the exit codes 0, 1, 2 and 64.
- **The `turbowl` Python package**: rdflib graphs and terms in and out,
  built with maturin.
- **The core builds for `wasm32-unknown-unknown`** with default features
  off.
- **Verification**: Nemo, OWL-RL, the W3C OWL 2 RL and RDF-MT tests,
  HermiT and a reference evaluator as oracles; chaos against the reference
  evaluator; `scaling_*` tests; a benchmark against Nemo and `reasonable`.
  Oracle faults F1 to F5, all OWL-RL's.

### Results

- W3C OWL 2 RL: 93 cases, every case Nemo passes. RDF-MT: 37 (Nemo 30).
- The 51 minimal inputs, the Brick inputs and the 108 building models
  equal Nemo exactly; 100,000 full and 100,000 incremental chaos
  iterations pass.
- Reasoning takes 3.0 ms on brick-small1, 128.5 ms on brick-buildings and
  735.1 ms on lubm-50. Nemo's reasoning takes 38 to 97 times as long on
  the six full-run scenarios. Loading is 84% of lubm-50's 4.66 s total. An
  added triple costs 0.2 ms. These were measured at a load average of 1.1
  to 2.3, not on a quiet machine.
