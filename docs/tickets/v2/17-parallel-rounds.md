# 17: All cores: parallel rounds

Status: done on 2026-11-06 (`9fec6d3`); the speed budget is measured at the
phase close-out. Blocked by: 15 (done).

A native-only cargo feature, on by default, runs rounds in parallel. At least two designs are sketched before one is built (per-family output buffers against partitioning by table). Each family of a round writes to its own buffer, each table's step runs independently, and a large recent part splits into chunks. Below a measured delta size, rounds stay sequential. The WebAssembly build has no threads.

## Acceptance
- [x] Chaos runs each iteration at 1 thread and at all cores and finds the same closure and diagnostics: the summary, the closure in read order and the diagnostics in order, after every `reason()`.
- [x] `scaling_*` tests of a round at 1 and at 12 threads; reasoning on lubm-50 is at least 5 times as fast on 8 or more cores at the close-out, and no slower on 1. The scaling tests pass (exponents 1.05 to 1.10 at one thread, 0.62 to 0.64 at twelve); the speed waits for the close-out. At load 2 to 4: one thread 423 to 438 ms against 739 to 803 ms before, all cores 103 to 114 ms. At the close-out: 3.9 times (393 against 102 ms), a gap the maintainer accepted on 2026-11-07.
- [x] `scripts/check-wasm.sh` passes without the feature.

## Design

### Where a round spends its time

Measured before this ticket, on lubm-10 (1.28 M base triples, the bench's
own data, default switches, one thread, on a shared machine): reasoning
took 136 ms over 6 rounds. The step took 116 ms of it. Within the step,
sorting the round's buffer of `(p, s, o)` triples took 83 ms, and the
tables' `changed()` took 9 ms. The rule families took 16 ms in all
(class 9, datatype 5, property 1.4). A parallel round must therefore make
the step parallel first. Running the families in parallel alone would gain
at most 1.1 times.

### Design A: per-family output buffers

- **Tasks.** Each family of a round runs as one task, with its own output
  buffer and its own findings. Inside a family, a loop over a large recent
  part splits into contiguous chunks, each with a buffer of its own.
- **Merge.** The buffers are joined in table order, then in chunk order.
  The step then works as before: one sort of every `(p, s, o)` triple,
  grouping by predicate, and each table's insert and `changed()` run as an
  independent task.
- **Cost on this engine.** The rules are unchanged, except for the loops
  that opt into chunks. The semi-naive contract is unchanged, because a
  chunk splits the outer loop of one join term. The merged buffer holds
  the same triples, and the same merges in the same order, as one thread
  gives. So the store, the canon, the rounds and every order a caller can
  read are identical by construction. What remains is the global sort: a
  parallel sort of 12-byte triples, which is bound by memory bandwidth,
  and a copy of the round's whole output to join the buffers.

### Design B: partitioning by table

- **Tasks.** The round's delta is partitioned by table, and a large
  table's delta is partitioned by subject range. Each part is a sub-round:
  its tuples are recent, and the parts before it count as stable. Every
  family runs on every part, with its own buffer. Each table's step runs
  on its own.
- **Cost on this engine.** The rules read a `Variable`: stable batches and
  one recent `Relation`. A part needs a borrowed view type in its place,
  at about 200 sites in `rules/`. A subject range of the `(s, o)` delta is
  not a contiguous range of the `(o, s)` index. So each split table's
  object-index delta is copied per part, and a later part reads up to k
  extra stable parts, which every leapjoin lookup pays for. Some families
  are not decomposable and must run whole: literal-values groups the new
  literals of a whole delta by value, datatype has a first-round pass over
  the whole store, and the rounds after a merge read `moved()`. The merges
  of a round arrive in a different order, so the canon would have to sort
  them, and the representatives, and therefore the order of `triples()`,
  would change. Each family runs k times per round, and pays its fixed
  per-call cost k times.

### Choice

The chosen design takes Design A's tasks and buffers, and applies Design
B's partitioning to the output instead of the input.

- **Buffers by predicate.** Each task's buffer is kept by predicate. An
  emit appends to its predicate's batch, and a one-entry cache of the last
  predicate reduces most emits to a compare and a push. Joining the
  buffers moves batches without copying them.
