# Version 1: a Rust OWL 2 RL reasoner with parity to `reasonable`

Status: in progress. Tickets: `docs/tickets/v1/`. Research:
`docs/research/reasonable-parity.md` (what `reasonable` does, at `fecee2b`),
turtle-lsp's `docs/research/reasoner-sota-2026-11.md` (the field). Decisions:
`docs/adr/`. Words: `CONTEXT.md`.

## Problem Statement

People who want OWL 2 RL reasoning in an editor, a browser, a Rust or Python
program, or (later) Protégé have no open, actively developed engine that is
fast, embeddable, builds for WebAssembly, and keeps its results current as
data changes. `reasonable` comes closest: Rust, OWL 2 RL, diagnostics,
Python bindings. But it is slow on real models (its own log shows 18.7 s for
Brick, about 15,000 triples), it implements part of OWL 2 RL, it has bugs
that change results, it throws away all state on any deletion, and it is
verified by hand-picked unit tests only.

## Solution

A new reasoner, in Rust, in its own repository. Version 1 does everything
`reasonable` does, with the same command line, the same Python methods and
the same diagnostics, and gives the same closure on the same input, except
where `reasonable` is wrong. It is built for speed from the start (term ids,
sorted relations, semi-naive evaluation, `owl:sameAs` by representatives),
and its correctness is judged by several independent oracles, not one.
Version 1 is the baseline for a series of measured experiments
(`docs/specs/experiments.md`).

## User Stories

Reasoning
1. As a modeller, I want to load Turtle, N-Triples, N3 and RDF/XML files and get the OWL 2 RL closure, so that I see what my model implies.
2. As a modeller, I want each rule `reasonable` implements to give the same inferences here, so that I can switch without surprises.
3. As a modeller, I want `owl:sameAs` handled without the closure blowing up, so that models with many aliases stay fast.
4. As a modeller, I want property chains, inverse, symmetric and transitive properties, domains and ranges, so that property axioms take effect.
5. As a modeller, I want class intersections, unions, someValuesFrom, allValuesFrom and hasValue, so that class definitions classify my data.
6. As a modeller, I want container membership (`rdf:_n`) handled as `reasonable` does, so that containers behave the same.
7. As a modeller, I want blank nodes in different files kept apart, so that unrelated `_:b` nodes are not merged.
8. As a modeller, I want language tags kept on output, so that my labels survive a round trip.
9. As a modeller, I want to choose the `owl-rl` profile (W3C rules only) or the `reasonable` profile (its conventions, such as typing every subject `owl:Thing`), so that I get standard output or a drop-in replacement.

Diagnostics
10. As a modeller, I want an individual in two disjoint classes reported (`OWLRL.CAX_DW`), so that I find contradictions.
11. As a modeller, I want disjoint properties, asymmetric and irreflexive properties, and members of `owl:Nothing` reported, so that every inconsistency `reasonable` finds is found here.
12. As a modeller, I want `owl:complementOf` violations reported (`OWLRL.CLS_COM`), which `reasonable` documents but does not emit.
13. As a modeller, I want ill-formed literals (`RDFS.DATATYPE`) and literals outside a property's datatype range (`RDFS.DATATYPE_RANGE`) reported, so that data errors show.
14. As a tool author, I want each diagnostic to carry its code, rule, severity, message and the terms involved, so that I can link it to the data.

Incremental
15. As an editor author, I want to add triples and run again without starting over, so that keystrokes stay cheap.
16. As an editor author, I want removing triples to give the right closure, so that deleted facts stop implying things.
17. As an editor author, I want an unchanged graph to cost nothing when I set it again, so that a refresh is free.

Interfaces
18. As a Rust developer, I want a small library interface (load, insert, remove, run, triples, diagnostics, write), so that embedding takes minutes.
19. As a command-line user, I want `reasonable`'s flags (`-o`, `--error-format text|json|ndjson`, `--fail-on`, `--max-diagnostics`, `--summary-only`) with the same meaning and exit codes, so that my scripts keep working.
20. As a Python user, I want `reasonable`'s `PyReasoner` methods (`load_file`, `from_graph`, `reason`, `update_graph`, ...) with rdflib terms in and out, so that `brickschema` and my notebooks can switch.
21. As a browser developer, I want the core to build for `wasm32-unknown-unknown`, so that turtle-lsp can embed it later.

