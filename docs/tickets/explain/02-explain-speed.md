# 02: Equality, literal values, and speed

Status: todo. Blocked by: 01.

Extend the explainer to every configuration, then meet the spec's latency
budgets and measure them. The interface stays as 01 froze it.

- **Equality**: first make the eq-* rules work through the canon,
  generically. Then build the fast path: a sameAs chain as the shortest
  path over the class's sameAs-concluding rule instances, so sameas-clique
  stays under budget.
- **Literal values and dt-not-type**, as the spec fixes them:
  - dt-eq and dt-type2 cite one closure triple that names the literal.
  - dt-diff is one proof shape: a generalized dt-diff step with the unlisted
    predicate (`crates/reference/src/vocab.rs`), then eq-rep steps, then
    eq-diff1. The checker reads that predicate.
  - dt-not-type has two cases. A literal ill-typed for its own datatype
    cites a closure triple that names it. A literal typed by a triple cites
    that triple's proof.
  - `literalValue` and `literalType` come from TurbOWL's datatype code.
- **Chaos and the oracle test in every configuration** (the limits 01 set
  are lifted).
- **Speed**: lazy indexes in `ExplainCache`, never in `Reasoner`. If
  computing every relevant fact explodes on transitive components or large
  classes (claros-l, sameas-clique), bound the search by iterative
  deepening on rank.
- **`turbowl-bench --explain`**: per scenario, a seeded 1,000 inferred
  triples and up to 1,000 diagnostics. It reports median, 99th percentile
  and max latency with a warm cache, the cold first-use time on its own, and
  the bytes the cache holds.
- **`scaling_explain`** in `crates/turbowl/src/scaling.rs`.

Each speed change is measured before and after, with its limiter named.

## Acceptance
- [ ] `crates/oracle/tests/explain.rs` and the chaos quick run pass in every configuration, default and complete included, none missing.
- [ ] Unit tests: a three-hop sameAs chain, a fact holding only through a representative, dt-eq, dt-diff into eq-diff1, both dt-not-type cases.
- [ ] Editor-size scenarios: median under 1 ms, 99th percentile under 10 ms per triple, warm.
- [ ] lubm-50, claros-l and sameas-clique: 99th percentile under 50 ms, warm; cold time reported.
- [ ] `scaling_explain` passes alone.
- [ ] `engine/` and `rules/` still show visibility and accessors only against `main`.
