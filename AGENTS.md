# AGENTS.md

TurbOWL is a fast OWL 2 RL reasoner in Rust: the library `crates/turbowl`,
a CLI, Python bindings, and a verification suite that judges it against
independent oracles.

## Start here

- Words: `CONTEXT.md`. Decisions: `docs/adr/`.
- Status: each spec in `docs/specs/` has its tickets in
  `docs/tickets/<name>/` (the README holds the status table and the
  departures) and its record in `docs/verification/`, which ends with what
  is open for the maintainer.
- Improving: the loop in `docs/specs/experiments.md`.

## Boundaries

- The public interface is `crates/turbowl/src/lib.rs`; everything else is
  `pub(crate)`. Add to it only when an adapter needs it, and say so in the
  crate doc.
- `engine/` knows no rules, and neither `engine/` nor `rules/` knows
  `explain/`: reasoning records nothing for explanations.
- The rule program is `oracles/rules/` (Nemo syntax). The oracles and the
  explainer both read it.
- Tools and data are fetched, never committed: `scripts/fetch-tools.sh`,
  `scripts/fetch-data.sh`.

## Build and test

```sh
cargo build --release --locked
cargo fmt --check
cargo clippy --release --locked --workspace --all-targets -- -D warnings
scripts/check-wasm.sh
timeout 120 cargo test --release --locked -p turbowl
timeout 120 cargo test --release --locked -p turbowl-oracle
timeout 120 cargo test --release --locked -p turbowl-reference --test chaos
timeout 120 cargo test --release --locked -p turbowl-cli
(cd crates/python && uv sync && timeout 120 uv run pytest)
```

- Release mode only. Every run under `timeout 120`, one test file at a time
  when a suite is longer.
- A failing `scaling_*` test on a loaded machine: rerun it alone
  (`-- scaling_<name> --test-threads=1`) before believing it.
- A chaos failure prints the env vars to replay it. Fix it with a unit test
  on the reduced input.
- 100,000 chaos iterations: each of the four shard tests alone (`--exact`)
  with `CHAOS_ITERS=100000`, about 1.5 to 2 hours each.
- The slow oracle tier: `ORACLE_SLOW=1`, one test at a time (`--exact`).
- `--workspace` needs Python 3.10 or later on PATH.
- Benchmarks only at load average under 1, never beside a build or test.
- Every full verification is stored: `docs/verification/<date>-<name>.md`,
  raw results in `docs/verification/data/`.

## Rules

- Correctness never trades for speed. A change that makes any oracle
  disagree is reverted or fixed.
- Never `#[ignore]` a test or exempt a check.
- One change per experiment, recorded in `docs/specs/experiments.md`,
  including the reverted ones.
- Commits carry the maintainer's name only, with no co-author
  lines.
- Nothing is published (crates.io, PyPI, GitHub) without the maintainer's
  approval.
- Fixtures use neutral names (`ex:`, `http://example.org/`).
