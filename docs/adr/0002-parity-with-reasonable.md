# ADR 0002: what parity with `reasonable` means

Status: superseded by ADR 0004, 2026-11-05.

## Context

Version 1 must match `reasonable` in function. The parity research
(`docs/research/reasonable-parity.md`) found that `reasonable` also has bugs
that change results (prp-ifp merging subjects whose objects differ, language
tags dropped on output, blank nodes merged across files, lists with a shared
tail merged, deletion resetting all state), behaviour outside OWL 2 RL (every
subject typed `owl:Thing`), and missing rules.

## Decision

- **Two profiles.** `reasonable` reproduces `reasonable`'s rule set and output
  conventions: only the rules it implements, `owl:Thing` typing of every
  subject, the same diagnostic codes, messages and order of fields. `owl-rl`
  is the W3C rule set and is the default. Version 1 implements the
  `reasonable` profile in full and builds `owl-rl` as the same rules minus the
  non-RL conventions; missing RL rules are added after version 1.
- **No bug is reproduced.** Where `reasonable` is wrong, both profiles do the
  right thing, and the oracle comparison lists the difference with an id
  (`D1`...) and a filter that removes exactly the affected triples or
  diagnostics from both sides before comparing. The list is in the v1 spec.
- **Interfaces match.** The CLI accepts `reasonable`'s flags with the same
  meaning, output formats and exit codes, except that usage errors exit 64
  (not 2, which stays `--fail-on`). The Python class offers `reasonable`'s
  methods with the same names and return shapes, and returns plain literals
  as rdflib does (not as `xsd:string`).
- **Parity is checked, not claimed.** The oracle suite runs the pinned
  `reasonable` release on the parity checklist's inputs and compares
  closures and diagnostics as sets.
- **`reasonable` is not the only judge of correctness.** It covers part of
  OWL 2 RL and has known bugs, so correctness is judged by several oracles
  (see `CONTEXT.md`): OWL-RL (Python), Nemo with an OWL 2 RL Datalog
  program, the W3C tests, a DL reasoner for soundness of subsumptions, and
  the reference evaluator. A documented difference from `reasonable` is
  accepted only when the other oracles agree with this project.

## Consequences

- Every documented difference needs a test showing this project's behaviour
  is the W3C one.
- Byte-for-byte output equality is not a goal; set equality of the closure
  is.
