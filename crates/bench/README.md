# turbowl-bench

Runs fixed scenarios for this repository's reasoner and its competitors Nemo,
Soufflé and VLog (`CONTEXT.md`, ADR 0006), on the same inputs and the same
rules, and reports the median and spread of the times, the peak memory, the
closure size and each engine's ratio to this reasoner. Each competitor's
closure must equal this reasoner's. Every speed claim, and every step of the improvement loop
(`docs/specs/experiments.md`), is judged by these numbers. Its scenarios
include the public datasets the RDFox papers measured
([Public datasets](#public-datasets)), and no run passes the memory cap
(16 GB by default; ADR 0006). `--scoreboard` reads results files and
prints each engine's value and the leader per scenario and axis, and the
estimates RDFox's published results allow ([The scoreboard](#the-scoreboard)).

| Engine | Version | What runs | How | Threads (`--threads one` / `all`) |
|---|---|---|---|---|
| `turbowl` | this build | this repository's `Reasoner`, with the configuration's switches | in process | one (`Threads::ONE`) / every core (`Threads::ALL`) |
| `nemo` | `nmo` 0.10.1 | the configuration's program, composed from `oracles/rules/` (`turbowl_oracle::programs::nemo`) | a subprocess: `nmo -I <input dir> -D <out dir> --report short` | as it runs by default in both modes, which is one thread: `nmo` has no thread option (its `--help` lists none), and its CPU time matches its wall clock (a smoke run on `uobm-010-l`) |
| `souffle` | Soufflé 2.5, Homebrew's bottle for macOS 27 on arm64 | the same program emitted for Soufflé (`programs::souffle`), compiled to a binary | a subprocess: the compiled binary, `-F <input dir> -D <out dir>` | one in both modes: the bottle is built without OpenMP (upstream turns it off on Apple silicon, issue #2476), so it refuses `-j` above 1 |
| `vlog` | VLog 1.3.7, inside Rulewerk 0.9.0's command-line client | the same program emitted in Rulewerk's syntax (`programs::vlog`) | a subprocess: `java -jar rulewerk-client-0.9.0.jar shell` on the fetched JRE, driven through its standard input | as Rulewerk runs it by default in both modes: Rulewerk exposes none of VLog's thread options (its Java interface to VLog takes no thread count). How many cores VLog then uses is not checked. |

Each competitor is fetched into `tools/` by `scripts/fetch-tools.sh`, pinned
by version and SHA-256, with no system install; `--list` shows each engine,
its pinned version and whether it is fetched. One that is not fetched (or is
at another version) is reported `skipped`. The competitors run full runs
only: the incremental scenarios are `unsupported` (a Datalog engine runs a
program once over its input; it has no change stream). `--threads` sets
this reasoner's threads only; each competitor runs as it does by default
in both modes ([Threads](#threads)).

**Smoke runs, not measurements** (2026-11-06, one run each, `--memory-cap
8`, 1-minute load average 12 to 52): every competitor's closure count
equals this reasoner's on `brick-small1` (26,381) and `lubm-1` (184,086), in
the default and the complete configuration; Nemo's equals it on
`uobm-010-l` (4,602,841) and `lubm-uba-10-l` (1,961,993).

**GraphDB** is not run. GraphDB Free needs a licence sent by e-mail, which
the owner has not obtained. With one, it would run on one core only (its
licence's limit), and its numbers could not be published without Graphwise's
written consent (`docs/research/competitors-2026-11.md` §4).

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
cargo run --release -p turbowl-bench -- --only lubm-uba-10-l --switch equality=off --json off.json
cargo run --release -p turbowl-bench -- --threads all --json all-cores.json
cargo run --release -p turbowl-bench -- --scoreboard default.json complete.json off.json
```

| Option | Meaning |
|---|---|
| `--list` | Print the scenarios and the engines, and stop. |
| `--only A,B` | Run only these scenarios. The opt-in ones (`sameas-clique` and every public dataset) run only this way. |
| `--engines A,B` | Run only these engines: `turbowl`, `nemo`, `souffle`, `vlog`. |
| `--complete` | Start from the complete configuration (every switch on: the complete W3C rule set) instead of the default switches, in every engine. |
| `--switch NAME=on\|off` | Turn one switch on or off, in every engine, after `--complete` wherever it comes; repeatable, and a later one for the same name wins (as the CLI's). A paper's program without `owl:sameAs` rules (Oxford's LUBM ones) is matched with `--switch equality=off` ([Published results](#published-results-and-estimates)). |
| `--threads one\|all` | This reasoner's threads: `one` (the default) or every core (`all`). The competitors run as they do by default in both modes ([Threads](#threads)). |
| `--runs N` | Runs per scenario and engine (default 5; the improvement loop needs 5 or more). |
| `--changes N` | Changes in the incremental scenarios (default 100). |
| `--timeout SECS` | Stop a run after this long and record `timeout` (default 600). A competitor is stopped by the child at this time; the child itself gets 30 s more. |
| `--memory-cap GB` | Stop a run whose memory passes this many GB (2^30 bytes) and record `over-memory` (default 16). |
| `--data-dir DIR` | Where the fetched models are (default `data/` at the repository root). |
| `--json FILE` | Also write the results file described below. |
| `--compare OLD NEW` | Print per-scenario deltas between two results files. |
| `--scoreboard FILE...` | Print the scoreboard of these results files ([The scoreboard](#the-scoreboard)). |

Build in release mode: a debug build prints a warning, and its numbers mean
nothing.

## How a run is measured

- Every run is a fresh process: the bench starts itself with a hidden
  `--one` mode, which loads the files, runs the scenario, and prints one JSON
  line. A fresh process gives each run its own peak memory and a cold
  allocator, the same for every engine.
- **Peak memory** is, on macOS, the peak of a process's physical footprint
  over its lifetime (`proc_pid_rusage`, `ri_lifetime_max_phys_footprint`):
  what the memory cap reads (below), at its highest. It counts the memory
  the process owns, compressed or swapped out included, and leaves out
  clean pages mapped from files (its own executable). The resident size
  (`ru_maxrss`, results files before schema 6) reads low under memory
  pressure: one run held 9.9 GB, 5.4 GB of it compressed, against a 7.6 GB
  peak resident size. For this reasoner it is the child's own, read just
  after the last `reason()`, before the closure check. For a competitor it
  is the engine's process (`nmo`, the compiled Soufflé program, the JVM
  running VLog), read once it has exited and before it is reaped: the whole
  process, output writing included. On Linux it is the peak resident size,
  as before. The table's `B/triple` column (`peak_bytes_per_closure_triple`
  in the results file) is the largest peak over the closure size: the
  memory rubric of `docs/specs/experiments.md`.
- **Load time** covers loading every input file in order (each engine's own
  parser). **Reasoning time** covers the first `reason()`. Both are wall
  clock. Soufflé and VLog report one number for both, recorded as the
  total; their load and reasoning times are blank. The headline number of a full run is the reasoning time; the
  results file also has the load time and their sum (`total`). See
  [What each engine's time includes](#what-each-engines-time-includes).
- In the incremental scenarios, each change (`insert` or `remove` of one
  triple) is timed from the change call through the next `reason()`, which
  works semi-naively from the stored closure. The headline number of a run
  is the median of its per-change times.
- Runs alternate between the engines (run 1 of each engine, then run 2, ...),
  so a drift in the machine falls on all of them.
- **The memory cap.** Every 20 ms the bench sums the memory held by the
  run's processes: the child and everything it started (a competitor's process),
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
  and compares that count with what the engine reports:
  `Summary::base + Summary::inferred` for this reasoner, the rows a
  competitor wrote to its closure file for a competitor (read back a line
  at a time, outside the timings). The count holds one 64-bit hash per
  triple (8 bytes, sorted; never the triples) and nothing more. A
  competitor's count must also equal this reasoner's on the same scenario.
  A mismatch, a closure size that differs between runs of the same engine,
  or a failed run is a **problem**: it is printed, listed in the results
  file, and makes the bench exit 1. A timeout, a run over the memory cap, a
  skipped scenario and an unsupported one are results, not problems.
- **Counters** (this reasoner only; experiment P0 of
  `docs/specs/experiments.md`). Each run records `Reasoner::profile()`
  (`turbowl::profile`): for the full run (the first `reason()`), per round
  the delta and each rule family's emitted tuples and time, and per step
  the time in sorting, in finding what changed (`changed`) and in all, with
  the distinct pairs it got and the tuples among them that were new; and,
  read just after the peak, the bytes the reasoner holds by part (the
  dictionary's text, entries and slots; the base; the store's `rdf:type`
  table, its other tables and its own structures, and its five largest
  tables; the canon; the findings and diagnostics; the rest). Bytes are
  capacities, what the process holds. The counts cost an integer add per
  family and round; the clock is read a few times per round and step; the
  bytes are walked after the timings. On WebAssembly every time is 0.
- **Threads** ([Threads](#threads)): this reasoner's runs set
  `Reasoner::set_threads` before loading, so parsing and reasoning both
  run on the mode's threads.
- A panic in this reasoner is caught by the child and recorded as an
  `error` with its message: a problem.
- Every engine's closure is the full one on every scenario, `owl:sameAs`
  included (ticket 11), so the times compare. A competitor's closure file
  also holds generalized triples (a literal subject, or a predicate that is
  not an IRI), which the rules derive; the count leaves them out, as this
  reasoner's reads do. This reasoner keeps one
  representative per `owl:sameAs` class (ADR 0003): its reasoning time and
  stored triples grow with the input, and the expanded closure (what
  `triples()` gives and the check counts) is built only when read, never
  stored. On `sameas-clique` the peak (read before the checks) follows the
  10,003 stored triples, not the 11 M expanded ones, so its `B/triple`
  is under 1.

### Threads

`--threads one` (the default) runs this reasoner on one thread
(`Threads::ONE`): every round and every parse on the run's own thread.
`--threads all` runs it on every core (`Threads::ALL`, the `Reasoner`'s own
default): a round or a document large enough is split over rayon's global
pool, whose size is the machine's core count (`meta.cores`) unless
`RAYON_NUM_THREADS` sets it. The mode reaches each child as an argument of
`--one`, the results file records it in `meta.threads` (1 for one thread,
0 for all cores), each of this reasoner's runs records the setting the
reasoner held (`threads`, the same numbers), and the text report's header
names it.

"All cores" changes only this reasoner. Each competitor runs as it does by
default, in both modes:

- **Nemo**: `nmo` 0.10.1 has no thread option, and by default it runs on
  one thread (its CPU time matched its wall clock on `uobm-010-l`).
- **Soufflé**: one thread. The bottle has no OpenMP, so the compiled
  program refuses `-j` above 1.
- **VLog**: as Rulewerk 0.9.0 runs it. Rulewerk exposes none of VLog's
  thread options, and its Java interface to VLog takes no thread count.
  How many cores VLog uses under it is not checked.

So an all-cores section of the scoreboard runs each competitor exactly as a
one-thread section does; only this reasoner's runs differ.

Results files written before this option (schema 8, and schema 7 from v2
ticket 17, commit `9fec6d3`, on) record `meta.threads` 1, but their
TurbOWL runs used every core: the bench did not set the threads, and a
`Reasoner` uses every core by default since ticket 17. `--compare` and
`--scoreboard` read every file before schema 9 as one thread.

### What each engine's time includes

The engines do not split their work at the same places, so compare the
columns that mean the same thing:

| | `turbowl` | `nemo` | `souffle` | `vlog` |
|---|---|---|---|---|
| load | parsing every file into the reasoner (`load_file`) | Nemo's `Data import` (its own report, `--report short`, whole milliseconds) | not split: in the total | not split: in the total |
| reasoning (the headline) | the first `reason()`: rules to fixpoint and the closure materialised in memory | Nemo's `Reasoning`: rules to fixpoint. Nemo loads tables lazily, so part of its parsing can fall here. | not split: in the total | not split: in the total |
| total | load plus reasoning | load plus reasoning | the compiled program's wall clock: process start (a few milliseconds), reading `input.tsv`, rules to fixpoint, and writing `closure.tsv` and the findings | VLog's own "Loading and materializing inferences ... finished in" (whole milliseconds): reading `input.nt` and rules to fixpoint |
| not in the total | process start, the closure check | process start, reading the program, writing `closure.nt` and the finding CSVs (`Data export`, in `export_median_ms`), process exit; all of it is in `wall_median_ms` | compiling the program (once per program, cached in `target/bench-souffle/bin/`; its first launch, which macOS checks, is also before any measured one) | starting the JVM and the shell, reading the program, exporting `closure.csv` and the finding CSVs (the shell's own times, summed in `export_median_ms`); all of it is in `wall_median_ms` |
| peak memory | the bench child, engine included | the `nmo` process, export included | the compiled program's process, writing included | the JVM's process, VLog's native memory and the exports included |

So `total` (load plus reasoning) is the fairest single comparison of all
four, and the only one for Soufflé and VLog; the headline compares
reasoning alone, between this reasoner and Nemo, where Nemo may carry some
of its parsing. Soufflé's total also holds its output writing, which the
others leave out. The wall clocks are recorded but not compared.

### What the competitors read

Every competitor reads the same converted input, written once per list of
input files before any run (not timed) and cached under
`target/bench-input/<key>/` (`src/convert.rs`). The key is the files'
paths, sizes and modification times; a conversion is written under a
temporary name and renamed whole, so a crashed one is redone, never read.

- The files are parsed with Oxigraph's parsers as this reasoner reads them:
  the format from the extension (Turtle, N-Triples, RDF/XML; `.n3` as
  Turtle), no base IRI, blank nodes scoped per file.
- Every literal and blank node is renamed to an IRI by the oracle's codec
  (`turbowl_oracle::Codec`), as the oracle feeds Nemo: the engines match
  terms as the W3C rules and this reasoner do, never by value (Nemo would
  read `"01"^^xsd:integer` as 1), and the literals the rules name get their
  fixed IRIs (`turbowl_oracle::NAMED_LITERALS`).
- `input.nt` holds the encoded triples as N-Triples (Nemo and VLog);
  `input.tsv` the same triples with the three terms separated by tabs
  (Soufflé); `literals.bin` a bit per number the codec gave, set for the
  literals.
- A closure is counted in its encoded form: the codec is a bijection, so
  distinct encoded triples are distinct triples. The bits tell which
  encoded terms are literals, so the count drops the generalized triples
  without holding the codec.

Each program is the configuration's, composed from `oracles/rules/`, run in
`target/bench-<engine>/<pid>/` and removed after the run. Nemo finds
`input.nt` through `-I`, Soufflé `input.tsv` through `-F`; VLog's program
names `input.nt` by its full path, since Rulewerk does not resolve a
relative path against the working directory. Rulewerk's shell exits 0 after
an error, so an `Error:` line in its output fails the run. Its
`materialize` command cannot be used: the client hands the subcommand's
name to the subcommand's own parser, which rejects it.

Since the program binds variable predicates through the properties the
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
  constant that pulls its memory exponent below 1 on small inputs. A competitor's is its engine's process alone.
- The spec's budget (`docs/specs/v1.md`, Performance) is a time exponent of
  at most 1.2 for this reasoner over LUBM 1, 10 and 50, recorded beside the
  competitors' exponents.

## The scoreboard

`--scoreboard FILE...` reads results files (schemas 1 to 9) and prints, in
Markdown, one section per configuration and thread mode: `default` and
`complete`, each with one thread and with all cores, then any other
configuration a file ran (`default, equality off`). A file's
`meta.threads` sets its mode: 1 (and every file before schema 9) one
thread, 0 all cores (`--threads all`). Only this reasoner's threads
differ between the two: the competitors run as they do by default in both
([Threads](#threads)). A mode no file
ran says so. When two files hold the same scenario and engine in the same
section, the later file's row is used.

Each section has a row per scenario and axis (reasoning, total, and peak
memory for a full run; the time per change and peak memory for an
incremental one) with every engine's median over its runs (min..max), and
the leader:

- The leader has the lowest median, and leads only when its range lies
  below every other engine's. Engines whose ranges overlap the lowest one's
  tie with it, as `--compare` flags a change only when ranges do not
  overlap. With one run, only equal values tie.
- An engine with no value cannot lead: `n/a (not split)` (Soufflé and VLog
  report load and reasoning as one time), its status (`timeout`,
  `over-memory`, `unsupported`, `skipped`, `error`), or `not run`.
- Under the table: how many rows turbowl leads or ties, and every row it
  does not lead, with the leader.

The last section is the estimates from published results, below.

## Published results and estimates

`published.json` records RDFox's published results and the numbers of every
engine that ran next to it in the same table (`docs/research/competitors-2026-11.md`
§1.2, with VLog 2016's, CIKM 2019's and GLog 2021's tables read again from
the papers): per row the paper, the dataset, the program and its variant,
whether the program axiomatises `owl:sameAs`, the fact counts before and
after as printed, and per engine its build, threads, seconds (load,
reasoning, or their sum, or a time the paper does not split), memory, and
the bench engine it is when it runs here (the bridge). `src/published.rs`
describes the format. `--list` still prints the counts from `src/datasets.rs`.

**The same job.** A paper counts the facts its Datalog program derives over
the data; the ontology is compiled into rules, not loaded. TurbOWL loads the
ontology as triples, so a run on a scenario some paper published records
its comparable counts (`published_counts`, not timed): before, the data's
distinct triples (the program file left out); after, the closure less the
triples of the ontology's own closure (the program file alone, in the same
configuration) that are not data. The configuration must match the
program: one that axiomatises `owl:sameAs` needs `equality` on, one without
`owl:sameAs` rules needs it off. That is the program's property, not its
variant's name: Oxford's Claros and UOBM programs, L included, axiomatise
equality (symmetry, transitivity and a replacement rule per predicate, no
reflexivity: `data/oxford/*/*.dlog`), its LUBM programs have no
`owl:sameAs` rule, and CIKM 2019 states its L programs leave it out. U and
REW are never the same job.

**An estimate** is RDFox's time in the paper x the bridge's time here / the
bridge's time in the paper, and is made only when every condition holds:

1. RDFox and the bridge both ran one thread in the paper (the bridge runs
   one here), and neither failed there;
2. the variant is L, LE or an AX program, and the program's equality
   matches the configuration's `equality` switch;
3. TurbOWL's comparable counts print as the paper's (to the precision
   printed), after a difference the data file explains (`explained`);
4. the bridge ran here on the same scenario, finished, and its closure
   equals TurbOWL's;
5. the paper's times and the bridge's time here measure the same work:
   reasoning, else load plus reasoning.

Otherwise the scoreboard lists it as refused, with the reason. An estimate
row gives the source, the bridge, both paper times, the bridge's time here,
the estimate, TurbOWL's time here, and RDFox's memory as published (not
scaled). A published RDFox number is a floor (ADR 0006).

**Claros L, smoke runs** (2026-11-06, one run each, `--memory-cap 16`,
1-minute load average 8 to 57: not measurements):

| Run | Closure | Ontology alone, not data | Comparable after | Papers |
|---|---|---|---|---|
| `claros-l`, `--switch equality=off` | 92,101,059 | 3,853 | 92,097,206 | 95.5 M (AAAI 2014, VLog 2016): -3.6%; 73.8 M (CIKM 2019, AIJ 2019): +24.8% |
| `claros-l`, default (equality on) | 114,396,494 | 3,853 | 114,392,641 | 95.5 M: +19.8%; 102 M AX (AAAI 2015): +12.1% |

The data's distinct triples are 18,793,277 in both: the papers' 18.8 M.
Neither "after" matches, and none can: `Claros_L.dlog` replaces equals only
in the predicates its ontology names, while 5.8 M data triples use
predicates it never names (`rdfs:label` 2.1 M, `rdf:value` 1.3 M, Claros'
`period_begin` and `period_end` 0.7 M, ...), which the W3C rules eq-rep-s
and eq-rep-o replace. With equality off TurbOWL replaces nothing (fewer
triples); with it on, it replaces everywhere (more). CIKM 2019's and AIJ
2019's Claros L is another program (1,310 rules, not 2,174). So no Claros L
estimate is made today, and three more conditions fail besides the counts:

- The bridge does not run here: VLog passed the 16 GB cap on `claros-l` in
  both configurations (16,393 MB with equality on, 16,467 MB off), where the
  paper's VLog held 2,406 MB. Here VLog runs the W3C rules, which match
  every triple's predicate as a variable, inside Rulewerk's JVM with the
  encoded input in memory; the paper's ran Oxford's compiled program over
  an on-disk store.
- The time: VLog 2016 times VLog's materialisation over a store built
  beforehand, and VLog here reports loading and reasoning as one number.
- CIKM 2019 states no threads; GLog 2021's Claros has 13.8 M facts, not
  18.8 M.

## The results file (`--json`)

Schema version 9. Times are in milliseconds, memory in bytes.

Version 9 records the thread mode (`--threads`): `meta.threads` is 1 for
one thread and 0 for all cores, and each of this reasoner's runs has
`threads`, the setting the reasoner held, in the same numbers (a
competitor's runs have none). Nothing else changes. A file before version
9 has `meta.threads` 1 (or none), read as one thread, though from v2
ticket 17 on its TurbOWL runs used every core ([Threads](#threads)).

Version 8 adds this reasoner's counters and memory breakdown: a `profile`
in each of its runs, and a `profile` in its summary (both described in the
sample below; a competitor's runs and summary have none). Nothing else
changes, and `--compare` and `--scoreboard` do not read them.

Version 7 adds `meta.switches` (the names of the switches that were on),
`meta.threads` (the threads each engine ran on: 1, as every file before
it) and the top-level `published_counts` (this reasoner's counts on the
scenarios a paper published, for the estimates: [Published
results](#published-results-and-estimates)). `meta.configuration` can now
also be `default` followed by the switches `--switch` turned from it
(`default, equality off`). Nothing else changes. Version 6 adds the engines `souffle` and `vlog` (and their entries in
`meta.versions`), `meta.configuration` (`default` or `complete`) and
`meta.peak_memory` (what the peak memory fields measure), and a run's
`total_ms`, for an engine that reports loading and reasoning as one number
(its `load_ms` and `reason_ms` are null). The peak memory fields keep their
names, but on macOS they now hold the lifetime peak physical footprint, not
the peak resident size: `--compare` warns when the two files measure peak
memory differently, or ran different configurations (a file before version
6 ran the default one). Version 5 adds the status `over-memory` and `meta.memory_cap_bytes` to
version 4, and changes nothing else. Version 4 renames this reasoner (the project was called turtle-reasoner
before 2026-11-05): the engine `turtle-reasoner` is now `turbowl`,
`x_turtle_reasoner` is `x_turbowl` and `meta.versions.turtle_reasoner` is
`meta.versions.turbowl`; nothing else changes. `--compare` reads an older
file's `turtle-reasoner` engine as `turbowl`. Version 3 adds the top-level
`scaling` list (the Scaling table) to version 2, and changes nothing else.
Version 2 changed from 1: the engines were renamed; a result's `status` can
be `unsupported`; each result has `x_turtle_reasoner`; the summary has
`export_median_ms` and `wall_median_ms`; Nemo's runs have `export_ms` and
`wall_ms`; and `meta.versions` has `nemo`. `--compare` and `--scoreboard` read versions 1 to 9.

Files written before version 2 of this project also hold the rows and the
`meta.versions` entry of an engine the bench no longer runs (ADR 0006).
`--compare` skips the rows of any engine it does not run, says how many it
skipped from which file, and compares the rest.

```jsonc
{
  "schema": 9,
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
    "configuration": "default",               // or "complete" (--complete), or "default, equality off" (--switch)
    "switches": ["equality", "property-chains", "checks", "identity", "schema"], // the switches on
    "threads": 1,                             // turbowl's thread mode (--threads): 1 one thread, 0 all cores
    "peak_memory": "lifetime peak physical footprint", // on Linux: "peak resident size"
    "data_dir": "/…/data",
    "versions": { "bench": "0.1.0", "turbowl": "0.1.0",
                  "nemo": "0.10.1",           // each competitor's; null when not fetched
                  "souffle": "2.5", "vlog": "1.3.7",
                  "rustc": "rustc 1.99.0 (…)" }
  },
  "results": [
    {
      "scenario": "lubm-1",
      "engine": "nemo",                       // turbowl | nemo | souffle | vlog
      "status": "ok",                         // ok | unsupported | skipped | timeout | over-memory | error
                                              //   (files from before ticket 10 also have unavailable: the stub)
      "message": null,                        // why, when not ok
      "inputs": ["/…/univ-bench.ttl", "/…/lubm-v1-1.ttl"],
      "summary": {                            // null when no run finished
        "measure": "reason_ms",               // or "change_median_ms" (incremental-*)
        "median_ms": 70.1, "min_ms": 68.9, "max_ms": 74.0,   // of the measure, over the runs
        "load_median_ms": 222.5,
        "reason_median_ms": 70.1,             // the first reason() (the full run)
        "total_median_ms": 292.6,             // load + first reason(), per run (or the run's total_ms)
        "export_median_ms": 95.0,             // Nemo and VLog only (else null): writing its output
        "wall_median_ms": 410.3,              // competitors only (else null): the engine's process, start to exit
        "peak_rss_bytes": 268468224,          // the largest peak of the runs (meta.peak_memory)
        "peak_rss_min_bytes": 266000000,      // the smallest
        "closure": 204521,                    // closure triples (base + inferred), last run
        "peak_bytes_per_closure_triple": 1312.7, // peak_rss_bytes / closure; null when the closure is 0
        "profile": {                          // turbowl only (schema 8): the runs' profiles summed up
          "rounds": 7,                        // the last run's
          "rules_median_ms": 3.04,            // medians over the runs of the full run's times:
          "step_median_ms": 1.75,             //   the rounds (families), the steps,
          "sort_median_ms": 1.16,             //   their sorts,
          "changed_median_ms": 0.5,           //   and their changes
          "memory": { "dictionary": 670976, "base": 196832, "store": 812408,
                      "rest": 1656, "total": 1681872 },   // the last run's, by part; rest = total - the others
          "bytes_per_closure_triple": {       // over the last run's closure
            "peak": 242.2,                    // = peak_bytes_per_closure_triple
            "dictionary": 25.4, "base": 7.5, "store": 30.8, "rest": 0.1,
            "not_counted": 178.5              // peak - total: the binary, allocator slack, transient peaks
          }
        }
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
          "export_ms": 95.0, "wall_ms": 410.3, // competitors' runs only
          // "total_ms": 300.0,               // Souffle's and VLog's runs: load_ms and reason_ms are null
          "closure": 204521,                  // what the engine reports
          "closure_counted": 204521,          // distinct triples counted
          "threads": 1,                       // turbowl only (schema 9): the setting it held, as meta.threads
          "profile": {                        // turbowl only (schema 8): Reasoner::profile()
            "families": ["class", "property"],  // the rule families that ran, in order
            "rounds": {                       // the full run's, one entry per round
              "delta": [14819, 2946],         // tuples new as the round starts
              "emitted": [[6000, 3084], [7000, 4124]], // per family: tuples emitted, duplicates included
              "family_ms": [[0.2, 0.1], [0.3, 0.2]],   // per family: its own time
              "ms": [0.31, 0.52]              // the round's wall time
            },
            "steps": {                        // one entry per step: one more than the rounds
              "ms": [0.15, 0.26, 0.01], "sort_ms": [0.07, 0.21, 0.0], "changed_ms": [0.0, 0.04, 0.0],
              "pairs": [14819, 3871, 4],      // distinct pairs the step got (the input, then a round's output)
              "new": [14819, 2946, 0]         // of them, new: the next round's delta
            },
            "round_count": 2, "rules_ms": 0.83, "step_ms": 0.42, "sort_ms": 0.28, "changed_ms": 0.04,
            "memory": {                       // bytes held at the end of the run (capacities)
              "dictionary_text": 396544, "dictionary_entries": 143360, "dictionary_slots": 131072,
              "base": 196832,
              "store_type": { "tuples": 2657, "stable": 73824, "recent": 128, "object_index": 37984 },
              "store_other": { "tuples": 23724, "stable": 468832, "recent": 101888, "object_index": 119112 },
              "store_rest": 10640,            // the table list, the predicate index, the queued triples
              "largest": [ { "predicate": "<http://www.w3.org/2000/01/rdf-schema#subClassOf>",
                             "tuples": 9267, "stable": 262240, "recent": 32864, "object_index": 117584 } ],
              "canon": 4, "findings": 0, "diagnostics": 0, "other": 1652,
              "total": 1681872                // the sum of the parts (largest is inside store_*)
            }
          } }
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
  "published_counts": [                       // the scenarios a paper published (published.json) that turbowl finished
    {
      "scenario": "claros-l",
      "before": 18793277,                     // the data's distinct triples (the program file left out)
      "closure": 114396494,                   // turbowl's closure over the program and the data
      "ontology_only": 3853,                  // triples of the program's own closure that are not data
      "after": 114392641                      // closure - ontology_only: comparable with a paper's "after"
    }
  ],
  "problems": []                              // strings; non-empty means exit 1
}
```

A run that ends early keeps the runs before it: `status` is the first
non-`ok` status, `runs` holds the finished ones.

## The breakdown

After the main table, the text report prints one line per scenario this
reasoner ran, from its summary's `profile`: the peak in bytes per closure
triple, what the reasoner holds after the run per closure triple, split
into the dictionary, the base, the store and the rest (canon, findings,
diagnostics, the rest), and what the held parts do not explain (`not
counted`: the binary, allocator slack and the transient peaks of loading
and of the steps); then the rounds and the medians of the time in the
rounds' rule families, in the steps, and in the steps' sorts and changes.
In a parallel step the sorts and changes are summed over threads, so they
can pass the step's time. For example (one run on a busy machine: the
format, not a measurement):

```text
| scenario | B/triple peak | dictionary | base | store | rest | not counted | rounds | rules ms | step ms | sort ms | changed ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| brick-small1 | 242.2 | 25.4 | 7.5 | 30.8 | 0.1 | 178.5 | 7 | 3.0 | 1.8 | 1.2 | 0.5 |
```

## Comparing two results files

`--compare OLD NEW` prints, for each scenario and engine in the new file, the
old and new medians, the change in percent, both ranges (min to max), and
the peak memory. It flags a change only when the two ranges do not overlap
(`SLOWER` or `faster`, `MORE` or `less` memory): a median that moved inside
the spread is noise until more runs say otherwise. It warns when the CPU,
the OS, the build, the change count, the configuration, what the peak
memory measures or the threads differ, or when either run started
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
