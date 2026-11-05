# 08: Rules: property family

Status: todo. Blocked by: 02, 03.

`crates/turtle-reasoner/src/rules/property.rs`: prp-dom, prp-rng, prp-fp, prp-ifp, prp-symp, prp-trp, prp-spo1, prp-spo2 (chains of any length, cyclic lists terminate), prp-eqp1/2, prp-inv1/2, in their W3C forms. Triples with a literal subject are derived like any other (generalized triples, as the spec says); only the reasoner's output leaves them out. Joins keyed on (p, x) or (p, y), not p alone. prp-inv1/2 (about a fifth of Brick's rule instances) copy the recent slice reversed, without a join. The family is registered in `rules/mod.rs` before the wave, with a stub. Tests use the shared harness, `rules/testing.rs`.

## Acceptance
- [ ] One test per rule with a minimal input and the expected inferences written out; chains of length 1 to 5; a cyclic list; prp-ifp merges only subjects whose objects are equal.
- [ ] Nemo agrees on each minimal input once ticket 10 lands.
- [ ] A `scaling_*` test: with a fixed schema, the family's run is linear in the instance data (sizes spanning 16×).