- **No global sort.** The step never sorts the round as a whole. Each
  table sorts and deduplicates its own 8-byte `(s, o)` pairs and runs its
  `changed()`. Its `(o, s)` index then takes the new pairs, flipped. The
  tables run in parallel, and a large table's sorts, merges and filters
  run in parallel too.
- **One code path.** The sequential path shares this code, so one thread
  also sorts per-table pairs instead of a whole round of triples.
- **Families as tasks.** The families of a round run as tasks
  (`Round::tasks`). A family's loop over a large recent part can split it
  into chunks (`Round::split`): dt-not-type's scan of the recent tuples,
  and the reasoner's count of each round's delta, do this.
- **Determinism by construction.** Batches are joined in family order,
  then in chunk order. Each table's pairs are sorted. The merges and the
  findings keep the order one thread gives them. The store therefore
  evolves exactly as it does on one thread, whatever the thread count or
  the schedule.

Design B was rejected. Its gain is in the rule families, which are 12% of
the measured round, and its costs are a rewrite of every family's
signatures, copies of the object-index deltas, families that must run
whole, and the loss of determinism by construction.

The patent noted in the speed spec (US11409698B2) covers a shared
lock-free store with per-thread reserved areas. This design shares no
mutable store: every task owns its buffers, and only the step, a separate
phase, writes the tables.

### As built

- **Interface.** The public interface gains `Threads` (`Threads::ONE`,
  `Threads::ALL`, the default, and `Threads::every_round()`),
  `Reasoner::set_threads` and `Reasoner::threads`. Chaos needs them: it runs
  every input at one thread and at all cores. The threads are rayon's
  global pool, so `RAYON_NUM_THREADS=1` also gives one thread, without a
  flag in any adapter. The `parallel` feature is on by default. rayon is a
  dependency only off WebAssembly.
- **Inside the engine.** `engine/parallel.rs` holds `Workers` and the
  parallel operations: tasks, chunks, a parallel sort and deduplication,
  merge and filter. Each operation has a one-thread form that gives the
  same result. The step advances the active tables as jobs. The object
  index of a table takes the subject index's new pairs, flipped, so it
  sorts no pair that is not new. A table's large sort, its tier merges and
  its filters run in parallel.
- **Loops read in chunks.** The following loops split a large recent part
  into chunks:
  - cax-sco, cax-eqc1, cax-eqc2 and scm-sco, through `Round::join`;
  - cls-int1's candidate check, cls-svf1 and cls-avf, in runs of a key;
  - the copies made by prp-dom, prp-rng, prp-spo1, prp-eqp1, prp-eqp2,
    prp-inv1 and prp-inv2;
  - dt-not-type's scan of the recent tuples;
  - the reasoner's count of the delta;
  - the queueing of the base in a full run.
- **Threshold.** `PARALLEL_FROM` is 65,536 tuples, measured by
  `scaling::parallel::timing_parallel_round_threshold`. All cores lost up
  to 32,768 base triples (0.75 times as fast) and won from 65,536
  (1.2 times).
- **Measured on lubm-50.** These numbers are a median of 3, in two sets,
  on this shared 12-core machine (load 2 to 4), timed with the reasoner's
  own `reason()`:

  | Build | Threads | Reasoning |
  |---|---|---|
  | Before this ticket (529a5d5) | one | 739 to 803 ms |
  | This ticket | one | 423 to 438 ms |
  | This ticket | all cores | 103 to 114 ms |

  With all cores, reasoning is 3.7 to 4.3 times as fast as with one
  thread, and 6.5 to 7.8 times as fast as before this ticket. One thread is
  faster than before because no step sorts the whole round. The budget
  (5 times, on a quiet machine) is measured at the close-out.
- **Still sequential.** These parts still run on one thread:
  - the step's job for `rdf:type`, the largest table, whose sort is
    parallel but which is still a single job;
  - cls-int1's candidate building;
  - the object indexes that rules request, built one table after another;
  - the families that are not split. Each family is one task, so this
    matters only when one family dominates a round.
