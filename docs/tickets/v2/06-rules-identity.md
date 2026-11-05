# 06: Rules: identity

Status: done. Blocked by: 04.

The `identity` group: prp-key, cls-maxc2, cls-maxqc3 and cls-maxqc4, which make individuals the same, in the core, the group's rule file, the reference evaluator, the generator and the minimal inputs. The merges go through the canon like any `owl:sameAs`.

## Acceptance
- [x] One minimal input per rule, with its expected inferences or findings; Nemo (where it can judge), OWL-RL and the reference evaluator agree with it.
- [x] The generator draws the new shapes (a test counts them over the first 300 seeds), and 20,000 full and 20,000 incremental chaos iterations pass with the group on.
- [x] The W3C cases `New-Feature-Keys-003` and `New-Feature-Keys-006` pass, and `New-Feature-ObjectQCR-002` passes once ticket 05 has landed too.
- [x] A `scaling_*` test: keys over many individuals and cls-maxc2 over many values grow linearly.
