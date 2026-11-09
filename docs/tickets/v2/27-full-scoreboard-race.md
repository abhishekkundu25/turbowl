# 27: The full-size scoreboard race

Status: todo, pending the maintainer. Blocked by: 21.

Phase 2's close-out ran a short race at the maintainer's request (2026-11-07):
the editor-size scenarios only. This ticket holds the full-size one for
later: every scoreboard scenario, the data-size ones included (uobm-010-l,
lubm-uba-10-l, owl2bench-rl-1 and -2, uobm-010-u, owl2bench-rl-5,
lubm-uba-100-l, claros-l), in the default and the complete configuration,
TurbOWL on one thread and on all cores, nothing over 16 GB, on a quiet
machine if the maintainer can give one. Phase 1's session took about 4 hours on
this machine, most of it the competitors' slow runs and timeouts. GLog and a
multi-core build of Soufflé may join it if the maintainer says so (the maintainer said
not now on 2026-11-07).

## Acceptance
- [ ] The scoreboard record holds every engine, scenario, axis, configuration and thread mode, each number with its run count, spread and limiter.
- [ ] Every competitor's closure equals TurbOWL's, or the difference is a listed competitor fault.
- [ ] The bench stops a competitor run once it passes 3 times TurbOWL's time on the same scenario and records it as more than 3 times slower (the maintainer, 2026-11-07).
