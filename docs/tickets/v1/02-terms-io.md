# 02: Terms, dictionary and I/O

Status: todo. Blocked by: 01.

`crates/turtle-reasoner/src/term.rs`: term ids (`u32` newtype), the dictionary (term ↔ id, idempotent interning, fast hashing), the vocabulary ids the rules use (rdf, rdfs, owl, xsd) interned up front at fixed ids. Blank nodes scoped per document. `crates/turtle-reasoner/src/io.rs`: parse Turtle, N-Triples, N3 and RDF/XML with `oxttl`/`oxrdfxml` into term-id triples; write Turtle and N-Triples with language tags kept; the format from a file extension or given explicitly; errors returned, never panics.

## Acceptance
- [ ] Unit tests: interning is idempotent; two documents' `_:b` differ; a literal keeps datatype and language; every vocabulary id is fixed.
- [ ] Round trip: parse then write then parse gives the same triple set for each format (fixtures in `crates/turtle-reasoner/src/../tests/fixtures/io/`).
- [ ] A malformed file gives an `Error` with the file and position.
