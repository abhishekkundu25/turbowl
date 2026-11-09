# Explanations: why a triple is inferred

Status: planned 2026-11-07, branch `explain`. Tickets: `docs/tickets/explain/`
(01 to 04). Background: `docs/research/explanations-2026-11.md`.

## Problem Statement

TurbOWL says *what* follows from a graph and *that* a graph contradicts
itself, never *why*. A user who sees `ex:alice a ex:Person` in the closure,
or a `cax-dw` error on `ex:bob`, has to work out by hand which of their
triples cause it. Nemo (tracing), Soufflé (provenance), RDFox (explain),
GraphDB and Stardog can answer; VLog, reasonable and owlrl cannot. Editors
need it most: turtle-lsp would show it on hover.

## Solution

An explanation is a **proof**: the goal, the W3C rule that concludes it,
and a proof of each premise, down to base triples. It is computed **on
demand, after reasoning**, from the closure of the last run. Nothing is
recorded while reasoning runs, so reasoning costs exactly what it costs
today.

- `Reasoner::explainer()` gives an `Explainer` over the last run's closure.
  `Explainer::explain` proves a triple, and `Explainer::explain_diagnostic`
  proves a diagnostic. `Reasoner::explain` and
  `Reasoner::explain_diagnostic` are one-call forms.
- The explainer runs the **declarative rule program** (`oracles/rules/`, the
  rules Nemo, Soufflé and VLog run, which every oracle holds equal to
  TurbOWL's) **backwards** over the closure. It finds the facts relevant to
  the goal, ranks them by derivation depth from the base, and keeps for
  each fact a rule instance whose premises all rank lower. So every proof
  is acyclic, grounded in base triples, and as shallow as the closure
  allows.
- `owl:sameAs`: the store keeps one representative per equality class. The
  explainer reads the classes from the canon and rebuilds the equality
  chain a proof needs.
- The CLI (`--explain`) and Python (`Reasoner.explain`) expose it.

## User Stories

1. As an ontology author, I want to ask why a triple is inferred and get the base triples that cause it, so that I can find the line to change.
2. As an ontology author, I want each step of a proof to name its W3C rule, so that I can see which axiom does the work.
3. As an ontology author, I want to ask why a diagnostic was reported and get the triples behind it, so that I can fix the contradiction.
4. As an editor author (turtle-lsp), I want a proof's leaves as base triples, so that I can link each one to its line in a file.
5. As an editor author, I want an explanation in about a millisecond on an editor-size workspace, so that hover feels instant.
6. As an editor author, I want to reuse one explainer for many hovers, so that its lookups are paid once.
7. As a data engineer, I want explanations on data-size closures (lubm-50, claros-l) in under 50 ms, so that I can debug a pipeline's output.
8. As a user, I want a triple equal to another through `owl:sameAs` explained with its chain of equalities, so that I see why two names are one.
9. As a user, I want the proof of a base triple to say it is base, so that asking about any closure triple gives an answer.
10. As a user, I want a triple outside the closure to give no proof, never a made-up one.
11. As a user, I want short proofs, so that I can read them.
12. As a user, I want a proof to share a subproof used twice, so that it does not repeat itself.
13. As a user, I want the proof written as an indented tree, so that I can read it in a terminal.
14. As a CLI user, I want `turbowl --explain '<s> <p> <o>' data.ttl` to print the proof, so that I can check a model without code.
15. As a CI user, I want proofs in the JSON report, with each diagnostic's proof on request, so that a failing check names its causes.
16. As a Python user, I want `Reasoner.explain(s, p, o)` returning a proof object with its steps and leaves, so that I can inspect it in a notebook.
17. As a user of switches, I want proofs to use only the rules the switches leave on, so that a proof never cites a rule that did not run.
18. As a user, I want lists (`owl:intersectionOf`, `owl:hasKey`, `owl:propertyChainAxiom`, `owl:AllDifferent`) shown as the list triples a step reads, not as helper relations.
19. As a user, I want explaining to leave the reasoner unchanged, so that I can explain and keep editing.
20. As the maintainer, I want reasoning speed and memory unchanged by this feature, so that the scoreboard claim stands.
21. As a maintainer, I want every proof checked by code that shares nothing with the explainer, so that a wrong proof fails a test.
22. As a maintainer, I want chaos to explain triples and diagnostics of every input and check each proof, so that a missing or wrong proof is found.
23. As a maintainer, I want every inferred triple of every minimal input explained and checked, so that each W3C rule has a proof test.
24. As a maintainer, I want explain latency on the bench, so that its budgets are measured, not guessed.

## Implementation Decisions

- **Public interface** (`crates/turbowl/src/lib.rs`, the crate doc says the
  adapters need it):
  - `Reasoner::explainer(&self) -> Result<Explainer<'_>, Error>`. An error
    when the base changed since the last run (`Error::NotReasoned`, a new
    variant): proofs are of the last run's closure, and there is none to
    read until `reason` runs.
  - `ExplainCache`: the explainer's lazy indexes, owned and `Send + Sync`
    (`OnceLock`), so an adapter can keep it between calls.
    `Reasoner::explainer_with(&self, ExplainCache)` reuses one, and
    `Explainer::into_cache()` gives it back. Python keeps one in its native
    object and drops it on any change.
  - `Explainer::explain(&self, triple: TripleRef) -> Option<Proof>` and
    `Explainer::explain_diagnostic(&self, &Diagnostic) -> Option<Proof>`;
    `None` when the triple is not in the closure (or the diagnostic not
    reported). `Reasoner::explain` and `Reasoner::explain_diagnostic` build
    an explainer and call it.
  - `Proof`: a DAG of steps in topological order, the goal last. A step is
    a `Statement` (three `Term`s: a generalized triple, since a proof may
    pass through a literal subject) or, for the root only, a diagnostic,
    with a `Reason`: `Base`, or `Rule { rule: &'static str, premises }`.
    A rule step may have no premises: cls-thing, cls-nothing1, prp-ap and
    dt-type1 hold of every closure.
    `Proof::leaves()` gives the base triples, `Proof::depth()`, and
    `Display` writes the indented tree. The types make a non-root
    diagnostic unrepresentable.
- **Nothing in reasoning changes.** `engine/` and `rules/` get at most
  visibility changes (`pub(crate)`) and read-only accessors; no logic, no new
  field filled while loading or reasoning. The `Reasoner` struct gains no
  field; `reasoner.rs` gains `pub(crate)` accessors (store, dictionary,
  base, findings, `same_as_itself`, run state). A loaded document's blank
  nodes, as the reads write them (`b_1`), are found again by the explainer's
  term lookup. The explainer's indexes and caches live in the `Explainer` and
  are built lazily, on first use.
- **The rule program** is read from `oracles/rules/*.rls` with
  `include_str!` and parsed by a small parser in the explain module (atoms,
  prefixes, `!=`, the `@export` lines that name finding relations, the
  `literalValue` and `literalType` imports). The groups the switches leave
  on are used. The parser rewrites `Tp(p, x, y)` to `T(x, p, y)` and drops
  the `usedProp` rules, which only steer Nemo's joins (`core.rls`), so no
  unrelated schema triple becomes a premise. Before
  the crate is published, the program moves inside the crate (a follow-up).
- **Backward search.** From the goal, find every rule instance whose head
  matches and whose body holds in the closure, recursively, giving the
  goal's relevant facts; if that set explodes (transitive components,
  large classes), bound it by iterative deepening on rank. Helper relations
  (`chainFrom`, `keyCell`, `setBefore`, `maxOneOf`, ...) are derived on demand, as goals
  of their own. Then rank the relevant facts forward from the base (a base
  fact is 0, an instance is one more than its highest premise), and build
  the proof from minimum-rank instances. A proof step is shown only for a
  `T` fact or the goal; a helper relation's subproof is folded into the
  step that reads it, so its premises are the triples, list cells included.
