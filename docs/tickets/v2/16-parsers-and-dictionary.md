# 16: Fast loading: parsers and the dictionary

Status: todo. Blocked by: 15.

TurbOWL's own N-Triples and Turtle parsers intern terms straight into term ids, through a faster dictionary: an arena, with each term's hash computed once. Blank-node scoping and error positions stay as they are. N3 and RDF/XML keep Oxigraph's parsers, which also cross-check every document the tests read. The hot path stops using Oxigraph's N-Triples and Turtle parsers.

## Acceptance
- [ ] Every document the suites read, plus generated documents, parses to the same triples as Oxigraph's parsers. Malformed documents fail at the same line and column.
- [ ] `scaling_*` tests: parsing and interning are linear in the input size.
- [ ] Loading lubm-50 takes at most half the Baseline's time on a quiet machine, measured at the phase close-out.
