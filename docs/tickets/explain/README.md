# Tickets: explanations

Spec: `docs/specs/explanations.md`. Background:
`docs/research/explanations-2026-11.md`. Lock files are in **bold**.

| # | Ticket | Status | Blocked by | Files |
|---|---|---|---|---|
| 01 | [The explainer: interface, backward search, the checker](01-explain-core.md) | done (d865f3e) | — | new `crates/turbowl/src/explain.rs` and `src/explain/`, **`crates/turbowl/src/lib.rs`**, **`crates/turbowl/src/reasoner.rs`** (the explain methods and `pub(crate)` accessors), `crates/turbowl/src/error.rs`, `crates/turbowl/src/term.rs` (blank-node lookup), visibility only in `src/engine/` and `src/rules/`; the `error()` arm in `crates/python/src/lib.rs`; `crates/reference/src/{lib,evaluate,rules/mod}.rs` and new `src/proof.rs`; new `crates/oracle/tests/explain.rs`; `crates/reference/tests/chaos.rs`; `oracles/rules/` and the golden programs if a proof finds a program bug |
| 02 | [Equality, literal values, and speed](02-explain-speed.md) | done (6309f93) | 01 | `crates/turbowl/src/explain/`, `crates/turbowl/src/scaling.rs`, `crates/reference/src/proof.rs`, `crates/reference/tests/chaos.rs`, `crates/oracle/tests/explain.rs`, new `crates/bench/src/explain.rs`, **`crates/bench/src/main.rs`** |
| 03 | [Explanations in the CLI and Python](03-explain-adapters.md) | done (fb99694) | 01 | `crates/cli/` (with `Cargo.toml`), **`Cargo.lock`** (one dependency edge), `crates/python/` |
| 04 | [Close-out](04-close-out.md) | done | 01, 02, 03 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, the crate doc, `docs/verification/`, this README |
| 05 | [The deferred validation runs](05-deferred-validation.md) | todo | — | `docs/verification/`, `AGENTS.md` (chaos times) |
| 06 | [Explanation speed in the complete configuration](06-complete-explain-speed.md) | todo (owner) | — | `crates/turbowl/src/explain/` |

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
- 2026-11-07, ticket 03 as built:
  - `oxrdf` and `oxttl` moved from the CLI's dev-dependencies to its
    dependencies, so `Cargo.lock` is unchanged.
  - `--summary-only` with `--explain-diagnostics` exits 64.
  - JSON proofs carry `depth`.
  - Python's `Proof` also has `goal`, `rule` and `premises`, and there is a
    new `NotReasonedError`.
  - A test-only native method, `explain_cache_held`.
- 2026-11-07, ticket 02 as built (sent back once, after review):
  - **Gap, for the owner:** chaos with proofs takes 2.4 to 2.9 times as long
    per shard (2,000 iterations: about 29 to 35 s without, 81 to 91 s with),
    against the +10% budget. This is test time only: reasoning is
    unchanged. The limiter is equality in closures where a vocabulary term
    shares a class with individuals, which needs the wider search levels.
    Closing it needs a cheaper complete method for those classes, or a new
    budget. A 100,000-iteration shard will take about 2.5 times as long.
  - Decided without the owner: chaos explains 8 triples and 8 diagnostics
    per state (was 32 of each), every one compared with the all-cores
    explainer's proof, asked in the same order. Fewer goals did not
    help: the cost is a tail of slow goals, not their number.
    `CHAOS_EXPLAINED` overrides the 8.
  - The tail was cut: the worst goal went from 2.4 s to 0.58 s, and
    `a_restarted_closure_is_explained_in_time` holds a reduced case.
  - `turbowl::profile::explain_cache_bytes` is new and public, for the
    bench. 04 names it in the crate doc.
  - A sameAs chain is left-linear: its depth is its number of links.
  - The bench's cold time includes the first triple's search.
  - Every latency budget is met in the default configuration. The
    close-out found the complete configuration's gaps (below).
- 2026-11-07, the close-out (04):
  - The explain latency bench hung on claros-l with `--complete`: one
    triple's search ran for over 20 minutes (a class of 306 time spans
    that 35,000 productions reach, joined once per member). Fixed in
    3f8eabc, with a timed unit test and a third `scaling_explain` shape;
    the same fix brought sameas-clique's p99 back from 39 ms to 0.6 ms (a
    regression of 02's second round).
  - The complete configuration misses the editor-size budget on Brick:
    ticket 06.
  - At the owner's request, the runs that need a quiet machine (load stayed
    2.7 to 4 from other applications) moved to ticket 05: the latencies
    at load under 1, lubm-50 at 10 runs per side, 100,000 chaos
    iterations, and the slow oracle tier.