- **What the program does not say** comes from TurbOWL's own datatype code,
  read-only: dt-not-type, left out of the program, and the literal facts
  the oracle computes (`literalValue`, `literalType`). The literals the
  oracle names as IRIs (`NAMED_LITERALS`) map back to their literals.
  - dt-eq and dt-type2 cite one closure triple that names the literal.
  - dt-not-type cites a closure triple that names the ill-typed literal, or,
    for a literal typed by a triple, that triple's proof.
  - dt-diff, which the core stores nowhere, has one shape: a generalized
    dt-diff step with the reference's unlisted predicate, then eq-rep
    steps, then eq-diff1.
- **Equality.** A fact about non-representative terms holds through its
  representative. Its proof is the eq-rep steps from a fact as stated,
  plus the sameAs chain, built as a shortest path over the class's
  sameAs-concluding rule instances (eq-sym, eq-trans as path edges; prp-fp,
  prp-ifp, prp-key, cls-maxc2, cls-maxqc3, cls-maxqc4, dt-eq as the edges
  themselves). Ticket 01 may run the eq-* rules generically; ticket 02
  makes classes fast. Ticket 01 covers configurations with `equality` and
  `literal-values` off; ticket 02 adds them.
- **The proof checker** lives in the reference evaluator
  (`turbowl_reference::check_proof`). For each step it runs that rule's
  family for one round over the step's premises alone, recording every
  (fact, rule) pair, and requires the step's conclusion under that rule's
  name. Two rules can share a premise set (prp-inv1 and prp-inv2 on
  `p owl:inverseOf p`), so either rule passes. It also requires that the leaves are in the base
  and the steps are acyclic. It shares no code with the explainer or the
  core's rules.

