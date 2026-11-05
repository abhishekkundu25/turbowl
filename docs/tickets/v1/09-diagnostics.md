# 09: Diagnostics and datatypes

Status: done. Blocked by: 02, 03.

`crates/turtle-reasoner/src/diagnostics.rs`: the `Diagnostic` type (code, severity, message, structured terms in the order documented on the oracle crate's `Finding`; codes are the W3C rule names), deduplication on (code, terms), our own messages. `crates/turtle-reasoner/src/rules/datatype.rs`: cax-dw, prp-pdw, prp-asyp, prp-irp, cls-nothing2 and cls-com as diagnostics; dt-not-type for the five checked datatypes with the reference evaluator's value-space rules (`crates/reference/src/datatype.rs`), including a literal typed by prp-rng after it was loaded. The family is registered in `rules/mod.rs` before the wave, with a stub; findings go through `Context::report`, and `Diagnostic::from_finding` (a stub until this ticket) makes the public form. Tests use the shared harness, `rules/testing.rs`.

## Acceptance
- [x] Each code fires on its minimal input with the expected terms; a property with a class range and an IRI object does not fire dt-not-type; Nemo agrees on every finding except dt-not-type, which OWL-RL and the W3C tests judge.
- [x] A `scaling_*` test: with a fixed schema, the family's run is linear in the instance data (sizes spanning 16×).
