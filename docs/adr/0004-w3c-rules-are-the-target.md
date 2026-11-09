# ADR 0004: the W3C rules are the target; `reasonable` is a speed baseline

Status: accepted, 2026-11-05. Supersedes ADR 0002.

## Context

ADR 0002 made parity with `reasonable` the goal of version 1: a
`reasonable` profile, its CLI flags and Python methods, and ten documented
differences (D1 to D10) filtered out of every comparison. Wave 2 built the
oracles and found that `reasonable` is a weak judge of correctness:

- It reports `RDFS.DATATYPE_RANGE` for every IRI or blank object of a
  property with a class range (`term_in_datatype_space`,
  `lib/src/reasoner.rs:2397`): thousands of false findings on Brick.
- It keeps one intersection or union list per class, so `cls-int1` and
  `cls-uni` miss the others.
- D6 and D5 are wider than their rows said: literal subjects have visible
  effects through `owl:sameAs` and diagnostics, and derived list triples
  are ignored.

Each of these would need another filter, and the reference evaluator had
started copying a bug to stay equal. Meanwhile the oracles that implement
the W3C rules agree with each other: Nemo running the W3C rule forms
(`oracles/nemo/owl2rl.rls`) matches `reasonable` everywhere except
`reasonable`'s faults, OWL-RL contains Nemo's closure apart from four
understood OWL-RL faults, and HermiT entails every inferred subsumption on
Brick.

The maintainer's goal is performance. Compatibility with `reasonable` costs
scope (a second profile, its message texts, its CLI and Python shapes) and
buys nothing toward it.

## Decision

- **Correctness is the W3C OWL 2 RL rule tables**, for the rules version 1
  implements (listed in `docs/specs/v1.md`). There is one profile; the
  `Profile` type leaves the public interface.
- **The judges** are Nemo running the W3C rule forms (exact closure
  equality, at scale), the W3C OWL 2 RL and RDF Semantics test suites,
  HermiT (soundness of inferred subsumptions), OWL-RL (containment: it
  implements more rules), and the reference evaluator (chaos). Where one
  judge is wrong, the fault is listed with a test and a filter, as before,
  but only for oracle faults; there are no differences from a target.
- **`reasonable` is a speed and memory baseline only**: it runs in the
  benchmark, never in a correctness test. Nemo joins it in the benchmark.
- **No compatibility.** No `owl:Thing` typing of every subject, no
  container-membership axioms (neither is an OWL 2 RL rule), our own
  diagnostic messages, our own CLI flags and Python API.

## Consequences

- D1 to D10 disappear; the oracle crate keeps a short list of oracle faults
  (OWL-RL's).
- The reference evaluator implements the W3C rules and is itself checked
  against Nemo.
- The v1 budgets compare speed and memory with `reasonable` and record the
  ratio to Nemo.
- `docs/research/reasonable-parity.md` stays as history and as a map of
  `reasonable` for the benchmark.
