# AGENTS.md

turtle-reasoner is a fast OWL 2 RL reasoner in Rust: one deep library
(`crates/turtle-reasoner`), a command line, Python bindings, and a
verification suite that judges it against several independent oracles.

## Start here

1. **What it is and the words it uses:** `CONTEXT.md`.
2. **Why it is built this way:** `docs/adr/` (engine, parity, equality).
3. **Where the work stands:**
   - Version 1, parity with `reasonable`: `docs/specs/v1-parity.md`, tickets
     in `docs/tickets/v1/` (the README there has the table, the waves and
     the log of departures).
   - After version 1, **the improvement loop** (rate, find, change,
     evaluate, decide, record): `docs/specs/experiments.md`. This is how the
     project improves; its table is the history, and its last section says
     how to continue cold.
4. **The newest verification record:** the latest file in
   `docs/verification/`.

## Where things live

- The public interface: `crates/turtle-reasoner/src/lib.rs` (the
  `Reasoner`, `Profile`, `Format`, `Summary`, `Diagnostic`, `Error`).
  Everything else in the crate is `pub(crate)`. Add to the interface only
  when an adapter (CLI, Python, oracle, bench) needs it, and say so in the
  crate doc.
- Internal modules: `term.rs` (term ids, dictionary), `io.rs` (parse,
  write), `engine/` (relations, rounds, joins; knows no rules),
  `rules/` (one file per rule family, registered one line each in
  `rules/mod.rs`), `diagnostics.rs`, `reasoner.rs`.
- Adapters: `crates/cli` (the `turtle-reasoner` binary), `crates/python`.
- Verification: `crates/oracle` (the oracles), `crates/reference` (the
  reference evaluator, the generator, chaos), `crates/bench` (benchmarks).
- Oracle tools and data are fetched, never committed:
  `scripts/fetch-tools.sh` into `tools/`, `scripts/fetch-data.sh` into
  `data/`. The OWL-RL oracle runs from `oracles/owlrl/` with `uv`.

## Build and test

Release mode only: timing tests and budgets mean nothing in a debug build.
Keep every test run under two minutes (`timeout 120`); run one test file at
a time when a suite is longer.

```sh
cargo build --release --locked
cargo fmt --check
cargo clippy --release --locked --all-targets -- -D warnings
scripts/check-wasm.sh                                   # the core for wasm32
timeout 120 cargo test --release --locked -p turtle-reasoner
timeout 120 cargo test --release --locked -p turtle-reasoner-oracle
timeout 120 cargo test --release --locked -p turtle-reasoner-reference --test chaos
cargo run --release -p turtle-reasoner-bench -- --list
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
