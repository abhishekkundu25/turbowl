# Tickets: explanations

Spec: `docs/specs/explanations.md`. Background:
`docs/research/explanations-2026-11.md`. Lock files are in **bold**.

| # | Ticket | Status | Blocked by | Files |
|---|---|---|---|---|
| 01 | [The explainer: interface, backward search, the checker](01-explain-core.md) | done (d865f3e) | — | new `crates/turbowl/src/explain.rs` and `src/explain/`, **`crates/turbowl/src/lib.rs`**, **`crates/turbowl/src/reasoner.rs`** (the explain methods and `pub(crate)` accessors), `crates/turbowl/src/error.rs`, `crates/turbowl/src/term.rs` (blank-node lookup), visibility only in `src/engine/` and `src/rules/`; the `error()` arm in `crates/python/src/lib.rs`; `crates/reference/src/{lib,evaluate,rules/mod}.rs` and new `src/proof.rs`; new `crates/oracle/tests/explain.rs`; `crates/reference/tests/chaos.rs`; `oracles/rules/` and the golden programs if a proof finds a program bug |
| 02 | [Equality, literal values, and speed](02-explain-speed.md) | todo | 01 | `crates/turbowl/src/explain/`, `crates/turbowl/src/scaling.rs`, `crates/reference/src/proof.rs`, `crates/reference/tests/chaos.rs`, `crates/oracle/tests/explain.rs`, new `crates/bench/src/explain.rs`, **`crates/bench/src/main.rs`** |
| 03 | [Explanations in the CLI and Python](03-explain-adapters.md) | todo | 01 | `crates/cli/` (with `Cargo.toml`), **`Cargo.lock`** (one dependency edge), `crates/python/` |
| 04 | [Close-out](04-close-out.md) | todo | 01, 02, 03 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, the crate doc, `docs/verification/`, this README |

## How the lock files stay apart

- **`crates/turbowl/src/lib.rs`** and **`crates/turbowl/src/reasoner.rs`**:
  only 01 edits them, adding the explain methods and the accessors. 02
  changes what is inside `explain/`, and 03 uses the interface as 01 freezes
  it.
- **`Cargo.lock`**: only 03, which adds `oxttl` to the CLI.
- **`crates/python/src/lib.rs`**: 01 adds one arm to `error()`; 03 owns the
  rest, a wave later.
- **`crates/bench/src/main.rs`**: only 02.
- 02 and 03 share no file.

## Waves

1. **01** alone: every other ticket reads its types.
2. **02, 03** together.
3. **04**, after both.

## Departures

- 2026-11-07, review before the start (a fresh Opus 5.5 reviewer). Changes
  made to the plan:
  - 01 covers configurations with `equality` and `literal-values` off, and 02
    adds them, because 01 was too big for one session.
  - Rule steps may have no premises (cls-thing and the other axiom rules).
  - The checker records every (fact, rule) pair of one round, so a premise
    set two rules share passes under either.
  - One proof shape for dt-diff.
  - An owned `ExplainCache`, which Python needs.
  - `Error::NotReasoned`'s Python arm goes in 01, so the workspace builds.
  - The `reasoner.rs` accessors, and the blank-node lookup of loaded files.
  - `Tp` is rewritten to `T`, so no unrelated premises appear.
  - Chaos explains final states only, 32 triples and 32 diagnostics from its
    own `Rng` stream.
  - The bench reports the cache's bytes rather than a peak it cannot
    separate, and cold time on its own.
  - The CLI gets `oxttl`.
- 2026-11-07, ticket 01 as built:
  - Chaos explains every iteration. One with `equality` or
    `literal-values` on explains a one-thread reasoner of its input with
    both off, until 02.
  - Chaos also requires the one-thread and all-cores proofs to be equal.
  - dt-type2 and dt-not-type were done in 01, through
    `explain/literals.rls`.
  - Iterative deepening and the cached instance graph were done in 01,
    because without them chaos took 8 times as long.
  - The new types `Step` and `Goal`.
  - Left for 02:
    - chaos is 23% slower with proofs (budget 10%);
    - the cache is matched to its reasoner by counts;
    - the first use copies the whole store into three sorted orders.
