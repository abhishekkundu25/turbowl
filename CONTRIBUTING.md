# Contributing

Issues and pull requests are welcome. Report a bug with the smallest input
that shows it, the command or code you ran, and what you expected.

Start with `CONTEXT.md`, which defines the words the project uses, and
`docs/adr/`, which records why it is built this way. `AGENTS.md` has the
full build, test and verification rules.

## Build and test

You need Rust 1.99. `rust-toolchain.toml` pins it, so `rustup` installs it
on the first build. These commands need nothing else:

```sh
cargo build --release --locked
cargo fmt --check
cargo clippy --release --locked -p turbowl -p turbowl-cli --all-targets -- -D warnings
cargo test --release --locked -p turbowl -p turbowl-cli
scripts/check-wasm.sh
```

The CI runs these on every push and pull request.

Build and test in release mode only. If a `scaling_*` test fails on a busy
machine, run it again on its own before you believe it:

```sh
cargo test --release --locked -p turbowl -- scaling_<name> --test-threads=1
```

The Python package needs Python 3.10 or later and `uv`:

```sh
cd crates/python && uv sync && uv run pytest
```

## The oracle, chaos and benchmark suites

Correctness is checked by independent oracles: Nemo, OWL-RL, HermiT, the
W3C tests and a reference evaluator. A change that makes any oracle
disagree is fixed or reverted. Correctness never trades for speed.

These suites need tools and data that are fetched, never committed:

```sh
scripts/fetch-tools.sh   # Nemo, HermiT, a JRE, and the bench's competitors, into tools/
scripts/fetch-data.sh    # the verification and benchmark data, into data/
```

Then:

```sh
cargo test --release --locked -p turbowl-oracle
cargo test --release --locked -p turbowl-reference --test chaos
cargo run --release -p turbowl-bench -- --list
```

A missing oracle is skipped locally. With `CI=true` it fails the run, so
keep these suites out of a CI job that has not fetched the tools.

A chaos failure prints the environment variables that replay it. Fix it
with a unit test on the reduced input.

## Style

- `cargo fmt` and clippy with `-D warnings` must pass.
- Never `#[ignore]` a test or exempt a check.
- The public interface is `crates/turbowl/src/lib.rs`. Everything else is
  `pub(crate)`. Add to it only when an adapter needs it.
- Fixtures use neutral names (`ex:`, `http://example.org/`).
- Prose is short plain sentences, with sentence-case headings.
- A speed change is measured before and after on a quiet machine, and
  recorded in `docs/specs/experiments.md`, kept or reverted.
