# 01: Remove `reasonable`

Status: done. Blocked by: None.

The bench runs TurbOWL and Nemo only. The `reasonable` crate leaves the dependency tree, with its engine adapter, budgets and docs (ADR 0006). Its repository stays a data source: Brick and the building models come from it. A results file with `reasonable` rows still loads, and `--compare` says it skipped those rows. `docs/specs/experiments.md` uses the new words: a competitor is another engine, the Baseline is the kept version, and Speed 10 means first on the scoreboard.

## Acceptance
- [x] `cargo tree -p turbowl-bench` shows no `reasonable`, and `git grep -nw reasonable -- crates/bench` finds only data-source mentions.
- [x] A bench test: `--compare` on `docs/verification/data/2026-11-05-wave-3/bench-3252f68.json` and a new file reports the skipped `reasonable` rows and compares the rest.
- [x] `cargo run --release -p turbowl-bench -- --list` shows two engines; the bench's tests pass.
- [x] `docs/specs/experiments.md` names the scoreboard in its rubric and uses "Baseline" only for the kept version.
