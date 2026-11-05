# 09: Diagnostics and datatypes

Status: todo. Blocked by: 02, 03.

`crates/turtle-reasoner/src/diagnostics.rs`: the `Diagnostic` type (code, rule, severity, message, structured terms in the order documented on the oracle crate's `Finding`), deduplication on (code, terms), our own messages. `crates/turtle-reasoner/src/rules/datatype.rs`: cax-dw, prp-pdw, prp-asyp, prp-irp, cls-nothing2 and cls-com as diagnostics; dt-not-type for the supported datatypes, including a literal typed by prp-rng after it was loaded. One registration line in **`rules/mod.rs`**.

## Acceptance
- [ ] Each code fires on its minimal input with the expected terms; a property with a class range and an IRI object does not fire dt-not-type; Nemo agrees on every finding except dt-not-type, which OWL-RL and the W3C tests judge.
