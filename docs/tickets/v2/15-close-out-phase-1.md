# 15: Close-out: phase 1

Status: done on 2026-11-06, with the gaps the record lists
(`docs/verification/2026-11-06-v2-phase-1.md`). Blocked by: 01 to 14
(done), and version 1's ticket 14 (done).

Full validation with nothing else running: every suite in both configurations, the slow oracle tier, 100,000 chaos iterations of each kind, and the W3C suites in the complete configuration. The defaults of `identity` and `schema` are set by ADR 0005's rule, from the research and a quiet-machine measurement. One scoreboard run on a quiet machine, with the data-size scenarios included and nothing over 16 GB. README, CHANGELOG, AGENTS and CONTEXT are brought up to date. A verification record goes in `docs/verification/`, and the log goes in this README.

## Acceptance
- [x] Every budget of the phase 1 spec passes, or the gap is listed and the maintainer is told before phase 2 starts. The record's Budgets and Gaps list each one, and the maintainer was told of each. The correctness budgets pass on the final code, `660e3bd`: every oracle in both configurations, the 108 building models, and 100,000 chaos iterations of each kind.
- [x] The W3C OWL 2 RL suite passes at least 101 cases in the complete configuration (the maintainer's budget since 2026-11-06); each of the 18 failures is listed with why rules cannot pass it. 101, equal to Nemo. Fifteen are beyond the rules; three are matters of how the runner reads a case (an import, and annotations under the Direct Semantics), listed as such.
- [ ] The scoreboard record holds every engine, scenario, axis, configuration and thread mode, each number with its run count, spread and limiter, and the RDFox estimates with their bridges. Done in part: the maintainer stopped the session after 4 hours ("record what you have measured and move on"). It holds the old defaults (`schema` and `identity` on) on all 16 scenarios of the session, one thread, and the complete configuration on the editor-size scenarios and three data-size ones. It lacks the new default against the competitors, the complete configuration on claros-l, uobm-010-u, owl2bench-rl-5 and lubm-uba-100-l, and a quiet machine (load about 2). No engine runs on all cores yet, and no RDFox estimate passes the bridge's conditions; the record gives the reasons.
- [x] The switch defaults are recorded with the measurement behind each. `schema` and `identity` are off by ADR 0005's speed rule; the research alone would have kept both on. Measured at a load of about 2, by the maintainer's choice: the gaps are many times the spread between runs.
