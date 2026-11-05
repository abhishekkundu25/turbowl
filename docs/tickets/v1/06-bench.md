# 06: The benchmark

Status: done. Blocked by: 01.

`crates/bench`: a binary that runs scenarios for this reasoner and for `reasonable` (same process, same inputs): Brick plus small1, soda_hall and the building models; a generated LUBM-style university data set (own generator, sizes 1, 10, 50 universities) with the univ-bench ontology; an incremental stream (add one triple, remove one triple, 100 times). Records median, min, max over N runs, peak memory, closure size, to JSON; `--compare a.json b.json`; `--list`; `--only`. Prints the load average and warns when it is over 1.

## Acceptance
- [ ] Runs on `reasonable` alone with the stub, and writes a JSON result; the format is documented in `crates/bench/README.md`.
