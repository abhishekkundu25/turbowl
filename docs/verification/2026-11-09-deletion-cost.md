# Baseline deletion cost: a small comparison

This measures the original full-run deletion path at `83b356e`. The later
implementation and comparison are in
[the removal record](2026-11-09-removals.md).

Deleting one fact costs approximately a full reasoning run on Brick plus
small1. An unchanged rerun performs no rule evaluation. This is a small
measurement of current behavior, with no change to the reasoner.

## Conditions

- Branch: `feat/incremental-deletions`, based on
  `83b356edde2eab0809a562275d4ec9d18af139f2`.
- Release build, Rust 1.99.0, macOS 27 on arm64, one reasoning thread,
  default switches. The host reports 12 logical CPUs.
- Measured on 2026-11-09 in Asia/Kolkata, 2026-11-08 at 23:03 UTC.
- The owner explicitly authorized approximate timings at the current
  machine load. The 1-minute load was 1.59 before and after. These are
  approximate timings, not a quiet-machine performance claim.
- No build or test from this session ran alongside the measurement.
- Eleven measured samples per case after one warmup per case, with case
  order rotated each round. Each sample loads its own reasoner. Loading,
  initial reasoning, closure checks and profile reads are outside the
  timer. Mutation cases time `insert` or `remove` through `reason()`.
- An unchanged sample averages 1,000 `reason()` calls; its result is
  reported per call. The mutable reasoner and returned summary pass
  through `black_box`.

## Workload and results

The input is the existing benchmark's Brick ontology and `small1.n3`:
14,817 base triples and 26,381 closure triples. The probe is one new fact:

```turtle
<http://example.org/deletion-bench/sensor>
    a <https://brickschema.org/schema/1.1/Brick#Air_Temperature_Sensor> .
```

It adds one base triple and 49 inferred triples, bringing the closure to
26,431 triples. Insertion starts without the probe; deletion starts with
it. The unchanged case also starts with it. The full run reasons from
loaded data without it, producing the same final closure as deletion.

| Operation | Median, ms | Min to max, ms | Reasoning rounds |
|---|---:|---:|---:|
| Unchanged `reason()` | <0.001 | all <0.001 | 0 |
| Insert one fact and reason | 0.890 | 0.878 to 1.255 | 5 |
| Delete one fact and reason | 1.996 | 1.963 to 2.210 | 7 |
| Full reasoning, parsing excluded | 2.047 | 2.000 to 2.182 | 7 |

Deletion is about 2.2 times the insertion cost for this particular fact.
The deletion and full-run ranges overlap; their small median difference
does not establish a performance difference.

## Why deletion costs this much

The built-in counters show the same seven rounds and rule emissions for
deletion and full reasoning: 29,846 class-family emissions and 17,799
property-family emissions in every sample. Deletion reruns inference
over the remaining base instead of limiting work to the probe's 50
affected closure triples.

The main measured work is the class family, median 1.063 ms, and store
steps, median 0.715 ms. The class family also dominates this insertion,
median 0.870 ms. These counters explain this workload's costs; this is
not an estimate of gains from implementing DRed or FBF.

Every measured result was checked as a complete triple set against its
expected closure. Every deletion matched a full run without the probe.
Base counts changed by exactly one for mutation cases; all unchanged
calls reported zero rounds; every result had zero diagnostics. There
were no failed cases.

## Reproduce and evidence

The [benchmark source](../../crates/bench/examples/deletion_cost.rs) is
self-contained and uses the public interface. On a quiet machine, or
with the owner's explicit load exception:

```sh
timeout 120 cargo build --release --locked --offline -p turbowl-bench --example deletion_cost
timeout 120 target/release/examples/deletion_cost > /tmp/deletion-cost.json
```

The build must finish before the measured command starts. The program
prints all samples and counters as JSON. It does not enforce the machine
load rule itself.

- [Raw samples](data/2026-11-09-deletion-cost/raw.json)
- [Summary](data/2026-11-09-deletion-cost/summary.json)
- [Environment, load, command and source hash](data/2026-11-09-deletion-cost/metadata.json)

The example builds in release mode and passes `cargo fmt --check` and
release Clippy with warnings denied.

## Open for the owner

The implementation plan remains DRed, then FBF. This measurement adds
no deletion optimization. Quiet-machine measurements and larger deletion
workloads remain outside this small comparison.
