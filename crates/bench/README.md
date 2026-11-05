# turbowl-bench

Runs fixed scenarios for this repository's reasoner and its competitor Nemo
(`CONTEXT.md`, ADR 0006), on the same input files, and reports the median and
spread of the times, the peak memory, the closure size and each engine's ratio
to this reasoner. Every speed claim, and every step of the improvement loop
(`docs/specs/experiments.md`), is judged by these numbers. Its scenarios
include the public datasets the RDFox papers measured
([Public datasets](#public-datasets)), and no run passes the memory cap
(16 GB by default; ADR 0006).

| Engine | What runs | How |
|---|---|---|
| `turbowl` | this repository's `Reasoner` (one rule set) | in process |
| `nemo` | the pinned `nmo` 0.10.1 (`tools/nemo/`, from `scripts/fetch-tools.sh`) with `oracles/nemo/owl2rl.rls`, the W3C forms of the v1 rule set | a subprocess. Without `tools/nemo` (or at another version) it is reported `skipped`. Full runs only: the incremental scenarios are `unsupported` (Nemo runs a program once over its imports; it has no change stream). |

## Run it on a quiet machine

Run the benchmark only when the 1-minute load average is under 1 and nothing
else is building, testing or benchmarking. The bench prints the load average
before and after, and warns (it does not stop) when either is over 1. A
result taken with a warning can show a trend, but it does not support a claim.

```sh
scripts/fetch-data.sh                                    # Brick, the building models and the small datasets into data/
scripts/fetch-data.sh --lubm 10,100                      # LUBM data from UBA, on demand
cargo run --release -p turbowl-bench -- --list   # the scenarios, the papers' counts and the engines
cargo run --release -p turbowl-bench -- --json results.json
cargo run --release -p turbowl-bench -- --only lubm-1,lubm-10 --engines nemo --runs 3
cargo run --release -p turbowl-bench -- --compare old.json new.json
```

| Option | Meaning |
|---|---|
| `--list` | Print the scenarios and the engines, and stop. |
| `--only A,B` | Run only these scenarios. The opt-in ones (`sameas-clique` and every public dataset) run only this way. |
| `--engines A,B` | Run only these engines: `turbowl`, `nemo`. |
| `--runs N` | Runs per scenario and engine (default 5; the improvement loop needs 5 or more). |
| `--changes N` | Changes in the incremental scenarios (default 100). |
| `--timeout SECS` | Stop a run after this long and record `timeout` (default 600). Nemo is stopped by the child at this time; the child itself gets 30 s more. |
| `--memory-cap GB` | Stop a run whose memory passes this many GB (2^30 bytes) and record `over-memory` (default 16). |
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
- **Peak memory** is a peak resident set size (`getrusage`, `ru_maxrss`,
  which macOS reports in bytes and Linux in kilobytes; the bench converts to
  bytes). For this reasoner it is the child's own (`RUSAGE_SELF`), read
  just after the last `reason()`, before the closure checks, which hold
  their own copies; it includes the bench binary itself. For Nemo it is
  `nmo`'s (`RUSAGE_CHILDREN` of the child, read after waiting for `nmo`, its
  only child): Nemo's whole process, output writing included. The table's `B/triple` column (`peak_bytes_per_closure_triple` in the results
  file) is the largest peak over the closure size: the memory rubric of
  `docs/specs/experiments.md`.
- **Load time** covers loading every input file in order (each engine's own
  parser). **Reasoning time** covers the first `reason()`. Both are wall
  clock. The headline number of a full run is the reasoning time; the
  results file also has the load time and their sum (`total`). See
  [What each engine's time includes](#what-each-engines-time-includes).
- In the incremental scenarios, each change (`insert` or `remove` of one
  triple) is timed from the change call through the next `reason()`, which
  works semi-naively from the stored closure. The headline number of a run
  is the median of its per-change times.
- Runs alternate between the engines (run 1 of each engine, then run 2, ...),
  so a drift in the machine falls on all of them.
- **The memory cap.** Every 20 ms the bench sums the memory held by the
  run's processes: the child and everything it started (Nemo's `nmo`),
  found with `proc_listchildpids` on macOS and `/proc/PID/task/*/children`
  on Linux. Held means in RAM or not: the physical footprint on macOS
  (`proc_pid_rusage`, `ri_phys_footprint`), `VmRSS` plus `VmSwap` on Linux.
  The resident size alone is not enough: under memory pressure macOS
  compresses a process's pages, and one smoke run held 69 GB while its
  resident size read 2 GB. Once the sum passes `--memory-cap`, the bench
  stops the child and each of those processes by its PID (`SIGKILL`) and
  records `over-memory`, with the memory it saw. Like a timeout, that is a
  result, not a problem; it records no time and stops that engine's later
  runs of the scenario. Memory allocated between two readings can take a
  run a little past the cap before it is stopped. A parent-side timeout
  stops the same processes.
- **Checks.** After each run, the child counts the distinct closure triples
  (by a 64-bit hash) and compares that count with what the engine reports:
  `Summary::base + Summary::inferred` for this reasoner, the number of
  lines Nemo wrote to `closure.nt` for Nemo (parsed back with `oxttl`,
  outside the timings). A mismatch, a closure size that differs between
  runs of the same engine, or a failed run is a **problem**: it is printed,
  listed in the results file, and makes the bench exit 1. A timeout, a run
  over the memory cap, a skipped scenario and an unsupported one are
  results, not problems.
- A panic in this reasoner is caught by the child and recorded as an
  `error` with its message: a problem.
- Every engine's closure is the full one on every scenario, `owl:sameAs`
  included (ticket 11), so the times compare. This reasoner keeps one
  representative per `owl:sameAs` class (ADR 0003): its reasoning time and
  stored triples grow with the input, and the expanded closure (what
  `triples()` gives and the check counts) is built only when read, never
  stored. On `sameas-clique` the peak (read before the checks) follows the
  10,003 stored triples, not the 11 M expanded ones, so its `B/triple`
  is under 1.

### What each engine's time includes

The engines do not split their work at the same places, so compare the
columns that mean the same thing:

| | `turbowl` | `nemo` |
|---|---|---|
| load | parsing every file into the reasoner (`load_file`) | Nemo's `Data import` (its own report, `--report short`, whole milliseconds) |
| reasoning (the headline) | the first `reason()`: rules to fixpoint and the closure materialised in memory | Nemo's `Reasoning`: rules to fixpoint. Nemo loads tables lazily, so part of its parsing can fall here. |
| not in load or reasoning | process start, the closure checks | process start, reading the program, writing `closure.nt` and the finding CSVs (`Data export`, in `export_median_ms`), process exit; all of it is in `wall_median_ms` |
| peak memory | the bench child, engine included | the `nmo` process, export included |

So `total` (load plus reasoning) is the fairest single comparison of the
two; the headline compares reasoning alone, where Nemo may carry some of
its parsing. Nemo's wall clock is recorded but not compared. Nemo also reads
the files with its own parser: it interprets typed literals as values (it
would write `"01"^^xsd:integer` as `"1"^^xsd:integer`), so its closure size
can differ from the others' by literals that are equal as values.

The program Nemo runs is `oracles/nemo/owl2rl.rls` with its one
`@import input` line replaced by one import per input file, in order
(`turtle` for `.ttl` and `.n3`, `ntriples` for `.nt`), written with the
results under `target/bench-nemo/<pid>/` and removed after the run.

Since `owl2rl.rls` binds variable predicates through the properties the
schema names and joins cax-eqc2 like cax-eqc1, Nemo finishes every LUBM
size. In one run on 2026-11-05, on a heavily loaded machine (1-minute load
average 175 before, 63 after), its reasoning took 1.1 s on `lubm-1`, 10.7 s
on `lubm-10` and 52 s on `lubm-50`. A Nemo run over `--timeout` is a
`timeout` result, not a problem; lower `--timeout` to keep a run short.

## Scenarios

`--list` prints them. The models come from `data/` (fetched by
`scripts/fetch-data.sh`); the bench finds each file by name anywhere under
`data/`, and each public dataset's files at the paths that script gives
them. When one is missing, the scenario is reported `skipped`, with the
file it looked for (for a dataset: `not fetched`, and the script's options
that fetch it). The generated inputs are written to `target/bench-data/`
before any run (not timed) and reused; a change to a generator changes the
version in its file name.

| Scenario | Input | What it measures |
|---|---|---|
| `brick-small1` | `Brick.n3` + `small1.n3` | A full run on the Brick ontology with a tiny model: mostly the cost of the ontology's own closure. `reasonable`'s README logs this case. |
| `brick-soda-hall` | `Brick.n3` + `soda_hall.n3` | A full run on Brick with a real building model (about 3,800 triples). |
| `brick-buildings` | `Brick.n3` + every `.ttl` in the `buildings/` folder of `reasonable`'s Python benchmark, loaded together | A full run on Brick with every building model of `reasonable`'s own benchmark at once. |
| `lubm-1`, `lubm-10`, `lubm-50` | `univ-bench.ttl` + generated data for 1, 10, 50 universities (about 0.12 M, 1.3 M, 6.5 M triples) | Full runs on instance data that grows: class hierarchies, intersections with `someValuesFrom`, a transitive property, sub-properties, inverses, domains and ranges. Shows how the cost grows with the data. |
| `incremental-add` | `Brick.n3` + `small1.n3`, then 100 additions | The cost of one added triple against a stored closure. Change *k* copies small1's triple *k* (cycling) onto a new subject `http://example.org/bench/added/k`, so each is a new entity of a real Brick class or relation. |
| `incremental-remove` | `Brick.n3` + `small1.n3`, then 100 removals | The cost of one removed triple. The removals are base triples without a blank node, spread evenly through the input files in order (deduplicated), removed one after another. A triple with a blank node is left out: parsed again for the change stream, its blank node is a new node to this reasoner (blank nodes are scoped per document), so removing it would remove nothing. |
| `sameas-clique` (opt-in) | generated: `a0 owl:sameAs a1`, ..., `a998 owl:sameAs a999`, and 10 triples on each alias | Equality: the clique has 1,000,000 `owl:sameAs` triples and each alias gets every alias's 10 triples, about 11 M closure triples once expanded. Not in the default run: run it with `--only sameas-clique`. |

### Public datasets

The datasets the RDFox papers measured, and OWL2Bench, each an opt-in
scenario per program it ships with (`src/datasets.rs`;
`docs/research/competitors-2026-11.md` §8). A name ends in its program: `-l`
(the OWL 2 RL part of the ontology), `-le` (L plus hand-written chain rules)
or `-u` (the upper bound), loaded as Oxford's OWL form of that program
(RDF/XML) before the data. `--list` prints each with the papers' fact counts
before and after reasoning (§1.2). Where a paper handles equality two ways,
AX counts every `owl:sameAs` consequence, as this bench's closure does, and
REW one representative per class. A paper's count is for its own Datalog
program; this bench runs the W3C rules on the OWL form, so the two can
differ (see `claros-le` below).

| Dataset | Source | Licence | Download | On disk | Fits 16 GB (TurbOWL, 2026-11-06) |
|---|---|---|---|---|---|
| Claros | Oxford, AAAI 2014: `krr-nas.cs.ox.ac.uk/2014/AAAI/RDFox/Claros.zip`, programs under `cs.ox.ac.uk/isg/tools/RDFox/2014/AAAI/input/Claros/` | none stated for the data (the CLAROS project called its material open to anyone for anything) | 169 MB | 3.2 GB | yes: L, LE, U |
| UOBM, 10 universities | Oxford, AAAI 2014: `UOBM-010.zip`, programs under `input/UOBM/` | none stated | 15 MB | 0.37 GB | L yes; U no |
| LUBM programs | Oxford, AAAI 2014: `input/LUBM/` | none stated | 0.1 MB | 0.1 MB | (with the data below) |
| OpenCyc | Oxford, AAAI 2015: `krr-nas.cs.ox.ac.uk/2015/AAAI/RDFox/Equality/OpenCyc.zip` | CC BY 3.0 (stated in the file) | 24 MB | 0.18 GB | no |
| OWL2Bench RL, 1, 2, 5 universities | Zenodo record 4764368 | CC BY 4.0, for reasoner benchmarking; generator Apache-2.0 | 48 MB | 48 MB | 1 yes (12.6 GB); 2 and 5 no |
| LUBM from UBA 1.7 | `swat.cse.lehigh.edu/projects/lubm/uba1.7.zip`, run with `--lubm N` | generator GPL v2 or later; its output carries none | 35 KB | 105 MB at 10 universities, 1.1 GB at 100 | 10 and 100 yes; 500 not run (91 M closure published: marginal) |
| LUBM, 1,000 universities (`--large`) | Oxford, AAAI 2014: `LUBM-01K.zip` | none stated | 0.73 GB | 23.9 GB | not fetched; L (182.4 M) expected over the cap |
| UOBM, 1,000 universities (`--large`) | Oxford, AAAI 2014: `UOBM-01K.zip` | none stated | 1.86 GB | 45.6 GB | not fetched; expected over |
| DBpedia 2014 subset (`--large`) | Oxford, AAAI 2014: `DBpedia.zip`, programs under `input/DBpedia/` | none stated for the subset; DBpedia is CC BY-SA 3.0 and GFDL | 1.35 GB | 15.9 GB | not fetched; L (118.3 M) marginal, LE over |
| UniProt (`--large`) | Oxford, AAAI 2015: `UniProt.zip` | none stated for the subset; UniProt is CC BY 4.0 | 2.02 GB | 27.4 GB | not fetched |

The script pins each download by SHA-256, except the four `--large` zips,
which this project has not downloaded: each is pinned by the CRC-32 its zip
records for the data file, read from the zip's central directory, and the
data is tested against it before unpacking. `Claros.ttl` is repaired as it
is unpacked: 84 of its lines have an IRI holding a backslash, a backtick or
a `%` that starts no escape, which a strict parser rejects (TurbOWL stopped
at line 8,795,709). The script percent-encodes those characters in IRIs
only (23 IRIs, none of whose encoded forms the file already holds), so
every triple stays and none merge: 18,793,277 distinct triples, as the
paper's 18.8 M.

Not in the bench: the AIJ 2019 programs (ChEMBL, Reactome, UniProt, UOBM)
and its DBpedia subset, whose links return 404, and UniProt's OWL form,
which never shipped (`uniprot` loads the data alone, so its closure is not
the paper's).

**Smoke runs, not measurements.** One run of TurbOWL per fetched scenario
on 2026-11-06, `--engines turbowl --runs 1`, on a busy machine (1-minute
load average 2.6 to 12.6). "Before" is the distinct input triples, counted
apart from the run; "after" is the bench's closure. The peak is the
child's peak resident size, which under memory pressure is below what it
held: `claros-u` held 9.9 GB, 5.4 GB of it compressed, against a 7.6 GB
peak.

| Scenario | Papers: before -> after | Smoke run: before -> after | Peak | Load, reasoning |
|---|---|---|---|---|
| `claros-l` | 18.8 M -> 95.5 M (AAAI 2014); -> 73.8 M (AIJ 2019) | 18,795,367 -> 91,006,354 | 7.7 GB | 14.6 s, 15.0 s |
| `claros-le` | 18.8 M -> 555.1 M (AAAI 2014) | 18,795,512 -> 225,618,509 | 11.5 GB | 14.1 s, 23.2 s |
| `claros-u` | none | 18,796,339 -> 233,417,610 | 7.4 GB | 20.4 s, 53.4 s |
| `uobm-010-l` | none | 2,217,941 -> 4,602,841 | 0.7 GB | 1.7 s, 1.0 s |
| `uobm-010-u` | 2.2 M -> 35.6 M (AAAI 2014) | 2,218,139 -> over-memory | over 16 GB | |
| `opencyc` | 2.4 M -> 1,176 M AX, 142 M REW (AAAI 2015) | 2,413,894 -> over-memory | over 16 GB | |
| `owl2bench-rl-1` | none (ABox 50,131 axioms) | 54,910 -> 1,449,561 | 12.6 GB | 0.1 s, 26.8 s |
| `owl2bench-rl-2` | none | 107,367 -> over-memory | over 16 GB | |
| `owl2bench-rl-5` | none | 349,702 -> over-memory | over 16 GB | |
| `lubm-uba-10-l` | none | 1,272,799 -> 1,961,993 | 0.2 GB | 0.8 s, 0.1 s |
| `lubm-uba-100-l` | none | 13,405,605 -> 20,659,897 | 2.0 GB | 7.9 s, 1.4 s |

`claros-le` derives 226 M triples where the paper's LE program derives
555 M. The likely cause: the LE program's self-loop rules (such as
`P_E31_Document(?X,?X) :- E31_Document(?X)`) appear in `Claros_LE.owl` as
five `owl:hasSelf` restrictions, which OWL 2 RL does not allow as a
superclass and the W3C rules do not read, so the `relatedDocuments` chains
built on them never start. `owl2bench-rl-1` needs 12.6 GB for a 1.4 M
closure (about 9,300 bytes per triple; LUBM needs about 150), and
`uobm-010-u` passes 16 GB where RDFox needed 1.1 GB: both are for the core
to explain.

### LUBM from UBA

`scripts/fetch-data.sh --lubm N` runs UBA 1.7 on the fetched JRE with
`-univ N -index 0 -seed 0` (the Lehigh page: these options "can produce the
same data sets as used in papers") and merges its one file per department
into `data/uba/lubm-N.owl`. UBA names each file's ontology by its own URL
(`rdf:about=""`), which no parser can resolve without a base; the merged
file has one header, naming it by the univ-bench IRI, as Oxford's
`LUBM-01K.ttl` does. Nothing else changes: on 10 universities the 189
files, so repaired one by one, hold the same 1,272,577 triples as the merged
file, and two runs of UBA write the same bytes. The `lubm-uba-*` scenarios
load Oxford's `LUBM_L.owl` first. No paper states the seed and index it
used; for parity with the published LUBM-01K, use `lubm-01k-*`.

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
It stays for the Scaling table and quick runs; the official data is in the
`lubm-uba-*` and `lubm-01k-*` scenarios below.

## The Scaling table

When one run includes two or more sizes of a scenario family (today only
`lubm`: `lubm-1`, `lubm-10`, `lubm-50`), the bench prints a **Scaling**
table after the main one: for each engine, the fitted growth exponent of
its total time (load plus reasoning, the median over the runs) and of its
peak memory (the largest peak) against the number of input triples. The
exponent is the least-squares slope of ln(value) against ln(input triples):
1 is linear, 2 quadratic. With two sizes it is the slope between them; an
engine with fewer than two finished sizes has no exponent, and its other
sizes are listed with their status.

- The input triples are counted after all the runs, by parsing each input
  file with `oxttl` (Turtle, N-Triples or N3, by extension); the count is
  not timed and includes the univ-bench ontology's triples.
- This reasoner's peak memory includes the bench binary itself, a
  constant that pulls its memory exponent below 1 on small inputs. Nemo's is the `nmo` process alone.
- The spec's budget (`docs/specs/v1.md`, Performance) is a time exponent of
  at most 1.2 for this reasoner over LUBM 1, 10 and 50, recorded beside the
  competitors' exponents.

## The results file (`--json`)

Schema version 5. Times are in milliseconds, memory in bytes.

Version 5 adds the status `over-memory` and `meta.memory_cap_bytes` to
version 4, and changes nothing else. Version 4 renames this reasoner (the project was called turtle-reasoner
before 2026-11-05): the engine `turtle-reasoner` is now `turbowl`,
`x_turtle_reasoner` is `x_turbowl` and `meta.versions.turtle_reasoner` is
`meta.versions.turbowl`; nothing else changes. `--compare` reads an older
file's `turtle-reasoner` engine as `turbowl`. Version 3 adds the top-level
`scaling` list (the Scaling table) to version 2, and changes nothing else.
Version 2 changed from 1: the engines were renamed; a result's `status` can
be `unsupported`; each result has `x_turtle_reasoner`; the summary has
`export_median_ms` and `wall_median_ms`; Nemo's runs have `export_ms` and
`wall_ms`; and `meta.versions` has `nemo`. `--compare` reads versions 1 to 5.

Files written before version 2 of this project also hold the rows and the
`meta.versions` entry of an engine the bench no longer runs (ADR 0006).
`--compare` skips the rows of any engine it does not run, says how many it
skipped from which file, and compares the rest.

```jsonc
{
  "schema": 5,
  "meta": {
    "date": "2026-11-05T10:00:00Z",          // UTC, when the file was written
    "commit": "fcca1da…", "dirty": false,     // the repository's HEAD; tracked files changed?
    "cpu": "Apple M6", "cores": 12, "memory_bytes": 34359738368,
    "os": "macOS 27.0 (kernel 27.0.0, aarch64)",
    "build": "release",
    "load_before": [0.42, 0.61, 0.70],        // 1, 5, 15 minute load averages
    "load_after": [0.95, 0.70, 0.72],
    "runs": 5, "changes": 100, "timeout_s": 600,
    "memory_cap_bytes": 17179869184,          // --memory-cap
    "data_dir": "/…/data",
    "versions": { "bench": "0.1.0", "turbowl": "0.1.0",
                  "nemo": "0.10.1",           // null when not fetched
                  "rustc": "rustc 1.99.0 (…)" }
  },
  "results": [
    {
      "scenario": "lubm-1",
      "engine": "nemo",                       // turbowl | nemo
      "status": "ok",                         // ok | unsupported | skipped | timeout | over-memory | error
                                              //   (files from before ticket 10 also have unavailable: the stub)
      "message": null,                        // why, when not ok
      "inputs": ["/…/univ-bench.ttl", "/…/lubm-v1-1.ttl"],
      "summary": {                            // null when no run finished
        "measure": "reason_ms",               // or "change_median_ms" (incremental-*)
        "median_ms": 70.1, "min_ms": 68.9, "max_ms": 74.0,   // of the measure, over the runs
        "load_median_ms": 222.5,
        "reason_median_ms": 70.1,             // the first reason() (the full run)
        "total_median_ms": 292.6,             // load + first reason(), per run
        "export_median_ms": 95.0,             // Nemo only (else null): writing its output
        "wall_median_ms": 410.3,              // Nemo only (else null): the nmo process, start to exit
        "peak_rss_bytes": 268468224,          // the largest peak of the runs
        "peak_rss_min_bytes": 266000000,      // the smallest
        "closure": 204521,                    // closure triples (base + inferred), last run
        "peak_bytes_per_closure_triple": 1312.7 // peak_rss_bytes / closure; null when the closure is 0
      },
      "x_turbowl": {                          // this engine over turbowl, same scenario;
        "time": 6.03,                         //   null when either has no finished run.
        "total": 1.42,                        //   median_ms, total_median_ms and
        "peak_rss": 1.95                      //   peak_rss_bytes ratios; over 1: slower or larger
      },
      "runs": [                               // one per finished run, in order
        { "status": "ok", "load_ms": 222.5, "reason_ms": 70.1,
          "change_ms": [],                    // per change, in order (incremental-*)
          "peak_rss_bytes": 268468224,
          "export_ms": 95.0, "wall_ms": 410.3, // Nemo's runs only
          "closure": 204521,                  // what the engine reports
          "closure_counted": 204521 }         // distinct triples counted
      ]
    }
  ],
  "scaling": [                                // one per family and engine; [] when no family ran at two sizes
    {
      "family": "lubm",
      "engine": "nemo",
      "time_exponent": 1.04,                  // of total_median_ms; null with fewer than 2 points
      "memory_exponent": 0.92,                // of peak_rss_bytes; null likewise
      "points": [                             // the sizes with a finished run, by input triples
        { "scenario": "lubm-1", "input_triples": 104097,
          "total_median_ms": 292.6, "peak_rss_bytes": 268468224 }
      ],
      "missing": [                            // the sizes without one, and why
        { "scenario": "lubm-50", "status": "timeout" }
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
with a load average over 1, and prints a changed closure size. It skips the
rows of an engine the bench no longer runs and says how many it skipped. It
reports only and exits 0.

## The budgets these numbers judge

From `docs/specs/v1.md` (Non-functional budgets, Performance), less the two
ratios to the engine ADR 0006 removed:

- The ratio to Nemo is recorded (Nemo's `x turbowl time` and
  `x turbowl total`); it is not a pass/fail budget.
- An incremental addition of one triple to Brick plus a building model
  costs under 10% of a full run (`incremental-add`'s median against
  `brick-small1`'s).