Verification and performance
22. As a maintainer, I want every parity claim checked against `reasonable`, OWL-RL, Nemo, the W3C tests, a DL reasoner and a reference evaluator, so that correctness does not rest on one implementation.
23. As a maintainer, I want generated inputs checked against the reference evaluator with a replayable seed, so that rare cases are found.
24. As a maintainer, I want a benchmark that runs this reasoner and `reasonable` on the same inputs and records times and memory, so that speed claims carry numbers.
25. As a maintainer, I want every verification run stored in the repository with its date, inputs, versions and results, so that results can be audited and compared later.
26. As a future agent, I want one entry point (`AGENTS.md`) that says how to build, test, verify and continue, so that I can pick the work up cold.

## Implementation Decisions

- **One deep module.** The core crate's public interface is the `Reasoner`
  (create with a profile; load a document; insert or remove triples; set the
  base triples; run; iterate the closure or only inferred triples; read
  diagnostics; write a document). Terms in and out are `oxrdf` types. Term
  ids, the dictionary, relations, the engine and the rule families are
  internal. Every adapter (CLI, Python, the oracle tests, the benchmark)
  uses only this interface.
- **Term ids and the dictionary.** Every term is interned to a `u32`. Blank
  nodes are scoped per document. Literals keep their datatype and language
  tag.
- **Input and output** use Oxigraph's parsers and serializers (`oxttl`,
  `oxrdfxml`, `oxrdf`): Turtle, N-Triples, N3 and RDF/XML in; Turtle and
  N-Triples out. `.n3` is parsed as N3 (a superset of the N-Triples that
  `reasonable` reads).
- **Engine** (ADR 0001): sorted, duplicate-free relations of term ids;
  semi-naive rounds with stable, recent and to-add parts; merge join,
  antijoin and leapjoin; predicate tables for the vocabulary the rules use.
- **Rules** are Rust code, one module per rule family, registered in one
  table. The `reasonable` profile runs exactly `reasonable`'s 27 deriving
  rules, its 5 diagnostic-only rules, and its `owl:Thing` typing; `owl-rl`
  runs the same rules without the `owl:Thing` typing (more RL rules come
  after version 1).
- **Equality** (ADR 0003): union-find representatives inside; output
  expands them.
- **Incremental runs**: additions are joined semi-naively against the
  stored closure. Any removal re-runs from the remaining base triples (as
  `reasonable` does), and setting an unchanged base costs nothing (unlike
  `reasonable`). Better deletion is an experiment.
- **Parity** (ADR 0002): the profiles, the interfaces, and the documented
  differences below.

### Documented differences from `reasonable`

| Id | `reasonable` | Here | Oracle filter |
|---|---|---|---|
| D1 | prp-ifp makes subjects equal whenever they share the property | only when the objects are equal (W3C) | drop `sameAs` and eq-rep triples from inputs with an `owl:InverseFunctionalProperty` and compare those against OWL-RL instead |
| D2 | `cls-com` never fires | `OWLRL.CLS_COM` fires | drop `CLS_COM` from our diagnostics |
| D3 | language tags dropped on output | kept | compare in memory, not via files |
| D4 | blank-node labels shared across files | scoped per document | inputs with blank nodes come from one file |
| D5 | lists with a shared tail merged | lists read structurally | drop inputs with shared tails, compare those against OWL-RL |
| D6 | literal-subject triples derived, then dropped with a log line | never derived | none needed (absent on both sides) |
| D7 | a second `update_graph` with an unchanged graph re-runs everything | no work | compare closures only |
| D8 | panics on I/O and parse errors; usage errors exit 2 | errors returned; usage errors exit 64 | CLI tests per exit code |
| D9 | JSON diagnostics leave newlines unescaped | valid JSON | parse before comparing |
| D10 | Python returns plain literals typed `xsd:string` | plain literals as rdflib makes them | compare as rdflib terms |

Each difference has a test showing that the other oracles agree with this
project.

## Testing Decisions

