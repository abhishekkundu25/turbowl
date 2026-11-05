# Fast OWL 2 RL materialisation: algorithms and techniques to adopt

Researched 2026-11-05. Repository state: `main` at `1e853e6`. The question:
which state-of-the-art algorithms and engineering techniques for fast OWL 2 RL
and Datalog materialisation can this project adopt, and what do the fastest
systems actually do?

This note builds on turtle-lsp's `docs/research/reasoner-sota-2026-11.md`,
cited here as "the survey" with line numbers in that file. It does not repeat
the survey's evaluations, licences, patents or integration material. Section 9
lists what this note corrects in the survey or adds to it.

Method. Papers were read in full text from the authors' PDFs or arXiv (text
extracted with `pypdf`). Numbers are quoted with the paper's hardware and data.
Repository facts come from `gh api` on 2026-11-05. Code in this repository is
cited as `path:line` at `1e853e6`. Three markers are used:

- **(analysis)**: my reasoning about this engine, not a claim from a source.
  The oracles and chaos tests must judge any change based on it.
- **(estimate)**: an expected effect I derived. It has not been measured.
- **(needs verification)**: a claim I could not check against a primary
  source, or a thin section. A second pass should deepen these.

None of this is legal advice. For patent facts, see the survey (§2.10, lines
495–515).

## Verification pass (2026-11-05)

A second pass checked the first draft against primary sources and local
data.

**What was checked.**

- Every number and quote attributed to the Oxford papers (AAAI 2014, both
  AAAI 2015 papers, IJCAI 2015, AIJ 2019, AAAI 2018, AAAI 2019, AIJ 2022,
  ISWC 2015), and to Inferray, VLog, Nemo, Free Join, DBSP, the hypertree
  paper, FlowLog, Soufflé's B-tree and Brie, Slider, Ascent (CC 2022,
  OOPSLA 2023) and Crepe: re-read from the PDFs, with setups added.
- Repository facts via `gh api` at pinned commits (datafrog `1164933`,
  Crepe `b26a580`, Ascent `e52c84b`, rayon `599425d`, reasonable `fecee2b`
  and its history).
- A sweep of 2023–2026 work (§7).
- Local facts: Brick's structure and rule-instance counts (§0.4), a re-run
  of the bench's baselines (§0.2), and the origin of the spec's 18.7 s
  (§0.3).
- Every `path:line` in this repository at `1e853e6`; two were off by two
  lines and were fixed.

**What changed.** About 45 edits: roughly 25 corrections, 15 additions of
missing setups or numbers, and the re-ranking. The notable corrections:

1. **Free Join was misread** (Summary 5, §2.2, §11). It beat DuckDB's binary
   joins on acyclic queries (geometric mean 2.94×); only classic Generic
   Join lost to them.
2. **E1 was over-estimated and the ranking changed** (§0.4, §10). On
   brick-small1, scm-sco is 51% of counted rule instances and cax-sco 5%.
   E2 is now first, then E8 and E1. Brick declares no transitive property.
3. **The baseline figures** (87 ms, 613 ms, from the task brief) do not
   match a re-run of the bench, and which column they were is unknown.
   Re-measured: reasonable reasons over brick-small1 in 12.6 ms (37.1 ms
   with loading) (§0.2). The spec's 18.7 s is a 2023 log of reasonable
   0.2.6 (§0.3).
4. **Equality.** RDFox picks the smallest resource as representative and
   rewrites the rules; AAAI 2015 says the constant problem "can still arise
   if" sameAs touches `rdf:`/`owl:` terms, not "exactly when". §4.3 now
   gives a concrete rule for ticket 11.
5. **The OpenCyc-L slowdown** (IJCAI 2015) comes from 200 idle rules, not
   cliques (§4.5). Summary 6 now names cliques, long chains and idle rules.
6. **Inferray's comparison** states no thread counts and no RDFox version
   (§1.1). **RDFox's 87×** is at 1,024 threads, not 128 (§6.1). **LUBML01K
   68 s against 8 s** compared two different stores (§7).
7. Smaller ones: RDFox's measured bytes per triple (60–67 B, Figure 2(b));
   DRed's range in AIJ 2019 Table 3 (0.02–40.37 s); DRed^c and B/F^c keep
   different counters; per-round counts are AIJ 2019 §5; licences of
   datafrog, Crepe, Feldera, FlowLog and Slider; Rust 1.86's calling rule
   for `#[target_feature]`; rayon's wasm fallback; two quote wordings and
   their sections.

All 19 **(needs verification)** markers of the first draft were resolved
or restated as "not stated in the source" or "none found".

**Still unverified.**

- What removed most of reasonable's 18.7 s between 0.2.6 and early 2026,
  and the build profile of the 2023 log: building 0.2.6 here was not
  permitted (§0.3).
- The KSEM 2026 "Chain-Aware Vectorized Datalog Reasoning" paper could not
  be opened (§7).
- Abstract-only items in §7, marked there.
- The rule-instance counts in §0.4 are a proxy for time; P0 must confirm
  them in this engine.
- §0.2's numbers were taken with a load average of 6–7: trends only.

## Summary

1. The fastest single-machine RL engines win by doing less work, not by
   faster joins:
   - RDFox's `owl:sameAs` rewriting cut derivations 3.8–85.5× (AAAI 2015,
     Table 2).
   - Hu et al.'s transitive-closure modules cut materialisation time
     3.8–109× (AAAI 2019, Table 1; thread count not stated).
   - VLog's redundant-rule pruning cut Claros/L from 3,169 s to 684 s, and
     to 644 s with memoisation (AAAI 2016, Table 3).
2. This engine already uses the layout of Inferray, which beat RDFox by
   1.6–4.2× on RDFS-Plus over LUBM (Inferray Table 3): one sorted (s, o) pair
   array per predicate, a lazy (o, s) copy, and merge joins. Its u32 ids make
   each pair half the size of Inferray's 64-bit pairs. The paper states
   neither system's thread count, and Inferray runs one thread per rule, so
   the comparison is not one of algorithms alone.
3. On the bench's Brick scenarios the work sits in different rules at
   different sizes (§0.4, counted over Nemo's closures). On brick-small1,
   scm-sco is 51% of the counted rule instances and cax-sco 5%. On
   brick-buildings, cax-sco is 35% and scm-sco 1%. Brick 1.1 and 1.5 declare
   no transitive or symmetric property and no property chain; they have 15
   and 21 inverse pairs. So the first experiments are a closure module for
   scm-sco (E2), then dropping cax-eqc1/2 (E8) and restricting cax-sco to
   types it did not produce (E1). The first draft's "15–40% faster" for E1
   on Brick is not supported.
4. Evaluate transitive rules over "external" facts, as a module that closes
   each delta within one round (Hu et al., Algorithm 4). Inferray's
   "transitive closure first" preprocessing is not safe on its own under
   OWL 2 RL, because transitivity and subclass facts can be derived during
   the run **(analysis)**.
5. Worst-case-optimal joins buy little here, because the W3C RL rule bodies
   are short joins on one key, mostly acyclic; the cyclic ones (cax-dw,
   cls-com) are triangles that leapjoin handles. Free Join does not show
   that binary plans are hard to beat: it beat DuckDB's binary joins on
   acyclic JOB by a geometric mean of 2.94×. The first draft misread it.
6. For removals, DRed fits a sorted-relation engine best: it works set at a
   time and keeps no per-fact counts. FBF adds backward chaining, which
   would need a head-directed "prove this fact" path in every rule family.
   Counting needs per-fact counts, and per-round counts for recursive rules.
   In Oxford's experiments, small deletions usually cost orders of magnitude
   less than rematerialisation. The exceptions: large cliques (Claros-LE),
   long chains (DAG-R, AAAI 2019 Table 2), and many rules that never fire
   (OpenCyc-L, IJCAI 2015).
7. ADR 0003 needs a different representative rule. RDFox picks the smallest
   resource in a total order and rewrites the rules' constants through ρ
   (AAAI 2015, Appendix A). Here the vocabulary is interned first, so "a
   vocabulary term wins, otherwise the larger class" keeps every rule
   constant a representative. Constants must still resolve through ρ for the
   rare merge of two vocabulary terms, followed by a full re-evaluation
   (§4.3, for ticket 11).
8. Parallelism: on 8 threads, RDFox reached 5.2–7.4× on inputs of
   2.2–691 M triples. This engine's round design (a read-only store and a
   per-round output buffer) can be parallelised per family and per table
   without RDFox's patented lock-free table. Threads stay behind a
   native-only feature: WebAssembly threads need nightly Rust and
   cross-origin isolation (rayon itself falls back to one thread on
   wasm32).
9. At this benchmark's scale, peak memory is dominated by something other
   than the closure: reasonable peaks at about 1,300 bytes per closure
   triple and Nemo at 310–920 (§0.2), against 8–16 bytes per stored triple
   here **(analysis)**. Measure a breakdown before changing the storage.
10. reasonable is a demanding baseline. The v1 spec's "18.7 s for Brick" is
    a 2023 log of version 0.2.6. The version the bench links reasons over
    the same input in 12.6 ms (§0.2, §0.3).
11. Corrections to the survey (§9), checked again in this pass:
    - datafrog's leapjoin is not leapfrog triejoin.
    - VLog does not use leapfrog joins.
    - FBF generalises B/F and DRed. It is not a new name for B/F.
    - Nemo Web's slowdown is confirmed at 1.6–2.0×.

---

## 0. Starting point

### 0.1 The engine at `1e853e6`

- **Relations.** A `Relation` is a sorted, duplicate-free `Vec`
  (`crates/turbowl/src/engine/relation.rs:7`), built by
  `sort_unstable` and `dedup` (`relation.rs:28`). A union merges from the
  back into the larger allocation (`relation.rs:45`, `:86`). Every skip
  gallops.
- **Variables.**
  - Each `Variable` has `stable` batches whose sizes at least double towards
    the front, a `recent` part, and a `to_add` part (`variable.rs:19-23`).
  - `changed()` tier-merges `recent` into `stable`, merges or sorts the
    queued batches, and drops known tuples by galloping through every stable
    batch (`variable.rs:57`).
  - `remap` moves rewritten tuples back into the deltas for equality
    (`variable.rs:172`).
- **Store.**
  - There is one `Table` per predicate: a (subject, object) `Variable`, plus
    an (object, subject) `Variable` built the first time a rule asks for it
    (`store.rs:15`, `:245`).
  - Every round, each family reads the store read-only and emits
    (p, s, o) triples into a buffer (`store.rs:225`, `engine/mod.rs:21-31`).
  - `step` sorts the whole buffer (`store.rs:124`) and queues each
    predicate's group in its table. The (o, s) copy is built with another
    sort (`store.rs:137`). Then `changed()` runs on every active table.
- **Joins.**
  - Merge join with galloping (`join.rs`).
  - Semi-naive join of two variables: recent × stable, stable × recent and
    recent × recent (`join.rs:54`).
  - Antijoin.
  - Leapjoin: for each prefix, the leaper with the fewest candidates proposes
    and the others intersect (`leapjoin.rs:57`, after datafrog's
    `src/treefrog.rs`).
- **Terms.** Ids are "dense `u32` in first-seen order" (`term.rs:4`, `:43`).
- **Lints.** The workspace denies `unsafe` (`Cargo.toml:21`).
- **Rules.** No rule family exists yet: `FAMILIES` is empty
  (`rules/mod.rs:18`). So there are no TurbOWL numbers, and every
  experiment below assumes that version 1 is finished.

### 0.2 Baselines measured here

The first draft quoted figures from the task brief (brick-small1: reasonable
87 ms and 39 MB, Nemo 423 ms and 24 MB; lubm-1: reasonable 613 ms and
260 MB, Nemo 1,421 ms and 60 MB), without saying which column of the bench
they were. The verification pass re-ran the bench:
`target/release/turbowl-bench --only brick-small1,brick-soda-hall,lubm-1
--engines reasonable,nemo --runs 3` at `1e853e6`, Apple M6, 12 cores,
32 GB, reasonable `=0.5.0-a1` (crates.io, linked in), `nmo` 0.10.1. The load
average was 6–7, so these are trends, not claims
(`crates/bench/README.md`, "Run it on a quiet machine").

| Scenario | Engine | Load ms | Reasoning ms, median (min–max) | Total ms | Peak MB | Closure |
|---|---|---:|---:|---:|---:|---:|
| brick-small1 | reasonable | 24.5 | 12.6 (12.4–14.5) | 37.1 | 38 | 31,580 |
| brick-small1 | Nemo | 56 | 195 (194–210) | 253 | 23 | 26,381 |
| brick-soda-hall | reasonable | 32.8 | 38.6 (36.0–40.9) | 71.4 | 57 | 64,545 |
| brick-soda-hall | Nemo | 88 | 560 (549–599) | 644 | 28 | 57,652 |
| lubm-1 | reasonable | 255.7 | 83.1 (76.9–105.9) | 332.8 | 259 | 204,521 |
| lubm-1 | Nemo | 260 | 601 (569–609) | 861 | 54 | 184,086 |

**(analysis)**

- reasonable's reasoning is 12.6 ms on brick-small1. Every figure in this
  note about "a Brick run" is about a run of tens of milliseconds, where
  fixed costs per round and per run count as much as rule work.
- reasonable's closure is larger than Nemo's because it adds triples that
  are not OWL 2 RL rules (ADR 0004, "No compatibility").
- Nemo is 7–15× slower in reasoning but uses half the memory or less.
  reasonable's peak is 1,270–1,330 bytes per closure triple; most of it is
  not the closure (§3.4).
- `crates/bench/README.md` says one Nemo rule (cls-svf1) took 99% of a
  110–200 s lubm-1 run. Nemo now takes 0.6 s on lubm-1. That sentence was
  written in `df9aab4` (13:24), before `79b2fe4` (13:37) changed
  `oracles/nemo/owl2rl.rls`, where cls-svf1 now reads `Tp(?p, ?u, ?v)`
  (line 123). The README sentence looks stale.

### 0.3 Where the spec's "18.7 s for Brick" comes from

