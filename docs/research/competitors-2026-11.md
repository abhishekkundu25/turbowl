# Competitors and datasets for an OWL 2 RL scoreboard

Researched 2026-11-05. The question: which reasoning engines and datasets
can define the scoreboard on which TurbOWL claims to be the best OWL 2 RL
reasoner, and can each one actually be run, and its numbers published, on the
owner's machine (Apple M6, aarch64, macOS 27.0, 32 GiB, 12 cores)?

This note does not repeat the architectures and published speeds in
`docs/research/sota-algorithms-2026-11.md` §7, or the licence table in
turtle-lsp's `docs/research/reasoner-sota-2026-11.md` §2.8 ("the survey").
It covers running terms, licences, rule coverage and data.

Method. Official docs, licence texts, release notes, repositories (`gh api`,
shallow clones), package registries (Homebrew, PyPI, Maven Central, Docker
Hub APIs) and the papers (authors' PDFs, text extracted with `pypdf`). Links
were checked with HTTP HEAD or range requests on 2026-11-05. Nothing was
installed, built or run. The VLog jars and the GraphDB 11.5.2 zip were read
by range request only: the zip directory, the licence file, the `.pie` rule
files and the Mach-O headers of the bundled libraries. Markers:

- **(unconfirmed)**: not established from a primary source.
- **(reading)**: my reading of code or rule files, not a documented statement.

None of this is legal advice.

## Summary

**Blockers first.**

1. **RDFox cannot be run or published.** Every RDFox server needs "a
   time-limited license key issued by Oxford Semantic Technologies"
   ([docs §2.4.1.4](https://docs.oxfordsemantic.tech/features-and-requirements.html)).
   No tier works without an agreement. The evaluation licence forbids
   disclosing "any Benchmarking Information" without applying to OST at
   least 30 days ahead (§1.4, §2.2(v), §2.3). The owner has no licence, so
   RDFox enters the scoreboard only through its published numbers (§1.2),
   bridged by engines that ran next to it in the same paper.
2. **Stardog is not comparable and its terms are hostile.** It "does not
   materialize inferences"; it rewrites queries. Both its Free and its trial
   or academic agreements forbid evaluating the software "for the purpose of
   competing with Stardog". A self-managed server needs a trial key (business
   email) or an academic licence. Leave it out.
3. **GraphDB Free runs natively on Apple silicon, but which licence governs
   publication is unclear.** The licence shipped in 11.5.2 has no publication
   clause. An older GraphDB Free licence still online (Art. 15.3) forbids
   publishing "evaluation results … without the written permission of the
   Licensor". Free is also capped at one core. Get written permission from
   Graphwise before publishing its numbers.
4. **Soufflé on macOS arm64 is single-threaded in practice.** The Homebrew
   2.5 bottle has no OpenMP dependency, and upstream's own Apple-silicon CI
   turns OpenMP off because of an open crash issue (#2476).
5. **VLog has no prebuilt command-line binary for macOS arm64.** A source
   build trips over CMake 4 (`cmake_minimum_required(2.8)`) and clones its
   dependencies at unpinned HEAD. Rulewerk's published jars do ship
   universal (x86_64 + arm64) VLog libraries, so the Java route is open.
   Neither route was run.
6. **Several published datasets are gone.** The AIJ 2019 data and rule
   links are all 404: ChEMBL, Reactome, the UniProt L/R programs, the UOBM
   L/R programs and the 2017 DBpedia subset. The AAAI 2014, AAAI 2015 and
   ISWC 2015 data and rule files still download with no click-through. Their
   pages license only the RDFox binary and state no licence for the data.

**What can be run here.** Nemo (prebuilt `aarch64-apple-darwin` release),
Soufflé (Homebrew bottle, one thread), VLog through Rulewerk's jars,
Apache Jena 6.2.0 (Java 21), owlrl 7.6.2 (Python), reasonable, and GraphDB
Free (one core, publication to be cleared).

**Data.** Oxford's 2014 LUBM, UOBM, Claros and DBpedia zips, with their L,
LE and U programs, are the common ground with the RDFox, VLog and CIKM 2019
papers. The official LUBM generator (UBA 1.7) still downloads, and its output
is deterministic. **LUBM-1K's L closure (182.4 M triples) is marginal in
32 GiB at 150 B per triple (80 %)**; LUBM-500 (91.1 M) fits comfortably.
Nemo's KR 2024 "LUBM-01k" is ChaseBench's existential-rule LUBM over CSV
relations, not an OWL 2 RL program. No dedicated OWL 2 RL materialisation
benchmark has been published since 2020; OWL2Bench (ISWC 2020) is the
nearest.

**Rule coverage differs.** Among the systems checked, only owlrl, GraphDB's
`owl2-rl` and one third-party Nemo program document coverage of most of the
W3C tables. Each leaves out some of the datatype rules. Jena's OWL rules
are OWL-Lite era, even behind the `OWL2_RL_MEM_RULES_INF` name. A
scoreboard must compare closures under a stated rule set, not just times
(§10).

---

## 1. RDFox

The owner cannot get a licence, so this section answers only two
questions.

### 1.1 Is any tier usable without a licence agreement?

No.

- "Creating an RDFox Server requires a time-limited license key issued by
  Oxford Semantic Technologies"
  ([features and requirements, §2.4.1.4 "License Key"](https://docs.oxfordsemantic.tech/features-and-requirements.html)).
- "If you do not have a license key file, you can request one at
  oxfordsemantic.tech/free-trial"
  ([getting started](https://docs.oxfordsemantic.tech/getting-started.html)).
  The [free-trial page](https://www.oxfordsemantic.tech/free-trial) is a
  form.
- The [downloads page](https://www.oxfordsemantic.tech/downloads): "RDFox
  and RDFox Mantix are used subject to the applicable RDFox licence
  agreement."
- "Sandbox" is a shell mode without persistence, not a way around the
  key ([5.4 getting started](https://docs.oxfordsemantic.tech/5.4/getting-started.html)).
- The 2014–2015 Oxford test pages still link the old research binaries
  under an "academic licence"
  (`http://www.cs.ox.ac.uk/isg/tools/RDFox/RDFox_Academic_Licence.txt`, linked from
  [2014/AAAI](https://www.cs.ox.ac.uk/isg/tools/RDFox/2014/AAAI/)). They
  are still licensed software. The ISWC 2015 source link
  (`cppRDFox_rev2067-src.zip`) returns 404.

The evaluation licence
([oxfordsemantic.tech/rdfox-evaluation-license](https://www.oxfordsemantic.tech/rdfox-evaluation-license),
read 2026-11-05):

- §1.4: "'Benchmarking Information' means any data, analysis or opinion …
  that relates or purports to relate to any test, evaluation, analysis or
  assessment of the performance of the Licensed Product whether the Licensed
  Product is named or kept anonymous".
- §2.2(v): you shall not, "other than in accordance with Section 2.3,
  disclose any Benchmarking Information".
- §2.3: "If you wish to publish any Benchmarking Information whether for
  academic, commercial or any other reason, then you may apply to OST to do
  so. You shall contact OST at least thirty (30) days in advance".
- §2.4: you "will inform OST of achievements of using the Licensed
  Product".

Nemo's authors asked for an academic licence for KR 2024 and did not get one
([KR 2024 paper, §5.2](https://iccl.inf.tu-dresden.de/w/images/f/fb/KR-2024-CR.pdf)).

Documented OWL 2 RL coverage, for the record: there is no per-rule table.
"The translation of OWL 2 axioms into rules … is performed on a best-effort
basis", and constructs that cannot be translated produce a warning
([reasoning docs §10.6.3, §10.6.2](https://docs.oxfordsemantic.tech/reasoning.html)).
Equality is a store parameter (`equality noUNA`, §10.1.2.4). An
`owl:sameAs` that contradicts `owl:differentFrom` shows up as an
`owl:Nothing` instance, which behaves like eq-diff1.

### 1.2 Published RDFox numbers, with bridge engines

Each table gives the hardware, the RDFox build and every other engine run
on the same machine in the same paper. A bridge works like this: run the
bridge engine and TurbOWL on the M6 on the same data and program, then scale
through the paper's ratio. Times are wall-clock seconds. "n.s." means not
stated.

**AAAI 2014.** Motik, Nenov, Piro, Horrocks, Olteanu, "Parallel
materialisation of Datalog programs in centralised, main-memory RDF systems"
([PDF](https://www.cs.ox.ac.uk/boris.motik/pubs/mnpho14parallel-materialisation-RDFox.pdf)).

- Table (a) machine: Dell, 2× Xeon E5-2650 (16 cores, 32 threads), 128 GB,
  RHEL 6.3 (§5 "Test Setting", p. 7).
- RDFox build: the 2014 research build, in its sequential and parallel
  variants; no version number. Each figure is the mean of 3 runs.
- Sizes are from Table 1 (p. 7); times and memory from Fig. 2, table (a)
  (p. 8).

| Data, program | Triples before → after | seq | 1 thr | 16 thr | 32 thr | Memory |
|---|---|---|---|---|---|---|
| Claros L | 18.8 M → 95.5 M | 1907 | 2477 | 179 | 127 | 4.2 GB |
| Claros LE | 18.8 M → 555.1 M | 4128 | 4989 | 415 | 285 | 18.0 GB |
| DBpedia L | 112.7 M → 118.3 M | 158 | 161 | 26 | 24 | 6.1 GB |
| DBpedia LE | 112.7 M → 1,529.7 M | 8457 | 9075 | 828 | 602 | 51.9 GB |
| LUBM-01K L | 133.6 M → 182.4 M | 68 | 73 | 8 | 7 | 9.3 GB |
| LUBM-01K LE | 133.6 M → 332.6 M | 913 | 947 | 88 | 71 | 13.8 GB |
| LUBM-01K U | 133.6 M → 219.0 M | 122 | 135 | 13 | 10 | 11.1 GB |
| LUBM-05K L | 691.1 M → 911.2 M | 422 | 442 | 42 | mem | 49.0 GB |
| UOBM-010 U | 2.2 M → 35.6 M | 2381 | 2738 | 256 | 168 | 1.1 GB |
| UOBM-01K L | 254.8 M → 429.6 M | 476 | 532 | 42 | 31 | 20.2 GB |

Same paper, Fig. 2 table (b): a second machine (2× Xeon E5-2643, 8 cores,
CentOS 6.4), one thread, import / materialisation in seconds. Cells were
reconstructed from garbled PDF text; only those read with confidence appear.

| Data | RDFox | OWLIM-Lite 5.3 | MonetDB (VP) | PostgreSQL 9.2 (VP) |
|---|---|---|---|---|
| LUBM-01K L | 332 / 71 | 2409 / 316 | 6136 / 27 | 7736 / 948 |
| UOBM-01K L | 632 / 467 | 4778 / timeout | 11063 / 358 | 13864 / 6708 |
| DBpedia L | 274 / 143 | 1735 / 14855 | 15499 / 354 | 5844 / mem |
| Claros L | 48 / 2062 | 300 / 14293 | 883 / mem | 1138 / 25026 |

**AAAI 2015.** "Handling owl:sameAs via rewriting"
([PDF, extended](https://www.cs.ox.ac.uk/boris.motik/pubs/mnph15owl-sameAs-rewriting.pdf)).

- Machine: 2× Xeon E5-2643 (8 cores, 16 threads), 128 GB, Fedora 20.
- RDFox build n.s.; no other engines.
- AX means equality axiomatised; REW means rewriting. TurbOWL's
  representatives correspond to REW.
- Source: Tables 2–3, p. 6.

| Data (rules / sameAs-head rules) | Triples before → after AX / REW | AX 1 thr / 16 thr | REW 1 thr / 16 thr | Memory AX / REW |
|---|---|---|---|---|
| Claros (1,312 / 42) | 19 M → 102 M / 79.7 M | 2042.9 / 153.4 | 65.8 / 6.9 | 4.5 / 3.6 GB |
| DBpedia (3,384 / 23) | 113 M → 139 M / 136 M | 219.8 / 27.5 | 31.7 / 3.6 | 6.9 / 7.0 GB |
| OpenCyc (261,067 / 3,781) | 2.4 M → 1,176 M / 142 M | 2093.7 / 254.0 | 119.9 / 52.3 | 35.9 / 4.6 GB |
| UniProt (451 / 60) | 123 M → 228 M / 228 M | 370.6 / 61.9 | 143.4 / 17.1 | 15.1 / 15.1 GB |
| UOBM-010 U (279 / 4) | 2.2 M → 36 M / 9.7 M | 2696.7 / 314.4 | 1152.7 / 137.9 | 1.2 / 0.4 GB |

**ISWC 2015.** "RDFox: a highly-scalable RDF store"
([PDF](https://www.cs.ox.ac.uk/boris.motik/pubs/npmhwb15RDFox-scalable.pdf),
Table 1, p. 13).

- Machine: Oracle SPARC T5-8, 128 cores (1,024 threads), 4 TB.
- RDFox revision: 2067 (the source zip's name).

| Data | Triples before → after | 1 thr | 128 thr | 1,024 thr | B/triple |
|---|---|---|---|---|---|
| LUBM-50K L | 6.7 G → 9.2 G | 27.0 k | 387.4 | — | 101 |
| Claros LE | 18.8 M → 539.2 M | 10.0 k | 225.7 | 124.9 | 36.9 |
| DBpedia LE | 112.7 M → 1.5 G | 31.2 k | 697.9 | 358.8 | 39.0 |

**AIJ 2019.** "Maintenance of datalog materialisations revisited", AI 269
([PDF](https://www.cs.ox.ac.uk/boris.motik/pubs/mnph19maintenance-revisited.pdf)).

- Machine: Dell R730, 2× Xeon E5-2640 (8 cores each), 512 GB, Fedora 24.
  One thread. RDFox rev. 2725 (journal test page).
- Sizes (Table 1, p. 36), facts before → after:
  - ChEMBL: L 289.2 M → 323.4 M; R → 1.0 G.
  - Claros: L 18.8 M → 73.8 M; LE → 533.3 M.
  - DBpedia: L 386.7 M → 643.8 M.
  - Reactome: L 12.5 M → 19.8 M; R → 369.4 M; U → 45.6 M.
  - UniProt: L 123.1 M → 174.7 M; R → 556.7 M.
  - UOBM-01K: L 254.8 M → 426.2 M; R → 1.3 G.
- The paper reports maintenance times only, with no full-materialisation
  times. Rematerialisation after a large deletion is in Table 8, p. 44.

**VLog, AAAI 2016.** Urbani, Jacobs, Krötzsch
([arXiv 1511.08915](https://arxiv.org/pdf/1511.08915), Table 2, p. 6).

- Machine: MacBook Pro, 2.2 GHz Core i7, 16 GB, OS X 10.10.5.
- Both engines single-threaded. RDFox is the 2014 sequential build,
  recompiled. VLog version n.s.
- Format: time in seconds / peak memory in MB.

| Data | RDFox | VLog |
|---|---|---|
| LUBM-1K L | 82 / 11,884 | 38 / 2,198 |
| LUBM-1K U | 148 / 14,593 | 80 / 2,418 |
| LUBM-1K LE | out of memory | 2175 / 9,818 |
| LUBM-5K L | out of memory | 196 / 8,280 |
| DBpedia L | 177 / 7,917 | 91 / 532 |
| Claros L | 2418 / 5,696 | 644 / 2,406 |
| Claros-S LE | 8.5 / 271 | 2.5 / 127 |

VLog alone on LUBM-1K with the 66 W3C OWL RL rules without the datatype and
equality rules ("O"): 236.5 s with memoisation and 1514 s without (Table 4).

**CIKM 2019.** Hu, Urbani, Motik, Horrocks
([PDF](https://www.cs.ox.ac.uk/boris.motik/pubs/humh19compressed-materialisation.pdf),
Tables 3–4, p. 8).

- Machine: Dell R720, 2× Xeon E5-2670, 256 GB.
- RDFox version and threads n.s.
- Format: load + materialisation = total, in seconds.

| Data | RDFox | VLog (RDF) | VLog (CSV) | CompMat |
|---|---|---|---|---|
| LUBM-1K L (→ 182.4 M) | 355 + 133.3 = 488.3 | 1233.7 | 300.1 | 266.8 |
| Reactome L | 53.0 | 44.0 | 27.5 | 47.3 |
| Claros L | 135.9 | 198.4 | 47.0 | 59.1 |
| Claros LE | 3492.1 | 2869.9 | 2684.0 | 10.2 k |

**GLog, PVLDB 2021** ([arXiv 2102.02753](https://arxiv.org/pdf/2102.02753),
§7.1, Tables 3 and 13).

- Machine: Intel i7 PC, 94.1 GiB, Ubuntu 16.04.
- RDFox build: the one bundled with ChaseBench. All engines single-threaded
  unless noted.
- Format: time in seconds / memory in MB.
- The LUBM size is not stated **(unconfirmed)**.

| Data | RDFox | VLog | GLog |
|---|---|---|---|
| LUBM L | 23 / 2,301 | 1.5 / 324 | 1.0–2.4 |
| LUBM LE | 116.6 / 3,140 | 170.5 / 2,725 | 16.1 |
| UOBM L | 10 / 784 | 7.3 | 2.6 |
| DBpedia L | 64.4 / 3,290 | 41.6 | 19 |
| Claros L | 2512 / 5,491 | 431 | 119 |

**Inferray, PVLDB 2016** ([PDF](http://www.vldb.org/pvldb/vol9/p468-subercaze.pdf),
Table 3).

- Machine: Xeon E3-1246v3, 32 GB. RDFox version n.s.
- Workload: RDFS-Plus on LUBM, times in milliseconds.

| LUBM size | Inferray | OWLIM-SE | RDFox |
|---|---|---|---|
| 1 M | 19 | 1,324 | 69 |
| 10 M | 540 | 6,175 | 855 |
| 50 M | 1,984 | 31,187 | 4,077 |
| 100 M | 2,514 | 72,098 | 10,613 |

**VFLog, AAAI 2025** ([arXiv 2501.13051](https://arxiv.org/pdf/2501.13051),
Table 3, p. 7).

- Machine: AMD EPYC 9534 (64 cores), 500 GB.
- Versions: **RDFox 7.1a**, all threads; Nemo 0.5.1; VLog through Rulewerk.
- Workload: ChaseBench LUBM with its existential rules removed. Whether
  the times include loading is n.s.

| Data | Nemo | VLog | RDFox 7.1a | VFLog CPU |
|---|---|---|---|---|
| LUBM-010 | 0.65 | 1.46 | 0.44 | 0.15 |
| LUBM-100 | 6.34 | 5.02 | 4.98 | 0.71 |
| LUBM-01K | 62.32 | 165.8 | 56.8 | 6.47 |

This is the only published number for a current RDFox release, and the
only one next to Nemo. Its data is ChaseBench's LUBM, not Oxford's.

**Prototypes on the RDFox codebase** (not the product). IJCAI 2023
([arXiv 2305.06854](https://arxiv.org/abs/2305.06854), Table 1): LUBM-500 L,
66,751,196 → 91,128,727 facts, standard semi-naive evaluation 95.73 s, 7.2 GB
(2× E5-2640, 512 GB).

**Bridge chains.**

- Oxford LUBM-1K L: RDFox (AAAI 2014, VLog 2016, CIKM 2019) to VLog (VLog
  2016, CIKM 2019). VLog can run on the M6 through Rulewerk (§2).
- ChaseBench LUBM-01K: RDFox 7.1a and VLog (VFLog 2025) to Nemo and VLog
  (VFLog 2025, KR 2024). Nemo has a prebuilt aarch64 macOS release (§7).
- Caveat: the paper machines are 2014–2025 x86 servers and laptops. A
  ratio carried across machines through a bridge engine is an estimate,
  not a measurement.

---

## 2. VLog and Rulewerk

**Activity** (`gh api`, 2026-11-05).

| | [karmaresearch/vlog](https://github.com/karmaresearch/vlog) | [knowsys/rulewerk](https://github.com/knowsys/rulewerk) |
|---|---|---|
| Archived | no | no |
| Last commit | 2023-07-16 (`10e23c7`) | 2024-06-12 (master); pushed 2025-07-12 |
| Latest release | v1.3.7, 2022-12-19 | v0.9.0, 2023-01-07 |
| Open issues / PRs | 13 / 1 | 51 / 8 |
| Licence | Apache-2.0 | Apache-2.0 |

- Rulewerk's README (line 9) points users to Nemo: "The recent Nemo rule
  engine also implements support for a Rulewerk-like Datalog-dialect, and
  might be a good choice". VLog's README has no maintenance statement.
- Maven Central: `org.semanticweb.rulewerk:vlog-java` 1.3.7 (2022-12-20);
  `rulewerk-vlog` 0.9.0 depends on it
  ([maven-metadata.xml](https://repo1.maven.org/maven2/org/semanticweb/rulewerk/vlog-java/maven-metadata.xml)).

**macOS arm64.**

- VLog's only CI is `.travis.yml` (Ubuntu xenial, gcc-5). Rulewerk's
  workflows run on `ubuntu-latest`. Neither tests macOS or arm64.
- [vlog#96](https://github.com/karmaresearch/vlog/issues/96) (closed
  2022-12-08): the 1.3.5 jar failed on an M1 ("have 'x86_64', need
  'arm64e'"). The fix passes `-DDIST=1` down to trident and kognac, which
  sets `CMAKE_OSX_ARCHITECTURES arm64 x86_64` (`CMakeLists.txt:15-16`).
  The same bug is [rulewerk#220](https://github.com/knowsys/rulewerk/issues/220).
- `vlog-java-1.3.7.jar` (Maven Central, 9.5 MB) and the
  `rulewerk-client-0.9.0.jar` release asset contain `libvlog-core`,
  `libkognac-core`, `libvlog-java`, `libtrident-core` and
  `libtrident-sparql` dylibs.
  - Each is a fat Mach-O with x86_64 and arm64 slices; the arm64 slice
    has `minos 12.6`.
  - They link only system libraries (`libcurl`, `libz`, `libc++`,
    `libSystem`).
  - This was checked from the headers only. Whether they load on macOS 27
    is **(unconfirmed)**.
- Building the command line from source:
  - `CMakeLists.txt:1` (and trident's and kognac's) declares
    `cmake_minimum_required (VERSION 2.8)`. CMake 4.0 removed
    "Compatibility with versions of CMake older than 3.5"
    ([4.0 release notes](https://cmake.org/cmake/help/latest/release/4.0.html));
    the workaround is `CMAKE_POLICY_VERSION_MINIMUM`. Homebrew's cmake is
    4.x.
  - The build hard-codes `gcc`/`g++` unless the environment overrides them
    (`CMakeLists.txt:3-10`).
  - It clones trident, and through it kognac, sparsehash and lz4, at
    unpinned HEAD (`CMakeLists.txt:93-94`).
  - Whether Apple clang builds it today is **(unconfirmed)**.
- The Docker image `karmaresearch/vlog:latest` (2021-07-07) is
  linux/amd64 only and predates the arm64 fix. There are no Python
  bindings; PyPI `pyvlog` is an unrelated package.

**Rule language.** The predicate position is an ordinary argument of a
ternary relation in both formats.

- VLog `.dlog`:
  - A term whose first character is upper case is a variable
    (`src/vlog/common/concepts.cpp:1209`).
  - `rdf:`, `rdfs:` and `owl:` expand to IRIs (`:1064-1083`).
  - Example: `T(X,P,Z) :- T(X,P,Y), T(Y,owl:sameAs,Z)`.
  - Data loads into a ternary EDB predicate, either through a Trident store
    (`./vlog load -i <nt> -o <kb>`, then `EDB0_type=Trident` in `edb.conf`)
    or `EDB0_type=INMEMORY` over `.nt` or `.csv` files
    ([wiki: Usage](https://github.com/karmaresearch/vlog/wiki/Usage),
    [Configuring-Databases](https://github.com/karmaresearch/vlog/wiki/Configuring-Databases)).
  - Run with `./vlog mat --edb edb.conf --rules <file>`.
  - Threads: `--multithreaded`, `--nthreads` and `--interRuleThreads`
    (`src/launcher/main.cpp:293-300`).
  - A rule whose head *predicate* is named `owl:sameAs` becomes an
    equality-generating dependency (`concepts.cpp:1443-1448`). This does not
    apply when sameAs is an argument.
- Rulewerk `.rls`:
  - `@source triple[3]: load-rdf("data.nt.gz") .` loads triples.
  - Variables are `?X` (universal) and `!X` (existential).
  - IRIs, full or prefixed, are constants
    ([wiki: Rule syntax by examples](https://github.com/knowsys/rulewerk/wiki/Rule-syntax-by-examples),
    lines 24, 59-78).

**Existing OWL 2 RL programs.**

- **VLog's AAAI 2016 material**
  ([`papers/aaai2016_support/rulesets/`](https://github.com/karmaresearch/vlog/tree/master/papers/aaai2016_support)):
  - Files: `LUBM_L`, `LUBM_LE`, `LUBM_U`, `Claros_L`, `Claros_LE`,
    `DBpedia_L`.
  - The README says these "are rewritings of the RDFox rules … that include
    additional rules for transforming triples (stored in the EDB layer) into
    IDB facts".
  - They are ontology-specific (for example
    `RP0(A) :- TE(A,rdf:type,<…#University>)`), not the generic W3C rules.
- **No official generic OWL 2 RL program.** Neither
  [knowsys/nemo-examples](https://github.com/knowsys/nemo-examples) nor
  Rulewerk has one. nemo-examples has OWL EL programs only
  (`examples/owl-el/...`).
- **Rulewerk's OWL API converter** throws `OwlFeatureNotSupportedException`
  for:
  - datatypes;
  - DifferentIndividuals and (inverse-)functional properties ("due to lack
    of equality support"), `OwlAxiomToRulesConverter.java:341-342, 432-433,
    548-549`;
  - max cardinality and nominals in superclass position
    (`ClassToRuleHeadConverter.java:139-152`).
- **Third-party programs:**
  - **Krarilotus/OWL-RS**
    [`benches/reasoning/nemo/owl2rl.rls`](https://github.com/Krarilotus/OWL-RS)
    (Apache-2.0, pushed 2026-11-03). A generic Nemo program covering
    "every rule of Tables 4-9 that derives triples, including the list-based
    ones", plus the pairwise consistency rules. It leaves out eq-ref, the
    datatype rules and the axiomatic triples (header, lines 8-11). This is
    the most complete OWL 2 RL program found for a Datalog engine.
  - **sparq-org/sparq**
    [`bench/reason-encodings/{nemo/owl-rl.rls, vlog/owl-rl.dlog}`](https://github.com/sparq-org/sparq/tree/main/bench/reason-encodings)
    (MIT). Only RDFS plus the OWL rules LUBM needs; equality, cardinality,
    keys and datatypes are "DELIBERATELY ABSENT" (README lines 38-49). Its
    VLog file confirms the ternary syntax with variable predicates.

**Blocks running on macOS arm64?** The Java route (Rulewerk jars with
universal dylibs) is open but untested. The command line needs a source
build with the workarounds above.

---

## 3. Soufflé

- **Version.** 2.5, 2025-03-24. Release assets are Linux x86_64 `.deb` and
  `.rpm` only. Master last committed 2026-10-31; 123 open issues; UPL-1.0
  ([releases](https://github.com/souffle-lang/souffle/releases)).
- **Install on macOS arm64.**
  - Homebrew core has `souffle` 2.5 with bottles `arm64_golden_gate`,
    `arm64_tahoe`, `arm64_sequoia`, `arm64_sonoma` and others
    ([formulae.brew.sh/formula/souffle](https://formulae.brew.sh/formula/souffle);
    `Formula/s/souffle.rb:8-17`). That `golden_gate` is macOS 27 is
    inferred from the tag order **(unconfirmed)**.
  - The formula builds with `-DSOUFFLE_DOMAIN_64BIT=ON` (line 36) and points
    compile mode at `/usr/bin/c++` (line 45), so compiled mode needs the
    Xcode Command Line Tools.
  - Its build dependencies are bison, cmake, mcpp and pkgconf, with **no
    libomp**.
  - The project's own [install page](https://souffle-lang.github.io/install)
    gives only `brew install --HEAD souffle-lang/souffle/souffle` (a tap last
    pushed 2023-12-31).
  - The [build page](https://souffle-lang.github.io/build) says macOS "does
    not have OpenMP/C++ … OpenMP is not required … but does improve
    runtime".
  - There is no official Docker image (Docker Hub has no
    `souffle-lang/souffle`).
- **Threads on Apple silicon.**
  - Without OpenMP, any `-j` other than 1 prints "This installation of
    Souffle does not support concurrent jobs." (`src/MainDriver.cpp:810-813`;
    [#1839](https://github.com/souffle-lang/souffle/issues/1839), about
    Homebrew).
  - Upstream's `AppleM-CMake` CI job (macos-15) sets
    `-DSOUFFLE_USE_OPENMP=OFF` with the comment "disable openmp on ARM
    architecture, see souffle-lang/souffle#2476"
    (`.github/workflows/CI-Tests.yml:160`).
  - [#2476](https://github.com/souffle-lang/souffle/issues/2476) (open,
    comments to Feb 2026) reports sporadic multi-threaded crashes on ARM
    (`OMP: Error #13: Assertion failure at kmp_dispatch.cpp(1298)`).
  - Other Apple-silicon issues:
    [#2170](https://github.com/souffle-lang/souffle/issues/2170) (open,
    tests failing on M1) and
    [#2076](https://github.com/souffle-lang/souffle/issues/2076) (open,
    LLVM clang build).
  - Conclusion: the bottle is sequential (inferred from the formula and
    the source, not run).
- **Flags** ([execute](https://souffle-lang.github.io/execute);
  `src/MainDriver.cpp:656-723`):
  - `-j, --jobs=<N>` ("N=auto for system default"; default 1).
  - `-c, --compile`; `-o, --dl-program=<FILE>`; `-g, --generate=<FILE>`.
  - `-p, --profile=<FILE>`.
  - `-a, --auto-schedule=<FILE>` with `--emit-statistics`
    ([autotuning](https://souffle-lang.github.io/autotuning)).
  - `-F` (fact directory) and `-D` (output directory).
  - KR 2024 ran `souffle run.dl --dl-program=compiled; ./compiled`
    (nemo-examples `evaluations/kr2024/README.md:65-71`).
- **Published OWL 2 RL programs.**
  - [balhoff/owlrl-datalog](https://github.com/balhoff/owlrl-datalog) (MIT,
    last push 2024-01-20), "Experimenting with OWL RL reasoning in Souffle".
    It parses the TBox into axiom tables, then applies ABox rules
    (`src/datalog/owl_rl_abox.dl:51-179`).
    - It covers prp-dom/rng/fp/ifp/irp/symp/asyp/trp, prp-spo1, prp-spo2
      (chains of length 2–4 only), prp-pdw, prp-inv1/2, prp-npa1/2,
      cls-int1, cls-com, cls-svf1/2, cls-avf, cax-sco and cax-dw.
    - It has no equality, no prp-key, no cardinality, no cls-hv, cls-oo or
      cls-uni, no cax-eqc, no scm-* and no dt-* rules.
    - It is a partial experiment, not a reference program.
  - The KR 2024 Soufflé program
    (`evaluations/kr2024/programs/lubm-1k/souffle/run.dl`) is ChaseBench's
    existential-rule LUBM, auto-translated from Nemo with Skolem ADTs. It is
    **not OWL 2 RL**. The same holds for the VLog and Nemo programs beside
    it.
- **Limits for a large triple relation.**
  - Values are 32-bit by default and can be built 64-bit
    ([types](https://souffle-lang.github.io/types); `RamTypes.h:39-50`; CMake
    option `SOUFFLE_DOMAIN_64BIT`). Symbols are ordinals, so a 32-bit build
    implies at most 2³¹ symbols **(unconfirmed: no documented limit)**.
  - The Homebrew build is 64-bit, which doubles every value
    ([#2389](https://github.com/souffle-lang/souffle/issues/2389)).
  - No bytes-per-tuple figure is documented. B-tree is the default
    representation ([relations](https://souffle-lang.github.io/relations)).
  - `eqrel` stores an equivalence relation by union-find, "linear" rather
    than quadratic. It must be binary over one domain
    (`SemanticChecker.cpp:751-763`), so `owl:sameAs` would be a separate
    binary relation, not the triple relation.

**Blocks running on macOS arm64?** Single-threaded: no. Multi-threaded: yes
in practice (no OpenMP in the bottle; open crash issue upstream).

---

## 4. GraphDB (Ontotext, trading as Graphwise)

- **Version.** 11.5.2, 2026-10-16
  ([release notes](https://graphdb.ontotext.com/documentation/11.5/release-notes.html)).
- **How Free is obtained.**
  - "Starting from GraphDB 11.0, GraphDB Free is no longer automatically
    distributed with GraphDB. If you want to use GraphDB Free, you must first
    request a free license from the Graphwise website"
    ([licensing](https://graphdb.ontotext.com/documentation/11.5/licensing.html)).
  - "You will receive the Free License on your email address"
    ([set up your license](https://graphdb.ontotext.com/documentation/11.5/set-up-your-license.html)).
  - The downloads on [graphwise.ai/components/graphdb](https://graphwise.ai/components/graphdb/)
    are public: a "MacOS Apple Silicon (.dmg)", an Intel `.dmg`, `.deb`,
    `.rpm`, `.msi` and the platform-independent `graphdb-11.5.2-dist.zip`
    (all 200).
  - The licence form's fields and the licence's duration are
    **(unconfirmed)**.
- **Limits of Free** (licensing page;
  [benchmark page](https://graphdb.ontotext.com/documentation/11.5/benchmark.html)):
  - "GraphDB Free supports a limit of two concurrent queries".
  - Concurrency "single core", at most 5 repositories, no 40-bit entity ids.
  - "Management of an unlimited number of RDF statements".
  - "GraphDB Free uses a single CPU core only" and "produces the slowest
    performance due to a limitation of a single write thread".
  - Java 21 is required except for the desktop installer
    ([requirements](https://graphdb.ontotext.com/documentation/11.5/requirements.html)).
- **Licence text.**
  - The 11.5.2 zip ships `doc/ONTOTEXT_LICENSE_AGREEMENT.txt`. It is
    byte-identical to
    [LICENSE-GraphDB.txt](https://graphdb.ontotext.com/LICENSE-GraphDB.txt)
    (last modified 2026-04-24), titled "Ontotext End-User License
    Agreement".
  - **It has no clause on publishing benchmark, evaluation or performance
    results.** It was searched for publish, benchmark, result, perform,
    compar and test.
  - Art. 2.1(c) limits Free to "1 CPU Core". Art. 3.3: "GraphDB Free only
    for non-production purposes (development, staging, testing)".
  - **Conflicting text.** The older
    [LICENSE-GraphDB-Free.txt](https://graphdb.ontotext.com/LICENSE-GraphDB-Free.txt)
    (last modified 2023-02-27, party "Sirma AI JSC") is still online. Its
    Art. 15.3: "The Licensee is not allowed to publish evaluation results
    obtained through the use of the Licensed Software without the written
    permission of the Licensor."
  - Which text an e-mailed Free licence refers to is **(unconfirmed)**.
  - The hosted Sandbox's
    [EULA §5](https://graphwise.ai/end-user-license-agreement/) forbids
    "Publishing … any performance benchmarks, comparative analyses, or
    evaluation results" without consent. That covers the hosted service,
    not a local install.
- **Rulesets**
  ([reasoning, "Predefined rulesets"](https://graphdb.ontotext.com/documentation/11.5/reasoning.html);
  [configuring a repository](https://graphdb.ontotext.com/documentation/11.5/configuring-a-repository.html)).
  - The `ruleset` values are `empty`, `rdfs`, `rdfsplus`, `owl-horst`,
    `owl-max`, `owl2-ql` and `owl2-rl`, plus `-optimized` variants. The
    default is `rdfsplus-optimized`.
  - "There are optimized versions of all rulesets that avoid some
    little-used inferences". In the optimized rulesets "rdfs:Resource is
    omitted"
    ([rules optimisations](https://graphdb.ontotext.com/documentation/11.5/rules-optimisations.html)).
  - **(reading)** In the shipped `builtin_owl2-rl.pie` and
    `builtin_owl2-rl-optimized.pie`, the 78 rule and consistency ids are
    identical. The optimized file drops 49 axiomatic triples
    (`rdfs:domain`/`rdfs:range` `rdfs:Resource` and `rdf:Property`).
  - Custom `.pie` rulesets can be loaded: "The predicate can be a variable,
    a full URI, or a short name" (reasoning, "Rules"). They are compiled to
    Java and need a JDK. There is no list construct; the shipped rules walk
    lists through helper statements in private contexts.
- **Reasoning switches** (exact names, configuring-a-repository):
  - `ruleset`, `disable-sameAs` and `check-for-inconsistencies` (default
    false).
  - Index options: `enable-context-index`, `enablePredicateList`,
    `enable-literal-index`, `in-memory-literal-properties`,
    `entity-id-size` and `entity-index-size`.
  - `imports` (schema loaded read-only), `read-only` and `query-timeout`.
  - Per query, `FROM onto:disable-sameAs` turns off sameAs expansion
    ([sameAs optimisation](https://graphdb.ontotext.com/documentation/11.5/sameas-optimisation.html)).
  - That page says owl:sameAs is "hard-coded" with one representative per
    class, "The rules are commented out in the .pie files", and the
    parameter "defaults to false". The configuration table lists the
    default as `true`, so the two pages disagree.
  - System properties: `-Dgraphdb.inference.concurrency` ("The default
    value is the number of cores") and `-Dgraphdb.inference.buffer`
    ([importrdf](https://graphdb.ontotext.com/documentation/11.5/loading-data-using-importrdf.html)).
    Whether Free's one-core licence caps inference threads is
    **(unconfirmed)**.
- **Inference model.** Forward chaining with "Total materialization".
  Retraction marks visited facts, then checks support by backward chaining
  (reasoning, "Retraction of assertions").
- **Platforms.** JVM. Native Apple-silicon installer. Docker
  `ontotext/graphdb:11.5.2` is linux/amd64 and linux/arm64.

**Blocks running or publishing?** Running: no (licence by e-mail; one core).
Publishing: unclear (see the conflicting text above). Ask Graphwise in writing.

---

## 5. Stardog

- **Version.** 12.2.1, 2026-10-26
  ([release notes](https://docs.stardog.com/release-notes/stardog-platform)).
  12.2.0 added "Support ARM docker image".
- **Reasoning model.** "Stardog performs reasoning in a lazy and
  late-binding fashion: it does not materialize inferences; rather,
  reasoning is performed at query time", by "a query rewriting approach"
  ([inference engine](https://docs.stardog.com/inference-engine/)).
  - The exception: "The sameAs inferences are computed and indexed eagerly"
    ([advanced reasoning, "Same As Reasoning"](https://docs.stardog.com/inference-engine/advanced-reasoning-features)).
  - No documented way to materialise the closure.
- **Levels.** `reasoning.type` takes RDFS, QL, RL, EL, SL (default) or NONE.
  "Any axiom outside the selected type will be ignored". "Support for DL
  reasoning type has been removed in version 9.0"
  ([database configuration](https://docs.stardog.com/operating-stardog/database-administration/database-configuration)).
- **Licences.**
  - The [pricing page](https://www.stardog.com/pricing/) says "Stardog Free
    is available at no cost … as a renewable 1-year license … available for
    commercial use". Its download link leads to Stardog Cloud.
  - The Cloud free tier stores "up to 1M edges"
    ([Stardog Cloud](https://www.stardog.com/stardog-cloud/)).
  - A self-managed server needs a licence file: "If you don't have a
    license, contact a Stardog sales representative"
    ([Docker install](https://docs.stardog.com/install-stardog/server/docker)).
  - Trial keys need "your valid business email"
    ([license request](https://www.stardog.com/license-request)).
  - An academic licence exists for students and staff at accredited
    institutions
    ([evaluation agreement](https://info.stardog.com/evaluation-agreement)).
- **Terms.**
  - The [Stardog Free licence](https://www.stardog.com/legal/stardog-free/)
    (revised 2022-12-13), §1: "for the sole purpose of evaluating Stardog".
    This conflicts with the pricing page.
  - §3.b: "Customer may not: … (iv) evaluate the Software for the purpose
    of competing with Stardog". The trial and academic agreement has the
    same clause.
  - Neither has an explicit benchmark-publication clause. The
    competing-purpose clause plausibly covers a rival reasoner's
    scoreboard.
- **Platforms.** JVM. The Docker page says "ARM-based processors (including
  Apple Silicon …) are not supported". Docker Hub's `stardog/stardog:12.2.1`
  is nevertheless amd64 and arm64.
- **Switches** (database configuration; exact names and defaults):
  - `reasoning.type` = SL.
  - `reasoning.sameas` = OFF; ON adds the reflexive, symmetric and
    transitive closure; FULL adds functional properties, inverse-functional
    properties and hasKey.
  - `reasoning.approximate` = false; `reasoning.stride` = false;
    `reasoning.consistency.automatic` = false; `reasoning.punning.enabled`
    = false; `reasoning.classify.eager` = true.
  - `reasoning.schema.graphs`; `reasoning.rewriting` = PER_SCOPE;
    `reasoning.rewrite.limit` = 10000.
  - `reasoning.domain.range.rule.optimization` = true and
    `reasoning.single.valued.property.optimization` = true. Both are
    documented as not conformant with RDFS/OWL semantics.
  - Reasoning is enabled per query with `reasoning=true`.
- User rules ("Blackout") allow "no variables in predicate position", so the
  W3C rules cannot be written as rules there.

**Blocks running or publishing?** Yes on both counts: no materialisation to
time, a licence key needed for local use, and the "competing" clause.

---

## 6. Apache Jena

- **Version and licence.** 6.2.0 (tag `jena-6.2.0`, 2026-07-27), Apache-2.0,
  Java 21 (`pom.xml:60`). The Homebrew `jena` bottle is architecture-free
  ("all").
- **Reasoners**
  ([inference documentation, "Available reasoners", "OWL coverage", "RDFS configuration"](https://jena.apache.org/documentation/inference/)):
  - `ReasonerRegistry.getTransitiveReasoner()`, `getRDFSReasoner()` and
    `getRDFSSimpleReasoner()`.
  - `getOWLReasoner()`: full, rules in `etc/owl-fb.rules`.
  - `getOWLMiniReasoner()`: `owl-fb-mini.rules`, "nearly the same but omits
    the forward entailments from minCardinality/someValuesFrom
    restrictions".
  - `getOWLMicroReasoner()`: `owl-fb-micro.rules`, "just supports RDFS plus
    the various property axioms, intersectionOf, unionOf (partial) and
    hasValue. It omits the cardinality restrictions and equality axioms".
  - "The critical constructs which go beyond OWL/lite and are not supported
    in the Jena OWL reasoner are complementOf and oneOf". However,
    `owl-fb.rules:579-588` has "Restricted support for oneOf".
- **RDFS levels.**
  - `full`: "all of the RDFS axioms and closure rules with the exception of
    bNode entailments and datatypes".
  - `default`: "omits the expensive checks for container membership
    properties".
  - `simple`: "just the transitive closure of subPropertyOf and subClassOf
    … It omits all of the axioms".
  - Constants are `ReasonerVocabulary.RDFS_FULL`, `RDFS_DEFAULT` and
    `RDFS_SIMPLE` (`RDFSRuleReasoner.java:41-65`).
- **Model specs.**
  - Legacy `OntModelSpec` is deprecated for removal (`OntModelSpec.java:55-57`).
    It has `OWL_MEM_RULE_INF`, `OWL_MEM_MINI_RULE_INF`,
    `OWL_MEM_MICRO_RULE_INF` and `RDFS_MEM_RDFS_INF`.
  - Its replacement, `org.apache.jena.ontapi.OntSpecification`, has
    `OWL2_FULL_MEM_RULES_INF`, `OWL2_FULL_MEM_MINI_RULES_INF` and
    `OWL2_FULL_MEM_MICRO_RULES_INF` (lines 150-168).
  - It also has `OWL2_RL_MEM_RULES_INF` (line 277), which uses the same
    `OWLFBRuleReasonerFactory`, the OWL-Lite-era rules, not an OWL 2 RL
    rule set (`OntSpecification.java:277-279`).
- **Engine modes.** `PROPruleMode`: "forward", "forwardRETE", "backward" or
  "hybrid" (default "hybrid"). The OWL Mini and Micro reasoners set hybrid
  mode (`OWLMiniReasoner.java:61`, `OWLMicroReasoner.java:68`).

Jena is usable as a reference for "what a common Java toolkit derives". It
is not an OWL 2 RL reasoner, so it cannot carry a correctness claim.

---

## 7. Switchable reasoning in practice

| System | Switches, named as documented | Source |
|---|---|---|
| owlrl 7.6.2 (Python, W3C-20150513 licence) | `DeductiveClosure(closure_class, improved_datatypes=True, rdfs_closure=False, axiomatic_triples=False, datatype_axioms=False)`; classes `RDFS_Semantics`, `OWLRL_Semantics`, `RDFS_OWLRL_Semantics`, `OWLRL_Extension`, `OWLRL_Extension_Trimming`; methods `use_improved_datatypes_conversions()`, `use_rdflib_datatypes_conversions()`. There is no `owl_closure` parameter. CLI script: `-w/--owlrl yes\|no`, `-r/--rdfs yes\|no`, `-e/--extras`, `-a/--axioms`, `-d/--daxioms`, `-t/--trimming`, `-m/--maximal`, `-i/--input_syntax`, `-o/--serialization`. The 7.6.2 wheel installs no `owlrl` command (no entry points; poetry-core build), and `auto` input maps `.nt` to RDF/XML. | `owlrl/__init__.py:347-354, 408-422, 193-204`; `scripts/owlrl:14-70`; `OWLRLExtras.py:100, 259-276` at `17c8bc4`; [PyPI](https://pypi.org/project/owlrl/) |
| Jena | Choice of reasoner or `OntSpecification`; `ReasonerVocabulary.PROPruleMode`, `PROPruleSet`, `PROPsetRDFSLevel`, `PROPenableOWLTranslation`, `PROPenableTGCCaching`, `PROPenableCMPScan`, `PROPenableFunctorFiltering`, `PROPtraceOn`, `PROPderivationLogging` | `ReasonerVocabulary.java:72-104`; [inference docs](https://jena.apache.org/documentation/inference/) |
| GraphDB | `ruleset` (and `-optimized`), `disable-sameAs`, `FROM onto:disable-sameAs`, `check-for-inconsistencies`, `imports` (read-only schema), `-Dgraphdb.inference.concurrency` | §4 |
| Stardog | `reasoning.type`, `reasoning.sameas` OFF/ON/FULL, `reasoning.approximate`, `reasoning.domain.range.rule.optimization`, `reasoning.single.valued.property.optimization`, per-query `reasoning=true` | §5 |
| RDFox | data-store parameter `equality` (for example `noUNA`); which axioms are imported (`importaxioms`); `import <rdfox:TBoxReasoning>` | [reasoning docs §10.1.2.4, §10.6.5–6](https://docs.oxfordsemantic.tech/reasoning.html) |
| VLog | the rule file; `mat --multithreaded --nthreads --interRuleThreads --restrictedChase --shufflerules` | `src/launcher/main.cpp:283-311` |
| Rulewerk | the rule file; `--chase-algorithm RESTRICTED_CHASE\|SKOLEM_CHASE` | [wiki: Standalone client](https://github.com/knowsys/rulewerk/wiki), lines 176-190 |
| Nemo | the rule file; `--param key=value` ("Overwrite global parameters in the rule file"); `-e/--export keep\|none\|idb\|edb\|all` | `nemo-cli/src/cli.rs:220-222` |
| reasonable 0.5.0-a1 (BSD-3) | Cargo feature `prp-spo2` (default on; `--no-default-features` turns it off); `rdf-12`; CLI `--fail-on`, `--max-diagnostics`, `--summary-only`; `ReasonerOptions { collect_diagnostics, max_diagnostics, dedupe }` | `lib/Cargo.toml:31-34`; `cli/src/main.rs:20-35`; `lib/src/reasoner.rs:102-118` at `fecee2b` |

Pattern: the materialisers (RDFox, GraphDB) and Stardog switch off equality
first, because it is the costliest feature. The Datalog engines (VLog, Nemo,
Soufflé) have no OWL switches: the rule file is the switch. owlrl switches
off axiomatic and datatype triples by default.

---

## 8. Datasets

### 8.1 What each paper used

The per-paper sizes are in the tables of §1.2. Rule counts per program,
from the papers:

| Data | AAAI 2014 (L / LE / U) | AAAI 2015 (eq) | AIJ 2019 (L / LE / R / U) | VLog 2016 (L / LE / U) |
|---|---|---|---|---|
| LUBM | 98 / 107 / 122 | — | — | 170 / 182 / 202 |
| UOBM | 407 (01K) / — / 518 (010) | 279 | 210 / — / 2,379 / — | — |
| Claros | 2,174 / 2,223 / 2,614 | 1,312 | 1,310 / 1,337 / — / — | 2,689 / 2,749 / 3,229 |
| DBpedia | 7,258 / 7,333 / — | 3,384 | 5,936 / — / — / — | 9,396 / — / — |
| UniProt | — | 451 | 451 / — / 12,018 / — | — |
| ChEMBL | — | — | 259 / — / 2,265 / — | — |
| Reactome | — | — | 541 / — / 21,385 / 842 | — |
| OpenCyc | — | 261,067 | — | — |

The VLog counts are higher because its rewrites add rules that copy EDB
triples into IDB predicates (VLog 2016, p. 5).

Nemo KR 2024 (Table 2, p. 9), rules / input facts / new facts derived:

- Galen-EL: 12 / 143,480 / 1,881,946.
- SNOMED-EL: 12 / 1,554,340 / 24,428,006.
- Doctors-1M: 5 / 9,515,900 / 792,500.
- Ont-256: 529 / 2,146,490 / 5,673,985.
- LUBM-01k: 136 / 133,573,854 / 186,742,694.
- Deep-100 and Deep-200.
- Wikidata truthy: 8.1 G triples (§5.3).

**(reading)** About 133.6 M of LUBM-01k's derivations are its 30 rules
copying each input fact into a target predicate. That leaves about 53 M new
facts, consistent with Oxford's L closure of 182.4 M.

**Variants**, as the papers define them (AIJ 2019 §10.1, p. 35; AAAI 2014 §5):

- **L (lower bound)**: drop the axioms that are not Datalog, giving the
  OWL 2 RL part translated to rules. Sound but incomplete.
- **U (upper bound)**: existentials become fresh constants and disjunctions
  become conjunctions. Complete but unsound.
- **LE**: L plus hand-written chain rules (for example LUBM colleagues,
  Claros refersToSame), which build large cliques.
- **R**: Kaminski et al.'s rewriting into Datalog. Sound, keeps more than L,
  highly recursive.

L is the variant closest to "OWL 2 RL on this ontology". Oxford also
publishes it as OWL: `owl/LUBM_L.owl` and `owl/LUBM_LE.owl` (the LE chains
as `owl:propertyChainAxiom` plus symmetric and transitive properties). An
OWL 2 RL reasoner can therefore load L and LE directly. Its W3C closure will
be larger than the `.dlog` closure, because it adds TBox and schema triples
and possibly eq-ref triples. The size of that W3C closure is **(unconfirmed)**.

### 8.2 Where to get each dataset today

Abbreviation: **Ox14** is
[cs.ox.ac.uk/isg/tools/RDFox/2014/AAAI/](https://www.cs.ox.ac.uk/isg/tools/RDFox/2014/AAAI/)
(200), whose zips are under `https://krr-nas.cs.ox.ac.uk/2014/AAAI/RDFox/`.
Those pages license only the RDFox binary ("distributed under the following
academic licence") and state nothing about the data or rules.

The sentence "only usable under Oxford's licence agreement" is on the
[AIJ 2019 journal page](https://www.cs.ox.ac.uk/isg/tools/RDFox/2017/RDFox-Journal/index.html)
only, directly after the binaries. The licence it links to returns 404.

| Data | Download today (checked 2026-11-05) | Size | Licence | Programs available |
|---|---|---|---|---|
| LUBM (Oxford) | Ox14 `LUBM-01K.zip` (200, 731,628,325 B), `LUBM-05K.zip` (200, 3,659,713,941 B); ISWC 2015 LUBM-50K, 11 × ~3.5 GB `.nt.gz` | 133.6 M; 691.1 M; 6.7 G | unstated (data); UBA generator GPL | Ox14 `LUBM_L/LE/U.dlog` (98/107/122) and `owl/LUBM_{L,LE,U}.owl`; VLog rewrites |
| LUBM (generator) | [UBA 1.7](https://swat.cse.lehigh.edu/projects/lubm/uba1.7.zip) (200, 35,075 B, Java); ontology [univ-bench.owl](https://swat.cse.lehigh.edu/onto/univ-bench.owl) (200); fork [rvesse/lubm-uba](https://github.com/rvesse/lubm-uba) (N-Triples output, threads; no LICENSE file) | about 133.6 K triples per university (Oxford 01K); 137.8 K (Guo et al. 2005, Table 1) | GPL v2+ (generator); output is yours | — |
| LUBM (ChaseBench) | [dbunibas/chasebench](https://github.com/dbunibas/chasebench) `scenarios/LUBM/data/01k/data.zip.001–006` | 133.6 M facts as CSV relations | **no licence** in the repository | Nemo/VLog/Soufflé/Gringo programs in [nemo-examples `evaluations/kr2024`](https://github.com/knowsys/nemo-examples/tree/main/evaluations/kr2024) (Apache-2.0); existential, not RL |
| UOBM | Ox14 `UOBM-010.zip` (200, 15,123,806 B), `UOBM-01K.zip` (200, 1,856,099,845 B); Oxford [UOBMGenerator](https://www.cs.ox.ac.uk/isg/tools/UOBMGenerator/) jar (200; no source or licence) | 2.2 M; 254.8 M | unstated | Ox14 `UOBM_L` (407), `UOBM_U` (518); AAAI 2015 `UOBM_U-eq` (279). AIJ 2019 L/R: **404** |
| Claros | Ox14 `Claros.zip` (200, 169,025,879 B); VLog repo `Claros-S.nt.gz` (12.3 MB). clarosnet.org is dead | 18.8 M | "CLAROS is open: the material is to be useable by anyone for anything" ([eng.ox.ac.uk/claros](https://eng.ox.ac.uk/claros/technical)); no formal licence **(unconfirmed)** | Ox14 L/LE/U (2,174/2,223/2,614); AAAI 2015 L-eq; VLog L/LE |
| DBpedia (2014 subset) | Ox14 `DBpedia.zip` (200, 1,348,349,738 B) | 112.7 M | CC BY-SA 3.0 and GFDL ([dbpedia.org/about](https://www.dbpedia.org/about/)) | Ox14 L/LE (7,258/7,333); AAAI 2015 L-eq (3,384). AIJ 2019 subset (386.7 M): **404** |
| UniProt | AAAI 2015 [`UniProt.zip`](https://krr-nas.cs.ox.ac.uk/2015/AAAI/RDFox/Equality/UniProt.zip) (200, 2,022,674,354 B); source at ftp.uniprot.org | 123 M | CC BY 4.0 (source); subset unstated | AAAI 2015 `UniProt-eq` (451). AIJ 2019 L/R: **404** |
| OpenCyc | AAAI 2015 `OpenCyc.zip` (200, 24,266,534 B) | 2.4 M | CC BY 3.0 per the archived Cycorp page; project discontinued | AAAI 2015 `OpenCyc-eq` (261,067 rules) |
| ChEMBL | source only: [ftp.ebi.ac.uk ChEMBL-RDF](https://ftp.ebi.ac.uk/pub/databases/chembl/ChEMBL-RDF/) (200) | 289.2 M (AIJ subset) | CC BY-SA 3.0 | AIJ 2019 L/R: **404** |
| Reactome | source only: [biopax.zip](https://reactome.org/download/current/biopax.zip) (200, 165 MB) | 12.5 M (AIJ subset) | CC0 | AIJ 2019 L/R/U: **404**. GLog's repository may hold a copy **(unconfirmed)** |
| Galen-EL | nemo-examples `examples/owl-el/from-preprocessed-csv/data` | 143 K facts | GALEN open-source licence | EL calculus, not RL |
| SNOMED CT | not public | 1.55 M facts | licensed (survey §4.4) | unusable on a public scoreboard |
| OWL2Bench | [kracr/owl2bench](https://github.com/kracr/owl2bench) (Apache-2.0); data [Zenodo 4764368](https://zenodo.org/records/4764368) (CC BY 4.0) | RL ABox 50,131 axioms (1 university) to 14.6 M (200) | as stated | RL TBox; no rule program |
| Wikidata truthy | `latest-truthy.nt.gz` (200, 71.8 GB) | about 8 G | CC0 | none |

**LUBM in detail.**

- UBA options are `-univ N -index I -seed S -onto <url>`. The Lehigh page
  says "Using options -index 0 -seed 0 can produce the same data sets as
  used in papers"
  ([LUBM project page](https://swat.cse.lehigh.edu/projects/lubm/)).
- Each university draws from its own `java.util.Random`, seeded from the
  base seed and the university index (UBA `Generator.java`). The output
  therefore depends only on (seed, index) and is deterministic.
- No RDFox, VLog or ChaseBench paper states the seed and index it used
  **(unconfirmed; probably 0 and 0)**. For exact parity with the published
  numbers, use Oxford's zips rather than regenerating.
- This repository's bench uses its own LUBM-style generator. Its numbers
  are not comparable with the papers' LUBM until the bench runs UBA output
  or the Oxford files, with the univ-bench ontology or `LUBM_L.owl`.

### 8.3 Does it fit in 32 GiB?

Budget: 32 GiB ≈ 34.36 × 10⁹ B, so at 150 B per closure triple the ceiling
is about 229 M closure triples. Bands: comfortable up to 60 % (about 137 M),
marginal from 60 to 100 %, no above. RDFox's own footprint was 51 B per
triple on LUBM L01K (9.3 GB / 182.4 M, AAAI 2014), so 150 B is conservative
for it and an assumption for TurbOWL.

| Instance | Closure | At 150 B | Share | Verdict |
|---|---|---|---|---|
| UOBM-010 U | 35.6 M | 5.0 GiB | 16 % | comfortable |
| Claros L (AIJ 2019 / AAAI 2014) | 73.8 M / 95.5 M | 10.3 / 13.3 GiB | 32 / 42 % | comfortable |
| LUBM-500 L | 91.1 M | 12.7 GiB | 40 % | comfortable |
| Claros REW / AX (AAAI 2015) | 79.7 M / 102 M | 11.1 / 14.2 GiB | 35 / 45 % | comfortable |
| DBpedia-2014 L | 118.3 M | 16.5 GiB | 52 % | comfortable |
| DBpedia REW / AX (AAAI 2015) | 136 M / 139 M | 19.0 / 19.4 GiB | 59 / 61 % | borderline |
| OpenCyc REW | 142 M | 19.8 GiB | 62 % | marginal (AX, 1,176 M, does not fit) |
| LUBM-1K L | 182.4 M | 25.5 GiB | 80 % | marginal |
| LUBM-1K U | 219.0 M | 30.6 GiB | 96 % | no in practice |
| UniProt (AAAI 2015) | 228 M | 31.9 GiB | 100 % | no |
| LUBM-1K LE | 332.6 M | 46.5 GiB | 145 % | no |
| UOBM-01K L | 429.6 M | | | no |
| Claros LE | 533–555 M | | | no |
| LUBM-5K L | 911.2 M | | | no |
| DBpedia LE | 1,529.7 M | | | no |

For LUBM L the closure grows by about 0.182 M triples per university
(182.4 M at 1K, 911.2 M at 5K, 91.1 M at 500). About 750 universities fit
comfortably, 1,000 is marginal and 1,250 is the ceiling. For LE the
comfortable size is about 400 universities, and for U about 600.

---

## 9. OWL 2 RL materialisation benchmarks since 2020

No dedicated OWL 2 RL materialisation benchmark was found for 2020–2026.
What exists:

1. **OWL2Bench**, ISWC 2020 Resources, LNCS 12507
   ([author PDF](https://gunjansingh1.github.io/files/owl2bench.pdf)).
   - A UOBM-based generator with EL, QL, RL and DL TBoxes and 22 SPARQL
     queries. It measures loading, consistency, classification and query
     time.
   - Its evaluation ran DL reasoners, GraphDB and Stardog, not RL
     materialisers. The RL TBox uses max-cardinality, so it exercises
     `owl:sameAs`.
   - No successor; OWL2Gen (ER 2024, CEUR Vol-3849) is related.
2. **ORE.** The ORE 2015 report: "In prior years we also had an RL track,
   but the number of RL-specific reasoners is very low"
   ([J. Autom. Reasoning 2017, "Tracks"](https://pmc.ncbi.nlm.nih.gov/articles/PMC6044265/)).
   That RL track was classification, not materialisation. No later edition
   was found **(unconfirmed)**.
3. **LDBC SPB 2.0**
   ([spec](https://ldbcouncil.org/docs/specifications/ldbc-spb-v2.0-specification.pdf);
   ldbc/ldbc_spb_bm_2.0, Apache-2.0).
   - Needs RDFS plus transitive and symmetric properties and sameAs, with
     an OWL 2 RL conformance check.
   - Scale factors 64 M, 256 M and 1 G triples.
   - GraphDB's own reference run on SPB-256 with OWL2-RL went from 256.1 M
     to 775.1 M statements
     ([GraphDB 11.0 benchmark](https://graphdb.ontotext.com/documentation/11.0/benchmark.html)).
4. **GLog** (PVLDB 2021, [karmaresearch/glog](https://github.com/karmaresearch/glog),
   Apache-2.0) reuses the Oxford LUBM, UOBM, DBpedia, Claros and Reactome
   programs, LUBM up to 128 K universities, and ChaseBench. Its data is on
   Google Drive; the original Bitbucket link returns 404.
5. **Oxford follow-ups.**
   - IJCAI 2023 data: [xinyuezhang.xyz/HDReasoning](https://xinyuezhang.xyz/HDReasoning/) (200).
   - AAAI 2024 data: [xinyuezhang.xyz/TCReasoning](https://xinyuezhang.xyz/TCReasoning/) (200).
   - AIJ 2022 modular-materialisation data: `krr-nas.cs.ox.ac.uk/2018/modular/` returns 403.
6. **Nemo KR 2024 and VFLog AAAI 2025**: ChaseBench and EL, as in §8.1.
7. **CaLiGraph as a challenge for OWL reasoners**, SemREC 2021 (CEUR
   Vol-3123). It tests `owl:hasValue` (cls-hv) at scale, from 43 K to
   54.9 M triples. Where to download the data is **(unconfirmed)**.
8. **Brick models in reasonable**
   ([gtfierro/reasonable](https://github.com/gtfierro/reasonable),
   `benches/python/buildings/`): 108 Brick 1.1 models of up to about 17.5 K
   triples. There is no paper, and the models' licence is **(unconfirmed)**.

---

## 10. Documented OWL 2 RL coverage

Key: **Y** documented or present in the shipped rules; **N** documented as
absent; **n/d** not documented. GraphDB entries are read from the shipped
`.pie` files **(reading)**. Jena entries map a documented construct to the
W3C rule.

| Rule | owlrl 7.6 | GraphDB `owl2-rl` (= `-optimized` except axioms) | Stardog RL/SL | Jena OWL / Mini / Micro | OWL-RS Nemo program | balhoff Soufflé |
|---|---|---|---|---|---|---|
| eq-ref | Y (`OWLRL.py:384-387`) | Y (`eq_ref`) | only with `reasoning.sameas` ON/FULL | n/d | N (header :10-11) | N |
| eq-sym/trans/rep | Y | in the engine (representatives); off with `disable-sameAs` | ON/FULL; "differs from the OWL semantics slightly" | full and mini; not micro | Y | N |
| eq-diff1 | Y (error, :404) | check, only when `check-for-inconsistencies=true` | n/d | full and mini (differentFrom) | Y | N |
| eq-diff2/3 | Y (:415) | via `owl_allDiff*`; **`owl:distinctmembers` misspelt in `owl_allDiff2`, so eq-diff3 likely never fires (reading)** | n/d | distinctMembers in full and mini; AllDifferent absent | Y | N |
| prp-key | Y (:578) | Y (`prp_key_1-3`) | `reasoning.sameas=FULL` | N (no hasKey) | Y | N |
| prp-npa1/2 | Y (:623) | check | n/d | N | Y | Y |
| prp-adp | Y (:516) | via pairwise `owl:propertyDisjointWith` (extra triples) | consistency only | N | Y | N |
| prp-ap | Y (:305-307) | axiomatic triples | n/d | n/d | N (axiomatic triples left out) | N |
| cls-maxc1/2 | Y (:728-747) | check for `"0"`; rule for `"1"^^xsd:nonNegativeInteger` only | consistency for 0 only; "Higher cardinality restrictions are ignored" | maxCardinality in full and mini | Y | N |
| cls-maxqc1-4 | Y (:755-777) | checks for 0; rules for 1 | n/d | N (no qualified cardinality) | Y | N |
| cls-oo | Y (:798) | Y (`cls_oo_1/2`) | ObjectOneOf as subclass only | doc: N; rules: restricted | Y | N |
| cax-adc | Y (:841) | via pairwise `owl:disjointWith` (extra triples) | consistency only | N | Y | N |
| scm-* | Y, all (:870–1007) | Y, all 20 | n/d per rule; classification exists | n/d | Y | N |
| dt-type1 | partial: only datatypes in use (:265) | axioms for all 32 RL datatypes | n/d | XSD datatypes declared (`owl-fb.rules:596+`) | N | N |
| dt-type2 | Y (:215) | N: "These are not implemented (and probably not implementable)" (`.pie:1191`) | n/d | n/d | N | N |
| dt-eq, dt-diff | N: "skipped on purpose" (:234-237) | N (same comment) | n/d | n/d | N | N |
| dt-not-type | N: "has not (yet?) been implemented" (:178-180) | N | n/d | n/d | N | N |

Notes:

- **owlrl datatypes** (`XsdDatatypes.py:30-66, 78`): the XSD numerics,
  `float`, `double`, string types, `boolean`, `hexBinary`, `base64Binary`,
  `anyURI`, `dateTime`, `dateTimeStamp`, `time`, `date`, `rdfs:Literal`,
  `rdf:XMLLiteral`, `rdf:HTML`, `rdf:langString` and `rdf:PlainLiteral`.
  `owl:real` is absent, and `owl:rational` appears only in
  `OWLRL_Extension`. owlrl "does *not* implement the so called Datatype
  entailment rules" (`OWLRL.py:77-82`).
- **GraphDB.** "GraphDB does not provide the extended support for typed
  literals … (D-entailment)" and "There are no inconsistency rules by
  default"
  ([OWL compliance](https://graphdb.ontotext.com/documentation/11.5/owl-compliance.html)).
  It has no rule for cls-int2 or cls-uni; their effect comes through
  scm-int, scm-uni and cax-sco. **(reading)** `prp_key_3_bottom` may
  over-derive when key lists are shared.
- **Stardog's RL table** is "a strict superset of how the RL profile is
  defined", but it lists axioms, not rules
  ([Blackout OWL support](https://docs.stardog.com/inference-engine/blackout-owl-support)).
- **RDFox** has no rule-level documentation (§1.1).
- **reasonable**: per its own docs, it implements none of eq-ref, eq-diff*,
  prp-key, prp-npa*, prp-adp, prp-ap, cls-maxc*, cls-maxqc*, cls-oo,
  cax-adc or dt-*. Of the scm-* rules it lists only scm-eqc1/2. See
  `docs/research/reasonable-parity.md`.

Consequence for the scoreboard: each system's closure differs from the W3C
closure in a known way. A comparison should state the rule set, normalise
the output (for example, by excluding axiomatic and datatype triples) and
report the closure size next to the time.

---

## Open points

- Whether an e-mailed GraphDB Free licence is governed by
  `LICENSE-GraphDB.txt` (no publication clause) or `LICENSE-GraphDB-Free.txt`
  (Art. 15.3). Ask Graphwise.
- Whether the VLog dylibs in the Rulewerk jars load on macOS 27, and
  whether VLog builds with Apple clang and CMake 4.
- Whether the Homebrew Soufflé bottle lacks OpenMP (inferred, not run).
- The UBA seed and index behind the Oxford and ChaseBench LUBM files.
- The size of the W3C OWL 2 RL/RDF closure of `LUBM_L.owl` plus LUBM-1K,
  as opposed to the `.dlog` closure. This decides whether LUBM-1K fits on
  the M6.
- A formal licence for the Oxford zips (data and rules), and for Claros.
- The LUBM size behind GLog's Tables 2–3.

## Sources read

Primary sources are cited inline. Local evidence saved during this research
(licence texts, `.pie` files, Oxford and VLog rule files, UBA source, paper
text) sits in this session's scratchpad and is not part of the repository.
