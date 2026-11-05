# 12: The CLI

Status: todo. Blocked by: 10.

`crates/cli`: the `turbowl` binary: input files, `-o` (default stdout), `--format`, `--inferred-only`, `--diagnostics text|json|ndjson`, `--fail-on <severity>`, `--max-diagnostics`, `--summary-only`. Logs to stderr. Exit codes: 0 success, 1 I/O or parse error, 2 a diagnostic at or above `--fail-on`, 64 usage error.

## Acceptance
- [ ] CLI tests for every flag and exit code; JSON output parses; output files equal the library's closure as sets.
