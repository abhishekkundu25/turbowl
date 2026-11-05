# AGENTS.md

TurbOWL is a fast OWL 2 RL reasoner in Rust: one deep library
(`crates/turbowl`), a command line, Python bindings, and a
verification suite that judges it against several independent oracles.

## Start here

1. **What it is and the words it uses:** `CONTEXT.md`.
2. **Why it is built this way:** `docs/adr/` (engine, equality, and
   ADR 0004: the W3C rules are the target, `reasonable` a speed baseline).
3. **Where the work stands:**
   - Version 1, a fast OWL 2 RL reasoner: `docs/specs/v1.md`, tickets
     in `docs/tickets/v1/` (the README there has the table, the waves and
     the log of departures).
   - After version 1, **the improvement loop** (rate, find, change,
     evaluate, decide, record): `docs/specs/experiments.md`. This is how the
     project improves; its table is the history, and its last section says
     how to continue cold.
4. **The newest verification record:** the latest file in
   `docs/verification/`.

## Where things live

- The public interface: `crates/turbowl/src/lib.rs` (the
  `Reasoner`, `Format`, `Summary`, `Diagnostic`, `Severity`, `Error`).
  Everything else in the crate is `pub(crate)`. Add to the interface only
  when an adapter (CLI, Python, oracle, bench) needs it, and say so in the
  crate doc.
- Internal modules: `term.rs` (term ids, dictionary), `io.rs` (parse,
  write), `engine/` (relations, rounds, joins; knows no rules),
  `rules/` (one file per rule family, registered one line each in
  `rules/mod.rs`), `diagnostics.rs`, `reasoner.rs`.
- Adapters: `crates/cli` (the `turbowl` binary; `--help` and the crate
  doc in `src/main.rs` define its report, the JSON fields and the exit
  codes 0, 1, 2 and 64), `crates/python`.
- Verification: `crates/oracle` (the oracles), `crates/reference` (the
  reference evaluator, the generator, chaos), `crates/bench` (benchmarks).
- Oracle tools and data are fetched, never committed:
  `scripts/fetch-tools.sh` into `tools/`, `scripts/fetch-data.sh` into
  `data/`. The OWL-RL oracle runs from `oracles/owlrl/` with `uv`.

## Build and test

Release mode only: timing tests and budgets mean nothing in a debug build.
Keep every test run under two minutes (`timeout 120`); run one test file at
a time when a suite is longer.

Scaling tests (`scaling_*`) fit how time grows with input size and fail on
anything worse than n log n (see the spec's Testing Decisions). A failure
prints its table of sizes and times. On a loaded machine, rerun the one test
alone (`-- scaling_<name> --test-threads=1`: the filter matches part of the
name, which sits under a module path) before believing it.

```sh
cargo build --release --locked
cargo fmt --check
cargo clippy --release --locked --all-targets -- -D warnings
scripts/check-wasm.sh                                   # the core for wasm32
timeout 120 cargo test --release --locked -p turbowl
timeout 120 cargo test --release --locked -p turbowl-oracle
timeout 120 cargo test --release --locked -p turbowl-reference --test chaos
timeout 120 cargo test --release --locked -p turbowl-cli  # runs the built binary
cargo run --release -p turbowl-bench -- --list
```

- A chaos failure prints `CHAOS_SEED`, `CHAOS_ITERS` and `CHAOS_ONLY` to
  replay it and saves the input under `target/chaos/`. Fix what it finds,
  with a unit test on the reduced input; never `#[ignore]` a test or exempt
  a check.
- An oracle that is not installed is reported `skipped`; with `CI=true` it
  fails.
- Benchmarks run only on a quiet machine (load average under 1), never next
  to a build or a test run.
- Every full verification is stored: `docs/verification/<date>-<name>.md`
  and its raw results in `docs/verification/data/`.

## Rules

- Correctness never trades for speed. A change that makes any oracle
  disagree is reverted or fixed, not accepted.
- One change per experiment, recorded in `docs/specs/experiments.md`,
  including the ones that are reverted.
- Commits carry the owner's name only: no AI attribution and no co-author
  lines.
- Nothing is published (crates.io, PyPI, GitHub) without the owner's
  approval.
- Fixtures use neutral names (`ex:`, `http://example.org/`).
