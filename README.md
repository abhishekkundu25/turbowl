# TurbOWL

TurbOWL is an OWL 2 RL reasoner in Rust. It computes the closure that the
W3C OWL 2 RL rule tables define (OWL 2 Profiles §4.3, Tables 4 to 9) and
reports the contradictions and ill-typed literals they define. It is one
library with a small interface, a command line, and Python bindings that
take and return rdflib graphs. It parses and reasons on every core, and
it saves a reasoned state to a snapshot that loads back without parsing
or reasoning. The core builds for WebAssembly. A run after a change
reuses the previous closure.

It is judged by independent oracles (Nemo, OWL-RL, the W3C tests, HermiT
and a reference evaluator), never by one reasoner.

**Status.** This is version 2, on `main`. Version 1 (`0.1.0`, tag `v0.1.0`
at `f9e3625`) implements 39 of the 78 rules. Version 2's phase 1 adds the
other 39, the switches and the scoreboard. Phase 2 adds speed: TurbOWL's
own parsers, every core, snapshots and the improvement loop. Both phases
are closed, with the gaps listed below. Phase 3, removals, is planned and
not started, at the owner's request.
Nothing is published: there is no crate on crates.io and no package on
PyPI. See `CHANGELOG.md`.

## Install

From a checkout, with Rust 1.99 or later:

```sh
cargo build --release --locked          # the library and the `turbowl` binary
./target/release/turbowl --help
```

The library is not on crates.io; use it as a path dependency:

```toml
[dependencies]
turbowl = { path = "path/to/turbowl/crates/turbowl" }
```

The `parallel` feature, on by default, brings the threads (rayon). Without
it, and always on WebAssembly, every round runs on the caller's thread.

The Python package needs Python 3.10 or later and `uv`. It builds with
maturin into `crates/python/.venv`:

```sh
cd crates/python && uv sync
uv run python -c "import turbowl; print(turbowl.__version__)"
```

## Use

### The command line

```sh
turbowl model.ttl -o closure.ttl                  # the closure, and a report on stdout
turbowl model.ttl --inferred-only                 # only the inferred triples, to stdout
turbowl model.ttl --summary-only --diagnostics json
turbowl --switch schema=on model.ttl              # one switch on
turbowl --complete model.ttl                      # every switch on: the W3C rule set
turbowl --switches                                # the switches, their defaults and costs
turbowl model.ttl --summary-only --save-snapshot state.snapshot
turbowl --load-snapshot state.snapshot more.ttl -o closure.ttl
RAYON_NUM_THREADS=1 turbowl model.ttl -o closure.ttl   # one thread
```

Each file's format comes from its extension (`.ttl`, `.nt`, `.n3`, `.rdf`,
`.owl`, `.xml`). The report is the diagnostics, then a summary, as text,
JSON or NDJSON. The exit status is 0 on success, 1 on an input or output
error, 2 on a diagnostic at or above `--fail-on` (default `error`), and 64
on a usage error. `turbowl --help` gives the details, and the JSON field
names are stable.

`--save-snapshot` saves the reasoner's state after the run.
`--load-snapshot` starts from a saved state, then adds the input files to
it; none are needed. A loaded state keeps its switches,
which `--complete` and `--switch` then change. A snapshot that is damaged,
or of another format or TurbOWL version, is refused with exit status 1.
The CLI has no threads option: it uses every core, and
`RAYON_NUM_THREADS=1` makes it use one.

### Rust

```rust
use turbowl::{Format, Reasoner, Switch, Switches};

let mut r = Reasoner::with_switches(Switches::default().on(Switch::Schema));
r.load_str(
    "@prefix ex: <http://example.org/> .
     @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
     ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .",
    Format::Turtle,
    None,
)?;
let summary = r.reason();
for triple in r.inferred() { /* ex:rex a ex:Animal, ... */ }
for d in r.diagnostics() { /* contradictions and ill-typed literals */ }
```

`Reasoner::new()` takes the default switches, and `Switches::complete()`
turns every switch on. `insert`, `remove` and `set_base` change the base
triples, and the next `reason()` reuses what it can.

