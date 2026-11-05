# 11: Switches in the CLI and Python

Status: done. Blocked by: 02.

The CLI takes `--switch NAME=on|off` (repeatable), `--complete`, and `--switches`, which lists each switch with its default. An unknown name is a usage error (exit 64). The JSON summary names the configuration. Python takes the same names (`Reasoner(switches={...})`, `complete=True`) and exposes the configuration; an unknown name raises `ValueError`. Names are parsed once, at each boundary.

## Acceptance
- [x] CLI tests for each flag, the exit code of an unknown name, and the summary's configuration field.
- [x] pytest: on the minimal inputs, Python's closure and diagnostics equal the Rust library's in the default and the complete configuration.
- [x] `--help` and the Python docs list the switches and say what each costs.
