# ADR 0003: `owl:sameAs` by representatives, expanded on output

Status: accepted, 2026-11-05; amended 2026-11-05 (vocabulary terms, from
`docs/research/sota-algorithms-2026-11.md` §4.3).

## Context

`reasonable` derives the full `owl:sameAs` closure with the eq-* rules: a
clique of n equal terms gives n² `sameAs` triples, and every triple about one
member is copied for each alias. Motik et al. (AAAI 2015, "Handling
owl:sameAs via rewriting") showed that rewriting each equality class to one
representative keeps the store small and the rules unchanged.

## Decision

The engine keeps a union-find of term ids. When `sameAs` merges two classes,
the larger class's representative wins, and tuples that mention the other
id are rewritten and re-added to the deltas (as egglog does: canonicalise,
then take the delta). Rules see only representatives.

Vocabulary terms, the constants the rules name (`rdf:type`,
`rdfs:subClassOf`, ...), need three more rules, because rules that look up
a table by a constant would otherwise find nothing after a merge:

- **Choice.** If a merged class contains a vocabulary term, the smallest
  vocabulary id wins. Vocabulary terms are interned first, so this is
  RDFox's "smallest id" rule wherever it matters (Motik et al., AAAI 2015,
  appendix A), and the larger-class rule everywhere else.
- **Constants through the map.** Rule families read their constants through
  the round (the representative of `vocab::X`), never as bare ids, so a
  `sameAs` between two vocabulary terms keeps every rule working.
- **Re-run when a constant moves.** When a vocabulary term's representative
  changes, semi-naive evaluation alone misses facts (RDFox re-evaluates the
  rewritten rules over all facts). Version 1 restarts the run from the base
  with the merged classes. Real data almost never does this.

When a merged-away term is used as a predicate, its whole table moves into
the representative's table: rewriting tuples inside a table does not move
its key. Output (`triples()`, files, Python) expands each
representative to its members, including the `x sameAs y` triples among
them, so the closure equals the one the eq-* rules would give.

## Consequences

- The oracle compares expanded closures; equal sets prove the rewriting is
  invisible.
- Diagnostics name the original terms where a message names a term.
- Incremental deletion of a `sameAs` triple must split classes; version 1
  answers any removal with a full run (as `reasonable` does), so the split is
  an experiment, not a v1 requirement.
