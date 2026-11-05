# ADR 0003: `owl:sameAs` by representatives, expanded on output

Status: accepted, 2026-11-05.

## Context

`reasonable` derives the full `owl:sameAs` closure with the eq-* rules: a
clique of n equal terms gives n² `sameAs` triples, and every triple about one
member is copied for each alias. Motik et al. (AAAI 2015, "Handling
owl:sameAs via rewriting") showed that rewriting each equality class to one
representative keeps the store small and the rules unchanged.

## Decision

The engine keeps a union-find of term ids. When `sameAs` merges two classes,
the larger class's representative wins, and tuples that mention the other
id are rewritten and re-added to the deltas. Rules see only
representatives. Output (`triples()`, files, Python) expands each
representative to its members, including the `x sameAs y` triples among
them, so the closure equals the one the eq-* rules would give.

## Consequences

- The oracle compares expanded closures; equal sets prove the rewriting is
  invisible.
- Diagnostics name the original terms where a message names a term.
- Incremental deletion of a `sameAs` triple must split classes; version 1
  answers any removal with a full run (as `reasonable` does), so the split is
  an experiment, not a v1 requirement.
