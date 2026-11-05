# 18: All cores: parallel parsing

Status: todo. Blocked by: 16, 17.

Large N-Triples inputs are split at line boundaries and parsed on separate threads into local term tables, then merged into the dictionary under the same feature as parallel rounds. Turtle stays sequential unless a measured split is found.

## Acceptance
- [ ] Parallel parsing gives the same dictionary contents and triples as sequential parsing on every test document and on generated large ones.
- [ ] Loading lubm-50 takes at most a third of the Baseline's time at the close-out.
- [ ] A `scaling_*` test at 1 and 12 threads.
