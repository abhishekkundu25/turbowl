# 08: Rules: the trivial group and cls-oo

Status: todo. Blocked by: 04.

The `trivial` group, off by default: eq-ref (added to the closure when it is read, never stored), scm-cls, scm-op, scm-dp, prp-ap and dt-type1. Also cls-oo (`owl:oneOf`) in the core. Each lands in the core, the rule files, the reference evaluator, the generator and the minimal inputs. Because eq-ref is produced on read and never stored, the checks cannot see it: with `trivial` on, eq-diff1, eq-diff2 and eq-diff3 must treat every term as `owl:sameAs` itself (so `x owl:differentFrom x` alone, or a member listed twice in an `owl:AllDifferent`, is a finding). The rule families get the run's switches through `Context` for this. The core cases `empty` and `no-thing-typing` state an exact closure and must not count the `trivial` group's triples.

## Acceptance
- [ ] One minimal input per rule, with its expected inferences or findings; Nemo (where it can judge), OWL-RL and the reference evaluator agree with it.
- [ ] The generator draws the new shapes (a test counts them over the first 300 seeds), and 20,000 full and 20,000 incremental chaos iterations pass with the group on.
- [ ] In the complete configuration, every IRI and blank node of the closure has its `owl:sameAs` to itself, and the summary counts them.
- [ ] OWL-RL's closure contains ours in the complete configuration on every minimal input and on Brick, apart from its listed faults.
- [ ] Default-configuration results are unchanged on every suite.