`docs/specs/v1.md` line 15: "its own log shows 18.7 s for Brick, about
15,000 triples". The source is the example log in reasonable's README
([README at `fecee2b`](https://github.com/gtfierro/reasonable/blob/fecee2b6d3713f97c86aad56e099674dcacea75b/README.md),
"Command Line Interface"):

```
$ reasonable example_models/ontologies/Brick.n3 example_models/small1.n3 -o myout.ttl
[2023-07-04T15:31:52Z INFO  reasonable::reasoner] Loaded 14803 triples from file example_models/ontologies/Brick.n3
[2023-07-04T15:31:52Z INFO  reasonable::reasoner] Loaded 14 triples from file example_models/small1.n3
[2023-07-04T15:32:11Z INFO  reasonable] Reasoning completed in 18.66sec
```

- **Input.** The same as `brick-small1`: `Brick.n3` (Brick 1.1) has not
  changed since commit `221b19d` (2020-01-29; `gh api
  repos/gtfierro/reasonable/commits?path=example_models/ontologies/Brick.n3`).
- **Version.** The log was added by commit `3173fa1` (2023-07-04, "fix
  output; update README"), which bumped `Cargo.toml` to `0.2.6`. It has not
  been updated since. The README does not give the machine or the build
  profile. The CLI it shows is the `reasonable` binary, which the README
  offers as a static nightly build.
- **What changed since.** PR #37 (commit `19ab409`, 2026-02-19,
  "Materialization optimizations (AI generated)") rewrote cls-int1 and
  cls-uni, which "iterated through all instances for every intersection
  definition". Its author reports Brick.n3 going "from 0.51s to 0.01s on my
  computer" ([PR #37](https://github.com/gtfierro/reasonable/pull/37)). So
  by early 2026 the pre-PR code took about 0.5 s, not 18.7 s. Which change
  between 0.2.6 and early 2026 removed the rest is not pinned. The removal of
  complement-class materialisation (`c8eb495`, 2025-09-01) is not it: Brick
  has no `owl:complementOf`. Building 0.2.6 here to time it was not
  permitted in this session **(needs verification)**.
- **Conclusion.** The 18.7 s is a three-year-old figure for reasonable 0.2.6.
  The version the bench links (0.5.0-a1, `fecee2b` plus one commit) reasons
  over the same input in 12.6 ms (§0.2), about 1,500× less.

Suggested wording for `docs/specs/v1.md`, problem statement (not edited
here): "`reasonable` comes closest … But it implements part of OWL 2 RL, it
has bugs that change results, it throws away all state on any deletion, and
it is verified by hand-picked unit tests only. It is no longer slow on Brick:
its README's 2023 log shows 18.7 s for Brick plus a 14-triple model
(version 0.2.6), but version 0.5.0-a1 reasons over the same input in about
13 ms here, so it is a demanding speed baseline."

### 0.4 Brick facts the experiments depend on

Method: `oracles/owlrl/.venv/bin/python -I` with rdflib 7.6.0 for the
asserted structure. For rule-instance counts, closures computed by the
pinned `nmo` 0.10.1 with `oracles/nemo/owl2rl.rls` (closure sizes equal the
bench's Nemo column, which cross-checks them). Semi-naive evaluation
considers each rule instance once per run whatever the order, so the number
of instances of a rule equals the number of body matches in the final
closure. Scripts and outputs are in this session's scratchpad
(`scripts/brick_facts.py`, `scripts/closure_counts.py`); they are not in the
repository.

**Asserted structure.**

| | `Brick.n3` (Brick 1.1, what the bench uses) | `data/brick/Brick.ttl` (1.5.0) |
|---|---|---|
| Triples | 14,803 | 62,579 |
| `rdfs:subClassOf` | 1,676: 999 between IRIs, 677 to blank-node restrictions | 2,217, all between IRIs |
| `owl:equivalentClass` | 113: 23 between IRIs, 90 to blank nodes (class ≡ intersection of restrictions) | 106, all between IRIs |
| Named classes in the IRI hierarchy | 890 | 1,824 |
| Depth (longest asserted path to a root) | max 7, mean 4.38. 0:3, 1:4, 2:30, 3:207, 4:256, 5:205, 6:135, 7:50 | max 10, mean 5.08. 0:9, 1:79, 2:42, 3:90, 4:421, 5:482, 6:333, 7:251, 8:84, 9:27, 10:6 |
| Ancestors per class (asserted IRI edges) | max 11, mean 4.74 | max 16, mean 5.85 |
| Classes with more than one IRI superclass | 108 | 308 |
| `owl:TransitiveProperty` | none | none |
| `owl:SymmetricProperty` | none | none |
| `owl:inverseOf` | 15 | 21 |
| `owl:propertyChainAxiom` | none | none |
| `rdfs:subPropertyOf` | 1 | 24 |
| `owl:intersectionOf`, `owl:hasValue` | 754, 398 | none (1.5 moved these to SHACL) |
| `rdfs:domain`, `rdfs:range` | 9, 11 | 2, 4 |
| Asymmetric and irreflexive properties | 20 each | 80 each |
| `owl:sameAs` | none | 2: `brick:Discharge_Air` and `brick:Supply_Air`, two `brick:Substance` instances |

Brick 1.1 and 1.5 both declare `hasPart`, `isPartOf`, `feeds` and the other
relations asymmetric and irreflexive, never transitive. The building models
do not add transitive declarations (the closures below have no
`rdf:type owl:TransitiveProperty` triple).

**Rule instances in a full run**, counted over the closures:

| Input | Closure | Types | cax-sco | scm-sco | cax-eqc1/2 | prp-dom + prp-rng | prp-inv1/2 | cls-hv1 + cls-hv2 | cls-int2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| brick-small1 | 26,381 | 2,657 | 2,342 | 26,476 | 20 | 11,202 | 11,224 | 354 | 318 |
| brick-soda-hall | 57,652 | 21,805 | 35,133 | 26,476 | 3,062 | 22,736 | 26,940 | 20,196 | 5,319 |
| brick-buildings | 1,255,681 | 699,568 | 1,197,338 | 26,683 | 103,854 | 508,282 | 587,714 | 813,480 | 203,106 |
| Brick 1.5 alone | 72,823 | 12,675 | 1,822 | 32,079 | 0 | 0 | 25,406 | 0 | 0 |

Over the rules counted, scm-sco is 51% of the instances on brick-small1 and
cax-sco 5%. On brick-buildings, cax-sco is 35% and scm-sco 1%. The
`subClassOf` closure is 9,267 pairs on Brick 1.1 (217 reflexive, from the
`owl:equivalentClass` cycles that scm-eqc1 creates). cls-int1, prp-spo1, the
list-reading rules and the diagnostics are not counted. Instances per
building instance: median 8 and max 49 closure types on brick-buildings.

**What the experiments would remove** (same method):

| Input | cax-sco, E1 (T0 includes cax-eqc's output) | cax-sco, E1 with E8 | scm-sco, E2 (external facts only) |
|---|---|---|---|
| brick-small1 | 2,342 → 892 (2.6×) | → 832 (2.8×) | 26,476 → 9,614 (2.75×) |
| brick-soda-hall | 35,133 → 28,609 (1.23×) | → 19,179 (1.83×) | 26,476 → 9,614 (2.75×) |
| brick-buildings | 1,197,338 → 1,024,046 (1.17×) | → 709,849 (1.69×) | 26,683 → 9,714 (2.75×) |
| Brick 1.5 alone | 1,822 → 1,349 (1.35×) | the same | 32,079 → 12,651 (2.54×) |

T0 is large because many types come from rules other than cax-sco: prp-dom
and prp-rng, cls-hv2, cls-int1 and cls-int2, and cax-eqc1/2 while they run.
Each of these types re-enters cax-sco. That is why E1 alone gains 1.2–2.6×
on cax-sco, not the "half the depth" of a single chain.

---

## 1. Evaluation strategy

### 1.1 Semi-naive variants

There are three shapes in the fast systems:

| Shape | Who | How derivations are not repeated |
|---|---|---|
| Round-based: all rules each round, against the previous round's state | This engine (`store.rs:88`); datafrog; Inferray (Algorithm 1, §3) | Each rule joins at least one recent part. |
| Fact-at-a-time, no rounds | RDFox (Motik et al., AAAI 2014, §3) | A thread takes the next unprocessed fact F and finds the rules with an atom that matches F, through a hash index of atoms with variables replaced by `*` (8 lookups per fact). It matches atoms before position i only to facts before F, and the remaining atoms to facts up to F. "Each rule instantiation is considered at most once" (§3, Theorem 1). |
| Rule-at-a-time, output visible at once | VLog (each rule application `rule[i]` at step i writes a new table Δ, AAAI 2016, "IDB layer"); Nemo (per-rule delta tables, KR 2024 §2) | The table ranges ∆[i, j] of each predicate. |

**Evidence.** No paper found compares these shapes on equal terms, in the
first pass or in the 2023–2026 sweep (§7). The nearest is Formulog's
"eager evaluation", a work-stealing worklist in place of semi-naive rounds,
whose 5.2–7.6× gains its authors call specific to SMT-heavy programs
(OOPSLA 2024, abstract).

- Inferray (rounds) beat RDFox (fact-at-a-time) by 1.58–4.22× across the
  seven LUBM sizes on RDFS-Plus (Table 3). The authors summarise this as "a
  factor 2 except on the Wikipedia Ontology" (§6.3). This is not a
  like-for-like test of the shapes: the paper states no thread count for
  either system, Inferray runs one thread per rule (§4.3), and the CPU (Xeon
  E3-1246 v3) has 4 cores and 8 hardware threads. Which RDFox version and
  configuration ran is not stated either; the paper cites only AAAI 2014 and
  thanks the RDFox authors "for their help in configuring their system".
- Inferray lost on RDFS-Full (Table 2). Its authors attribute the loss to
  their duplicate-elimination step (§6.2).
- Nemo selects "rule application orders with fewer rule applications
  overall" from a dependency analysis (KR 2024 §2).

**Fit (analysis).**

- Rounds suit sorted relations: one sort per round and one merge per table.
- Two costs come with rounds:
  - Each round sorts the round's whole output (`store.rs:124`) and runs
    `changed()` on every active table.
  - A chain of k dependent derivations takes k rounds.
- Both costs can be measured, and neither needs a different evaluation shape.
  The first experiment iteration should count rounds and derivations per
  family (P0, §10).

### 1.2 Rule ordering, stratification, schema first

**What.**

- Split the program into strata by dependency and run each stratum to a
  fixpoint (DBRDF in AAAI 2014 §5, "Comparison Systems").
- Run transitive closures first, then the fixpoint (Inferray §3,
  Algorithm 1).
- Order rule applications by a dependency analysis (Nemo, KR 2024 §2).

**Evidence.** Stratification helps maintenance. With stratification, "the
running times of DRed improve by a factor 5.85 on average, and the numbers of
derivations reduce by a factor for 7.52" (Motik et al., AIJ 2019, §10.3.1,
Table 3). Neither pass found an isolated number for "schema first" in full
materialisation. Inferray's transitive-closure-first
numbers are mixed with the gain from Nuutila's algorithm (§1.3).

**Fit (analysis).**

- In the v1 rule set, only scm-sco and scm-eqc1 derive schema triples
  (`docs/specs/v1.md`, "The rule set of version 1"). Equality and generalized
  triples can also produce them.
- To run a TBox subset of the families to a fixpoint, and then all families,
  the engine needs no change: `Store::run` continues semi-naively from the
  stored state (`store.rs:85-88`).
- The closure cannot change, because phase two runs every rule.
- The expected effect: data rules join against a closed hierarchy from their
  first round, instead of against partial closures. This is experiment E3.

### 1.3 Transitive closure and hierarchy shortcuts

**(a) Inferray: closure before the fixpoint, with Nuutila's algorithm**
(Subercaze et al., PVLDB 9(6), 2016, §4.1).

How it works:

1. Detect strongly connected components and build the quotient graph.
2. Visit the components in reverse topological order. Compute each reachable
   set as the union of its successors' sets, stored as sets of intervals.
   Bit vectors are used in the component detection.
3. Before this, split the graph into connected components with union-find and
   renumber the nodes densely.

It applies this to `subClassOf`, `subPropertyOf`, every transitive property,
and `owl:sameAs` (§4.1).

Evidence (Table 4). Setup: Xeon E3 1246v3 (4 cores, 8 threads), 8 MB L3
cache, 32 GB, Linux 3.13, JDK 7u67, mean of 5 runs after 2 warm-ups, 15 min
timeout; thread counts not stated (§6). Times for a single subClassOf
chain:

| Chain length | Inferray | 2014 RDFox | OWLIM-SE |
|---|---|---|---|
| 100 | 22 ms | 8 ms | 45 ms |
| 2,500 | 612 ms | 87,358 ms | 361,231 ms |
| 5,000 | not quoted | 597,286 ms | no result |
| 25,000 | 14,712 ms (313 M triples) | no result ("–") | no result ("–") |

Caveat. Inferray assumes that every "p is transitive" statement "must hold
prior to the inference process" (§4.1). Under the W3C rules over generalized
triples, this is false **(analysis)**:

- cax-sco derives `p rdf:type owl:TransitiveProperty` from a subclass of
  `owl:TransitiveProperty`.
- scm-eqc1 derives `subClassOf` facts during the run.
- `owl:sameAs` merges change both.

So a closure-first step can only be a fast path. The transitivity rules must
stay.

**(b) Hu, Motik, Horrocks: modular materialisation** (AAAI 2019; arXiv
1811.02304; journal version AIJ 2022, survey line 392).

- **Transitivity module (§5).** The module keeps the "external" facts X_R:
  the R-facts that other rules produced. The invariant: "each fact R(a0,an) is
  produced by a chain {R(a0,a1),...,R(an−1,an)} ⊆ X_R of 'external' facts".
  So the closure needs only pairs where at least one fact is in X_R. The
  paper mentions a program rewriting with the same effect (R replaced by X_R
  in the heads of other rules and in one body atom of the transitivity
  rule) but does not use it. `Add_tc` (Algorithm 4, lines 41–48) closes each
  delta with a work queue (lines 44–47).
- **Symmetric-transitive module (§6).** It keeps connected components and
  adds all pairs when two components merge.

Evidence. Setup: Dell PowerEdge R730, 512 GB, 2× Xeon E5-2640 2.6 GHz,
Fedora 27 (§7). No thread count is stated in the AAAI PDF, in arXiv
1811.02304v2 or its appendix (proofs only), or in the AIJ 2022 version
(which used other hardware: 2× E5-2660, 128 GB). Treat it as unknown.

Table 1, full materialisation:

| Data | Mat-Mod | Plain semi-naive | Speed-up |
|---|---|---|---|
| Claros-LE | 733.55 s | 3,593.32 s | 4.9× |
| LUBM-LE | 291.90 s | 1,100.22 s | 3.8× |
| DBpedia-SKOS | 103.23 s | 3,623.37 s | 35× |
| DAG-R (10 k nodes, 100 k edges) | 29.60 s | 3,238.86 s | 109× |

Table 2, deleting 1,000 facts:

| Data | DRed^c-Mod | DRed^c |
|---|---|---|
| Claros-LE | 0.93 s | 1,035.28 s |
| LUBM-LE | 0.32 s | 3.87 s |
| DBpedia-SKOS | 21.77 s | 691.32 s |
| DAG-R | 64.92 s | 3,005.11 s |

Small deletions are the mean of 10 samples of 1,000 facts (Table 2). On DAG-R
the modular deletion (64.92 s) costs more than modular materialisation from
scratch (29.60 s): over long chains, overdeletion and rederivation dominate.

The survey's quote, transitive or symmetric rules "can have a cubic number of
applications, yet they can derive at most a quadratic number of facts" (line
369), is from the abstract of the AIJ 2022 version (vol. 308, 103726). The
AAAI 2019 paper says "O(n³) rule applications, whereas a custom algorithm can
achieve the same goal using only O(n²) steps" (§1).

**(c) VLog: redundant and subsumed rules** (Urbani, Jacobs, Krötzsch,
AAAI 2016, "Dynamic Optimization").

- **Redundant rules.** Rule r is not applied to the output of rule o when
  resolving the two gives a trivially redundant rule. This check runs on the
  data. Evidence (MacBook Pro, 2.2 GHz i7, 16 GB, Table 3): Claros/L took
  3,169 s with no optimisation, 684 s with this one alone, and 644 s with both
  optimisations. It made little difference on LUBM and DBpedia.
- **Subsumed rules** (not implemented in VLog). "r1 does not need to be
  applied to inferences produced by r2 in step o if r3 has already been
  applied to all facts up until step o". The paper adds: "It is safe for a
  rule to subsume part of its own application in this way."

**(d) Ascent's BYODS.** A relation can be backed by `trrel_uf`, "a union-find
based data structure" for transitive relations
([s-arash/ascent README](https://github.com/s-arash/ascent), BYODS section).

**Fit (analysis).**

- **scm-sco and prp-trp: experiment E2.** On brick-small1, scm-sco is 51% of
  the rule instances counted in §0.4, and the module would cut them 2.75×.
  One family per transitive predicate
  keeps X_R as an internal table: the pairs not produced by the transitivity
  rule itself. It closes each round's delta with a work queue (Hu et al.,
  Algorithm 4, lines 41–47) and emits all new pairs in the same round.
  Requirements:
  - Internal tables must be left out of the output: `Store::triples()` and
    `len()` iterate every table.
  - When a predicate becomes transitive during the run, X_R must be seeded
    from all of its current facts.
- **cax-sco: experiment E1.** Apply cax-sco only to `rdf:type` facts that
  cax-sco did not produce (call them T0), joined with `subClassOf`.
  - Why the closure is unchanged: every `type(x, D)` that cax-sco derives
    rests on a chain `type(x, C0)` (in T0), `sco(C0, C1)`, …, `sco(Ck, D)`.
    scm-sco is in v1, so it derives `sco(C0, D)`. The restricted rule then
    derives `type(x, D)` when that fact arrives, because the semi-naive join
    pairs the new `sco` delta with T0.
  - The gain: for one instance at the bottom of a chain of depth d, the work
    falls from d(d+1)/2 rule instances to d. On Brick the measured gain is
    smaller, 1.2–2.6× on cax-sco (§0.4), because many types enter T0 from
    other rules and the hierarchy is shallow (mean 4.7 ancestors).
  - Other rules (cax-dw, cls-int1 and so on) still read the full `rdf:type`
    table.
  - This is VLog's subsumed-rule case with r1 = r2 = r3 = cax-sco and a
    transitive `subClassOf`. The argument checks out: scm-sco closes every
    `subClassOf` fact whatever produced it, so no chain is lost.
- **prp-trp.** Neither Brick 1.1 nor 1.5 declares a transitive property
  (§0.4). Only lubm-* exercises prp-trp (`subOrganizationOf`, chains of
  length 2).
- **prp-spo1.** The same trick does not apply in v1: scm-spo is not in the
  rule set, so `subPropertyOf` is never closed. It would need an internal,
  unexported closure.
- **cax-eqc1 and cax-eqc2: experiment E8.** These are redundant given
  scm-eqc1 and cax-sco, which are both in v1. scm-eqc1 turns `c1 eqc c2` into
  `c1 sco c2` and `c2 sco c1`, and cax-sco then derives what cax-eqc1/2
  derive, one round later. This holds over generalized triples too: neither
  rule constrains the terms. With E1, E8 also shrinks T0: cax-eqc's output
  would otherwise re-enter cax-sco (§0.4).

### 1.4 Specialised rules versus generic Datalog

**What.**

- RDFox: the fixed W3C rules over the triple encoding are "inefficient
  because the fixed program's rules contain complex joins". Ontology-specific
  programs (Grosof et al. 2003) have shorter rules, and are complete only if
  the data does not contain "'non-sensical' triples such as ⟨rdf:type,
  rdf:type, rdf:type⟩" (AAAI 2014 §2). AAAI 2015 §2 repeats the "complex
  joins" point but not this quote.
- "Due to simpler rules, OWLIM-Lite consistently processed the custom
  datalog programs several times faster than the fixed OWL 2 RL/RDF rules"
  (AAAI 2014 §5, "Comparison Systems").
- Inferray classifies its rules (α, β, γ, δ, same-as, θ) and implements each
  class once. For example, a δ-rule copies the second antecedent's table,
  possibly reversed, into the result (§4.4).
- VLog shares column objects for rules that only copy data (AAAI 2016, "IDB
  layer").

**Fit (analysis).**

- ADR 0004 fixes the W3C rules over generalized triples as the target, so
  RDFox's "no nonsensical triples" shortcut is not available.
- ADR 0001 compiles the fixed rules into Rust over predicate tables. A schema
  atom such as `rdfs:domain` becomes a small relation scanned in the outer
  loop, and the data atom becomes a table looked up by predicate id. That is
  the shape of an ontology-specific program, without an interpreter.
- What remains is a review point for tickets 07–09, not an experiment:
  - Put the schema relation in the outermost loop.
  - Copy whole key ranges when a rule is a pure copy. For example, prp-dom
    types every subject in table p with C.

### 1.5 Inferray's sort-merge evaluation per predicate

**What** (Inferray §4.2–§5).

- Vertical partitioning: (s, o) arrays sorted per property. A ⟨o,s⟩-sorted
  copy is "computed lazily upon need. This cache may be cleared at runtime if
  memory is exhausted", and is invalidated when the table changes (§4.2).
- Each round: sort the inferred triples, remove duplicates, and merge them
  into the main store, keeping the new triples as the next delta (§4.3,
  Figure 2).
- Sorting is "the cornerstone" (§5):
  - Dense numbering: properties count down from 2³², resources count up from
    2³² + 1 (§5.1).
  - Counting sort for pairs when the array is larger than its value range
    (§5.2).
  - Adaptive MSD radix sort otherwise (§5.3).

**Evidence** (Table 1, Java, millions of pairs per second, range r and size
n). For r = 1 M and n = 25 M:

| Sort | Million pairs per second |
|---|---|
| Inferray's counting sort | 66.8 |
| Inferray's adaptive MSD radix | 19.1 |
| Generic mergesort | 16.2 |
| Generic quicksort | 5.2 |
| SIMD radix, Radix128 (adapted from Satish et al., SIGMOD 2010, not run on Inferray's machine) | 22.7 |

Inferray's sorts were "at least on par with state-of-the art algorithms for
83.3% of the considered cases", three times faster at best, and at 70% in the
worst case (5 M elements, 25.58 bits of entropy) (§5.4).

**Fit (analysis).**

- The layout is the same as this engine's.
- The differences:
  - This engine uses u32 ids, not 64-bit ones.
  - It keeps (o, s) up to date as a second `Variable` (`store.rs:137`)
    instead of invalidating it.
  - It removes duplicates in `changed()` (`variable.rs:57`).
- Ids are dense by construction (`term.rs:4`), which is Inferray's
  precondition for counting and radix sorts.
- Whether a radix sort beats Rust's `sort_unstable` on (u32, u32) or
  (u32, u32, u32) keys has not been measured here. Oatlog reports an MSD
  radix sort of u32 rows 1–1.3× faster than `sort_unstable` (§7). This is
  experiment E6, which waits for a profile.

---

## 2. Joins

### 2.1 What the W3C RL rule bodies need (analysis)

Shapes, by the rule tables of OWL 2 Profiles §4.3:

| Shape | Rules (v1 set) | Operator here |
|---|---|---|
| A data atom and a schema atom, one shared variable | prp-dom, prp-rng, prp-spo1, prp-symp, prp-inv1/2, prp-eqp1/2, cax-sco, cax-eqc1/2, cls-hv1/2, cls-int2, cls-uni (scm-eqc1 has one schema atom) | merge join, schema outermost |
| Path (acyclic) | scm-sco, prp-trp, prp-spo2 (chains of any length), cls-svf1, cls-avf | binary merge joins, one order per delta position |
| Star on one variable | cls-int1: `type(x,C1) ∧ … ∧ type(x,Cn)`; prp-fp and prp-ifp (a self-join on the subject or the object) | leapjoin: intersect n sorted instance lists; merge self-join |
| Two atoms sharing both variables | prp-pdw `p1(x,y) ∧ p2(x,y)`; prp-asyp `p(x,y) ∧ p(y,x)` | one merge over (s, o) pairs, against (s, o) or (o, s) |
| Triangle (cyclic) | cax-dw: `disjointWith(c1,c2) ∧ type(x,c1) ∧ type(x,c2)`; cls-com, the same shape with `complementOf` | leapjoin: for each delta `type(x,c1)`, intersect x's types with c1's disjoint classes |

### 2.2 Evidence on worst-case-optimal joins

- Free Join (Wang, Willsey, Suciu, PACMMOD 1(2), June 2023,
  doi:10.1145/3589295; arXiv 2301.10841; implemented in Rust). Setup:
  MacBook Air M1, 16 GB, "all systems are configured to run single-threaded";
  "binary join" is DuckDB's hash join (§5.1).
  - The paper argues against the belief that "WCOJ is designed for cyclic
    queries" (§1). On acyclic JOB, Free Join beat DuckDB's binary join by a
    geometric mean of 2.94× (up to 19.36×, minimum 0.85×) and Generic Join
    by 9.61× (up to 31.6×). On cyclic LSQB, up to 15.45× and 4.08× (§5.2).
  - What lost to binary joins on acyclic queries was classic Generic Join,
    not worst-case-optimal joins as such. Free Join wins by being able to
    behave like a binary plan where that is best, with lazy, column-oriented
    tries (COLT, hash maps per level).
  - Lesson for this engine (analysis): a worst-case-optimal join is not
    slower on acyclic bodies when it is built well, but the W3C rule bodies
    are two- or three-atom joins on one key, where a merge join already is
    the efficient plan. The data structure (hash tries) does not carry over
    to sorted relations.
- Nemo uses leapfrog triejoin over its column tries (KR 2024 §2).
- Hypertree decompositions for complex rules: "usually significantly faster
  than the baseline, sometimes by orders of magnitude" (Zhang, Hu, Nenov,
  Horrocks, IJCAI 2023, pp. 3383–3393, doi:10.24963/ijcai.2023/377). Setup:
  Dell R730, 512 GB, 2× Xeon E5-2640, thread count not stated. Table 2,
  materialisation: LUBM-500 L+C 29,577.90 s standard against 554.00 s
  combined; YAGO 155,022 s against 366.03 s. On plain LUBM L (short rules)
  the decomposition alone was slower (740.81 s against 95.73 s), and the
  combined method 75.50 s. The W3C RL rules are of the short kind.

### 2.3 Join order

- RDFox reorders the atoms of each delta-position query with "a simple greedy
  strategy" (AAAI 2014 §3).
- Motik et al. show that a plan chosen for full materialisation can be poor
  for a small maintenance delta (AIJ 2019 §9.2, Example 37).

**Fit (analysis).** Merge join, antijoin and leapjoin cover every shape in
§2.1. In hand-written families, each semi-naive variant can have its own loop
order, with the smaller side outermost. This is a review point for prp-spo2
(ticket 08), not an experiment. No worst-case-optimal-join experiment is
proposed (§11).

---

## 3. Storage, indexing and memory

### 3.1 Layouts and bytes per triple

| System | Layout | Bytes per triple (as published, or computed from published numbers) |
|---|---|---|
| RDFox, 2014 | A six-column triple table (s, p, o and three "next" pointers). `I_s`, `I_p` and `I_o` are arrays indexed by id. `I_sp`, `I_op` and `I_spo` are open-addressing hash tables of triple pointers (AAAI 2014 §4). | A worst case of "at most 80 bytes per triple", or 46 with 4-byte pointers, dictionary excluded (§4). Measured, Figure 2(b): ClarosL 60 B, LUBML01K 65 B, ClarosLE 44 B, DBpediaL 67 B per triple after materialisation. |
| RDFox, 2015 | The same family | "As low as 36.9 bytes per triple" (ISWC 2015 abstract): Claros with the ClarosLE program and rewriting, 539.2 M triples, 4-byte pointers. Large cliques keep the indexes small. LUBM-50K needed 101 B per triple (§5, Table 1). |
| VLog | On-disk input layer with six permutation indexes. Derived facts live in column tables, one per rule application, sorted and run-length encoded (AAAI 2016) | Peak memory for LUBM1K/L: 2,198 MB, against 11,884 MB for 2014 RDFox (Table 2) |
| Nemo | Sorted column tables accessed as tries, with "run-length encoding with increments"; strings and IRIs are dictionary ids (KR 2024 §2) | Wikidata, 8.1 G triples: two index orders took 220 GB, "not counting the string dictionary" (§5.3), about 27 B per triple for both orders (computed) |
| Inferray | Sorted pair arrays per property, 64-bit ids, lazy ⟨o,s⟩ copy | 16 B per pair per order (computed from §4.2) |
| Soufflé | Specialised B-trees; Brie for dense data | Brie used "on average 2× less space than a highly optimized B-tree" in an OpenJDK points-to analysis (PMAM 2019 §1; 4× Xeon E5-4650, 32 cores, 256 GB) |
| This engine | Sorted (u32, u32) pairs per predicate, lazy (o, s) copy | 8 B per pair per order: 8–16 B per triple at steady state, plus transient copies while merging **(analysis)** |

### 3.2 Cache behaviour

- Inferray's L1 data-cache miss rate stayed at about 3.4% whatever the rule
  set. 2014 RDFox had 1.2% on RDFS, rising to 11% on WordNet with RDFS-Plus
  (Inferray §6.2, §6.4).
- RDFox's own explanation: index nested-loop joins over hash indexes access
  memory randomly. Its authors suggest that "hyperthreading and a high degree
  of parallelism can compensate for this drawback" (AAAI 2014 §1; §5 says
  "can mitigate the effect of CPU stalls due to random memory access").

### 3.3 Compression

- VLog and Nemo compress sorted columns with run-length encoding (VLog:
  maximal runs of a constant become ⟨a, n⟩; Nemo adds increments).
- Brie "effectively compresses dense data". It is up to 4× faster than
  B-trees in a points-to analysis (up to 15× in micro-benchmarks), but
  "Bries only perform well for particular workloads exhibiting relations
  with high-density data" (Jordan et al., PMAM 2019, abstract and §1).
- Zhang, Hu, Nenov, Horrocks, "Optimised Storage for Datalog Reasoning"
  (AAAI 2024, pp. 10748–10755, doi:10.1609/aaai.v38i9.28947) store the
  output of transitive and union rules compactly, "sometimes by orders of
  magnitude" less memory (abstract; not read in full). Relevant to E2 if
  memory, not time, becomes the target.

**Fit (analysis).** In this engine, the (o, s) index of `rdf:type` has long
runs of the same key (a class). Storing the distinct keys, offsets and values
(CSR) would roughly halve its size **(estimate)**. That needs joins and
leapers that read CSR parts. This is experiment E7, for memory, and only if
the breakdown in §3.4 shows that relations dominate.

### 3.4 Where memory goes at this benchmark's scale (analysis)

- lubm-1 is "about 0.1 M triples" of input (`crates/bench/src/scenario.rs:56`;
  0.12 M per `crates/bench/README.md`). Its closure is 184,086 triples
  (Nemo, §0.2); at 16 B per triple that is about 3 MB, far below
  reasonable's 259 MB and Nemo's 54 MB peaks.
- So the measured peaks are dominated by something else: the term dictionary
  (strings), parser buffers, the allocator, or the binary. The bench should
  report a breakdown (P0) before any storage experiment.
- Transient peaks in this engine:
  - `Relation::merge` extends the larger `Vec` (`relation.rs:86-88`) and may
    reallocate to twice its capacity.
  - `changed()` merges tiers (`variable.rs:57`).
  - The first request for an (o, s) index sorts a copy of every pair of that
    predicate (`store.rs:117`).

---

## 4. Equality

### 4.1 What RDFox does

Source: Motik, Nenov, Piro, Horrocks, "Handling owl:sameAs via Rewriting",
AAAI 2015.

- **The cost of the axioms (§3).** For a clique of n equal terms, the
  equality rules derive n² `sameAs` triples "via 2n³ + n² + n derivations".
  Each triple whose terms lie in cliques of sizes n_s, n_p and n_o expands to
  n_s·n_p·n_o triples, "each of which is derived n_s + n_p + n_o times".
- **The algorithm (§4 and Appendix A of the extended version,
  arXiv:1411.3622).**
  - A map ρ sends each term to its representative. RDFox updates ρ without
    locks.
  - The representative is the smaller resource: "the smaller resource
    (according to an arbitrary total order) is selected as the
    representative for the other one … An ordering on resources is needed to
    prevent cyclic merges and to ensure uniqueness of the algorithm's result"
    (Appendix A; Algorithm 4, lines 7–8: `c := min{a,b}`,
    `ρ.mergeInto(d,c)`). In the implementation, the order is the resource
    id. Nothing in the paper chooses by class size.
  - Two queues hold rewritten rules and outdated resources.
  - Outdated facts are marked, not removed, and dropped at the end.
  - Rules are rewritten serially when all threads are idle.
- **Rules must be rewritten too.** "Whenever a resource r is rewritten in
  the data, r must also be rewritten in the rules; hence, the rule set cannot
  be assumed to be fixed during the course of materialisation" (§1). "The
  problem is less acute when using a fixed rule set …, but it can still arise
  if owl:sameAs triples involve rdf: or owl: resources (with a fixed rule
  set, these are the only resources occurring in rule bodies)" (§3). When
  all threads are idle, one thread "updates P to ρ(P), reindexes it, and
  inserts the updated rules (if any) into the queue R of rules for
  reevaluation" (§4); a rewritten rule is evaluated again over all facts
  (Appendix A, Algorithms 1–2).
- **Later Oxford work.** The IJCAI 2015 paper orders constants "such that ≈
  is the smallest constant", so `owl:sameAs` is always its own
  representative, and normalises and re-applies the rules after each merge
  (§2; Algorithm 6, lines 51–58). Benedikt, Motik and Tsamoura (AAAI 2018,
  goal-driven query answering with equality) restate the smallest-term
  choice and say it is used in RDFox. No Oxford paper found (2014–2022)
  prefers rule constants in general, or chooses by class size.
- **Evidence.** Setup: 2× Xeon E5-2643, 8 cores (16 virtual), 128 GB.

  | Data | Triples, axioms / rewriting | Derivations, axioms / rewriting | One thread, axioms / rewriting |
  |---|---|---|---|
  | Claros | 1.28× fewer | 11,009 M / 128 M (85.5×) | 2,042.9 s / 65.8 s (31.1×) |
  | OpenCyc | 1,176 M / 141 M (7.8×) | 45.9× fewer | 2,093.7 s / 119.9 s (17.5×) |
  | UniProt | the same (1.0×) | 8.5× fewer | 370.6 s / 143.4 s (2.6×) |
  | UOBM | 3.2× fewer | 3.8× fewer | 2,696.7 s / 1,152.7 s (2.3×) |

  Sources: Tables 2 and 3. The authors conclude: "the primary benefit of
  rewriting lies in less work needed to match the rules, rather than … in
  reducing the number of derived triples" (§6).

### 4.2 What this project plans (ADR 0003)

- A union-find of term ids, in which "the larger class's representative
  wins" (`docs/adr/0003-equality-by-representatives.md:16`).
- Rewritten tuples go back into the deltas (`variable.rs:172`).
- Output expands each representative to the members of its class.
- Ticket 11 has not been implemented yet.

### 4.3 Gap: the constants in the rules (analysis; correctness, not speed)

The rule families look up tables by constant predicate ids (`rdf:type`,
`rdfs:subClassOf`, …) and compare against constant class ids (`owl:Thing`,
`owl:Nothing`).

- Suppose `ex:t owl:sameAs rdf:type`, and the class of `ex:t` is larger. Then
  under ADR 0003's "larger class wins", `ex:t` becomes the representative,
  the `rdf:type` table is rewritten under `ex:t`'s id, and every rule that
  asks for `rdf:type` finds nothing.
- RDFox avoids this by rewriting the rules (§4.1). This engine's rules are
  compiled Rust with `vocab::` constants, so rewriting them means resolving
  each constant through ρ at the start of each round.
- This engine's ids make RDFox's own choice nearly free: the vocabulary is
  interned first, so every term a rule uses as a constant has a smaller id
  than any input term (`term.rs:7-9`, `vocab` at `term.rs:485`).
  "Smallest id wins", RDFox's rule, therefore always keeps a vocabulary
  term as its own representative when it is merged with input terms.

**Recommendation for ticket 11.**

1. **Representative.** If the merged class contains a vocabulary term
   (id below `vocab::IRIS.len()`), the smallest such id is the
   representative. Otherwise keep ADR 0003's "larger class wins", which
   bounds rewriting at O(n log n) moves over n merges. Pure smallest-id
   (RDFox's choice) is also correct, but a long chain of merges into a small
   low-id class can rewrite the big class each time. Record the change in
   ADR 0003, which now says only "the larger class's representative wins".
2. **Constants through ρ.** Rule families take their constants from the
   `Round` (for example a `Round::constant(vocab::RDF_TYPE)` that returns
   ρ(id)), not as bare `vocab::` ids. That covers the one case that rule 1
   does not: two vocabulary terms merged with each other
   (`rdfs:subClassOf owl:sameAs owl:equivalentClass`).
3. **Rules whose constant changed are new rules.** After such a merge, a
   family that now reads table ρ(c) has never joined that table's stable
   tuples with its other atoms. `remap` moves only the tuples that change,
   so semi-naive evaluation alone is incomplete here. RDFox re-evaluates
   rewritten rules over all facts. The simplest equivalent here: when the
   representative of any vocabulary term changes, mark every table's tuples
   recent and run the next round over everything (one naive round), or
   re-run from the base, as v1 already does for removals. It is rare and
   only has to be correct.
4. **Predicate merges.** When a merged-away term is used as a predicate
   (eq-rep-p's case), its whole table must move. `Store` keys tables by
   predicate id (`store.rs:54`), so ticket 11 must move that table's pairs
   into the representative's table (as deltas) and point the index at it.
   `Variable::remap` rewrites the tuples inside a table, not table keys.
5. **Tests.** The chaos generator and the eq-* minimal inputs should include
   `sameAs` on vocabulary terms: one vocabulary term with an input term in
   each position, and two vocabulary terms. Nemo and the reference
   evaluator, which axiomatise equality, are the judges.

Prior art outside Oxford agrees with the rest of ticket 11's design.
egglog canonicalises every tuple after each rule step and takes the
semi-naive delta after that rebuild (PLDI 2023), as `Variable::remap` does by
queueing rewritten tuples in `to_add` (`variable.rs:168-172`). Eqlog keeps
the id with the most tuples as representative (arXiv 2302.05792), a variant
of "larger class wins" measured in tuples rather than members.

This is a correctness item for ticket 11, not a ranked experiment.

### 4.4 The cost of expanding the output

The `sameas-clique` scenario has 1,000 aliases with 10 triples each, "about
11 M closure triples expanded" (`scenario.rs:90-92`). Its time is bounded by
the output size, so it measures expansion more than reasoning **(analysis)**.
Streaming the expansion, with no materialised copy, keeps its peak memory near
the stored closure. This should be checked once ticket 11 exists.

### 4.5 Equality and incremental updates

Source: Motik, Nenov, Piro, Horrocks, "Combining Rewriting and Incremental
Materialisation Maintenance for Datalog Programs with Equality", IJCAI 2015
(arXiv 1505.00212). Setup: 2× Xeon E5-2670, 2.6 GHz, 256 GB, Fedora 20 (§4,
"Test Setting"); the paper states no thread count. Table 1 compares B/F≈
with rematerialisation with rewriting (Remat≈) and B/F with axiomatised
equality (B/F^A).

| Data | Facts deleted | B/F≈ | Remat≈ | B/F^A |
|---|---|---|---|---|
| DBpedia-L | 100 | 0.3 s | 47.5 s | 8.9 s |
| Claros-LE | 100 | 16.1 s | 4,397 s | 5,703 s |
| Claros-LE | 10 k | 4,300 s | 4,627 s | not run |
| OpenCyc-L | 1 k | 1,062 s | 222 s | 5,537 s |

At 10 k facts, Claros-LE is close to break-even. Two causes of slow updates
are named (Discussion):

- **Cliques.** "B/F always recomputes each changed clique", said of
  UOBM-100-U, where one constant has a class of 3,930 members.
- **Idle rules, on OpenCyc-L** (incremental 1,062 s against 222 s for
  rematerialisation). "OpenCyc-L contains about 200 rules of the form ⟨x,
  type, y⟩ ← ⟨x, Ri, y⟩ that never fire during forward chaining; however,
  to check provability of ⟨a, type, C⟩, Algorithm 4 considers … each of the
  200 rules. After removing all such 'idle' rules manually, B/F≈ and Remat≈
  could update 1k tuples in roughly the same time." The first draft of this
  note blamed cliques for OpenCyc-L; that was wrong.

**Fit (analysis).** v1 answers every removal with a full run (ADR 0003).
Keep that policy for any removal that touches a merged class. Use incremental
deletion only for removals that touch no merged class (§5, experiment E4).

---

## 5. Incremental maintenance and deletion

### 5.1 The algorithms

| Algorithm | Idea | Source |
|---|---|---|
| DRed | Overdelete every consequence of the removed facts, rederive what still has support, then insert forward. | Gupta, Mumick, Subrahmanian, SIGMOD 1993 (survey line 387) |
| B/F | Before deleting a fact, look for another proof by backward chaining. "Several orders of magnitude more efficient than the DRed algorithm on some inputs, and it was never significantly less efficient." | Motik et al., AAAI 2015, abstract and Table 2 (single-threaded RDFox, 2× Xeon E5-2670, 256 GB). Examples: UOBM-1k-Uo, 100 facts deleted: B/F 5.4 s, DRed 8,797.6 s, rematerialisation 3,482.0 s. Claros-LE, 2.5 k deleted: B/F 10.3 s, DRed 8,077.4 s, remat 5,235.1 s; at 10 k deleted B/F jumps to 3,907.2 s ("one clique shrinks by one vertex"). LUBM-1k-L, 10 M deleted: B/F 102.2 s, DRed 127.9 s, remat 99.5 s. B/F beats DRed "up to the 'equilibrium' point … on Claros-L, this involves deleting 12% of the explicit facts". |
| FBF, optimised DRed, recursive counting | FBF mixes backward chaining with overdeletion under a strategy. "Both DRed and B/F correspond to instances of FBF parameterised with suitable strategies." Counting is extended to recursive rules. | Motik et al., AIJ 269 (2019), §1, §5–§7 |
| DRed^c, B/F^c | DRed^c keeps two counters per fact, non-recursive and recursive derivations, which removes all backward rule evaluation and limits overdeletion. B/F^c keeps only the non-recursive counter, which removes backward evaluation for non-recursive rules (§3). | Hu, Motik, Horrocks, AAAI 2018, arXiv 1711.03987 (survey line 391) |
| Modular (transitive and symmetric-transitive modules) | Specialised deletion for transitive and clique-shaped relations. | Hu et al., AAAI 2019 (§1.3 above) |
| B/F≈ | B/F combined with sameAs rewriting. | IJCAI 2015 (§4.5) |
| Differential dataflow, DBSP | Collections of differences with weights (Z-sets), and nested time for recursion. | DBSP: Budiu, McSherry, Ryzhyk, Tannen, PVLDB 16(7):1601–1614, 2023, doi:10.14778/3587136.3587137; extended in VLDB Journal 34(4), 2025, doi:10.1007/s00778-025-00922-y ("we give a general algorithm for solving the incremental view maintenance problem for arbitrary DBSP programs", including "monotonic and non-monotonic recursion"). |

### 5.2 Evidence

Source: AIJ 2019. Setup: Dell PowerEdge R730, 512 GB, 2× Xeon E5-2640
2.6 GHz, Fedora 24; "we conducted all tests on a single thread" (§10.2).

**Deleting 1,000 random facts** (Table 3, without stratification; mean of 30
samples, with 95% confidence intervals as large as the values for Claros LE):

| Instance | Counting | DRed | FBF |
|---|---|---|---|
| UOBM-01K R | 60.11 s | 15.36 s | 0.34 s |
| ChEMBL L | 11.98 s | 0.02 s | 0.02 s |
| Claros LE | 26.15 s | 5.45 k s | 582.89 s |

"DRed and FBF are faster than counting, often by several orders of magnitude,
in all cases except for ClarosLE, and for ReactomeR and DRed", and "FBF is
fastest on average across all test instances, followed by DRed and then
counting" (§10.3.1). Claros LE is the clique case (`relatedPlaces`; "the
largest one connects 2,270 constants"). Excluding it, DRed ranges from
0.02 s to 40.37 s (ReactomeR) in Table 3.

**Break-even points**, where an incremental delete takes as long as
rematerialising (Table 7, without stratification; one seed, binary search
to within 2.5%):

| Instance | Counting | DRed | FBF |
|---|---|---|---|
| ClarosLE | 1.5 M (8%) | 18.4 k (0%) | 73.4 k (0%) |
| DBpediaL | 205.4 M (53%) | 60.4 M (16%) | 72.5 M (19%) |
| UOBM-01KL | 119.4 M (47%) | 79.9 M (31%) | 95.5 M (37%) |

For large updates, "counting is the best on average, followed by FBF and then
DRed", and "its break-even point was never smallest" (§10.4.1).

**Memory.** "An important drawback of the counting algorithm … is increased
memory use: facts are associated with counters whose count is, in the worst
case, determined by the total number of rule instances fired by the seminaïve
algorithm" (§6, opening). Recursive counting keeps counts per iteration (the
"trace", §5).

**Implementation (§9).**

- The paper's implementation stores all facts of a predicate in one relation,
  with "a bit-mask … that reflects membership in various sets" (§9.1).
- "Nonrepetition is not essential for the correctness of DRed and FBF", but
  the counting algorithm needs it (§9.2).

### 5.3 Fit with a sorted-relation engine (analysis)

- **Additions** are already handled the right way. "Adding facts to E is
  trivial as one can simply continue from where the initial materialisation
  has finished" (IJCAI 2015 §1), and `Store::run` does exactly that
  (`store.rs:85-88`).
- **DRed** fits.
  - Overdeletion can reuse the forward rule families: run them on a store
    whose recent parts hold the deleted facts and whose stable parts hold the
    old closure. Removing the overdeleted set needs a sorted difference.
  - The new code is in rederivation: deciding whether a fact is derivable in
    one step from the survivors. The cheapest correct version reuses the
    forward families for one naive round over the survivors and keeps only
    the outputs that were overdeleted. That costs about one round of a full
    run. A head-directed check per family would be faster, but it is new code
    in every family.
- **FBF** needs that per-family check, plus backward chaining with a set of
  checked facts and a set of proved facts (AIJ 2019 §7). It is the second
  step after DRed.
- **Counting** conflicts with this engine's set semantics:
  - `changed()` drops duplicates instead of adding up their counts.
  - Recursive counting needs counts per round.
  - Neither fits sorted, duplicate-free relations without doubling their
    width.
- **Differential dataflow or DBSP** would replace the engine (ADR 0001) with
  (tuple, time, weight) collections.
  - FlowLog, built on differential dataflow, competes with Soufflé from 4 to
    64 threads (§6.3).
  - FlowLog publishes no single-threaded runs and no runs on inputs of
    editor size; its runs start at 4 threads and include CSV loading. Whether
    it builds for wasm32 was not checked; differential dataflow depends on
    timely dataflow's worker threads **(analysis)**.

---

## 6. Parallelism

### 6.1 RDFox

Source: AAAI 2014. Setup: Dell, 2× Xeon E5-2650, "16 physical cores,
extended to 32 virtual cores via hyperthreading", 128 GB, RDFox capped at
100 GB (§5, "Test Setting"). Speed-ups are against the parallel store on one
thread, from Figure 2(a):

| Threads | Speed-up |
|---|---|
| 8 | 5.2× (LUBML01K) to 7.4× (ClarosL, UOBML01K), all 12 tests (inputs of 2.2–691.1 M triples, Table 1) |
| 16 | 6.1× (DBpediaL) to 13.9× (ClarosL), 11 tests (LUBMLE05K ran out of memory) |

- DBpediaL saturates at 6.6× with 32 threads, and only 28.1% of its triples
  are "active", that is, have a rule applied to them.
- The concurrent storage has a cost even on one thread: "the overhead of
  lock-free updates is between 10% and 30%". For example, ClarosL took
  1,907 s with sequential storage and 2,477 s with concurrent storage on one
  thread.
- The paper is inconsistent with itself on 32 virtual cores: 19.3× in §1,
  19.5× in §5.
- With rewriting, the 2015 paper reports 5.2–6.7× on 8 threads (AAAI 2015,
  Table 3; 2× Xeon E5-2643, 8 physical cores).
- On a SPARC T5-8 (128 cores, 1,024 hardware threads, 4 TB), RDFox reached
  "speedups of up to 87 times" and "reasoning rates of up to 6.1 million
  triples per second" (ISWC 2015, abstract). The 87× is DBpedia with
  1,024 threads; at 128 threads the best is 69.6× (LUBM-50K) (§5,
  Table 1).
- The parallel storage design is the subject of US11409698B2, active to about
  2035 (survey lines 503, 600).

### 6.2 Inferray

Each rule runs "on a dedicated thread and holds its own inferred property
table to avoid potential contention". Merging "deals with independent
property tables" and is "easily parallelized" (§4.3). The paper gives no
isolated speed-up for this, and no thread count for any run (checked in
this pass).

### 6.3 Soufflé and its successors

- Soufflé's B-tree uses optimistic locking and "hints" that reuse earlier
  traversals. In parallel micro-benchmarks it was "up to 59× higher
  performance than state-of-the-art industrial standards"; integrated into
  Soufflé, it gave "3× higher, overall system performance". Setup: 4×
  Xeon E5-4650, 8 cores each, 256 GB, GCC 5.4 -O3 (Jordan, Subotić, Zhao,
  Scholz, PPoPP 2019, pp. 327–339, abstract and evaluation).
- FlowLog's authors (PVLDB 19(3):361–374, the VLDB 2026 volume,
  doi:10.14778/3778092.3778098; arXiv 2511.00865). Setup: CloudLab, 2× AMD
  EPYC 7543 (64 physical cores), 256 GB; times end to end including CSV
  loading, median over up to five join orders, 900 s timeout:
  - Soufflé and Flan "only distribute the outermost for-loop among threads",
    which "is insufficient to saturate resources even for simple recursive
    queries".
  - On CSPA (httpd), Soufflé was fastest at 4 threads (67.8 s against
    FlowLog's 112.6 s). From 4 to 64 threads, FlowLog gained 7.8× (to
    14.4 s) and Soufflé 1.3× (to 50.9 s) (Table 1, §10.1).
- Ascent offers `ascent_par!` on rayon (README.MD:84–89 at `e52c84b`).

### 6.4 Fit (analysis)

- A `Round` reads the store immutably and writes only to its own buffer
  (`store.rs:225`; `engine/mod.rs:21-31`). So:
  - Families can run on a rayon pool, each with its own buffer.
  - In `step`, the insertion and `changed()` for each table are independent,
    so tables can run in parallel.
  - Within a family, the recent slice can be split into chunks. This is the
    outer-loop parallelism that FlowLog finds limited at 64 threads, which is
    enough on 4–16 cores **(estimate)**.
- This is a different design from the patented one, which uses per-thread
  reserved areas and buckets locked by compare-and-set (survey line 503).
  Whether the claims read on it is a question for a professional.
- RDFox's numbers come from inputs of millions of triples; its smallest,
  UOBMU010 with 2.2 M, gained 6.1× on 8 threads (AAAI 2014, Figure 2(a)).
  Neither pass found a published speed-up for inputs of Brick's size
  (15–200 k triples). A
  sequential path is needed for small deltas, both to avoid pool overhead and
  because WebAssembly has no threads (§8).
- This is experiment E5.

---

## 7. Systems

Repository facts are from `gh api` on 2026-11-05.

| System | Architecture | What makes it fast | Licence and status | Published numbers on RL-like data |
|---|---|---|---|---|
| RDFox | C++. Triple table with linked lists and hash indexes. Parallel fact-at-a-time semi-naive evaluation; sameAs rewriting; B/F maintenance (survey line 400). | Even work distribution with no rounds; ontology-specific programs; rewriting. | Proprietary, owned by Samsung (survey §2.8). | §6.1 and §4.1. AAAI 2014, LUBML01K: 68 s with the sequential store; the parallel store took 73 s on one thread and 8 s on 16 (8.7×). 2016 comparison (VLog Table 2, sequential 2014 RDFox): LUBM1K/L 82 s and 11,884 MB. |
| Nemo | Rust. Sorted column tries with run-length encoding; leapfrog triejoin; semi-naive evaluation with per-rule delta tables; rule order from dependency analysis (KR 2024 §2). | Compact columns and streaming multi-way joins. | `APACHE-2.0 OR MIT` (`Cargo.toml`); pushed 2026-11-04. | Dell XPS 13, i7-1165G7, 16 GB; "Nemo is currently running in a single thread" (§6). Times include loading; mean of 3 runs (KR 2024 Table 2). LUBM-01k (133.6 M facts, 136 rules): 167.6 s. Galen: 5.4 s. Nemo Web (Firefox 125, under 4 GB) is "1.6−2.0 times slower" and ran out of memory on LUBM-01k. |
| VLog | C++. On-disk input layer; column tables per rule application with run-length encoding; merge or hash joins chosen by heuristic; redundant-rule pruning; memoisation (AAAI 2016). | Memory: 6–46% of 2014 RDFox. Dynamic pruning. | Apache-2.0; last push 2023-07-16. | Table 2, MacBook Pro 2.2 GHz i7, 16 GB, both systems single-threaded (VLog "does not use parallelism"; RDFox recompiled with "the sequential storage variant"): LUBM1K/L 38 s and 2,198 MB; Claros/L 644 s (RDFox 2,418 s); LUBM5K/L 196 s (RDFox ran out of memory). |
| Soufflé | Compiles Datalog to parallel C++; specialised B-trees; Brie for dense data. | Compiled join loops; data structures specialised for Datalog. | UPL-1.0; pushed 2026-10-31. | Compiled LUBM-01k: 145.7 s (KR 2024 Table 2). |
| Inferray | Java. Vertical partitioning with sorted 64-bit pairs; Nuutila's closure first; counting and radix sorts; one thread per rule. | Sequential memory access; closure outside the fixpoint. | Apache-2.0 ([jsubercaze/inferray](https://github.com/jsubercaze/inferray)); inactive since 2016-11-20. | RDFS-Plus on LUBM, RDFox in parentheses: 1 M 19 ms (69 ms); 10 M 540 ms (855 ms); 100 M 2,514 ms (10,613 ms) (Table 3). Thread counts and RDFox's version not stated (§1.1). |
| Slider | Java. Incremental RDFS and ρdf reasoner for streams, rules on a thread pool. | Not read in depth. | Apache-2.0 ([juleschevalier/slider](https://github.com/juleschevalier/slider) LICENSE.txt; last push 2016-01-21). | Chevalier, Subercaze, Gravier, Laforest, SIGMOD 2015 demo, pp. 1081–1086, doi:10.1145/2723372.2735363 ([HAL](https://hal.science/hal-01163676)). "On average 71.47% faster than OWLIM-SE, with a throughput up to 36,000 Triples/sec" (§3); times include parsing; 4-core AMD at 1.4 GHz, 16 GB. Table 1 has two losses (wikipedia RDFS −23.42%). |
| datafrog | Rust. Sorted `Vec` relations, `Variable`s with stable, recent and to-add parts, leapjoin (`src/treefrog.rs:6-45` at `1164933`). | Small; merge joins over sorted data. | Apache-2.0 or MIT (Cargo.toml, both licence files); pushed 2026-10-28. | None for RL. reasonable is built on it (survey line 442). |
| Crepe | Rust procedural macro: "Semi-naive evaluation", "Stratified negation", "Automatic generation of indices for relations" (README). | Rules compiled with the program. Relations are `HashSet`s; each index is a `HashMap` from the bound columns to a `Vec` of tuples (`src/lib.rs:763-765` at `b26a580`). | MIT or Apache-2.0 (`Cargo.toml`); pushed 2025-12-14. | None for RL, and no published numbers. The README says transitive closure on about 10⁶ relations runs "at comparable speed to compiled Souffle". |
| Ascent | Rust macros; lattices; `ascent_par!` on rayon; BYODS, for example `trrel_uf` (README). | Custom data structures per relation. | MIT; pushed 2026-08-29. | CC 2022 (pp. 77–88, doi:10.1145/3497776.3517779; Core i7-8650U, 16 GB, Ascent single-threaded): 0.99–4.1× slower than datafrog on Polonius (Table 1), 0.8–1.6× of compiled Soufflé on 5-clique (Table 3). BYODS (PACMPL 7(OOPSLA2), art. 264, doi:10.1145/3622840; Ryzen 9 4900H, 32 GB): `trrel_uf` 0.022 s against 1.5 s for Soufflé on email-Eu, 39 s against 595 s on ca-AstroPh, and alone in finishing BrightKite (775 s) (Table 3); "around an order of magnitude" faster and up to about 30× less memory. |
| DBSP / Feldera | Rust; Z-sets; incremental view maintenance of any DBSP program, including recursion (PVLDB 16(7), 2023). | Incremental by construction. | MIT, except code behind the `feldera-enterprise` feature, which is why GitHub's detector says NOASSERTION; pushed 2026-11-05. | None for RL. |
| Differential dataflow | Rust, on timely dataflow. | Arrangements: "sharing indexed, consistent state and computation between the operators" of concurrent dataflows (McSherry, Lattuada, Schwarzkopf, Roscoe, PVLDB 13(10):1793–1806, 2020, §1). | MIT; pushed 2026-11-04. | None for RL. |
| DDlog | Datalog compiled to differential dataflow. | Not read. | MIT; archived ([vmware-archive/differential-datalog](https://github.com/vmware-archive/differential-datalog), last push 2023-07-07). | None. |
| FlowLog | Datalog on differential dataflow, with a relational IR per rule, "logic fusion", "subplan reuse" and Boolean specialisation (PVLDB 19(3), 2025). | Reuse of indexes across rules; scaling with threads. | MIT or Apache-2.0 ([flowlog-rs/flowlog](https://github.com/flowlog-rs/flowlog); the paper's link to hdz284/FlowLog is dead). | Program analysis, not RL. Fastest in 21 of 41 cases at 4 threads and 36 of 41 at 64 (§6.3). |
| reasonable | Rust on datafrog, with the W3C rules written by hand. | Not read in this note. | BSD-3-Clause. | The speed baseline here (§0.2; `docs/research/reasonable-parity.md`). |

**Work from 2023–2026.** A sweep in this pass covered arXiv, Crossref, the
ISWC 2024 accepted papers, Datalog 2.0 2024 (CEUR Vol-3801), publisher
pages and the leads named in the brief. DBLP was unreachable (a bot check),
so venues were confirmed through Crossref, DOIs and publisher pages. No new
OWL 2 RL materialisation algorithm, and no new `owl:sameAs` materialisation
work, was found for 2023–2026; RDFox's AAAI and IJCAI 2015 papers are still
the latest on equality. The relevant work is general Datalog:

| Work | Venue | Idea | Numbers (setup) | For this engine |
|---|---|---|---|---|
| egglog: Zhang, Wang, Flatt, Cao, Zucker, Rosenthal, Tatlock, Willsey, "Better Together: Unifying Datalog and Equality Saturation" | PACMPL 7(PLDI), art. 125, 2023, doi:10.1145/3591239 | Ids in a union-find; after each rule step, "rebuild" canonicalises every tuple until no merge is left; the semi-naive delta is taken after the rebuild, so rewritten tuples fire rules again. Shows that joining through an equivalence relation (Soufflé's eqrel) costs more than canonicalising. | 4.96× faster than the fastest sound Soufflé encoding on Steensgaard points-to (postgresql); hardware not found in the text | Confirms ticket 11's design (`Variable::remap` queues rewritten tuples as the next delta). Read in full by the sweep. |
| Eqlog: Bidlingmaier, "An Evaluation Algorithm for Datalog with Equality" | arXiv 2302.05792, 2023, no venue found | Per-id occurrence lists; on a merge only the losing id's tuples are re-normalised. The id with the longer occurrence list survives. Each round runs equality conclusions first, then normalisation, then the other rules. | None | "Larger class wins" by tuple count rather than member count is a cheap variant for ticket 11. Occurrence lists probably cost more than a rescan of sorted arrays at this scale **(analysis)**. |
| Oatlog: Gustafsson, Magnusson, Luque Cerpa | EGRAPHS 2025 (PLDI workshop) abstract; MSc thesis, Chalmers 2025 | egglog compiled ahead of time by a Rust macro; u32 ids; union-find without path compression, merging younger ids into older; MSD radix sort of rows, 1–1.3× faster than `sort_unstable`; replaced static B+trees with hash maps because access was random | Over 100× faster than egglog under 100 e-nodes, about 2× at 10⁶ (Ryzen 5900X) | A modest prior for E6: radix sort of u32 rows gained 1–1.3× there. |
| Rassau, Schuhknecht, "One Size Does NOT Fit All: On the Importance of Physical Representations for Datalog Evaluation" | arXiv 2602.05651, 2026, preprint | Sorted array, B+tree, radix tree and hash table compared for Datalog relations | A sorted array with one batch sort per round beat the worst choice by almost 11× when new tuples are unique; with many duplicates in the new tuples a hash table was about 5× faster, because duplicate removal dominates (i9-12900K, 128 GB) | Supports this engine's layout. The risk it names is duplicates: P0's "emitted against new" per family measures it. cax-sco emits mostly duplicates on deep hierarchies (§0.4). |
| Zhang, Hu, Nenov, Horrocks, "Optimised Storage for Datalog Reasoning" | AAAI 2024, doi:10.1609/aaai.v38i9.28947 | Storage schemes per rule: transitive closure as interval labels over the SCC condensation; a scheme for union rules | DBpedia `broader` (159,561 facts, 29.1 M after closure): standard 2,845 s and 1,481 MB, Hu et al.'s TC module 28.2 s, the TC scheme 8.44 s and 250 MB. On the full program over a small input, no gain (31.4 s against 33.4 s) and more memory (Dell R730, 2× E5-2640, 512 GB) | Computing a closure by SCC and topological order is E2's follow-up. Compressed storage does not help an engine that must output every triple. |
| Hypertree decompositions (Zhang, Hu, Nenov, Horrocks) | IJCAI 2023 | §2.2 | §2.2 | Not relevant: W3C RL rules are short. |
| Free Join (Wang, Willsey, Suciu) | PACMMOD 1(2), 2023 | §2.2 | §2.2 | Not relevant to hand-written two-atom joins. |
| VFLog: Sun, Kumar, Gilray, Micinski, "Column-Oriented Datalog on the GPU" | AAAI 2025, doi:10.1609/aaai.v39i14.33665 | Sorted columns with a hash map from value to (offset, count) | LUBM-01K: 0.16 s on an H100, 6.47 s for its own CPU build, Nemo 62.3 s, RDFox 56.8 s (EPYC 9534, 64 cores) | GPU: not relevant. The offsets index is E7's CSR idea. |
| GPUlog (ASPLOS 2025), multi-GPU Datalog (ICS 2025), SRDatalog (arXiv 2604.20073) | | GPU and distributed engines | abstracts only | Not relevant to single-threaded wasm. |
| Carac: Herlihy, Martres, Ailamaki, Odersky, "Adaptive Recursive Query Optimization" | ICDE 2024 | Join reordering at run time | abstract only | Not relevant: join orders here are hand-written. |
| Flan: Abeysinghe, Xhebraj, Rompf | PACMPL (POPL) 2024, doi:10.1145/3632928 | Datalog by staging in Scala | 1.4–12.5× over other engines (abstract) | Not relevant. |
| Bembenek, Greenberg, Chong, "Making Formulog Fast" | PACMPL (OOPSLA) 2024, doi:10.1145/3689754 | "Eager evaluation": a work-stealing worklist instead of semi-naive rounds | 5.2× and 7.6× on SMT-heavy workloads; the authors say the gain is specific to SMT | Not relevant. |
| FlowLog (§6.3), DBSP (§5.1), Ascent BYODS (§1.3(d), §7) | PVLDB 19(3); PVLDB 16(7); OOPSLA 2023 | | | As discussed above. |
| Nemo group: Ivliev, Krötzsch, Marx, "SPARQLing Datalog …" (ESWC 2026, doi:10.1007/978-3-032-25156-5_27); "Recovering Explanations from Transformed Rule-Based Ontologies" (ISWC 2026, doi:10.1007/978-3-032-42026-8_10) | | SPARQL import with magic sets; explanations | abstracts only | Not relevant to speed. No Nemo performance paper was found for 2025–2026. |
| Du, Lin, Ma, "Chain-Aware Vectorized Datalog Reasoning over Large Knowledge Graphs" | KSEM 2026, doi:10.1007/978-981-92-2868-3_16 | unknown | could not be opened (login) | Unverified lead **(needs verification)**. |
| Incremental DatalogMTL (arXiv 2511.12169; AAAI 2026 per the sweep) | | temporal Datalog | not read | Not relevant. |

One finding about Brick itself: Brick 1.5 states its class definitions
(intersections, `owl:hasValue`) as SHACL rules, not OWL (§0.4: 754
`owl:intersectionOf` and 398 `owl:hasValue` in 1.1, none in 1.5), and
py-brickschema runs those through a SHACL-AF engine. On Brick 1.5 an
OWL 2 RL reasoner derives less than on 1.1; the bench's Brick scenarios use
1.1.

The survey covers the new Rust OWL reasoners (lines 165–168, 484–493).

---

## 8. WebAssembly constraints

The core must build for `wasm32-unknown-unknown` with default features off
(`docs/specs/v1.md`, "Build").

| Technique | On wasm32 | Source |
|---|---|---|
| Sequential algorithms: the transitive-closure modules, restricted cax-sco, schema first, DRed, union-find, radix sort, CSR | Unchanged: pure safe Rust with no threads or I/O. | **(analysis)** |
| Threads (experiment E5) | Need nightly Rust, `+atomics,+bulk-memory`, `-Z build-std`, and a cross-origin-isolated page. Rust's own docs call multi-threaded WebAssembly "still in a somewhat 'early days' phase". rayon 1.12 compiles for wasm32 without atomics and "will fall back to sequential iteration" (README, "Usage with WebAssembly"; `rayon-core/src/registry.rs:207-226`), so the feature need not be absent there, but a feature that the wasm build leaves off keeps the dependency out of it. | Survey §3.6, line 583; [core::arch::wasm32](https://doc.rust-lang.org/core/arch/wasm32/index.html) |
| SIMD | `simd128` is enabled with `#[target_feature(enable = "simd128")]` or `-Ctarget-feature=+simd128`. Arithmetic intrinsics such as `i32x4_add` are safe functions; memory intrinsics such as `v128_load` are unsafe. Since Rust 1.86, safe functions can carry `#[target_feature]`, but such a function "can only be safely called from other functions marked with the target feature attribute"; elsewhere the call needs `unsafe`. The workspace denies `unsafe` (`Cargo.toml:21`), so SIMD here means auto-vectorisation, safe lane intrinsics inside `#[target_feature]` functions, or a crate-wide `-Ctarget-feature=+simd128` (not verified to make the calls safe). Relaxed SIMD is a separate feature. | [core::arch::wasm32](https://doc.rust-lang.org/core/arch/wasm32/index.html); [Rust 1.86 release notes](https://blog.rust-lang.org/2025/04/03/Rust-1.86.0/) |
| Memory | wasm32 is capped at 4 GiB. Memory64 costs "from just 10% to over 100%". Nemo Web ran out of memory on LUBM-01k, where native Nemo took 167.6 s. Compact u32 relations (and CSR, E7) matter more in the browser than natively. | Survey §3.6, line 585; KR 2024 Table 2 |
| Speed | Nemo Web is "1.6−2.0 times slower than the command-line client". SPEC CPU ran 45–55% slower on 2019 engines. | KR 2024 §5.2; survey line 586 |

---

## 9. Corrections and additions to the survey

Line numbers refer to turtle-lsp's `docs/research/reasoner-sota-2026-11.md`.

1. **Line 509** (patents, leapfrog tree-join row): "That describes leapfrog
   triejoin, which Nemo and datafrog's leapjoin use."
   - Nemo uses leapfrog triejoin (KR 2024 §2).
   - datafrog's leapjoin is not the leapfrog iterator over trie indexes.
     For each source tuple, it extends one variable: the leaper with the
     fewest candidates proposes, and the others intersect
     (`src/treefrog.rs:15-35` at `1164933`). A chain of leapjoins does bind
     variables one at a time over sorted arrays, so the difference is one of
     mechanism. ADR 0001 already makes this distinction.
   - Whether the claims read on leapjoin is a legal question.
2. **Line 9** (summary): "columnar storage with leapfrog joins (VLog, Nemo)".
   VLog is columnar, but it "decides heuristically to use a hash or a merge
   join" (AAAI 2016, materialisation section). Only Nemo uses leapfrog
   triejoin.
3. **Line 390**: "Forward/Backward/Forward (FBF, the journal name for the B/F
   idea)". The journal presents FBF as a generalisation: "both DRed and B/F
   correspond to instances of FBF parameterised with suitable strategies"
   (AIJ 2019 §1).
4. **Line 124**: "FBF generally fastest on small (1,000-fact) updates" is
   consistent with §10.3.1. Add the other half of the result: for large
   updates, counting has the largest break-even points on average
   (Table 7, §10.4.1).
5. **Lines 338–339** (open question on Nemo Web): resolved. The text reads
   "just 1.6−2.0 times slower than the command-line client" (KR 2024 §5.2).
6. **Line 376** (RDFox AAAI 2014): correct. Add two points:
   - The concurrent storage costs 10–30% on one thread.
   - The paper itself gives both 19.3× (§1) and 19.5× (§5, Figure 2(a))
     for 32 virtual cores.
7. **Line 411** (sameAs rewriting) notes that rules are rewritten as well.
   Add the consequence for a fixed rule set: the problem "can still arise if
   owl:sameAs triples involve rdf: or owl: resources" (AAAI 2015 §3), and
   RDFox picks the smallest resource as representative (Appendix A). See
   §4.3.
8. **Not in the survey:**
   - Inferray (PVLDB 2016)
   - Slider (SIGMOD 2015)
   - Soufflé's B-tree and Brie (PPoPP and PMAM 2019)
   - Free Join
   - FlowLog
   - Hypertree decompositions for Datalog
   - VLog's redundant and subsumed rules
   - RDFox's finding that the fixed W3C rule set runs slower than
     ontology-specific programs (AAAI 2014 §2, §5)
9. **Lines 121–122** (Nemo and VLog rows): the numbers check out against
   KR 2024 Table 2 and AAAI 2016 Table 2, including Claros/L RDFox 2,418 s.
10. **Line 369** cites the AIJ 2022 paper for the "cubic number of
    applications" quote: correct, it is in that paper's abstract (not in
    the AAAI 2019 version).

The verification pass checked items 1–9 against the sources. Item 1 holds
as a description of mechanism; items 2–7 and 9 hold; item 7 was reworded
("can still arise if", not "exactly when").

---

## 10. Ranked experiments for `docs/specs/experiments.md`

Ranked by expected speed-up per unit of effort. Every experiment assumes v1
is finished and verified.

**How to measure.** Use `crates/bench` (this reasoner, reasonable and Nemo on
the same inputs). Take the median and spread of 5 or more runs on a quiet
machine, record peak memory and closure size, and use `--compare` against the
kept baseline (`docs/specs/experiments.md`, "Rules"; `docs/specs/v1.md`,
"Benchmark").

The verification pass re-ranked the list on the rule-instance counts of
§0.4. "Counted instances" below means the rules in §0.4's table; cls-int1,
prp-spo1, list reading, diagnostics and fixed costs per round are not in
them, so P0 must confirm every share before an experiment is judged.

| Rank | Id | Change | Main scenarios | Effort | Expected effect **(estimate)** |
|---|---|---|---|---|---|
| — | P0 | Counters: tuples emitted and new per family and round, rounds, time in the sort and in `changed()`, a memory breakdown | all | small | None. It makes the others measurable, and should reproduce §0.4's counts. |
| 1 | E2 | Transitive-closure module over external facts, for scm-sco first (prp-trp later) | brick-small1, brick-soda-hall | small to medium | scm-sco instances 26,476 → 9,614 on brick-small1: about 30% of counted instances. Closes the hierarchy within one round. |
| 2 | E8 | Stop evaluating cax-eqc1/2, which scm-eqc1 and cax-sco cover | brick-soda-hall, brick-buildings | trivial | Removes 3,062 (soda) and 103,854 (buildings) instances, 2–3% of counted. Makes E1 larger. |
| 3 | E1 | cax-sco only from types it did not produce (T0), joined with `subClassOf` | brick-buildings, brick-soda-hall | small | After E8: cax-sco 1.20 M → 0.71 M instances on buildings (about 14% of counted), 35 k → 19 k on soda (about 10%), about 3% on brick-small1 |
| 4 | E3 | Schema first: TBox families to a fixpoint, then all families | brick-*, lubm-1 | very small | Fewer rounds; 0–10%, less after E2, which already closes the hierarchy in one round |
| 5 | E9 | Copy rules without copying: prp-inv1/2 output as a view of the inverse table's (o, s) index (VLog's shared columns) | brick-* | medium | prp-inv1/2 are 21% of counted instances on brick-small1 and 17% on buildings. Gain unknown until P0 shows what a copy costs. |
| 6 | E4 | DRed for removals that touch no merged class and no vocabulary term; full run otherwise and above a threshold | incremental-remove | high | Cost per removal falls from one full run to about one round of one |
| 7 | E5 | Parallel rounds: families and per-table steps on rayon, native only | lubm-10, lubm-50, brick-buildings | medium | lubm-50 2–4× faster on 8 threads; brick-small1 unchanged |
| 8 | E6 | Radix or counting sort in `step`, `from_vec` and `changed()`, only if P0 shows sorting above about 20% of a run | lubm-50, brick-buildings | small to medium | Unknown until profiled |
| 9 | E7 | CSR storage for the (o, s) index of `rdf:type` | lubm-50, brick-buildings (memory) | medium | Peak memory of relations falls by about 25–50% |

Why the order changed. The first draft put E1 first with "brick full run
15–40% faster". On the bench's main Brick scenario, brick-small1, cax-sco is
5% of counted instances and scm-sco 51% (§0.4). E1 matters on
brick-buildings, and twice as much after E8. Brick declares no transitive
property, so E2's prp-trp half is for lubm-* only. E9 is new: Brick's 15
inverse pairs make prp-inv1/2 one of the largest rule groups.

Speed context. reasonable reasons over brick-small1 in 12.6 ms (§0.2). On
that scenario, a 30% cut in rule instances may not show up if fixed costs
per round dominate. P0's time split decides.

Not ranked: the representative choice and constants through ρ (§4.3). It is
a correctness item for ticket 11.

### P0. Counters before claims

**Hypothesis.** Experiments E1–E3, E6 and E7 make claims about derivations,
rounds, sort time and memory. Without counters, the "explain-the-number" rule
cannot be met.

**Change.** Record, per run:

- tuples emitted and tuples new, per family and per round (emitted tuples
  are rule instances; they should match §0.4's counts on the Brick inputs)
- the number of rounds
- time in the sort at `store.rs:124` and in `changed()`
- bytes held by relations, against the dictionary and everything else

**Measure.** The counters appear in the bench result file. The check is that
time does not regress by more than the run-to-run spread.

**Risks.** The counters add overhead. Gate them behind a feature, or keep them
to one integer add per batch.

**When.** Counting emitted tuples per family costs one `len()` before and
after each family's call in the round loop. Ticket 10, which writes that
loop, can include it at no extra design cost.

### E2. Transitive-closure module for scm-sco (then prp-trp)

**Hypothesis.** The plain rule R(x,z) ← R(x,y), R(y,z) considers every
pair of R-facts that meet, about n³/6 instances on a chain of n nodes.
Joining only external facts with R considers about n²/2, and a work queue
closes each delta in one round. On Brick's hierarchy, scm-sco is the
largest rule group on brick-small1.

**Source.** Hu, Motik, Horrocks, AAAI 2019, §5 and Algorithm 4; Table 1
(3.8–109×, rule sets far larger than Brick's). Inferray's Table 4 shows
naive engines collapsing on chains of 2,500 nodes. Measured here (§0.4):
scm-sco instances 26,476 → 9,614 on brick-small1 and soda (2.75×), 32,079
→ 12,651 on Brick 1.5 (2.54×).

**Change.**

- One module for `rdfs:subClassOf`, with an internal X_R table: the
  `subClassOf` facts from the input, scm-eqc1, and any other rule or
  equality rewrite, but not from scm-sco itself.
- It closes each round's delta with a work queue (Algorithm 4) and emits
  every new pair in the same round.
- X_R is internal: `Store::triples()` and `len()` iterate every table
  (`store.rs:199-214`), so they must skip it.
- Second step, a separate iteration: the same module for each
  `owl:TransitiveProperty`, seeding X_R from all current facts when a
  predicate becomes transitive during the run. Only lubm-* has one
  (`subOrganizationOf`, chains of length 2: `lubm.rs:111`, `:204`, `:210`),
  so it gains little on this bench.

**Expected effect (estimate).** About 30% fewer counted instances on
brick-small1, and fewer rounds, since the hierarchy closes in one round.
Under 1% on brick-buildings, where the TBox is a small part.

**Measure.** brick-small1, brick-soda-hall and Brick 1.5 via the oracle
inputs: time, rounds, scm-sco instances from P0. lubm-1 for no regression.

**Risks.**

- Every producer of `subClassOf` must feed X_R, including equality
  rewriting and generalized triples (prp-spo1 with `rdfs:subClassOf` as a
  super-property). The oracles catch a miss.
- At 12.6 ms for reasonable on brick-small1, fixed costs may hide the gain.

**Follow-up.** If scm-sco stays hot on large, cyclic hierarchies, compute
the TBox closure with Nuutila's algorithm (Inferray §4.1) as a fast path,
keeping this module as the fallback. Brick 1.1 has 217 reflexive
`subClassOf` pairs in its closure, from the cycles scm-eqc1 creates.

### E8. Drop cax-eqc1 and cax-eqc2

**Hypothesis.** scm-eqc1 together with cax-sco derives every triple that
cax-eqc1/2 derive (§1.3), so evaluating cax-eqc1/2 only adds duplicate
derivations.

**Change.** Remove cax-eqc1/2 from the class family while scm-eqc1 and
cax-sco are both active.

**Expected effect (estimate).** From §0.4: 20 instances on brick-small1,
3,062 on brick-soda-hall and 103,854 on brick-buildings, 2–3% of counted
instances. Brick 1.1 has 113 `owl:equivalentClass` triples (23 between
named classes, 90 to intersection restrictions); Brick 1.5 has 106. The
larger value of E8 is that it shrinks T0 for E1 (1.17× becomes 1.69× on
buildings).

**Measure.** brick-soda-hall and brick-buildings: time and type
derivations; all oracles.

**Risks.**

- Results arrive one round later.
- The v1 spec lists cax-eqc1/2 as rules. Their triples are still derived,
  but a rule test that expects them in the same round would change.
- If a later feature switches scm-eqc1 off, cax-eqc must come back.

### E1. cax-sco only from types it did not produce

**Hypothesis.** cax-sco re-derives supertypes from supertypes.
Restricting cax-sco's input to T0 (types not produced by cax-sco) and
joining it with the `subClassOf` closure derives the same closure with
fewer instances: on a chain of depth d, d instead of d(d+1)/2.

**Source.** VLog's subsumed rules (AAAI 2016, "Dynamic Optimization");
Hu et al.'s external facts (AAAI 2019 §5). The argument for an unchanged
closure is in §1.3 **(analysis)**. Measured here (§0.4): with E8,
cax-sco instances 1,197,338 → 709,849 on brick-buildings (1.69×), 35,133
→ 19,179 on soda (1.83×), 2,342 → 832 on brick-small1 (2.8×).

**Change.**

- Add an internal predicate for T0, with an id outside the dictionary's
  range (`term.rs:88-104` reserves `u32::MAX`).
- Every emitter of `rdf:type` except cax-sco also emits into T0. So does
  the loading of input triples.
- cax-sco joins T0 with `rdfs:subClassOf`.
- T0 is left out of `triples()` and `len()`.

**Expected effect (estimate).** After E8: about 14% fewer counted
instances on brick-buildings, 11% on soda, 3% on brick-small1. Small on
lubm-*, whose hierarchy is shallow (`lubm.rs:28-45`). Memory grows by T0: 565,592
pairs on brick-buildings, about 4.5 MB at 8 B per pair.

**Measure.** brick-buildings, brick-soda-hall, brick-small1, lubm-1 and
lubm-10: time, and type derivations from P0. All oracles and chaos for
closure equality.

**Risks.**

- Every path into the `rdf:type` table must also feed T0, including
  equality remapping and generalized triples (a rule with a variable
  predicate can emit `rdf:type`). Missing one breaks completeness; the
  oracles would catch it.
- Under future DRed (E4), T0 must also be maintained.

### E3. Schema first

**Hypothesis.** Running scm-sco, scm-eqc1 and the list-reading families to
a fixpoint before the data rules means the data rules never join against a
partial hierarchy. That cuts rounds and repeated derivations.

**Source.** Inferray, Algorithm 1. Stratification cut DRed's derivations
7.52× in maintenance (AIJ 2019 §10.3.1), which is a different setting. No
isolated number for full materialisation was found.

**Change.** The reasoner runs `store.run(tbox_families)` and then
`store.run(all_families)`. `Store::run` continues semi-naively
(`store.rs:85-88`). The engine does not change.

**Expected effect (estimate).** 0–10% on brick-*, near zero on lubm-*. Rule
instances do not change: semi-naive evaluation considers each instance once
whatever the order (§0.4). Only rounds and the sizes of intermediate deltas
change. After E2, which closes the hierarchy within one round, little is
left.

**Measure.** brick-small1, brick-buildings and lubm-1: time and rounds.

**Risks.** None for correctness, because phase two runs every rule. Too
small a gain can be lost in noise: use 10 runs.

### E9. Copy rules without copying

**Hypothesis.** prp-inv1/2 copy one table, reversed, into another. Brick
1.1 has 15 inverse pairs and Brick 1.5 has 21. These rules are 21% of the
counted instances on brick-small1 and 17% on brick-buildings (§0.4). If q
is the inverse of p, q's (s, o) pairs are p's (o, s) pairs, so q can be
served from p's object index instead of being derived, sorted and merged.

**Source.** VLog shares column objects between rules that only copy data
(AAAI 2016, "IDB layer"). The rest is analysis.

**Change.** A table can be an alias: "the reverse of p". Reads of q go to
p's (o, s) index; output iterates both. Both directions must be merged
when either table gains facts from another rule.

**Expected effect (estimate).** Unknown until P0 shows the cost of a copy
relative to a join. Memory falls by one table per inverse pair.

**Measure.** brick-*: time, peak memory, and prp-inv instances from P0.

**Risks.** Every reader of a predicate must go through the alias. Facts
derived for q by other rules (prp-dom, prp-spo1, equality) must still land
in p's object index. This touches the `Store` interface, so it comes after
the cheaper items.

### E4. DRed for removals

**Hypothesis.** Removing one data triple from Brick plus a building model
affects a small part of the closure, while v1 re-runs everything.

**Source.**

- AIJ 2019, Table 3: deleting 1,000 facts took DRed 0.02–40.37 s on every
  instance except the clique case ClarosLE (5.45 k s); FBF was fastest on
  average.
- AIJ 2019, Table 7: DRed's break-even point can be close to 0% on hard
  programs (ClarosLE, ReactomeU).
- Hu et al. 2019, Table 2; its DAG-R row shows deletion over long chains
  costing more than materialising from scratch.
- AAAI 2015 (B/F), Table 2: DRed took 8,797.6 s to delete 100 facts from
  UOBM-1k-Uo, where B/F took 5.4 s and rematerialisation 3,482.0 s.

**Change.**

1. Overdeletion with the forward families, on a store whose recent parts
   hold the removed facts and whose stable parts hold the old closure.
2. A sorted difference.
3. Rederivation as one naive round of every family over the survivors,
   keeping outputs in the overdeleted set.
4. Semi-naive reinsertion.
5. A fallback to the full run when the removal touches a merged equality
   class or a vocabulary term, or when overdeletion passes a threshold (for
   example 20% of the closure, to be tuned).

Diagnostics are recomputed for affected terms.

**Expected effect (estimate).** The median cost per change in
incremental-remove becomes a fraction of brick-small1's full run. How large a
fraction depends on how many rounds a full run takes, since rederivation
costs about one round. With a head-directed check per family (the next step,
toward FBF), it becomes proportional to the change.

**Measure.**

- incremental-remove: the median and p95 of the cost per change, against a
  brick-small1 full run.
- The incremental equality suite (`docs/specs/v1.md`, "Incremental").
- Chaos.

**Risks.** This is the largest change on the list. Rederivation through one
naive round can cost as much as a full run on shallow programs, so measure the
round count first (P0). Diagnostics must be retracted correctly.

### E5. Parallel rounds, native only

**Hypothesis.** Within a round, families only read the store (`store.rs:225`),
and the step for each table is independent. Running both on rayon, and
splitting large recent slices, speeds up large inputs.

**Source.**

- Inferray §4.3 does the same: one thread per rule, merges per table.
- RDFox scaled 5.2–7.4× on 8 threads (AAAI 2014, Figure 2(a)).
- FlowLog: outer-loop-only parallelism scales poorly at 64 threads
  (arXiv 2511.00865).

**Change.**

- A `parallel` cargo feature, on for native builds and absent with default
  features off (wasm32).
- One `Pending` buffer per family, concatenated before the global sort.
- A size threshold below which everything runs sequentially.

**Expected effect (estimate).** lubm-50 is 2–4× faster on 8 threads and
lubm-10 less so. brick-small1 is unchanged, thanks to the threshold. Peak
memory grows by one buffer per thread.

**Measure.** lubm-10, lubm-50 and brick-buildings at 1, 4 and 8 threads, plus
brick-small1 for overhead, plus a wasm32 build check.

**Risks.**

- The serial global sort at `store.rs:124` bounds the speed-up (Amdahl), so
  E6 or a sort per table may be needed.
- rayon adds a dependency, which counts against the Embeddability score.
- The patent question in §6.4 applies.

### E6. A faster sort, gated on the profile

**Hypothesis.** If P0 shows sorting above about 20% of a run, a counting or
LSD radix sort on packed keys (u64 for (s, o), 96 bits for (p, s, o)) beats
`sort_unstable`, because ids are dense (`term.rs:4`).

**Source.** Inferray §5 and Table 1.

**Change.** A radix path in `Relation::from_vec` (`relation.rs:28`), in
`step` (`store.rs:124`) and in the many-batch case of `changed()`
(`variable.rs:76`), for inputs above a size threshold. Safe Rust.

**Expected effect.** Unknown; it depends on the profile.

**Measure.** lubm-50 and brick-buildings: time, and the sort share from P0.

**Risks.** The second buffer doubles peak memory during the sort. A radix
sort can lose to a comparison sort on small or sparse inputs.

### E7. CSR for the (o, s) index of `rdf:type`

**Hypothesis.** Sorted by class, `rdf:type`'s (o, s) pairs repeat each class
key many times. Storing distinct keys, offsets and values cuts the index from
8 B per pair to about 4 B.

**Source.** VLog and Nemo run-length encoding (AAAI 2016; KR 2024 §2).

**Change.** A CSR variant for stable batches of the (o, s) index. Joins and
leapers read key ranges from the offsets.

**Expected effect (estimate).** The relations' share of peak memory falls by
25–50% on lubm-50. There is no gain unless P0 shows that relations dominate
peak memory.

**Measure.** lubm-50 and brick-buildings: peak memory and time.

**Risks.** Touches every join path. Merges get slower.

---

## 11. What not to do

- **Do not build full leapfrog triejoin or trie storage.**
  - The W3C RL bodies are short and acyclic, apart from two triangle-shaped
    diagnostics, which leapjoin already handles (§2.1).
  - The W3C bodies are joins of two or three atoms on one key, where a
    merge join is already the efficient plan (§2.1). Free Join shows that a
    well-built worst-case-optimal join need not lose on acyclic queries
    (§2.2), not that it would win on these.
  - The leapfrog patents run to about 2033 (survey line 509).
- **Do not copy RDFox's lock-free hash triple table.**
  - Its accesses are random (Inferray §2.2, §6.4).
  - Its concurrency costs 10–30% even on one thread (AAAI 2014 §5).
  - Single-threaded WebAssembly gains nothing from it.
  - It is the subject of US11409698B2, to about 2035 (survey line 503).
- **Do not start deletion with counting.** It needs per-fact counts (AIJ
  2019 §6), and per-round counts for recursive rules (§5). It conflicts with the
  set semantics of `changed()`. It was the slowest on small deletions in most
  cases (UOBM-01K R: 60.11 s against 0.34 s for FBF, Table 3); ClarosLE was
  the exception.
- **Do not replace the engine with differential dataflow or DBSP now.**
  - It would undo ADR 0001.
  - Each tuple gains a time and a weight.
  - Its strength shows at many threads (FlowLog). There is no evidence at
    editor scale or on wasm32 (§5.3).
- **Do not move to 64-bit term ids,** as Inferray uses (§4.2). They double
  the size of every pair, and wasm32 is capped at 4 GiB (§8).
- **Do not treat "transitive closure first" as a complete preprocessing
  step.** Transitivity and subclass facts can be derived during the run under
  the W3C rules (§1.3). Keep the rules as the fallback.
- **Do not rewrite only data for `sameAs`.** The rules' constants must follow
  the representatives (AAAI 2015 §3; §4.3 here).
- **Do not parallelise before profiling the serial step.** The global sort
  and the table merges bound the speed-up (§6.4).
- **Do not use RETE.** It needs a graph with random memory access
  (Inferray §2.2). OWLIM, which Inferray's authors call "the RETE approach",
  was "slower than Inferray and RDFox, by at least a factor 7" on RDFS-Plus
  (§6.3–§6.4).
- **Do not judge a speed experiment by triple counts.** RDFox found that
  rewriting saves derivations more than triples (AAAI 2015 §6). Count
  derivations (P0).

---

## 12. Open questions

Answered in the verification pass: Brick's depth and `owl:equivalentClass`
use (§0.4); transitive properties and chains in Brick (none, §0.4); the
18.7 s figure (§0.3); B/F's Table 2 (§5.1); thread counts (Inferray, Hu et
al. 2019, the IJCAI 2015 paper and the hypertree paper state none; §1.1,
§1.3, §4.5, §2.2); parallel speed-ups at Brick's size (none published,
§6.4); rayon on wasm32 (it falls back to one thread, §8); the venues
(§2.2, §5.1, §6.3, §7); new OWL 2 RL algorithms 2023–2026 (none found, §7).

Still open:

1. Do rule-instance shares (§0.4) track time in this engine, or do fixed
   costs per round dominate a 10 ms Brick run? P0 answers it.
2. How large is the dictionary per term (`term.rs:133-135`)? This decides
   whether memory work should start there rather than in relations.
3. Which change between reasonable 0.2.6 (2023) and early 2026 removed most
   of the 18.7 s? Only of historical interest; timing 0.2.6 needs a build of
   it, which this session was not permitted to run.
4. Does a crate-wide `-Ctarget-feature=+simd128` make calls to safe
   `#[target_feature]` intrinsics safe without `unsafe`? It matters only if
   SIMD is ever tried under `unsafe_code = "deny"`.
5. "Chain-Aware Vectorized Datalog Reasoning over Large Knowledge Graphs"
   (KSEM 2026) could not be opened.

## Sources read

All PDFs were read in full text on 2026-11-05. The verification pass
re-read every paper below from its PDF (text extracted with pypdf 6.21.4),
including the B/F paper in full, and added the sources at the end.

- Subercaze, Gravier, Chevalier, Laforest, "Inferray: fast in-memory RDF
  inference", PVLDB 9(6):468–479, 2016,
  <https://www.vldb.org/pvldb/vol9/p468-subercaze.pdf>.
- Motik, Nenov, Piro, Horrocks, Olteanu, "Parallel Materialisation of Datalog
  Programs in Centralised, Main-Memory RDF Systems", AAAI 2014,
  <https://www.cs.ox.ac.uk/boris.motik/pubs/mnpho14parallel-materialisation-RDFox.pdf>.
- Motik, Nenov, Piro, Horrocks, "Handling owl:sameAs via Rewriting",
  AAAI 2015,
  <https://www.cs.ox.ac.uk/boris.motik/pubs/mnph15owl-sameAs-rewriting.pdf>.
- Motik, Nenov, Piro, Horrocks, "Incremental Update of Datalog
  Materialisation: the Backward/Forward Algorithm", AAAI 2015 (in full in
  the verification pass),
  <https://www.cs.ox.ac.uk/boris.motik/pubs/mnph15incremental-BF.pdf>.
- Motik, Nenov, Piro, Horrocks, "Combining Rewriting and Incremental
  Materialisation Maintenance for Datalog Programs with Equality",
  IJCAI 2015, <https://arxiv.org/abs/1505.00212>.
- Motik, Nenov, Piro, Horrocks, "Maintenance of datalog materialisations
  revisited", Artif. Intell. 269 (2019),
  <https://www.cs.ox.ac.uk/boris.motik/pubs/mnph19maintenance-revisited.pdf>.
- Hu, Motik, Horrocks, "Modular Materialisation of Datalog Programs",
  AAAI 2019,
  <https://www.cs.ox.ac.uk/boris.motik/pubs/hmh19modular-materialisation.pdf>.
- Nenov, Piro, Motik, Horrocks, Wu, Banerjee, "RDFox: A Highly-Scalable RDF
  Store", ISWC 2015 (abstract, §5 and Table 1),
  <https://www.cs.ox.ac.uk/boris.motik/pubs/npmhwb15RDFox-scalable.pdf>.
- Urbani, Jacobs, Krötzsch, "Column-Oriented Datalog Materialization for
  Large Knowledge Graphs", AAAI 2016, <https://arxiv.org/abs/1511.08915>.
- Ivliev, Gerlach, Meusel, Steinberg, Krötzsch, "Nemo: Your Friendly and
  Versatile Rule Reasoning Toolkit", KR 2024,
  <https://iccl.inf.tu-dresden.de/w/images/f/fb/KR-2024-CR.pdf>.
- Wang, Willsey, Suciu, "Free Join: Unifying Worst-Case Optimal and
  Traditional Joins", <https://arxiv.org/abs/2301.10841>.
- Budiu, McSherry, Ryzhyk, Tannen, "DBSP: Automatic Incremental View
  Maintenance for Rich Query Languages", <https://arxiv.org/abs/2203.16684>.
- Zhao, Yu, Rao, Frisk, Fan, Koutris, "FlowLog: Efficient and Extensible
  Datalog via Incrementality", <https://arxiv.org/abs/2511.00865>.
- Jordan, Subotić, Zhao, Scholz, "A Specialized B-tree for Concurrent Datalog
  Evaluation", PPoPP 2019,
  <https://davidwzhao.github.io/pdf/PPOPP2019.pdf>.
- Jordan, Subotić, Zhao, Scholz, "Brie: A Specialized Trie for Concurrent
  Datalog", PMAM 2019, <https://davidwzhao.github.io/pdf/PMAM2019.pdf>.
- Zhang, Hu, Nenov, Horrocks, "Enhancing Datalog Reasoning with Hypertree
  Decompositions", IJCAI 2023, <https://arxiv.org/abs/2305.06854>.
- Added in the verification pass:
  - Hu, Motik, Horrocks, "Optimised Maintenance of Datalog
    Materialisations", AAAI 2018, <https://arxiv.org/abs/1711.03987>.
  - Hu, Motik, Horrocks, "Modular materialisation of Datalog programs",
    Artif. Intell. 308 (2022), doi:10.1016/j.artint.2022.103726 (abstract,
    setup).
  - Benedikt, Motik, Tsamoura, "Goal-Driven Query Answering for Existential
    Rules with Equality", AAAI 2018 (representatives).
  - Chevalier et al., "Slider", SIGMOD 2015 demo,
    <https://hal.science/hal-01163676>.
  - Sahebolamri, Gilray, Micinski, "Seamless Deductive Inference via
    Macros", CC 2022,
    <https://s-arash.github.io/ascent/cc22main-p95-seamless-deductive-inference-via-macros.pdf>;
    Sahebolamri, Barrett, Moore, Micinski, "Bring Your Own Data Structures
    to Datalog", OOPSLA 2023, <https://kmicinski.com/assets/byods.pdf>.
  - McSherry, Lattuada, Schwarzkopf, Roscoe, "Shared Arrangements", PVLDB
    13(10), 2020, <https://www.vldb.org/pvldb/vol13/p1793-mcsherry.pdf>.
  - Zhang, Wang, Flatt, Cao, Zucker, Rosenthal, Tatlock, Willsey, "Better
    Together: Unifying Datalog and Equality Saturation", PLDI 2023,
    <https://arxiv.org/abs/2304.04332>.
  - Bidlingmaier, "An Evaluation Algorithm for Datalog with Equality",
    <https://arxiv.org/abs/2302.05792>.
  - Nappa, Zhao, Subotić, Scholz, "Fast Parallel Equivalence Relations in a
    Datalog Compiler", PACT 2019, doi:10.1109/PACT.2019.00015.
  - Zhang, Hu, Nenov, Horrocks, "Optimised Storage for Datalog Reasoning",
    AAAI 2024, <https://arxiv.org/abs/2312.11297>.
  - Rassau, Schuhknecht, "One Size Does NOT Fit All",
    <https://arxiv.org/abs/2602.05651>.
  - Sun, Kumar, Gilray, Micinski, "Column-Oriented Datalog on the GPU",
    AAAI 2025, <https://arxiv.org/abs/2501.13051>.
  - Gustafsson, Magnusson, "Oatlog", MSc thesis, Chalmers 2025.
  - Abstracts only: Carac (ICDE 2024), Flan (POPL 2024), Formulog
    (OOPSLA 2024), GPUlog (ASPLOS 2025), SPARQLing Datalog (ESWC 2026),
    Recovering Explanations (ISWC 2026).
  - reasonable: README at `fecee2b`, commits `3173fa1`, `c8eb495`,
    `19ab409`, PR #37, and the source at `3173fa1`, via `gh api`.
- Repositories, via `gh api` on 2026-11-05:
  - jsubercaze/inferray
  - knowsys/nemo
  - karmaresearch/vlog
  - souffle-lang/souffle
  - rust-lang/datafrog (`src/treefrog.rs`)
  - ekzhang/crepe (README, `src/lib.rs` at `b26a580`)
  - s-arash/ascent (README at `e52c84b`)
  - juleschevalier/slider, flowlog-rs/flowlog, rayon-rs/rayon (`599425d`)
  - feldera/feldera (LICENSE, README)
  - TimelyDataflow/differential-dataflow
  - vmware-archive/differential-datalog
- Rust documentation:
  - <https://doc.rust-lang.org/core/arch/wasm32/index.html>
  - <https://blog.rust-lang.org/2025/04/03/Rust-1.86.0/>