```rust
use std::path::Path;
use turbowl::{Reasoner, Threads};

r.set_threads(Threads::ONE);                  // the default is Threads::ALL
r.save_snapshot(Path::new("state.snapshot"))?;
let mut again = Reasoner::from_snapshot(Path::new("state.snapshot"))?;
```

- **Loading.** N-Triples and Turtle are read by TurbOWL's own parsers,
  which intern each term straight into its id. N3 and RDF/XML use
  Oxigraph's. `load_file` reads N-Triples and Turtle in windows of 32 MiB,
  so a large file is never in memory whole. A document of 384 KiB or more
  is cut at line starts and parsed in pieces on every core. The triples,
  the term ids and the errors, with their line and column, are the same as
  a sequential parse gives.
- **Threads.** A run uses every core by default (`Threads::ALL`). A round
  of 65,536 tuples or more runs its rule families as parallel tasks and
  advances each predicate's table on its own thread; smaller rounds, such
  as those after an editor's change, stay on the caller's thread.
  `Threads::ONE` runs on one thread. The closure, the diagnostics, the
  summary and the order of every read are the same at any thread count.
- **Snapshots.** `save_snapshot` writes the reasoner's whole state to a
  file, atomically (a temporary file, then a rename). `from_snapshot`
  gives it back: the same closure, diagnostics and switches, and the same
  next run. `write_snapshot` and `from_snapshot_bytes` do the same with
  bytes, for WebAssembly. The format is versioned and checksummed (version
  2, `crates/turbowl/src/snapshot.rs`); another version, or a damaged
  file, is refused with `Error::Snapshot`. lubm-50's snapshot is 285 MB.
- **Counters.** `Reasoner::profile` tells what the last run did, round by
  round and step by step, and the bytes the reasoner holds, by part. It is
  for the bench, which records it beside each time; the CLI and Python do
  not expose it, and its fields may change.

### Python

```python
from rdflib import Graph, Namespace, RDF, RDFS
import turbowl

EX = Namespace("http://example.org/")
g = Graph()
g.add((EX.Dog, RDFS.subClassOf, EX.Animal))
g.add((EX.rex, RDF.type, EX.Dog))

r = turbowl.Reasoner(switches={"schema": True})   # or turbowl.Reasoner(complete=True)
r.insert(g)
summary = r.reason()
assert (EX.rex, RDF.type, EX.Animal) in r.inferred()
closure = r.graph()                               # an rdflib Graph

r.save_snapshot("state.snapshot")
again = turbowl.Reasoner.from_snapshot("state.snapshot")
```

A refused snapshot raises `turbowl.SnapshotError`. Parsing, reasoning,
writing and snapshots release the GIL. The package's documentation is
`help(turbowl)`.

## The rules and the switches

Every rule belongs to the always-on core or to exactly one switch, a named
group of rules (ADR 0005). A switch is off by default when other reasoners
rarely implement its rules, or when it slows reasoning by more than 5% on a
scoreboard scenario. The complete configuration, every switch on, is the
W3C rule set.

| Switch | Rules | Default | Why |
|---|---|---|---|
| none (the core) | cax-sco, cax-eqc1/2, cls-thing, cls-nothing1, cls-int1/2, cls-uni, cls-svf1/2, cls-avf, cls-hv1/2, cls-oo, prp-dom, prp-rng, prp-fp, prp-ifp, prp-symp, prp-trp, prp-spo1, prp-eqp1/2, prp-inv1/2, scm-sco, scm-eqc1 | always on | |
| `equality` | eq-sym, eq-trans, eq-rep-s/p/o | on | Version 1's behaviour, as in other OWL 2 RL reasoners |
| `property-chains` | prp-spo2 | on | Version 1's behaviour |
| `checks` | cax-dw, cax-adc, cls-com, cls-nothing2, cls-maxc1, cls-maxqc1/2, prp-irp, prp-asyp, prp-pdw, prp-adp, prp-npa1/2, eq-diff1/2/3, dt-not-type | on | Version 1's behaviour: contradictions and ill-typed literals are reported |
| `identity` | prp-key, cls-maxc2, cls-maxqc3/4 | off | 26% slower on uobm-010-u, where it grows the closure from 21.3 M to 39.1 M triples, and 6% on claros-l |
| `schema` | scm-eqc2, scm-spo, scm-eqp1/2, scm-dom1/2, scm-rng1/2, scm-hv, scm-svf1/2, scm-avf1/2, scm-int, scm-uni | off | 22% to 39% slower on every editor-size scenario and up to 62% on the data-size ones; 35% on LUBM at every size, where it adds 99 triples |
| `trivial` | eq-ref, scm-cls, scm-op, scm-dp, prp-ap, dt-type1, dt-type2 | off | Rarely implemented elsewhere, and filler: `x owl:sameAs x` for every term, four triples per class, each literal's datatypes |
| `literal-values` | dt-eq, dt-diff | off | Rarely implemented elsewhere; it reads every literal's value. dt-diff only checks: it lists no `owl:differentFrom` triples |

