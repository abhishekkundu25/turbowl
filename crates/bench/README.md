# turtle-reasoner-bench

Runs fixed scenarios for this repository's reasoner (both profiles) and for
`reasonable` (crates.io `=0.5.0-a1`, linked in), on the same input files, and
reports the median and spread of the times, the peak memory and the closure
size. Every speed claim, and every step of the improvement loop
(`docs/specs/experiments.md`), is judged by these numbers.

## Run it on a quiet machine

Run the benchmark only when the 1-minute load average is under 1 and nothing
else is building, testing or benchmarking. The bench prints the load average
before and after, and warns (it does not stop) when either is over 1. A
result taken with a warning can show a trend, but it does not support a claim.

```sh
scripts/fetch-data.sh                                    # Brick and the building models into data/
cargo run --release -p turtle-reasoner-bench -- --list   # the scenarios
cargo run --release -p turtle-reasoner-bench -- --json results.json
cargo run --release -p turtle-reasoner-bench -- --only lubm-1,lubm-10 --engines reasonable --runs 3
cargo run --release -p turtle-reasoner-bench -- --compare old.json new.json
```

| Option | Meaning |
|---|---|
| `--list` | Print the scenarios and stop. |
| `--only A,B` | Run only these scenarios. |
| `--engines A,B` | Run only these engines: `turtle-owl-rl`, `turtle-reasonable`, `reasonable`. |
| `--runs N` | Runs per scenario and engine (default 5; the improvement loop needs 5 or more). |
| `--changes N` | Changes in the incremental scenarios (default 100). |
| `--timeout SECS` | Stop a run after this long and record `timeout` (default 600). |
| `--data-dir DIR` | Where the fetched models are (default `data/` at the repository root). |
| `--json FILE` | Also write the results file described below. |
| `--compare OLD NEW` | Print per-scenario deltas between two results files. |

Build in release mode: a debug build prints a warning, and its numbers mean
nothing.

## How a run is measured

- Every run is a fresh process: the bench starts itself with a hidden
  `--one` mode, which loads the files, runs the scenario, and prints one JSON
  line. A fresh process gives each run its own peak memory and a cold
  allocator, the same for every engine.
- **Peak memory** is the child's peak resident set size (`getrusage`,
  `ru_maxrss`, which macOS reports in bytes and Linux in kilobytes; the
  bench converts to bytes). The child reads it just after the last
  `reason()`, before the closure checks, which hold their own copies. It
  includes the bench binary itself, about the same for every engine. The
  table's `B/triple` column (`peak_bytes_per_closure_triple` in the results
  file) is the largest peak over the closure size: the memory rubric of
  `docs/specs/experiments.md`.
- **Load time** covers loading every input file in order (each engine's own
  parser). **Reasoning time** covers the first `reason()`. Both are wall
  clock. The headline number of a full run is the reasoning time; the
  results file also has the load time and their sum.
- In the incremental scenarios, each change is timed from the change call
  through the next `reason()`. The headline number of a run is the median of
  its per-change times.
- Runs alternate between the engines (run 1 of each engine, then run 2, ...),
  so a drift in the machine falls on all of them.
- **Checks.** After each run, the child counts the distinct closure triples
  (by a 64-bit hash) and compares that count with what the engine reports:
  `Summary::base + Summary::inferred` for this reasoner, the length of
  `view_output()` for `reasonable`. A mismatch, a closure size that differs
  between runs of the same engine, or a failed run is a **problem**: it is
  printed, listed in the results file, and makes the bench exit 1. A
  timeout, a skipped scenario and the stub reasoner are results, not
  problems.
- While this reasoner is the stub (before ticket 10), its runs panic with
  `unimplemented!`. The child catches the panic and the engine is marked
  `unavailable`, with no numbers; `reasonable` still runs.

### How each engine makes a change

| Change | This reasoner | `reasonable` |
|---|---|---|
| Add a triple | `insert`, then `reason` (semi-naive from the stored closure) | `load_triples`, then `reason` (its incremental path) |
| Remove a triple | `remove`, then `reason` | `set_base_triples` with the base minus the triple, then `reason`. It has no single-triple removal: this is what its Python `update_graph` does, and any removal makes its next run a full one. The base copy it needs is taken once, outside the timings. |

## Scenarios

`--list` prints them. The models come from `data/` (fetched by
`scripts/fetch-data.sh`); the bench finds each file by name anywhere under
`data/`. When one is missing, the scenario is reported `skipped`, with the
file it looked for. The generated inputs are written to `target/bench-data/`
before any run (not timed) and reused; a change to a generator changes the
version in its file name.

