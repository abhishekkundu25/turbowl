# 04: Verification by configuration

Status: done. Blocked by: 03.

The reference evaluator honours the switches, and its rules are split into one file per group. The generator and the minimal inputs are split the same way, with an empty file for each new group. The oracle suite runs every input in the default and the complete configuration, and the W3C suites run in the complete one. Chaos draws each iteration's configuration from the seed, and its replay line names the configuration. The W3C report's map from vocabulary to likely rules covers every rule.

## Acceptance
- [x] The minimal, W3C and Brick oracle tests pass in both configurations.
- [x] reference = Nemo on the minimal inputs and 150 generated ones, in both configurations.
- [x] Chaos compares 800 iterations by default, across configurations, and a failure prints `CHAOS_SEED`, `CHAOS_ITERS`, `CHAOS_ONLY` and the configuration.
- [x] A test checks that every group has its file in the reference evaluator, the generator, the minimal inputs and `oracles/rules/`.