## Testing Decisions

- **Proof checks on the minimal inputs** (`crates/oracle/src/cases/`): every
  inferred triple and every diagnostic of every case, default and complete
  configurations, explained and checked. A new test file,
  `crates/oracle/tests/explain.rs`, next to `minimal.rs`.
- **Chaos** (`crates/reference/tests/chaos.rs`): each iteration explains up
  to 32 inferred triples and 32 diagnostics of its final state (the full
  test's run, the incremental test's last state), sampled from their own
  `Rng` stream so inputs replay unchanged, with one explainer per state,
  and checks each proof. A missing proof fails as a disagreement does,
  with the replay line. Chaos draws random configurations, which the
  oracles never compared with the program; a missing proof there is a
  program or explainer bug, fixed with a minimal case.
- **Unit tests** in `crates/turbowl/src/explain/tests.rs`: the parser, the
  ranks, folding of helper relations, a base triple, a triple outside the
  closure, a cycle in the closure (`p` symmetric) that must still give an
  acyclic proof, `NotReasoned`, a blank node from `load_str`, a reused
  `ExplainCache`, a reasoner restored from a snapshot, and (02) a sameAs
  chain of three hops.
- **Adapters**: `crates/cli/tests/cli.rs` (text and JSON output, a triple
  not in the closure, a malformed `--explain` value exits 64) and
  `crates/python/tests/test_explain.py`.
- **The bench**: `turbowl-bench --explain` explains a seeded sample of
  1,000 inferred triples and up to 1,000 diagnostics per scenario, and
  reports median, 99th percentile and max latency with a warm cache, the
  cold first-use time on its own, and the bytes the cache holds (the
  process peak cannot separate it from reasoning).
- **Scaling**: one test (`scaling_explain`) that explaining one triple
  grows at most n log n with the closure, as `crates/turbowl/src/scaling.rs`
  does for the rest.

## Non-functional budgets (pass/fail)

- **Reasoning unchanged**: on the editor-size scenarios and lubm-50, in the
  default and the complete configurations, one thread and all cores,
  TurbOWL's load, reasoning and total times and peak memory are within
  the spread of `main`'s, measured alternately (5 runs per side; all-cores
  peak memory on 10 single runs per side, as AGENTS.md requires); and
  `engine/` and `rules/` show no change but visibility in `git diff main`.
- Explaining one inferred triple on an editor-size scenario (Brick with
  small1 and soda hall, lubm-1): median under 1 ms, 99th percentile under
  10 ms.
- On lubm-50 and claros-l: 99th percentile under 50 ms. On sameas-clique:
  99th percentile under 50 ms.
- Every proof the tests produce passes the checker; none missing.
- Chaos: 100,000 iterations of each kind (two shards of 50,000 each) pass,
  each shard at most 10% slower than at phase 2's close-out.
- fmt, clippy, the WebAssembly build and every oracle pass.

## Out of Scope

- All proofs, or minimal sets of axioms (Protégé-style justifications).
  One shallow proof per question; justifications are a later feature.
- Recording provenance while reasoning.
- turtle-lsp integration: that repo's own spec, once this lands.
- Explaining triples that are not in the closure ("why not?").
- Publishing anything.
- Moving the rule program inside the crate (needed before publishing; the
  golden programs name `oracles/rules/` paths): a follow-up ticket.

## Principles

- **Foundational thinking**: the proof types (`Proof`, `Statement`,
  `Reason`) and the parsed rule program come first in ticket 01; the
  search and every adapter read them.
- **Model the domain**: a proof is a DAG with typed reasons, not a string;
  the rule program is data the search walks, not 78 hand-written backward
  rules.
- **Type system discipline**: `Reason::Base` against `Reason::Rule`, the
  root-only diagnostic, `Error::NotReasoned` instead of a stale answer.
- **Boundary discipline**: the CLI parses `--explain`'s triple once, as
  N-Triples, and Python converts its terms once; the explainer trusts its
  input.
- **Explain the number**: each latency names the scenario, the sample, the
  spread and what bounds it (search or lookups); "reasoning unchanged" is
  shown by alternating runs, not one.
- **Build the lever**: the proof checker is the lever. Every proof any
  test produces goes through it, so correctness is one rerunnable check.
- **Experience first**: the `Display` tree and the CLI output are written
  for a person reading a terminal; proofs fold helper relations away.
