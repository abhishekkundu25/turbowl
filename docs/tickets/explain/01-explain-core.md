# 01: The explainer: interface, backward search, the checker

Status: todo. Blocked by: none.

Build the spec's core (`docs/specs/explanations.md`) for every configuration
with `equality` and `literal-values` off. Ticket 02 adds those two. The
public interface is frozen here, so 02 and 03 build on it unchanged.

- **Interface**: `Proof`, `Statement`, `Reason` (a rule step may have no
  premises: cls-thing, cls-nothing1, prp-ap, dt-type1), `ExplainCache`
  (owned, `Send + Sync`, lazy parts in `OnceLock`), `Explainer<'_>`,
  `Reasoner::{explainer, explainer_with, explain, explain_diagnostic}`,
  `Explainer::into_cache`, and `Error::NotReasoned`, with its arm in
  `crates/python/src/lib.rs`'s `error()` so the workspace builds. All of
  it goes in the crate doc.
- **Accessors in `reasoner.rs`**, `pub(crate)`: the store, the dictionary,
  base membership, the findings, `same_as_itself` and the run state.
  Lookups of a loaded document's blank nodes: the reverse of
  `write_blank_label` in `term.rs`, so a triple read back from `triples()`
  is found.
- **The parser** over `oracles/rules/*.rls` (`include_str!`). It rewrites
  `Tp(p, x, y)` to `T(x, p, y)` and drops the `usedProp` rules (sound by
  `core.rls`'s note on Tp), so no unrelated schema triple becomes a
  premise.
- **The search**: relevant facts, ranks, and folding of the other helper
  relations into the step that reads them.
- **`turbowl_reference::check_proof`**, new `crates/reference/src/proof.rs`.
  For each step, run that rule's family for **one round** over the step's
  premises alone, recording every (fact, rule) pair, not only the first
  rule. Require the step's conclusion under its rule. Also require leaves
  in the base and acyclic steps. Needs `View::new` and the families'
  `rules` lists reachable in the crate (`evaluate.rs`, `rules/mod.rs`).
- **Proof tests**: `crates/oracle/tests/explain.rs` and chaos, both in the
  configurations above.
  - Chaos explains only the full test's state and the incremental test's
    final state.
  - It explains up to 32 inferred triples and 32 diagnostics, sampled from
    their own `Rng` stream (as `CONFIGURATIONS` is), so inputs and splits
    replay unchanged.
  - It uses one explainer per state.

A proof that goes missing in a random configuration is a bug in the
program or the explainer. Fix it in `oracles/rules/` or `explain/`, with a
minimal case, regenerated golden programs (`crates/oracle/tests/programs/`)
and a rerun of `crates/oracle/tests/reference.rs`.

`engine/` and `rules/` get visibility changes and read-only accessors only;
`Reasoner` gains no field.

## Acceptance
- [ ] `crates/turbowl/src/explain/tests.rs`: the parser (Tp rewritten), ranks, folding, a base triple, an axiom step, a triple outside the closure, a cyclic closure (a symmetric property) giving an acyclic proof, `NotReasoned`, a blank node from `load_str`, an explainer reused through `ExplainCache`, a reasoner restored from a snapshot.
- [ ] `crates/reference/src/proof.rs` unit tests: a valid proof passes; a wrong rule name, a missing premise, a non-base leaf and a cycle each fail; prp-inv1 and prp-inv2 on `p owl:inverseOf p` both pass.
- [ ] `crates/oracle/tests/explain.rs`: every inferred triple and diagnostic of every minimal input, explained and passing `check_proof`, in the default configuration with `equality` off and in the complete configuration with `equality` and `literal-values` off; none missing.
- [ ] Chaos quick run (400 iterations of each kind) passes, with proofs checked.
- [ ] `git diff main -- crates/turbowl/src/engine crates/turbowl/src/rules` shows visibility and accessors only.
- [ ] fmt, `clippy --workspace`, `scripts/check-wasm.sh`, `--no-default-features` build, and the turbowl, oracle and reference suites pass.
