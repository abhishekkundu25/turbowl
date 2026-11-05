# 03: One rule source for every engine

Status: done. Blocked by: 01, 02.

The W3C rule forms move from `oracles/nemo/owl2rl.rls` into one file per switch group (core included) under `oracles/rules/`, in Nemo's syntax. The oracle crate composes the program for a configuration and emits it for Nemo, Soufflé and VLog. The oracle and the bench run the composed Nemo program. This is the lever that keeps the competitors' programs from drifting apart.

## Acceptance
- [x] The default configuration's composed Nemo program gives version 1's results: the minimal, W3C and Brick oracle tests pass unchanged.
- [x] Golden tests for the Soufflé and VLog programs of the default and the complete configuration, written against each engine's documented syntax.
- [x] A test: every rule of the spec's table appears in exactly one group file.
- [x] Nemo runs the complete configuration, with its empty groups, without error.
