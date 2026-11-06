# TurbOWL

TurbOWL is an OWL 2 RL reasoner in Rust. It computes the closure that the
W3C OWL 2 RL rule tables define (OWL 2 Profiles §4.3, Tables 4 to 9) and
reports the contradictions and ill-typed literals they define. It is one
library with a small interface, a command line, and Python bindings that
take and return rdflib graphs. The core builds for WebAssembly. A run
after a change reuses the previous closure.

It is judged by independent oracles (Nemo, OWL-RL, the W3C tests, HermiT
and a reference evaluator), never by one reasoner.

**Status.** This is the `v2` branch. Version 1 (`0.1.0`, `main` at
`f9e3625`) implements 39 of the 78 rules. Version 2's phase 1 adds the
other 39, the switches and the scoreboard; it is closed, with the gaps
listed below. Phase 2, speed, is next. Nothing is published: there is no
crate on crates.io and no package on PyPI. See `CHANGELOG.md`.

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
```

Each file's format comes from its extension (`.ttl`, `.nt`, `.n3`, `.rdf`,
`.owl`, `.xml`). The report is the diagnostics, then a summary, as text,
JSON or NDJSON. The exit status is 0 on success, 1 on an input or output
error, 2 on a diagnostic at or above `--fail-on` (default `error`), and 64
on a usage error. `turbowl --help` gives the details, and the JSON field
names are stable.

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
```

The package's documentation is `help(turbowl)`.

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
  seed.

Phase 1's results, in the default and the complete configuration
(`docs/verification/2026-11-06-v2-phase-1.md`):

| Judge | Result |
|---|---|
| Minimal inputs, one or more per rule | Equal to Nemo, contained in OWL-RL after F1 to F15, equal to the reference evaluator |
| W3C OWL 2 RL (complete configuration) | 101 of 119 runnable cases, every case Nemo passes (Nemo 101, OWL-RL 100) |
| W3C RDF-MT (complete configuration) | 35; no consistent case reported inconsistent |
| Brick with its models, and the 108 building models | Equal to Nemo, closures and findings |
| 100,000 full chaos runs | 98,942 compared, all equal; 1,058 too large for the reference evaluator |
| 100,000 incremental chaos runs | Found two inputs past the 30 s watchdog, one here (prp-key on a branching key list) and one in the reference evaluator; both fixed |

The final code, with the new defaults, is verified again: 100,000 chaos
iterations of each kind and the slow oracle tier. The record's "Final
code" section holds the result.

The 18 W3C OWL 2 RL cases that fail, fail on Nemo and OWL-RL too. Fifteen
expect a conclusion no OWL 2 RL rule derives: 8 hold by refutation (a
difference or a complement type that no rule concludes), 3 need reasoning
about datatypes, 1 needs a new term, and 3 need a rule the tables do not
have. The other three depend on how the test runner reads a case: an
import, and annotations under the Direct Semantics. The record gives the
reason for each.

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
checked against TurbOWL's. RDFox cannot be run here and counts only through
its published results. Phase 1's session (`scoreboard.txt` in
`docs/verification/data/2026-11-06-v2-phase-1/`) has these caveats:

- **The old defaults.** The competitors ran before the defaults were
  decided, with `schema` and `identity` on. The new default (both off) has
  no competitor runs.
- **A busy machine.** The 1-minute load average was 1.8 to 2.9, by the
  owner's choice. Each gap below is many times the spread between runs,
  so a claim within a few percent waits for a quiet machine.
- **Gaps.** The owner stopped the session after 4 hours. The complete
  configuration did not run on claros-l, uobm-010-u, owl2bench-rl-5 and
  lubm-uba-100-l. No engine runs on all cores yet. No RDFox estimate meets
  the bridge's conditions. VLog's closure on owl2bench-rl-1 differed in one
  run of three. The 16 GB cap overshoots by up to 314 MB before it stops a
  run.

One thread, total time (loading and reasoning), median of 5 runs at editor
size and 3 or 1 at data size. The last column is how many times longer
the fastest other engine takes:

| Scenario | TurbOWL | Fastest other | Ratio |
|---|---:|---|---:|
| brick-small1 | 13.4 ms | Soufflé, 149 ms | 11× |
| brick-buildings | 265 ms | VLog, 4.52 s | 17× |
| lubm-1 | 81.8 ms | Soufflé, 208 ms | 2.5× |
| lubm-50 | 4.90 s | Soufflé, 15.6 s | 3.2× |
| uobm-010-l | 1.97 s | Soufflé, 10.1 s | 5.1× |
| owl2bench-rl-1 | 205 ms | Soufflé, 44.1 s | 215× |
| owl2bench-rl-2 | 687 ms | Soufflé, 354 s | 516× |
| uobm-010-u, owl2bench-rl-5, claros-l (1 run) | 4.31 s, 7.64 s, 36.4 s | none finished in 600 s and 16 GB | |

In the old defaults, TurbOWL leads all 46 scenario and axis pairs where
some engine finished: reasoning time, total time, time per incremental
change and peak memory. In the complete configuration at editor size it
leads all 22, and the fastest other engine takes 6.0 to 16 times as long.

What limits the numbers: on LUBM, loading is 79% of TurbOWL's total on
lubm-50, which is where the others come closest (2.5 to 3.2 times) and
where phase 2's parser work aims. Soufflé's time includes writing the
closure, and its program is compiled beforehand, untimed. VLog's time
varies by up to 168% between runs, and it is VLog's own report, which
leaves out the JVM's start and the export: its wall time is 1.1 to 18
times as long, so every ratio against VLog favours VLog. The record has every cell, its range and
the leader per scenario and axis.

### Against version 1

Version 1 at `f9e3625` is the Baseline (`docs/verification/2026-11-06-v1.md`).
On the editor-size scenarios, version 2's reasoning time is 0% to 7%
above version 1's, and the two changes over 5% lie inside the runs'
spread. That run had `schema` off and `identity` on, which costs nothing
measurable there. It meets the phase 1 budget on this machine, but not
yet on a quiet one.

## Working on it

`AGENTS.md` says how to build, test and verify, and where the work stands.
`CONTEXT.md` defines the words the project uses, and `docs/adr/` records
why it is built this way. Every full verification is in
`docs/verification/`, with its raw results.

## License

MIT or Apache-2.0, at your option.
