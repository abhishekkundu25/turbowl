# 12: Competitors: VLog, Soufflé, GraphDB

Status: todo. Blocked by: 03, 13.

Each competitor is fetched and pinned into `tools/` by version and checksum, with no system install, as `docs/research/competitors-2026-11.md` says each can be. GraphDB comes in only if its licence allows publishing numbers. Each runs the composed program for a configuration as a subprocess. Its closure is checked against TurbOWL's, and its times and peak memory are recorded as Nemo's are. Where an engine runs in parallel, all-cores rows are added. Where it has its own OWL mode, that mode gets its own rows. A competitor that is not fetched is `skipped`.

## Acceptance
- [ ] `--list` shows each engine and whether it is fetched.
- [ ] On brick-small1 and lubm-1, every fetched competitor's closure count equals TurbOWL's in the default and the complete configuration.
- [ ] Unit tests for each adapter's output parsing and its error and timeout paths.
- [ ] The bench README says, for each competitor, its version, how it is run, and what its time includes.
