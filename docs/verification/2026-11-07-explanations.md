# Verification: explanations

Date: 2026-11-07. Branch `explain`, tickets in `docs/tickets/explain/`,
spec `docs/specs/explanations.md`. Apple M6, 12 cores, 32 GB, macOS 27.0.
Raw data: `docs/verification/data/2026-11-07-explanations/`.

Some runs that need a quiet machine are not in this record. The owner
moved them to ticket 05 (`docs/tickets/explain/05-deferred-validation.md`):
the explain latencies at load under 1, lubm-50 at 10 runs per side,
100,000 chaos iterations and the slow oracle tier.

## Results against the budgets

| budget | result |
|---|---|
| Reasoning unchanged: times and peak memory within `main`'s spread | **met** (one +0.3% to be rechecked in 05; memory +0.1 to 0.15 MiB of code on the small inputs) |
| `engine/` and `rules/` unchanged but for visibility | **met**: no change at all |
| Editor-size explain: median under 1 ms, p99 under 10 ms | **met in the default configuration**; **gap in the complete one** on Brick (ticket 06) |
| lubm-50, claros-l, sameas-clique: p99 under 50 ms | **met** in both configurations |
| Every proof passes the checker; none missing | **met** |
| Chaos: each shard at most 10% slower | **gap**: about 2.7 times as long (test time only) |
| fmt, clippy, WebAssembly, every oracle | **met** (the slow tier is in 05) |

## Reasoning unchanged

`turbowl-bench --engines turbowl --runs 1`, alternating `main` (1f50498)
and `explain` (6309f93), 5 runs per side on one thread and 10 on all
cores, on brick-small1, brick-soda-hall, lubm-1 and lubm-50, in the
default and the complete configurations, from 10:33 at load 0.99. The
fix that landed after (3f8eabc) changes `explain/` only.

The full table is `data/2026-11-07-explanations/bench-compare.md` (median
and middle half per side, made by `agg.py`). Of 64 measures, 53 have the
`explain` median within `main`'s spread. The other 11:

- Six are faster or equal: load and total times on brick-soda-hall, and
  memory on lubm-1 and lubm-50, 0.0 to 2.2% lower.
- Four are peak memory on brick-small1 and brick-soda-hall, +1.1 to +2.6%,
  or 0.07 to 0.15 MiB. The binary's code grew by 176 KiB (`__TEXT`, 2,408,448
  to 2,588,672 bytes), and these inputs' footprint is mostly code. On
  lubm-50 memory is the same (459.5 MiB, 1,552 MiB complete on all cores).
- One is time: lubm-50, default, one thread, reasoning 382.06 against
  381.10 ms (+0.3%), the middle halves almost touching. No reasoning
  code changed. Ticket 05 reruns it at 10 runs per side.

## Explain latency

`turbowl-bench --explain`, one thread, a seeded 1,000 inferred triples and
up to 1,000 diagnostics per scenario, warm cache, each scenario alone,
after 3f8eabc. **Load 1.5 to 2.3, over the bench's limit of 1**; ticket 05
repeats them quiet.

Default configuration:

| scenario | median ms | p99 ms | max ms | cold ms | cache MB |
|---|---|---|---|---|---|
| brick-small1 | 0.009 | 0.214 | 0.313 | 0.586 | 5.0 |
| brick-soda-hall | 0.031 | 0.324 | 0.483 | 0.216 | 13.4 |
| lubm-1 | 0.003 | 0.069 | 0.143 | 0.278 | 3.5 |
| lubm-50 | 0.007 | 0.110 | 2.219 | 0.295 | 8.6 |
| sameas-clique | 0.177 | 0.598 | 2.356 | 1.787 | 102.5 |
| claros-l | 0.026 | 0.450 | 73.3 | 25.2 | 167.1 |

claros-l's 1,000 diagnostics: median 0.749, p99 1.442, max 2.280 ms.

Complete configuration:

| scenario | median ms | p99 ms | max ms | cold ms | cache MB |
|---|---|---|---|---|---|
| brick-small1 | 2.209 | **157.0** | 3,909.6 | 120.5 | 2,607.5 |
| brick-soda-hall | 0.271 | **161.1** | 998.6 | 19.1 | 1,402.4 |
| lubm-1 | 0.012 | 0.204 | 4.020 | 0.460 | 14.0 |
| lubm-50 | 0.029 | 0.276 | 0.469 | 271.7 | 194.6 |
| sameas-clique | 0.201 | 0.695 | 1.748 | 390.5 | 176.1 |
| claros-l | 0.173 | 4.873 | 1,989.4 | 256.8 | 1,444.7 |

claros-l's diagnostics: median 0.843, p99 1.538, max 3.901 ms. No proof
missing in any scenario.

What limits them:

- Brick, complete: level-0 fact joins in the schema rules' helper calls,
  not equality. The gap was there before the close-out's fix (median 3.6
  ms, p99 146 ms). Ticket 06.
- claros-l's max: the first goal that touches the large time-span class
  builds the lazy `rdf:type`-by-object index (68 M pairs) and the class's
  links. The complete cache is mostly by-object copies of about 160
  predicate tables.

The close-out's first latency run (`bench/explain-default-before-fix.*`,
load 2.7) found two faults, both fixed in 3f8eabc:

- claros-l with `--complete` hung: one triple's search ran for over 20
  minutes. A cls-maxc2 class of 306 time spans, which about 35,000
  productions name, had its sameAs links joined once per member
  (members × incoming × members). Now a large class is joined once.
  Tests: `a_class_that_many_triples_reach_is_linked_in_time` and the
  `functional_hub` shape of `scaling_explain`.
- sameas-clique's p99 was 37.8 ms: 02's second round ranked the whole
  graph again after most goals. It is now kept to the fallback levels: p99
  0.6 ms.

## Proofs and tests

- `crates/oracle/tests/explain.rs`: every inferred triple and diagnostic of
  every minimal input, explained and checked by
  `turbowl_reference::check_proof`. Default: 457 triples and 57
  diagnostics. Complete: 11,644 and 69. Both also with `equality` off. None
  missing.
- Chaos quick run (400 iterations per kind): all seven tests pass, with 8
  triples and 8 diagnostics explained per state, all checked, and each
  equal to the all-cores explainer's proof.
- During 02, the class-mode search and the breadth-first chains were forced
  on for every class. The oracle explain test and chaos at 2,000
  iterations still passed.
- `-p turbowl`: 353 tests. CLI: 35. Python: 809. Oracle `explain`,
  `minimal` and `reference`, fmt, workspace clippy, `check-wasm.sh` and
  `scaling_explain` (exponents 0.002, 0.981 and linear for the hub) all
  pass on the merged tree.

## Chaos time (gap)

`full_runs_match_the_reference_1` at `CHAOS_ITERS=2000`, alternating:

| | seconds |
|---|---|
| `CHAOS_EXPLAINED=0` | 30.7 |
| 8 triples and 8 diagnostics per state (the default) | 83.0 |

All four shards, as 02 measured them (two runs each, load 1.7 to 2.8):

| shard | without proofs, s | with proofs, s | overhead |
|---|---|---|---|
| full 0 | 29.1, 28.4 | 81.2, 81.9 | +183% |
| full 1 | 30.6, 30.6 | 82.6, 82.7 | +170% |
| incremental 0 | 35.1, 35.0 | 84.0, 83.8 | +139% |
| incremental 1 | 35.0, 35.1 | 91.1, 90.3 | +159% |

The budget was +10%. This is test time; reasoning is unchanged. The
limiter is equality in random closures where a vocabulary term shares a
class with individuals and literals: those goals need the wider search
levels. Fewer goals do not help much (one goal per kind already doubles
the time), because the cost is a tail of slow goals. Closing the gap
needs a cheaper complete method for such classes, or a new budget: the
owner's decision.

## Decisions taken without the owner

The owner said to assume what fits the goal and record it here.

1. Chaos explains 8 triples and 8 diagnostics per state, not 32 of each.
   Every one is compared with the all-cores explainer's proof, asked in the
   same order. Every chaos input is still explained. `CHAOS_EXPLAINED`
   overrides the 8.
2. The chaos time budget is reported as a gap and not traded for fewer
   checks.
3. Ticket 01 split by switches (equality and literal values in 02), after
   the ticket review.
4. One dt-diff proof shape: a generalized step with the reference's
   unlisted predicate, then eq-rep, then eq-diff1.
5. A sameAs chain is left-linear in small classes: its depth is its number
   of links. A large class uses a breadth-first tree.
6. `turbowl::profile::explain_cache_bytes` is public, for the bench.
7. The bench's cold time includes the first triple's search.
8. The CLI's `oxrdf` and `oxttl` moved from dev-dependencies to
   dependencies, so `Cargo.lock` did not change. `--summary-only` with
   `--explain-diagnostics` exits 64.
9. Python gained `NotReasonedError` and a test-only native
   `explain_cache_held`.
10. The complete configuration's Brick gap is a follow-up (ticket 06), not
    fixed in this close-out.

At the owner's request: the quiet-machine runs moved to ticket 05.

## Open for the owner

- The chaos budget: accept about 2.7 times as long, or fund a cheaper
  method (above).
- Ticket 06: complete-configuration speed on Brick.
- Ticket 05: the deferred runs.
- Earlier, still open: moving `oracles/rules/` into the crate before
  publishing (the explainer reads it with `include_str!`), and turtle-lsp's
  integration (its own spec).