| Scenario | Input | What it measures |
|---|---|---|
| `brick-small1` | `Brick.n3` + `small1.n3` | A full run on the Brick ontology with a tiny model: mostly the cost of the ontology's own closure. `reasonable`'s README logs this case. |
| `brick-soda-hall` | `Brick.n3` + `soda_hall.n3` | A full run on Brick with a real building model (about 3,800 triples). |
| `brick-buildings` | `Brick.n3` + every `.ttl` in the `buildings/` folder of `reasonable`'s Python benchmark, loaded together | A full run on Brick with every building model of `reasonable`'s own benchmark at once. |
| `lubm-1`, `lubm-10`, `lubm-50` | `univ-bench.ttl` + generated data for 1, 10, 50 universities (about 0.12 M, 1.3 M, 6.5 M triples) | Full runs on instance data that grows: class hierarchies, intersections with `someValuesFrom`, a transitive property, sub-properties, inverses, domains and ranges. Shows how the cost grows with the data. |
| `incremental-add` | `Brick.n3` + `small1.n3`, then 100 additions | The cost of one added triple against a stored closure. Change *k* copies small1's triple *k* (cycling) onto a new subject `http://example.org/bench/added/k`, so each is a new entity of a real Brick class or relation. |
| `incremental-remove` | `Brick.n3` + `small1.n3`, then 100 removals | The cost of one removed triple. The removals are base triples spread evenly through the input files in order (deduplicated), removed one after another. |
| `sameas-clique` | generated: `a0 owl:sameAs a1`, ..., `a998 owl:sameAs a999`, and 10 triples on each alias | Equality: the clique has 1,000,000 `owl:sameAs` triples and each alias gets every alias's 10 triples, about 11 M closure triples once expanded. |

### LUBM-style data

The LUBM scenarios use this crate's own generator (`src/lubm.rs`), shaped
after the Lehigh University Benchmark's data generator (UBA): universities
with 15 to 25 departments; each department with research groups, full,
associate and assistant professors and lecturers in UBA's ranges, courses
and graduate courses, publications by rank, undergraduate students (8 to 14
per faculty member) and graduate students (3 to 4 per faculty member) who
take courses, have advisors, and are teaching or research assistants. The
random choices come from a fixed seed, so the same size gives the same
bytes everywhere. The ontology is univ-bench's class and property axioms,
written out by hand as Turtle in the same file, without the two axioms
OWL 2 RL does not allow (an existential on the right of a subclass axiom).
Names are neutral (`http://example.org/univ-bench#`,
`http://example.org/lubm/`).

**This is LUBM-style data, not the official UBA output.** Compare its numbers
only with other runs of this generator, never with published LUBM figures.

## The results file (`--json`)

Schema version 1. Times are in milliseconds, memory in bytes.

```jsonc
{
  "schema": 1,
  "meta": {
    "date": "2026-11-05T10:00:00Z",          // UTC, when the file was written
    "commit": "fcca1da…", "dirty": false,     // the repository's HEAD; tracked files changed?
    "cpu": "Apple M6", "cores": 12, "memory_bytes": 34359738368,
    "os": "macOS 27.0 (kernel 27.0.0, aarch64)",
    "build": "release",
    "load_before": [0.42, 0.61, 0.70],        // 1, 5, 15 minute load averages
    "load_after": [0.95, 0.70, 0.72],
    "runs": 5, "changes": 100, "timeout_s": 600,
    "data_dir": "/…/data",
    "versions": { "bench": "0.1.0", "turtle_reasoner": "0.1.0",
                  "reasonable": "0.5.0-a1", "rustc": "rustc 1.99.0 (…)" }
  },
  "results": [
    {
      "scenario": "lubm-1",
      "engine": "reasonable",                 // turtle-owl-rl | turtle-reasonable | reasonable
      "status": "ok",                         // ok | unavailable | skipped | timeout | error
      "message": null,                        // why, when not ok
      "inputs": ["/…/univ-bench.ttl", "/…/lubm-v1-1.ttl"],
      "summary": {                            // null when no run finished
        "measure": "reason_ms",               // or "change_median_ms" (incremental-*)
        "median_ms": 70.1, "min_ms": 68.9, "max_ms": 74.0,   // of the measure, over the runs
        "load_median_ms": 222.5,
        "reason_median_ms": 70.1,             // the first reason() (the full run)
        "total_median_ms": 292.6,             // load + first reason(), per run
        "peak_rss_bytes": 268468224,          // the largest peak of the runs
        "peak_rss_min_bytes": 266000000,      // the smallest
        "closure": 204521,                    // closure triples (base + inferred), last run
        "peak_bytes_per_closure_triple": 1312.7 // peak_rss_bytes / closure; null when the closure is 0
      },
      "runs": [                               // one per finished run, in order
        { "status": "ok", "load_ms": 222.5, "reason_ms": 70.1,
          "change_ms": [],                    // per change, in order (incremental-*)
          "peak_rss_bytes": 268468224,
          "closure": 204521,                  // what the engine reports
          "closure_counted": 204521 }         // distinct triples counted
      ]
    }
  ],
  "problems": []                              // strings; non-empty means exit 1
}
```

A run that ends early keeps the runs before it: `status` is the first
non-`ok` status, `runs` holds the finished ones.

## Comparing two results files

`--compare OLD NEW` prints, for each scenario and engine in the new file, the
old and new medians, the change in percent, both ranges (min to max), and
the peak memory. It flags a change only when the two ranges do not overlap
(`SLOWER` or `faster`, `MORE` or `less` memory): a median that moved inside
the spread is noise until more runs say otherwise. It warns when the CPU,
the OS, the build or the change count differ, or when either run started
with a load average over 1, and prints a changed closure size. It reports
only and exits 0.

## The budgets these numbers judge

From `docs/specs/v1-parity.md`: this reasoner's full-run time is below
`reasonable`'s on every scenario (the `x reasonable time` column under 1),
its peak memory is below `reasonable`'s on every scenario
(`x reasonable mem` under 1), and an incremental addition of one triple
costs under 10% of a full run (`incremental-add`'s median against
`brick-small1`'s).
