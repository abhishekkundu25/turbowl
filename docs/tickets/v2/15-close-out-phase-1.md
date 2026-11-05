# 15: Close-out: phase 1

Status: todo. Blocked by: 01 to 14, and version 1's ticket 14.

Full validation with nothing else running: every suite in both configurations, the slow oracle tier, 100,000 chaos iterations of each kind, and the W3C suites in the complete configuration. The defaults of `identity` and `schema` are set by ADR 0005's rule, from the research and a quiet-machine measurement. One scoreboard run on a quiet machine, with the data-size scenarios included and nothing over 16 GB. README, CHANGELOG, AGENTS and CONTEXT are brought up to date. A verification record goes in `docs/verification/`, and the log goes in this README.

## Acceptance
- [ ] Every budget of the phase 1 spec passes, or the gap is listed and the owner is told before phase 2 starts.
- [ ] The W3C OWL 2 RL suite passes at least 113 cases in the complete configuration; each failure is listed with why rules cannot pass it.
- [ ] The scoreboard record holds every engine, scenario, axis, configuration and thread mode, each number with its run count, spread and limiter, and the RDFox estimates with their bridges.
- [ ] The switch defaults are recorded with the measurement behind each.
