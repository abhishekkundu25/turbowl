# 26: OWL-RL's datatype and identity faults

Status: done. Blocked by: 08.

Tickets 06 and 09 found OWL-RL (owlrl 7.6.2, rdflib 7.6.0) wrong in ways
no test filters yet, because the minimal inputs avoid them or OWL-RL
over-derives, which containment never catches. Each becomes a listed
oracle fault in `crates/oracle/src/differences.rs`, numbered after F6, with
a test that shows Nemo or the reference evaluator agreeing with TurbOWL
where OWL-RL does not:
- Numbers are read with Python's `int()` and `Decimal()`: it accepts
  `"٣"^^xsd:integer`, `"1_000"^^xsd:decimal` and `"NaN"^^xsd:decimal`.
- Its double and float ranges are wrong: `"0"`, `"-0"`, `INF` and `NaN` are
  reported as ill-typed, and so are `1E400` and `1e-400`, which XML Schema
  1.1 rounds.
- Other lexical spaces: booleans are case-insensitive; `hexBinary` takes
  odd lengths and a `0x` prefix and rejects `""`; `base64Binary` rejects
  single spaces and `""` and takes nonzero padding bits; `normalizedString`
  rejects `""`; `Name` and `NMTOKEN` use `\w`; `anyURI` rejects `"?q"` and
  `":x"`.
- `dateTime` is read with `time.strptime`: it rejects years 0000, negative
  years, five-digit years, `24:00:00` and more than six fraction digits,
  and accepts `+15:00`, `+01:60`, a one-digit month and a lowercase `t`.
- `rdf:XMLLiteral` is parsed as a whole document, so `"plain text"`, `""`
  and top-level CDATA are rejected.
- rdflib rewrites `xsd:token` and `xsd:normalizedString` forms even with
  `NORMALIZE_LITERALS=False`, so its closure holds another literal.
- Its prp-key never checks a key's last property (`range(0, len(pis)-1)`),
  so a one-property key makes every typed individual equal.
- Its cardinality rules compare values, so `"1"^^xsd:integer` fires.

## Acceptance
- [x] Each fault has an id, a description citing the OWLRL.py or rdflib lines, a one-input test, and a filter that removes exactly what it explains (none where nothing needs filtering).
- [x] The minimal inputs regain the literals ticket 09 removed to avoid them (the `xsd:token` form), now covered by their fault.
