# 08: Rules: the trivial group and cls-oo

Status: todo. Blocked by: 04.

The `trivial` group, off by default: eq-ref (added to the closure when it is read, never stored), scm-cls, scm-op, scm-dp, prp-ap and dt-type1. Also cls-oo (`owl:oneOf`) in the core. Each lands in the core, the rule files, the reference evaluator, the generator and the minimal inputs.

## Acceptance
- [ ] One minimal input per rule, with its expected inferences or findings; Nemo (where it can judge), OWL-RL and the reference evaluator agree with it.
- [ ] The generator draws the new shapes (a test counts them over the first 300 seeds), and 20,000 full and 20,000 incremental chaos iterations pass with the group on.
- [ ] In the complete configuration, every IRI and blank node of the closure has its `owl:sameAs` to itself, and the summary counts them.
- [ ] OWL-RL's closure contains ours in the complete configuration on every minimal input and on Brick, apart from its listed faults.
- [ ] Default-configuration results are unchanged on every suite.
