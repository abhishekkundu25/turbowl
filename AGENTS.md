# AGENTS.md

TurbOWL is a fast OWL 2 RL reasoner in Rust: one deep library
(`crates/turbowl`), a command line, Python bindings, and a
verification suite that judges it against several independent oracles.

## Start here

1. **What it is and the words it uses:** `CONTEXT.md`.
2. **Why it is built this way:** `docs/adr/` (engine, equality, ADR 0004:
   the W3C rules are the target, ADR 0005: switches, and ADR 0006: the
   scoreboard replaces `reasonable`).
3. **Where the work stands:**
   - Version 1, a fast OWL 2 RL reasoner, is done: `docs/specs/v1.md`
     (with each budget's result), tickets in `docs/tickets/v1/` (the
     README there has the table, the waves and the log of departures). It
     is `f9e3625` on `main`, and its Baseline numbers are
     `docs/verification/2026-11-06-v1.md`.
   - Version 2, the best OWL 2 RL reasoner, in three phases:
     `docs/specs/v2-1-scoreboard-and-rules.md`, `v2-2-speed.md` and
     `v2-3-removals.md`, tickets in `docs/tickets/v2/` (ADRs 0005 and
     0006). It was built on the `v2` branch and merged into `main`; version 1
     is the tag `v0.1.0`. Phase 1 (every W3C OWL 2 RL
     rule, the switches, the scoreboard) is closed:
     `docs/verification/2026-11-06-v2-phase-1.md` has its results and the
     gaps the owner accepted, and the tickets README its follow-ups. Phase
     2 (speed: own parsers, every core, snapshots, loop iterations 1 to 11)
     is closed with gaps: `docs/verification/2026-11-07-v2-phase-2.md`.
     The owner accepted the all-cores speed-up and memory gaps; the
     snapshot's 9.2 times against 10 and one-thread loading wait for the
     owner. Phase 3, removals (tickets 22 to 24), is planned and not started. Ticket 27, the
     full-size scoreboard race, is pending the owner.
   - After version 1, **the improvement loop** (rate, find, change,
     evaluate, decide, record): `docs/specs/experiments.md`. This is how the
     project improves; its table is the history, and its last section says
     how to continue cold.
4. **The newest verification record:** the latest file in
   `docs/verification/`.

## Where things live

- The public interface: `crates/turbowl/src/lib.rs` (the
  `Reasoner`, `Format`, `Summary`, `Diagnostic`, `Severity`, `Error`,
  `Switch`, `Switches`, `Threads`, and the `profile` module for the
  bench). The crate doc's switch table gives each switch's default and the
  reason for it.
  Everything else in the crate is `pub(crate)`. Add to the interface only
  when an adapter (CLI, Python, oracle, bench) needs it, and say so in the
  crate doc.
- Internal modules: `term.rs` (term ids, dictionary), `io.rs` and `io/`
  (parse, write; TurbOWL's own N-Triples and Turtle parsers, parsing in
  pieces, and `cross_check.rs`, their parity with Oxigraph's), `engine/`
  (relations, rounds, joins, `parallel.rs` for the threads, and
  `canon.rs`, the union-find of `owl:sameAs` whose representatives the
  store rewrites into; knows no rules), `rules/` (one file per rule
  family, registered one line each in `rules/mod.rs` with the switch it
  belongs to),
  `switch.rs` (the switches and their defaults), `snapshot.rs` (the
  snapshot format, documented there), `diagnostics.rs`, `reasoner.rs`.
- Adapters: `crates/cli` (the `turbowl` binary; `--help` and the crate
  doc in `src/main.rs` define its report, the JSON fields and the exit
  codes 0, 1, 2 and 64), `crates/python` (the `turbowl` Python package:
  `src/lib.rs` is the native module `turbowl._native`, `python/turbowl/`
  the API and its documentation, `tests/` the pytest suite, and
  `examples/minimal_cases.rs` the Rust side of its comparison on the
  minimal inputs).
- Verification: `crates/oracle` (the oracles; `src/differences.rs` lists
  the oracle faults), `crates/reference` (the reference evaluator, the
  generator, chaos), `crates/bench` (the benchmark and the scoreboard
  against Nemo, Soufflé and VLog).
- The rule program: `oracles/rules/`, one file per switch group in Nemo's
  syntax. The oracle crate composes it for a configuration and emits it
  for Nemo, Soufflé and VLog.
- Oracle and competitor tools and the data are fetched, never committed:
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
cargo clippy --release --locked --workspace --all-targets -- -D warnings
scripts/check-wasm.sh                                   # the core for wasm32
timeout 120 cargo test --release --locked -p turbowl
timeout 120 cargo test --release --locked -p turbowl-oracle
timeout 120 cargo test --release --locked -p turbowl-reference --test chaos
timeout 120 cargo test --release --locked -p turbowl-cli  # runs the built binary
(cd crates/python && uv sync && timeout 120 uv run pytest)  # builds with maturin into .venv
cargo run --release -p turbowl-bench -- --list
```

- A chaos failure prints `CHAOS_SEED`, `CHAOS_ITERS`, `CHAOS_ONLY` and
  `CHAOS_SWITCHES` to replay it and saves the input under `target/chaos/`.
  Fix what it finds, with a unit test on the reduced input; never
  `#[ignore]` a test or exempt a check.
- Every chaos iteration also runs the core on one thread and on all cores
  (every round parallel) and compares the two: the summary, the closure in
  read order and the diagnostics.
- 100,000 chaos iterations: run each of the four shard tests alone
  (`--exact`) with `CHAOS_ITERS=100000`; each shard holds 50,000. A shard
  takes about 35 to 42 minutes (phase 2's close-out: full shards about 35,
  incremental ones about 42, the four side by side). The four can run in
  parallel from one built test binary.
- The slow oracle tier runs with `ORACLE_SLOW=1`, one test at a time
  (`--exact`). The Brick variants and soda hall take under a minute each.
  `building_models` (108 models) takes about half an hour, the one test
  that cannot fit in two minutes.
- An oracle that is not installed is reported `skipped`; with `CI=true` it
  fails.
- `--workspace` builds the Python bindings too. Their build script (pyo3's)
  needs Python 3.10 or later on PATH.
- After a clean of ignored files deletes the built native module,
  `uv sync --reinstall-package turbowl` in `crates/python` builds it again.
- The bench runs TurbOWL on one thread by default; `--threads all` runs it
  on every core. The competitors run as they do by default in both modes.
- Benchmarks run only on a quiet machine (load average under 1), never next
  to a build or a test run.
- The loop judges the all-cores peak memory on 10 single runs per side,
  alternating the Baseline and the change, by medians and middle halves:
  the largest of the bench's 3 runs is noise on all cores.
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
