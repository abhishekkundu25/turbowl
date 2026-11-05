# turtle-reasoner: domain language

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
- **Profile**: which rules run. `owl-rl` is the W3C OWL 2 RL rule set this
  project implements; `reasonable` is the compatibility profile that
  reproduces the rule set and output conventions of `reasonable` (see
  ADR 0002).

## Results

- **Diagnostic**: a finding about the data, not an error of the program: an
  inconsistency (`OWLRL.CAX_DW`) or an ill-formed literal (`RDFS.DATATYPE`).
  It has a code, the rule, a severity, a message and structured fields (the
  terms involved).
- **Run**: one call that takes the reasoner from its current base triples to
  the fixpoint.
- **Incremental run**: a run after base triples were added or removed, which
  reuses the previous closure instead of starting over.

## Verification

- **Oracle**: an independent source of the right answer, run on the same
  input and compared as sets. There are several, each judging something
  different, and they are compared with each other too:
  - **`reasonable`** (a pinned release): parity in the `reasonable` profile,
    except for the documented differences.
  - **OWL-RL** (the Python RDFLib implementation of the W3C OWL 2 RL rule
    tables): closures in the `owl-rl` profile on small and medium inputs.
  - **Nemo** running an OWL 2 RL Datalog program: closures at scale, on a
    different engine.
  - **The W3C tests**: the OWL 2 RL test cases and the RDF Semantics
    entailment tests, the standard's own expected results.
  - **A DL reasoner** (HermiT or Konclude): soundness of inferred class and
    property subsumptions, which full OWL 2 DL must also entail.
  - **The reference evaluator** (below), for generated inputs.
- **Reference evaluator**: a deliberately simple, slow evaluator of the same
  rules (nested loops to a fixpoint), used for chaos runs.
- **Triangulation**: when two oracles agree with each other and not with this
  project, the bug is here; when one oracle disagrees with all the others, it
  is investigated and, if it is wrong, becomes a documented difference.
- **Documented difference**: a place where this project and the oracle
  disagree on purpose, because the oracle is wrong (see the parity research).
  Each one has an id (`D1`, `D2`, ...) and a filter in the oracle comparison.
- **Chaos run**: generated inputs from a seed, checked against the reference
  evaluator; a failure prints the seed to replay it.
- **Experiment**: one measured change after version 1, with a hypothesis, a
  result, and a decision (kept or reverted), recorded in
  `docs/specs/experiments.md`.
