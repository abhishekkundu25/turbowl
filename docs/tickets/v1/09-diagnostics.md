# 09: Diagnostics and datatypes

Status: todo. Blocked by: 02, 03.

`crates/turtle-reasoner/src/diagnostics.rs`: the `Diagnostic` type (code, rule, severity, message, structured terms), deduplication on (code, message), the exact messages of `reasonable`. `crates/turtle-reasoner/src/rules/datatype.rs`: cax-dw, prp-pdw, prp-asyp, prp-irp, cls-nothing2 and cls-com as diagnostics; `RDFS.DATATYPE` (the five datatypes `reasonable` checks) and `RDFS.DATATYPE_RANGE`, including the retroactive case. One registration line in **`rules/mod.rs`**.

## Acceptance
- [ ] Each code fires on the parity checklist's minimal input with `reasonable`'s message; `CLS_COM` fires (D2); the negative cases of items 14 and 15 do not fire.
