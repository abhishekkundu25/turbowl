# Tickets: RDF 1.2 triple terms

Background: turtle-lsp embeds TurbOWL as a path dependency
(`default-features = false`) and turns on oxrdf's `rdf-12` feature, so
Cargo unifies features and compiles TurbOWL against an oxrdf whose
`TermRef` has a `Triple` variant. TurbOWL does not compile that way: the
matches in `crates/turbowl/src/term.rs` (`intern_term`, `get_term`) miss the
variant. The owner chose, on 2026-11-07, to treat a triple term as an
**opaque term**.

| # | Ticket | Status | Blocked by | Files |
|---|---|---|---|---|
| 01 | [A triple term is an opaque term](01-opaque-triple-terms.md) | done | — | `crates/turbowl/src/term.rs`, `crates/turbowl/Cargo.toml` (an `rdf-12` feature), tests beside `term.rs` and in `crates/turbowl/tests/`, `scripts/check-wasm.sh` or a sibling check, the crate doc's feature list, `CHANGELOG.md` |

One ticket, one wave.

## Departures

- 2026-11-07, light verification (owner's choice): `-p turbowl-oracle`, the chaos run and the benchmark were not run.
- The feature forwards to `oxttl?/rdf-12` and `oxrdfxml?/rdf-12` too: those parsers do not compile against an RDF 1.2 oxrdf without their own feature.
- Under `rdf-12`, the two oxttl parity tests (`every_test_document_parses_as_oxttl_does`, `generated_documents_parse_as_oxttl_does`) are compiled out: oxttl then reads RDF 1.2 Turtle and TurbOWL's parser reads 1.1. They still run in the default build.
- Follow-ups: a directional literal (`"x"@en--ltr`) loses its direction under `rdf-12`; the python, oracle and bench crates do not build with `rdf-12` yet.