The defaults of `identity` and `schema` were set by measurement on
2026-11-06 (`docs/verification/2026-11-06-v2-phase-1.md`). By the research
alone, both would be on: owlrl, GraphDB and the OWL-RS Nemo program
implement them. So the defaults give less than those reasoners do: no key
merges and no schema subsumptions. Each group comes back with one switch.

dt-not-type and the value rules use the 32 datatypes OWL 2 RL supports
(OWL 2 Profiles §4.2.1). `owl:sameAs` is handled by representatives
(ADR 0003): equal terms share one id inside, and reading the closure
expands them. The bench's clique of 1,000 aliases, 11,000,002 triples
once expanded, reasons in 0.333 ms and stores 10,003 tuples (wave 4
record).

## Verification

Each closure is compared as a set with several oracles, each judging
something different (`crates/oracle`, `crates/reference`):

- **Nemo** runs the W3C forms of the same rules, composed from one file per
  switch group (`oracles/rules/`): closures and findings must be equal.
- **OWL-RL** (owlrl on rdflib) must contain the closure, after its listed
  faults (below).
- **The W3C tests**: the OWL 2 RL test cases and the RDF 1.1 Semantics
  entailment tests.
- **HermiT** must entail every inferred subsumption it can judge.
- **The reference evaluator**, a simple and slow evaluator of the same
  rules, judges chaos runs: generated inputs and configurations from a
  seed. Each chaos iteration also runs the input on one thread and on all
  cores, and the two must give the same summary, closure (in read order)
  and diagnostics.

Phase 2's results on the final code, in the default and the complete
configuration (`docs/verification/2026-11-07-v2-phase-2.md`):

| Judge | Result |
|---|---|
| Every suite: the library, the CLI, the oracle files, the reference evaluator, the bench, pytest (787) | Passed |
| W3C OWL 2 RL (complete configuration) | 101 of 119 runnable cases, every case Nemo passes (Nemo 101, OWL-RL 100) |
| W3C RDF-MT (complete configuration) | 35 of 51 runnable cases (Nemo 28, OWL-RL 30) |
| Brick with small1 and its three variants, and with soda hall | Equal to Nemo, closures and findings |
| The 108 building models | Equal to Nemo, closures and findings |
| 100,000 full chaos runs | 99,152 compared, all equal; 848 too large for the reference evaluator |
| 100,000 incremental chaos runs | 99,176 compared, all equal; 824 too large for the reference evaluator |

TurbOWL's own parsers are held to Oxigraph's by a cross-check
(`crates/turbowl/src/io/cross_check.rs`): 16,235 parses of 221 files and
5,193 strings from the sources, and 120,000 generated documents, give the
same triples, term ids and errors. The same check compares the parse in
pieces with the sequential one. A snapshot's round trip gives the same
closure, diagnostics and switches on the minimal inputs and on Brick, and
every changed byte or truncation of a small snapshot is refused.

The 18 W3C OWL 2 RL cases that fail, fail on Nemo and OWL-RL too. Fifteen
expect a conclusion no OWL 2 RL rule derives: 8 hold by refutation (a
difference or a complement type that no rule concludes), 3 need reasoning
about datatypes, 1 needs a new term, and 3 need a rule the tables do not
have. The other three depend on how the test runner reads a case: an
import, and annotations under the Direct Semantics. The phase 1 record
(`docs/verification/2026-11-06-v2-phase-1.md`) gives the reason for each.

### Oracle faults