- **The interface is the test surface.** Every suite drives the `Reasoner`
  (or the CLI, or Python) and compares closures and diagnostics as sets.
  Unit tests inside the engine are the exception: the engine has its own
  internal seam and its own comparison with a naive evaluator.
- **Oracle suite** (`crates/oracle`): runs `reasonable` (pinned crates.io
  release) in process, OWL-RL through `uv run` in a project-local
  environment, Nemo and the DL reasoner as pinned binaries under `tools/`
  (gitignored, fetched by `scripts/fetch-tools.sh`), on the parity
  checklist's inputs (`docs/research/reasonable-parity.md`, items 1 to 22),
  `reasonable`'s own unit-test inputs, Brick and the building models
  (fetched by `scripts/fetch-data.sh` into `data/`, gitignored), and the
  W3C test archives. An oracle that is not installed is reported as skipped;
  with `CI=true` it fails (as turtle-lsp's corpus tests do).
- **Rule tests**: one minimal input per rule and per diagnostic, with the
  expected inferences written out.
- **Chaos** (as turtle-lsp's `tests/chaos.rs`): generated ontologies and data
  from a seed, compared with the reference evaluator; `CHAOS_SEED`,
  `CHAOS_ITERS` and `CHAOS_ONLY` replay a case; a failure saves the input.
- **Incremental**: for generated inputs, every split into batches of
  additions, and every removal, gives the same closure as one full run.
- **Benchmark** (`crates/bench`, as turtle-lsp's `bench/perf.mjs`): fixed
  scenarios (Brick variants, the building models, a generated LUBM-style
  university set at several sizes, an incremental edit stream), each run
  for this reasoner and for `reasonable`, recording median, spread, peak
  memory and the closure size; `--compare` diffs two result files.
- **Stored verification**: each full verification writes
  `docs/verification/<date>-<name>.md` (what ran, versions, counts, numbers,
  failures) and the raw results under `docs/verification/data/`.

## Principles

- **foundational-thinking, model-the-domain**: term ids, the dictionary and
  relations first; one rule table, one module per rule family.
- **type-system-discipline**: newtypes for term ids and predicates; no
  stringly typed rules.
- **boundary-discipline**: parsing, the CLI and Python validate; the engine
  and rules trust term ids.
- **exhaust-the-design-space**: the engine choice (ADR 0001) weighed three
  bases.
- **make-operations-idempotent**: loading the same document twice, setting
  the same base twice and running twice change nothing.
- **encode-lessons-in-structure**: every documented difference is a filter
  with a test; every oracle runs in the suite, not by hand.
- **explain-the-number**: every benchmark number records its run count,
  spread and what bounds it.
- **separate-before-serializing-shared-state**: slices own disjoint files;
  the rule table is the one lock file, edited one line per rule family.

## Non-functional budgets (pass/fail)

- Every parity checklist item passes against `reasonable`, after the
  documented-difference filters.
- OWL-RL and Nemo agree with the `owl-rl` profile's closure on every input
  they finish in 10 minutes.
- Every W3C OWL 2 RL positive entailment test whose rules version 1
  implements passes; every one it does not implement is listed by name.
- The DL reasoner entails every class and property subsumption inferred on
  Brick.
- 100,000 chaos iterations pass.
- The full-run time is below `reasonable`'s on every benchmark scenario on
  this machine; the ratio is recorded.
- Peak memory is below `reasonable`'s on every benchmark scenario.
- An incremental addition of one triple to Brick plus a building model costs
  under 10% of a full run.
- The core crate builds for `wasm32-unknown-unknown` with default features
  off.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are
  clean.

## Out of Scope

- OWL 2 RL rules `reasonable` does not implement (the rest of `scm-*`,
  `prp-key`, `cls-maxc*`, `eq-diff*`, `dt-*`): after version 1, as
  experiments.
- Deletion without a full re-run (DRed, counting, backward/forward): an
  experiment.
- Parallel rounds, columnar storage, worst-case-optimal joins: experiments.
- EL classification, a Protégé plugin, an OWL API adapter, turtle-lsp
  integration, a published WebAssembly package.
- Explanations (why a triple holds).
- Publishing (crates.io, PyPI, GitHub): only with the owner's approval.
