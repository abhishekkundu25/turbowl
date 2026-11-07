# 04: Close-out: explanations

Status: todo. Blocked by: 01, 02, 03.

The full validation, run by the orchestrator with nothing else running, then
the docs:

- **Reasoning unchanged**: the bench on the editor-size scenarios and
  lubm-50, default and complete, one thread and all cores, 5 runs per side
  alternating `main` and `explain`; times and peak memory within `main`'s
  spread. `git diff main` on `engine/` and `rules/` shows visibility and
  accessors only.
- The explain latencies from `--explain`, against the budgets.
- Every suite, the oracles (slow tier for Brick and soda hall), 100,000
  chaos iterations per shard with times against phase 2's.
- Docs: `README.md` (an Explanations section), `CHANGELOG.md`, `AGENTS.md`
  (where explain lives), the crate doc, the verification record
  `docs/verification/<date>-explanations.md` with raw data under
  `docs/verification/data/`, and this README's statuses and departures.
  The decisions taken without the owner are listed in the record.

## Acceptance
- [ ] Every budget in the spec met, or each gap stated with its numbers.
- [ ] The verification record and its raw data committed.
- [ ] README, CHANGELOG, AGENTS.md and the tickets README updated.
