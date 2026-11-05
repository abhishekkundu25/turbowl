# 07: Rules: the schema

Status: todo. Blocked by: 04.

The `schema` group: scm-eqc2, scm-spo, scm-eqp1, scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv, scm-svf1, scm-svf2, scm-avf1, scm-avf2, scm-int and scm-uni, in the core, the group's rule file, the reference evaluator, the generator and the minimal inputs.

## Acceptance
- [ ] One minimal input per rule, with its expected inferences or findings; Nemo (where it can judge), OWL-RL and the reference evaluator agree with it.
- [ ] The generator draws the new shapes (a test counts them over the first 300 seeds), and 20,000 full and 20,000 incremental chaos iterations pass with the group on.
- [ ] The W3C cases `WebOnt-equivalentClass-003`, `WebOnt-equivalentProperty-002` and `WebOnt-equivalentProperty-003` pass, and `WebOnt-I4.6-003` once ticket 08's scm-cls lands. (`WebOnt-I5.8-006`, `-008` and `-009` need datatype subsumptions, such as `xsd:byte` ⊑ `xsd:short`, that no OWL 2 RL rule derives; Nemo and OWL-RL fail them too.)
- [ ] HermiT entails every subsumption the group adds on Brick and its variants.
- [ ] A `scaling_*` test over a deep class and property hierarchy grows at most as n log n.
