# Footprint probe (loop iteration 7)

Reads, at marks inside one `reason()` on lubm-50, the process's footprint,
its lifetime peak (what the bench's `peak_rss_bytes` reads) and the bytes
the allocator holds in use. macOS only. Nothing here is landed: the marks
go into a copy of the source tree.

## Files

- `insert_marks.py`: adds `crates/turbowl/src/probe_marks.rs` (a hook) and
  calls to it in `engine/store.rs` of a copied tree. It finds each site by
  a line that exists both before and after iteration 7, and fails when one
  is missing. The marks are: `step_begins`/`step_ends`, `round_begins`, the
  first build of an object index (`index_build_begins`/`_ends`, table index
  and its pair count), and each table job's four phases (`sort`, `change`,
  `index_sort`, `index_change`, both in a parallel step and on one thread).
  An index phase's count is 0 for a table with no object index.
- `crate/`: the probe program (`src/main.rs`) and its manifest template.
  It loads the files, sets the hook, reasons once and prints the summary,
  the tables below and every mark (`M label tT nN ms footprint life used
  allocated`, MB). Its release profile is the repository's (fat LTO, one
  codegen unit, unwind).
- `build.sh <name> <repo> <commit> [patch]`: `git archive`s the commit
  from the repository (read only), applies the patch if given, adds the
  marks, and builds `work/<name>/probe/target/release/turbowl-probe`.
- `run.sh <runs> <all|one> <data dir> <side>...`: runs the sides
  alternately, each run its own process, into `out/probe-<side>-<threads>-<i>.txt`.
  The data dir holds `univ-bench.ttl` and `lubm-v1-50.ttl` (the bench's
  `target/bench-data` after a lubm-50 bench run).
- `summarize.py <out dir> <threads> <side>...`: per run, the lifetime peak,
  reasoning time and most in use; means over the runs of each large first
  index build, each large index sort, and each phase's footprint growth.

## Rerun iteration 7's comparison

```sh
cd <this folder>
REPO=<a TurbOWL checkout that has commit 820f8b1>
./build.sh before "$REPO" 820f8b1
./build.sh after "$REPO" 820f8b1 "$PWD/../e07.patch"
./run.sh 3 all "$REPO/target/bench-data" before after
python3 -I summarize.py out all before after
```

To probe another change, build its sides the same way: the commit it
starts from, and its diff as the patch.

## Reading the tables

- **INDEX BUILDS**: a first build runs on the caller's thread before the
  step's jobs start, so nothing else allocates between its two marks: its
  `+foot` and `+life` are the build's own. `MB` is the index's size
  (8 bytes a pair).
- **INDEX SORTS**: a job runs beside the other tables' jobs in a parallel
  step, so its window counts their allocations too. Read `+life` (the
  lifetime peak) as what the window added to the final peak.
- **PHASES**: `step k` is a step's two marks; `round k` runs from the step
  before it to the next.
- The marks sample: nothing reads the bytes in use between them.
  `malloc_zone_statistics` walks every zone, so a run with the probe is a
  little slower than a bench run.
- Tables are numbered in the order the store makes them. On lubm-50, `t0`
  is `rdf:type` (1,141,746 pairs at its first build) and `t15`
  `takesCourse` (1,421,819).
