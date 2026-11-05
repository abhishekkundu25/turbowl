# 07: Rules: class family

Status: done. Blocked by: 02, 03.

`crates/turbowl/src/rules/class.rs`: cax-sco, cax-eqc1/2, cls-thing, cls-nothing1, cls-int1/2, cls-uni, cls-svf1/2, cls-avf, cls-hv1/2, scm-sco, scm-eqc1, in their W3C forms (`docs/specs/v1.md`, the rule set). Every `intersectionOf`/`unionOf` list of a class counts, read structurally, including lists derived during the run. Keep scm-sco, cax-sco and cax-eqc1/2 as separate functions: scm-sco is about half of Brick's rule instances, and the first experiments each replace one of them (`docs/research/sota-algorithms-2026-11.md` §10). The family is registered in `rules/mod.rs` before the wave, with a stub; it fills `rules` and `FACTS` (the heads of cls-thing and cls-nothing1) in its own file. Tests use the shared harness, `rules/testing.rs`.

## Acceptance
- [x] One test per rule with a minimal input and the expected inferences written out; a class with two intersection lists; two lists sharing a tail.
- [x] Nemo (`oracles/nemo/owl2rl.rls`) agrees on each minimal input once ticket 10 lands.
- [x] A `scaling_*` test: with a fixed schema, the family's run is linear in the instance data (sizes spanning 16×).
