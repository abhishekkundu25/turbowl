# 08: Rules: property family

Status: todo. Blocked by: 02, 03.

`crates/turtle-reasoner/src/rules/property.rs`: prp-dom, prp-rng, prp-fp, prp-ifp (W3C form, D1), prp-symp, prp-trp, prp-spo1, prp-spo2 (chains of any length, cyclic lists terminate; behind the `prp-spo2` feature, on by default, as in `reasonable`), prp-eqp1/2, prp-inv1/2, container membership (`rdf:_n`). Joins keyed on (p, x) or (p, y), not p alone. One registration line in **`rules/mod.rs`**.

## Acceptance
- [ ] One test per rule, `reasonable`'s inputs, expected inferences written out; chains of length 1 to 5; a cyclic list.
