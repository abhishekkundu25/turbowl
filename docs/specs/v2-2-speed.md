# Version 2, phase 2: lead the scoreboard

Status: planned; starts when phase 1 closes (ticket 15). Tickets:
`docs/tickets/v2/` (16 to 21). Decisions: ADR 0001 (engine), ADR 0005,
ADR 0006. Background: `docs/research/sota-algorithms-2026-11.md` (§6 and
§10), the phase 1 scoreboard record in `docs/verification/`.

## Problem Statement

On version 1's numbers, loading takes most of the total time: on lubm-50,
3.9 s of loading against 0.74 s of reasoning. The reasoner uses one of the
machine's 12 cores, while Soufflé and RDFox run in parallel (RDFox gains 5
to 14 times on 8 to 16 threads). The engine experiments ranked by the
research (a transitive-closure module for the class hierarchy, dropping
redundant rules, schema first, shared inverse tables, compact indexes,
faster sorting, a filter per tier) have not been tried. A user with a large
dataset waits on parsing and on a single core, and the scoreboard shows
where the competitors lead.

Memory is the widest gap. On lubm-50, TurbOWL's peak is about 150 bytes per
closure triple. The papers put RDFox at 51 bytes per triple on LUBM-1K L
(AAAI 2014: 9.3 GB for 182.4 M triples) and VLog at about 12 (AAAI 2016:
2,198 MB). At 150 bytes, LUBM-1K, the dataset that bridges to RDFox's
published numbers, needs about 27 GB and does not fit under the 16 GB cap
(`docs/research/competitors-2026-11.md` §1.2, §8.3).

## Solution

- **Fast loading.** TurbOWL parses N-Triples and Turtle with its own
  parsers, which intern terms straight into term ids. Large N-Triples
  inputs are parsed in parallel. A faster dictionary sits beneath them. A
  snapshot format saves a reasoner's state and loads it back without
  parsing. Oxigraph's parsers stay for N3 and RDF/XML, and as the
  cross-check for every document the tests read.
- **All cores.** Rounds run in parallel in native builds: the rule
  families of a round, the step of each table, and large deltas split into
  chunks. Small deltas stay sequential, and the WebAssembly build stays
  single-threaded.
- **The improvement loop** (`docs/specs/experiments.md`) starts, one
  measured change per iteration. The order: P0 counters, then E2, E8, E1,
  E3, E9, E6, E7 and E10, re-ranked by what the scoreboard shows.

## User Stories

1. As a user with a large dataset, I want loading to take a fraction of today's time, so that the total time is mostly reasoning.
2. As a user, I want N-Triples files parsed on every core, so that big dumps load in seconds.
3. As a user, I want to save a reasoned state and load it back instantly, so that I do not parse and reason the same data twice.
4. As a user, I want a snapshot from another version refused with a clear error, so that I never load a state the reasoner cannot read.
5. As a user, I want reasoning to use every core on a large input, so that it finishes as fast as the machine allows.
6. As a user, I want the same closure and diagnostics whatever the thread count, so that parallelism never changes answers.
7. As an editor author, I want small incremental changes to stay sequential, so that keystrokes never wait on a thread pool.
8. As a browser developer, I want the WebAssembly build to keep working single-threaded, so that the browser path never breaks.
9. As a user, I want the parsers to reject the same malformed documents as before, with errors that name the line and column, so that speed costs no diagnostics.
10. As the owner, I want every speed change measured against the Baseline and kept only when it wins, so that the code only gets faster.
11. As the owner, I want each experiment's number to name what limits it, so that I know why it is not twice as good.
12. As the owner, I want `unsafe` code only in small audited hot spots with a measured gain and tests, so that speed does not cost safety elsewhere.
13. As the owner, I want the scoreboard rerun at the end of the phase, so that I see where TurbOWL now leads.
14. As a maintainer, I want parsing, the dictionary, the snapshot and the parallel rounds covered by `scaling_*` tests, so that none of them grows worse than n log n.
15. As a future agent, I want each experiment recorded in the loop's table, kept or reverted, so that the next round starts from what was learned.
16. As the owner, I want peak memory per triple halved, so that LUBM-1K runs under the 16 GB cap and TurbOWL can be compared with RDFox's published numbers on it.

## Implementation Decisions

- **Parsers** for N-Triples and Turtle that write term ids, not `oxrdf`
  terms, through the dictionary. Blank-node scoping per document and the
  error positions stay as they are. N3 and RDF/XML keep Oxigraph's
  parsers.
