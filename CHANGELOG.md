# Changelog

## 0.2.1 — 2026-11-10

### Added

- **A cancel handle that trips itself.** `Cancel::with_check` takes a
  function, calls it at each of the handle's checks, and trips when it
  returns true; the handle then stays tripped and the function is not
  called again. It needs no second thread, so a deadline works in a
  browser: `Cancel::with_check(move || Instant::now() >= deadline)`. The
  caller supplies the clock. `cancel()` still trips such a handle, and
  clones share it.

## 0.2.0 — 2026-11-10

### Added

- **Every OWL 2 RL rule.** All 78 rules of OWL 2 Profiles §4.3, Tables 4
  to 9, where 0.1.0 had 39. New are prp-key, cls-maxc2, cls-maxqc3 and
  cls-maxqc4; the schema rules scm-eqc2, scm-spo, scm-eqp1/2, scm-dom1/2,
  scm-rng1/2, scm-hv, scm-svf1/2, scm-avf1/2, scm-int and scm-uni; eq-ref,
  scm-cls, scm-op, scm-dp, prp-ap, dt-type1 and dt-type2; dt-eq and
  dt-diff; cls-oo; and the checks cax-adc, cls-maxc1, cls-maxqc1/2,
  prp-adp, prp-npa1/2 and eq-diff1/2/3.
- **Switches.** Seven named groups of rules (`Switch`, `Switches`,
  `Reasoner::with_switches`, `set_switches`). `equality`,
  `property-chains` and `checks` are on by default; `identity`, `schema`,
  `trivial` and `literal-values` are off. `Switches::complete()` turns
  every switch on, which is the W3C rule set. The CLI has
  `--switch NAME=on|off`, `--complete` and `--switches`; Python has
  `Reasoner(switches={...}, complete=...)`.
- **The OWL 2 RL datatype map.** The 32 datatypes of OWL 2 Profiles
  §4.2.1, used by dt-not-type, dt-type1, dt-type2, dt-eq and dt-diff.
- **Explanations.** `Reasoner::explain` and `Reasoner::explain_diagnostic`
  say why a triple is in the closure or why a diagnostic was reported: a
  proof from base triples, one W3C rule per step, in the user's terms.
  `Reasoner::explainer` answers many questions over one closure, and
  `ExplainCache` keeps its indexes between calls. Nothing is recorded
  while reasoning. The CLI has `--explain` and `--explain-diagnostics`;
  Python has `Reasoner.explain`, `Reasoner.explain_diagnostic` and
  `Proof`. Asking after a change without reasoning again is
  `Error::NotReasoned`.
- **Cancellation.** `Cancel`, a handle any thread can trip, stops
  `Reasoner::reason_cancellable` before its next round and
  `Explainer::explain_cancellable` between its steps, with
  `Error::Cancelled`. A cancelled run keeps no partial result. Python has
  `turbowl.Cancel`, `cancel=`, `CancelledError`, and Ctrl+C.
- **Snapshots.** A reasoner's whole state in one versioned, checksummed
  file: `save_snapshot` (written atomically), `from_snapshot`, and
  `write_snapshot` and `from_snapshot_bytes` for WebAssembly. A damaged
  snapshot, or one of another format, is refused with `Error::Snapshot`.
  The CLI has `--save-snapshot` and `--load-snapshot`; Python has
  `save_snapshot`, `from_snapshot` and `SnapshotError`.
- **TurbOWL's own N-Triples and Turtle parsers**, ported from oxttl 0.2.5.
  They intern each term straight into its id and give the same triples,
  term ids and errors as Oxigraph's. N3 and RDF/XML use Oxigraph's.
- **Every core.** A document of 384 KiB or more is parsed in pieces on
  every core, and a round of 65,536 tuples or more runs its rule families
  in parallel (`Threads`, `Reasoner::set_threads`, the `parallel` feature,
  on by default). The closure, the diagnostics and the order of every read
  are the same at any thread count. `RAYON_NUM_THREADS=1` runs on one.
- **Windowed reading.** `Reasoner::load_file` reads N-Triples and Turtle in
  windows of 32 MiB, so a large file is never in memory whole.
- **RDF 1.2 triple terms** with the `rdf-12` feature, off by default. A
  triple term (`<<( s p o )>>`) is an opaque term: rules carry it but never
  look inside it, and the triple inside is not asserted.
- **Counters.** `Reasoner::profile` reports each round, each step and the
  bytes the reasoner holds, by part. It is meant for benchmarks; its
  fields may change.

### Changed

- **Licence: MIT**, as turtle-lsp. It was MIT OR Apache-2.0.
- **A run uses every core by default** (`Threads::ALL`). Small rounds,
  such as those after an editor's change, stay on the caller's thread.
- **dt-not-type checks the 32 datatypes of OWL 2 RL**, not 5. An untagged
  `rdf:langString` literal is no longer reported, since `rdf:langString`
  is not an OWL 2 RL datatype.
- **Transitive properties close a large table in one step**: a closure
  that took over 8 GB takes 81 MB.
- **prp-key** reads a key list by pairs along its paths, not by every
  path: an input that took 32 s takes 0.16 s.
- **The `identity` and `schema` switches are off by default.** `schema`
  made most measured scenarios 22% to 62% slower, and `identity` up to 26%.
- **The CLI's exit status 1** also covers a refused snapshot, and a missing
  input file is a usage error only without `--load-snapshot`.
- **Python** releases the GIL for snapshots, as it does for parsing,
  reasoning and writing.

### Removed

- **The `prp-spo2` cargo feature.** Property chains are the
  `property-chains` switch, in every build, WebAssembly included.

### Internal

- **Verification by configuration.** The rule program is one file per
  switch group, composed for any configuration and emitted for Nemo,
  Soufflé and VLog. The reference evaluator honours the switches, chaos
  draws a configuration per iteration and compares one thread with all
  cores, and the oracle suite runs every input in the default and the
  complete configuration.
- **The benchmark** compares TurbOWL with Nemo, Soufflé and VLog on the
  same rules, on public datasets (LUBM, UOBM, Claros, OWL2Bench, Brick),
  with a 16 GB memory cap. `reasonable` is no longer in it.

## 0.1.0 — 2026-11-05

### Added

- **The library.** `Reasoner` loads a file or a string, inserts and
  removes triples, sets the base, reasons, and reads the closure, the
  inferred triples and the diagnostics. Terms in and out are `oxrdf`
  types.
- **Turtle, N-Triples, N3 and RDF/XML**, in and out. Blank nodes belong to
  their document.
- **39 rules of the W3C OWL 2 RL tables**, and contradictions and
  ill-typed literals reported as diagnostics.
- **`owl:sameAs` by representatives.** Equal terms share one id inside;
  reading the closure expands them.
- **Incremental runs.** Added triples are joined against the stored
  closure, and setting an unchanged base costs nothing.
- **The `turbowl` command**: the closure to a file or stdout, diagnostics
  as text, JSON or NDJSON, `--fail-on`, `--max-diagnostics`,
  `--summary-only`, and stable exit codes.
- **The `turbowl` Python package**, with rdflib graphs and terms in and
  out.
- **WebAssembly.** The core builds for `wasm32-unknown-unknown`.
- **Verification** against Nemo, OWL-RL, the W3C OWL 2 RL and RDF-MT tests,
  HermiT and a reference evaluator, with chaos testing.
