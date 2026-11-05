# 05: Rules: the new checks

Status: done. Blocked by: 04.

The `checks` group gains eq-diff1, eq-diff2, eq-diff3, prp-npa1, prp-npa2, cax-adc, prp-adp, cls-maxc1, cls-maxqc1 and cls-maxqc2, in the core, the group's rule file, the reference evaluator, the generator and the minimal inputs. Each finding carries its terms in a documented order, as version 1's do.

## Acceptance
- [x] One minimal input per rule, with its expected inferences or findings; Nemo (where it can judge), OWL-RL and the reference evaluator agree with it.
- [x] The generator draws the new shapes (a test counts them over the first 300 seeds), and 20,000 full and 20,000 incremental chaos iterations pass with the group on.
- [x] The W3C cases that name these rules pass: `WebOnt-differentFrom-001`, `owl2-rl-rules-fp-differentFrom`, `owl2-rl-rules-ifp-differentFrom`, `New-Feature-NegativeDataPropertyAssertion-001`, `New-Feature-NegativeObjectPropertyAssertion-001`, `DisjointClasses-003`, `New-Feature-DisjointDataProperties-002` and `New-Feature-DisjointObjectProperties-002`.
- [x] A `scaling_*` test for eq-diff2/3 over a large `owl:AllDifferent` list (linear).
