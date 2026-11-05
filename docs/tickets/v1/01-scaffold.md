# 01: Scaffold and the interface

Status: todo. Blocked by: none.

The Cargo workspace (`crates/turtle-reasoner` core, `crates/cli`, `crates/python`, `crates/oracle`, `crates/reference`, `crates/bench`), `rust-toolchain.toml` (1.99.0), release profile (fat LTO, one codegen unit), `rustfmt.toml`, clippy lints, `.gitignore` (`target/`, `data/`, `tools/`, `.venv/`), `AGENTS.md` (entry point), `README.md` (stub), and the core's public interface as compiling stubs: `Reasoner`, `Profile`, `Format`, `Diagnostic`, `Summary`, `Error`; the internal module list with `term`, `io`, `engine`, `rules` (with the rule table), `diagnostics`, `reasoner`.

## Acceptance
- [ ] `cargo build --release --locked`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` pass.
- [ ] `cargo build -p turtle-reasoner --no-default-features --target wasm32-unknown-unknown` passes.
- [ ] The public interface is documented in `lib.rs` and listed in `AGENTS.md`.
