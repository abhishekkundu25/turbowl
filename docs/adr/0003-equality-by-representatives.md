# ADR 0003: `owl:sameAs` by representatives, expanded on output

Status: accepted, 2026-11-05; amended 2026-11-05 (vocabulary terms, from
`docs/research/sota-algorithms-2026-11.md` §4.3); amended again 2026-11-05
by ticket 11, as built (`rdf:nil`, restarts, findings); amended 2026-11-06 by
version 2's ticket 08 (eq-ref).

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

## Amendment: as built (ticket 11)

- **The canon** (`crates/turbowl/src/engine/canon.rs`) is a union-find in
  which every member points straight at its representative, so a rule reads
  a representative with one array read. "Larger" counts members; a tie goes
  to the smaller id.
- **`rdf:nil` is watched.** The W3C list rules test `rdf:nil` for identity
  (`?r != rdf:nil` in Datalog form), which the rewriting cannot see: a term
  equal to `rdf:nil` but not `rdf:nil` itself can be a later list cell. So
  when `rdf:nil`'s class holds another term, the list rules read its
  representative both as the end of a list and as a cell. The first merge
  that gives `rdf:nil` a class restarts the run, like a moved constant, so
  that this reading is fixed for a run.
- **A restart** keeps the canon and the findings found so far: both follow
  from the base. A restart happens at most once per constant moved, plus
  once for `rdf:nil`.
- **Findings** are kept in representatives and expanded on output to every
  choice of members, since the rule that concludes them matches each choice
  in the expanded closure. dt-not-type is the exception: its rule looks
  inside a literal and names a datatype constant, so the datatype family
  checks each literal member of a typed class itself, and names the
  literal.
- **Counts.** The reasoner keeps the number of RDF triples in the expanded
  closure (each stored triple counts the product of its classes' sizes, by
  position), adds each round's deltas in a run without merges, and counts
  the store again after a run that merged.

## Amendment: eq-ref (version 2's ticket 08)

With the `trivial` switch on, eq-ref gives `x owl:sameAs x` for every term.
The store already holds `r owl:sameAs r` for every class of two or more
terms, and its expansion gives each member's reflexive pair, so eq-ref adds
a triple only for a term alone in its class. The reasoner adds those when
the closure is read (`triples()`, files, Python) and counts them in the
summary, without storing them.

That is exact only while no rule reads `owl:sameAs` as a property the
schema names: `owl:sameAs rdfs:domain c`, a restriction on it, a chain or
a key that lists it, or a term made equal to it. Each such rule finds
`owl:sameAs` as the subject or object of a stored triple. Once it is, the
trivial family stores `t owl:sameAs t` for every term of the closure,
literals included, and from then on for the terms of each round's new
triples. Real data almost never does this; the generator does, so chaos
covers it.