A place where an oracle departs from the W3C rules or the datatype
definitions they rest on, shown against their text, is an oracle fault.
Each has an id, a test, and a filter only where the oracle misses
something. All fifteen are faults of OWL-RL (owlrl 7.6.2 on rdflib 7.6.0);
Nemo, HermiT and the W3C tests have none. The list, with the source lines
and the W3C text, is `crates/oracle/src/differences.rs`.

| Fault | What OWL-RL does |
|---|---|
| F1 | Its scm-sco and scm-spo never derive `c rdfs:subClassOf c` or `p rdfs:subPropertyOf p`, so scm-eqc2 and scm-eqp2 miss `c owl:equivalentClass c` and `p owl:equivalentProperty p` |
| F2 | Its cls-com finding names the second class where the individual belongs |
| F3 | Its dt-not-type accepts a lexical form with leading or trailing whitespace |
| F4 | It fails on a cyclic list |
| F5 | Its dt-not-type checks a literal only against its own datatype, never a type from prp-rng |
| F6 | Its eq-diff2, eq-diff3 and prp-adp miss positions of a list |
| F7 | Its dt-not-type reads numbers with Python's `int()` and `Decimal()` |
| F8 | Its dt-not-type rejects zeros, INF, NaN and rounded numerals of `xsd:double` and `xsd:float` |
| F9 | Its dt-not-type has the wrong lexical spaces for booleans, binaries, the XML name types, `xsd:normalizedString` and `xsd:anyURI` |
| F10 | Its dt-not-type reads `xsd:dateTime` with `time.strptime` |
| F11 | Its dt-not-type parses an `rdf:XMLLiteral` as a whole document |
| F12 | rdflib rewrites the lexical forms of `xsd:token` and `xsd:normalizedString` literals |
| F13 | Its prp-key never compares a key's last property |
| F14 | Its cardinality rules compare Python values, so a double, float or boolean cardinality fires them |
| F15 | It skips dt-eq and dt-diff, and its dt-type2 types a literal with its own datatype and that datatype's supertypes only |

F8, F11, F13 and F14, and parts of F9 and F10, only derive or report more
than the W3C rules, which containment never sees: they filter nothing.

## The scoreboard

The scoreboard (ADR 0006) measures TurbOWL against Nemo 0.10.1, Soufflé
2.5 and VLog 1.3.7, each running the same W3C rules, with each closure
checked against TurbOWL's. RDFox cannot be run here and counts only
through its published results; no estimate meets the bridge's conditions
yet.

### Phase 2's race

Phase 2's close-out ran a short race, at the owner's request
(`docs/verification/2026-11-07-v2-phase-2.md`, raw results in
`docs/verification/data/2026-11-07-v2-phase-2/`). Its caveats:

- **Editor size only.** The editor-size scenarios, in the default and the
  complete configuration. The data-size scenarios ran with TurbOWL alone.
  The full-size race is ticket 27, pending the owner.
- **A busy machine.** The 1-minute load average was 3 to 5. Each gap below
  is many times the spread between runs, so a claim within a few percent
  waits for a quiet machine.
- **The 3-times rule.** By the owner's rule, a competitor run that passes
  3 times TurbOWL's time may be stopped and is recorded as more than 3
  times slower. It was applied once, by hand: VLog on lubm-50 in the
  complete configuration, stopped after about 9 minutes.
- **Competitors on one thread.** Nemo has no thread option, and Soufflé's
  build runs on one thread. VLog's time is its own report, which leaves
  out the JVM's start, so every ratio against VLog favours VLog.

Total time (loading and reasoning), in the default configuration, median
of 3 runs. The last column is how many times longer the fastest other
engine takes than TurbOWL on one thread:

| Scenario | TurbOWL, one thread | TurbOWL, all cores | Fastest other | Ratio |
|---|---:|---:|---|---:|
| brick-small1 | 11.5 ms | 10.9 ms | Soufflé, 100 ms | 8.7× |
| brick-soda-hall | 16.1 ms | 15.7 ms | Soufflé, 200 ms | 12× |
| brick-buildings | 158 ms | 93.7 ms | VLog, 4.14 s | 26× |
| lubm-1 | 29.7 ms | 19.0 ms | Soufflé, 200 ms | 6.7× |
| lubm-10 | 380 ms | 170 ms | Soufflé, 2.63 s | 6.9× |
| lubm-50 | 2.02 s | 892 ms | Soufflé, 15.8 s | 7.8× |

