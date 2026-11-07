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
| 01 | [A triple term is an opaque term](01-opaque-triple-terms.md) | todo | — | `crates/turbowl/src/term.rs`, `crates/turbowl/Cargo.toml` (an `rdf-12` feature), tests beside `term.rs` and in `crates/turbowl/tests/`, `scripts/check-wasm.sh` or a sibling check, the crate doc's feature list, `CHANGELOG.md` |

One ticket, one wave.

## Departures

None yet.
