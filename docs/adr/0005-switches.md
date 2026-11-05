# ADR 0005: switches turn groups of rules on and off

Status: accepted, 2026-11-05. Amends ADR 0004 ("there is one profile").

## Context

ADR 0004 removed the `reasonable` profile: one rule set, no configuration.
That kept verification simple, but version 2 adds every rule of the W3C
OWL 2 RL tables (`docs/specs/v2-1-scoreboard-and-rules.md`), and some of
them cost a lot for little:

- eq-ref adds `x owl:sameAs x` for every term, scm-cls four trivial triples
  per class, dt-type2 a type for every literal.
- dt-diff adds an `owl:differentFrom` triple for every pair of literals with
  different values, which grows with the square of the literals.
- Other reasoners leave several groups out (OWL-RL's output on an empty
  graph is 106 triples, this reasoner's 2).

Users also asked to trade inference for speed: a caller who never needs
`owl:sameAs` reasoning pays for it anyway. Other systems offer this:
rulesets, reasoning levels, optimized variants
(`docs/research/competitors-2026-11.md`).

## Decision

- **Switches, not profiles.** A switch is a named group of rules that a
  caller turns on or off (`CONTEXT.md`). In OWL 2 a profile is EL, QL or RL,
  so the word stays out.
- **A few coarse switches.** Every rule belongs to the always-on core or to
  exactly one switch, so no two switches overlap and none has to stay in
  step with another. The groups and their defaults are in the phase 1 spec.
- **The default rule.** A switch is off by default when other reasoners
  rarely implement its rules, or when turning it on slows reasoning by more
  than 5% on a scoreboard scenario. The switches that keep version 1's
  behaviour (`equality`, `property-chains`, `checks`) stay on.
- **The complete configuration** turns every switch on. It is the W3C rule
  set, and the W3C conformance runs use it.
- **Every configuration stays verified.** Nemo's program and the reference
  evaluator are composed from the same groups, and chaos draws its
  configuration from the seed.
- **The cargo feature `prp-spo2` goes:** the `property-chains` switch
  replaces it, at run time, in every build including WebAssembly.

## Consequences

- The public interface gains `Switch`, `Switches` and three methods on
  `Reasoner`; the CLI and Python expose the same names.
- Changing switches between runs forces a full run.
- The oracle suite runs each input in the default and the complete
  configuration; the W3C suites in the complete one.
- The speed race on the scoreboard is run in both configurations, so a win
  never comes from rules turned off.