On one thread, TurbOWL leads all 22 scenario and axis pairs where an
engine finished, in both configurations: reasoning time, total time, peak
memory, and time per change, which no competitor supports. The all-cores
runs are TurbOWL's alone. In the complete configuration (one run), the
fastest other engine takes 10.6 to 28 times as long as TurbOWL on one
thread, and VLog went over 16 GB on brick-buildings, lubm-1 and lubm-10.
Every engine that finished gave TurbOWL's closure size. An update costs 0.2 ms per added triple and
1.9 ms per removal in the default configuration.

What limits the numbers: loading is still most of TurbOWL's total on
lubm-50, 80% on one thread and 89% on all cores. Soufflé's time includes
writing the closure, and its program is compiled beforehand, untimed.

Phase 1's session (`docs/verification/2026-11-06-v2-phase-1.md`) is still
the only race at data size. It ran in the old defaults (`schema` and
`identity` on), on one thread, before phase 2's work. TurbOWL led all 46
scenario and axis pairs where some engine finished, and on uobm-010-u,
owl2bench-rl-5 and claros-l no other engine finished in 600 s and 16 GB.

### Against version 1

Version 1 at `f9e3625` is the Baseline (`docs/verification/2026-11-06-v1.md`,
measured at a load average of 1.1 to 2.3). Total time in the default
configuration, against phase 2's race:

| Scenario | Version 1 | Version 2, one thread | Version 2, all cores |
|---|---:|---:|---:|
| brick-small1 | 12.3 ms | 11.5 ms | 10.9 ms |
| brick-soda-hall | 17.6 ms | 16.1 ms | 15.7 ms |
| brick-buildings | 249 ms | 158 ms | 93.7 ms |
| lubm-1 | 77.5 ms | 29.7 ms | 19.0 ms |
| lubm-10 | 861 ms | 380 ms | 170 ms |
| lubm-50 | 4.66 s | 2.02 s | 892 ms |

On lubm-50, 5 runs each:

- **Loading:** 3.93 s in version 1; 1.52 s on one thread and 0.80 s on all
  cores.
- **Reasoning:** 735 ms in version 1; 393 ms on one thread and 102 ms on
  all cores.
- **Peak memory:** 150 bytes per closure triple in version 1; 46.6 on one
  thread and 53.0 on all cores.
- **A snapshot** of the reasoned state loads in 0.10 s, against 0.92 s to
  parse and reason (CLI wall time, all cores).

The improvement loop (`docs/specs/experiments.md`, iterations 1 to 11)
made most of the memory gain: windowed reading, relations sized to their
contents, fuller dictionary slots, and the base as pairs by predicate.

### Gaps

Two gaps the owner accepted on 2026-11-07:

- **The all-cores speed-up.** lubm-50 reasons 3.9 times as fast on all 12
  cores as on one (393 against 102 ms), against a budget of 5. Round 1
  nearly saturates the pool, 6 of whose 12 cores are Efficiency cores,
  and the same work costs about four times the CPU on all cores.
- **The all-cores memory.** lubm-50's peak on all cores is about 53 bytes
  per closure triple, against 50. One thread meets the budget at 46.6.
  Closing it needs a large structural change: sharing the base with the
  tables, or splitting the dictionary's IRIs.

Two gaps that wait for the owner:

- **The snapshot.** Loading lubm-50's snapshot is 9.2 times as fast as
  parsing and reasoning it on all cores, against 10, and about 19 times on
  one thread. Ticket 19 measured 15 times; parsing and reasoning got
  faster since.
- **Loading on one thread.** lubm-50 loads in 1.52 s on one thread,
  against a bar of 1.31 s (a third of version 1's 3.93 s). All cores meet
  it.

The candidates left are in the loop's "For the next agent" section.

## Working on it

`AGENTS.md` says how to build, test and verify, and where the work stands.
`CONTEXT.md` defines the words the project uses, and `docs/adr/` records
why it is built this way. Every full verification is in
`docs/verification/`, with its raw results.

## License

MIT or Apache-2.0, at your option.
