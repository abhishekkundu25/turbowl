# 07: Rules: class family

Status: todo. Blocked by: 02, 03.

`crates/turtle-reasoner/src/rules/class.rs`: cax-sco, cax-eqc1/2, cls-thing, cls-nothing1, cls-int1/2, cls-uni, cls-svf1/2, cls-avf, cls-hv1/2, scm-sco, scm-eqc1, in their W3C forms (`docs/specs/v1.md`, the rule set). Every `intersectionOf`/`unionOf` list of a class counts, read structurally, including lists derived during the run. One registration line in **`rules/mod.rs`** (the first rule family adds `rules: RuleFn` to `Family`).

## Acceptance
- [ ] One test per rule with a minimal input and the expected inferences written out; a class with two intersection lists; two lists sharing a tail.
- [ ] Nemo (`oracles/nemo/owl2rl.rls`) agrees on each minimal input once ticket 10 lands.
