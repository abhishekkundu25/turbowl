# 12: CLI parity

Status: todo. Blocked by: 10.

`crates/cli`: the `turtle-reasoner` binary with `reasonable`'s arguments and options (input files, `-o` default `output.ttl`, `--error-format text|json|ndjson`, `--fail-on`, `--max-diagnostics`, `--summary-only`), plus `--profile owl-rl|reasonable` and `--inferred-only`. Logs to stderr, diagnostics to stdout; exit 2 for `--fail-on`, 64 for usage errors (D8), 1 for I/O and parse errors.

## Acceptance
- [ ] CLI tests for parity checklist item 17 and `reasonable`'s 5 CLI tests, comparing output with `reasonable`'s CLI where both run.