- **Parallel N-Triples**: the input is split at line boundaries into
  chunks, parsed on separate threads into local term tables, then merged
  into the dictionary. Turtle stays sequential unless a measured split is
  found.
- **The dictionary**: terms stored in an arena, with a hash of each term
  computed once. The interning contract does not change: the same term
  always gets the same id.
- **The snapshot**: a versioned, checksummed binary file holding the
  dictionary, the base, the closure as stored, the canon, the findings and
  the switches. `Reasoner` gains save and load. A version or checksum
  mismatch is an error.
- **Parallel rounds** behind a native-only cargo feature that is on by
  default:
  - each family of a round gets its own output buffer;
  - each table's step runs independently;
  - a large recent part is split into chunks.
  Below a measured delta size, rounds run sequentially. `scripts/check-wasm.sh`
  builds without the feature.
- **`unsafe`** is allowed only in a module that holds one hot spot, with a
  `SAFETY` comment, a test, and a benchmark gain recorded in the loop's
  table. The workspace lint stays `deny` everywhere else.
- **The loop** follows `docs/specs/experiments.md`. Each iteration runs the
  editor-size scenarios on a quiet machine. The data-size scenarios run
  only at the phase close-out.

## Testing Decisions

- **Parser equivalence**: every document the test suites read, plus
  generated documents, parses to the same triples as Oxigraph's parsers.
  Malformed inputs give errors at the same line and column (prior art:
  `crates/turbowl/src/io.rs` tests, `crates/cli/tests/cli.rs`).
- **The snapshot**: a round trip gives the same closure, diagnostics and
  switches; a changed byte or version is refused.
- **Thread-count invariance**: chaos runs each iteration at 1 thread and at
  all cores and compares the two (`crates/reference/tests/chaos.rs`); the
  oracle suites run at the default thread count.
- **Scaling**: `scaling_*` tests for parsing (bytes in, time out), interning,
  the snapshot, and a round at 1 and at 12 threads (prior art:
  `crates/turbowl/src/scaling.rs`).
- **The scoreboard** reruns at the close-out in both configurations and
  both thread modes.

## Non-functional budgets (pass/fail)

- Loading lubm-50 (the bench's own data) takes at most a third of the
  Baseline's time (median of 5, quiet machine).
- The total time of every editor-size and data-size scenario is below the
  Baseline's.
- Reasoning on lubm-50 with 8 or more cores is at least 5 times as fast
  as with one (RDFox reached 5.2 to 7.4 times on 8 threads, AAAI 2014), and
  single-threaded reasoning is no slower than before the parallel change.
  Raised from twice by the owner on 2026-11-06.
- Loading a snapshot of lubm-50's reasoned state is at least 10 times
  faster than parsing and reasoning it.
- Peak memory on lubm-50 is at most 50 bytes per closure triple (RDFox
  uses 37 to 51), so that LUBM-1K L (182.4 M triples) fits in about 9 GB.
  Lowered from 85 by the owner on 2026-11-06.
- Every oracle agrees at every thread count, and 100,000 full and 100,000
  incremental chaos iterations pass.
- The WebAssembly build passes; fmt and clippy are clean.
- No benchmark run of any engine passes 16 GB.

## Out of Scope

- Incremental removal (phase 3).
- GPUs, distributed reasoning, and SIMD beyond auto-vectorisation unless
  an experiment measures a gain.
- New rules or switches (phase 1 owns them).
- Publishing numbers without the owner's approval.

## Further Notes

The research estimates 2 to 4 times on 8 threads for lubm-50 (E5) and
names one patent (US11409698B2, RDFox's parallel store) whose claims may
cover a lock-free shared store. This design gives each family its own
buffer and runs each table's step independently, a different design. It
is not a legal opinion.

## Principles

- **exhaust-the-design-space**: the parallel round has no precedent here;
  ticket 17 sketches at least two designs (per-family buffers against
  per-table partitioning) before building one.
- **migrate-callers-then-delete-legacy-apis**: the hot path stops using
  Oxigraph's N-Triples and Turtle parsers outright (ticket 16).
- **foundational-thinking**: the dictionary and the snapshot format are
  data structures first (tickets 16, 19).
- **make-operations-idempotent**: saving a snapshot writes a temporary file
  and renames it, so a crash never leaves half a snapshot (ticket 19).
- **separate-before-serializing-shared-state**: parallel families write to
  their own buffers, and parsing threads to their own term tables
  (tickets 17, 18).
- **explain-the-number**: every experiment and budget names its run count,
  spread and limiter (tickets 20, 21).
- **boundary-discipline**: the parsers validate at the input edge, and the
  engine trusts term ids (ticket 16).
