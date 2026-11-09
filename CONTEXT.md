# TurbOWL: domain language

The words this project uses, so that code, specs, tickets and reviews say the
same thing. Architecture words (module, interface, depth, seam, adapter,
leverage, locality) follow the codebase-design vocabulary.

## Data

- **Term**: an IRI, a blank node or a literal, as RDF defines them.
- **Term id**: the `u32` the dictionary gives a term. Every relation, rule and
  index works on term ids; terms appear only at the edges (input, output,
  diagnostics).
- **Dictionary**: the two-way map between terms and term ids. Interning is
  idempotent: the same term always gets the same id.
- **Document**: one input (a file or a string). Blank-node labels are scoped
  per document: `_:b` in two documents is two different blank nodes.
- **Base triples**: the triples loaded from documents or inserted by a caller.
- **Inferred triples**: the triples the rules derive that are not base
  triples.
- **Closure**: base triples plus inferred triples, after a reasoning run
  reaches its fixpoint.

## Reasoning

- **Rule**: one OWL 2 RL or RDFS entailment rule, named as in the W3C OWL 2
  RL tables (`cax-sco`, `prp-dom`, `eq-trans`) or the RDF Semantics
  (`rdfs9`).
- **Rule family**: the rules that share a module: class (`cax-*`, `cls-*`),
  property (`prp-*`), equality (`eq-*`), schema (`scm-*`), datatype (`dt-*`,
  `rdfs` datatype checks).
- **Relation**: a sorted, duplicate-free set of term-id tuples. A
  **predicate table** is the relation of one predicate's (subject, object)
  pairs.
- **Delta**: the tuples a round of evaluation added that earlier rounds did
  not have. Semi-naive evaluation joins only against deltas.
- **Fixpoint**: the state where a round adds nothing.
- **Representative**: for a set of terms that `owl:sameAs` makes equal, the one
  term id that stands for all of them inside the engine. Output expands it
  back to every member.
- **Canon**: the union-find of term ids that says which terms are equal and
  which id represents each class. A merge rewrites the stored tuples into
  representatives and puts them in the deltas again.
- **Restart**: a run that starts again from the base, keeping the canon,
  because a merge changed what a rule constant (`rdf:type`, `rdf:nil`, ...)
  means. Rare: two vocabulary terms made equal, or `rdf:nil` made equal to
  another term.
- **Cancel**: a handle a caller trips, from any thread, to stop a run or a
  proof search at its next check (a run checks before each round, a search
  between its steps). A handle can also trip itself at a check when a
  function the caller gives says so, as a deadline does. A cancelled run
  keeps no partial result: the reasoner is as loaded and not yet reasoned.
- **Rule set**: the W3C OWL 2 RL rules TurbOWL implements. Version 1 has
  part of them (`docs/specs/v1.md`); version 2 has all of them.
- **Switch**: a named group of rules that a caller turns on or off. A group
  is off by default when other reasoners rarely implement it or when it
  slows reasoning down; everything else is on. Not a profile: in OWL 2, a
  profile is EL, QL or RL.
- **Configuration**: the switches that are on for a run. The **default
  configuration** has each switch at its default; the **complete
  configuration** has every switch on, which is the W3C rule set.
- **Thread mode**: one thread or all cores (`Threads::ONE`,
  `Threads::ALL`, the default). A round below a measured size runs on the
  caller's thread in either mode. The closure, the diagnostics and the
  order of every read are the same in both.

## Results

- **Diagnostic**: a finding about the data, not an error of the program: an
  inconsistency (`OWLRL.CAX_DW`) or an ill-formed literal (`RDFS.DATATYPE`).
  It has a code, the rule, a severity, a message and structured fields (the
  terms involved).
- **Run**: one call that takes the reasoner from its current base triples to
  the fixpoint.
- **Incremental run**: a run after base triples were added or removed, which
  reuses the previous closure instead of starting over.
- **Snapshot**: a reasoner's whole state (the dictionary, the base, the
  stored closure, the canon, the findings, the switches) in one versioned,
  checksummed file or byte string, loaded back without parsing or
  reasoning. One of another format or TurbOWL version is refused.
- **Counters**: what the last run did, per round and per step, and the
  bytes the reasoner holds, by part (`Reasoner::profile`, for the bench).
  The code calls them the profile; not an OWL 2 profile.

## Verification

- **Oracle**: an independent source of the right answer, run on the same
  input and compared as sets. There are several, each judging something
  different, and they are compared with each other too:
  - **Nemo** running the W3C forms of the rule set as a Datalog program
    (one file per switch group in `oracles/rules/`, composed for a
    configuration): exact closures at scale, on a different engine.
  - **OWL-RL** (the Python RDFLib implementation of the W3C OWL 2 RL rule
    tables): a superset closure on small and medium inputs, since it
    implements more rules.
  - **The W3C tests**: the OWL 2 RL test cases and the RDF Semantics
    entailment tests, the standard's own expected results.
  - **A DL reasoner** (HermiT): soundness of inferred class and
    property subsumptions, which full OWL 2 DL must also entail.
  - **The reference evaluator** (below), for generated inputs.
- **Reference evaluator**: a deliberately simple, slow evaluator of the same
  rules (nested loops to a fixpoint), used for chaos runs.
- **Triangulation**: when two oracles agree with each other and not with this
  project, the bug is here; when one oracle disagrees with all the others, it
  is investigated and, if it is wrong, becomes an oracle fault.
- **Oracle fault**: a place where an oracle departs from the W3C rules or
  the datatype definitions they rest on, shown against their text. Each
  one has an id (`F1`, `F2`, ...) and a test. It has a filter in the oracle
  comparison only where the oracle misses something: an oracle that
  derives more is invisible to containment.
- **Competitor**: another reasoner measured against this one on the same
  inputs, for speed and memory, never for correctness. Nemo is both an
  oracle and a competitor. A competitor's closure must have the same size
  as this project's; a difference that holds is a **competitor fault**,
  listed with a test.
- **Scenario**: one benchmark input and what is measured on it. An
  **editor-size** scenario is one of the bench's default runs (Brick with
  its models, the generated LUBM-style data, the incremental streams); a
  **data-size** scenario is a public dataset, run only when asked.
- **Scoreboard**: what the claim "the best OWL 2 RL reasoner" is measured
  on: the competitors, the scenarios, and the measures. Correctness and
  coverage are gates; then reasoning time, total time (loading plus
  reasoning), time per incremental change, and peak memory, single-threaded
  and on all cores.
- **Published result**: a competitor's number taken from its paper instead
  of measured here. It counts only on the same dataset, with the same fact
  counts before and after reasoning, scaled to this machine through a
  **bridge** (an engine that ran next to it in the paper and runs here
  too), and it is always labelled an estimate.
- **Baseline**: this project's own last kept version, which each experiment
  is compared against.
- **Chaos run**: generated inputs from a seed, checked against the reference
  evaluator, and on one thread against all cores; a failure prints the
  seed to replay it.
- **Experiment**: one measured change after version 1, with a hypothesis, a
  result, and a decision (kept or reverted), recorded in
  `docs/specs/experiments.md`.
