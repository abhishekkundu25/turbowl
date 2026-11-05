# Tickets: version 2 (the best OWL 2 RL reasoner)

Specs: `docs/specs/v2-1-scoreboard-and-rules.md` (phase 1, tickets 01 to
15), `docs/specs/v2-2-speed.md` (phase 2, 16 to 21),
`docs/specs/v2-3-removals.md` (phase 3, 22 to 24). Decisions: ADR 0005
(switches), ADR 0006 (the scoreboard). Lock files are in **bold**.

| # | Ticket | Blocked by | Files |
|---|---|---|---|
| 01 | [Remove `reasonable`](01-remove-reasonable.md) | — | `crates/bench/` (`Cargo.toml`, `src/engine.rs`, **`src/main.rs`**, `src/scaling.rs`, `src/compare.rs`, **`README.md`**), **`Cargo.lock`**, `docs/specs/experiments.md`, one doc line in `crates/oracle/src/lib.rs` |
| 02 | [Switches: the seam](02-switches.md) | — | `crates/turbowl/`: `Cargo.toml`, `src/lib.rs`, new `src/switch.rs`, `src/reasoner.rs`, **`src/rules/mod.rs`**, **`src/term.rs`**, new `src/rules/checks.rs`, `src/rules/datatype.rs`, `src/rules/property.rs`, `src/rules/property/chain.rs`, new empty `src/rules/{identity,schema,trivial,literal_values}.rs` and `src/rules/class/enumerations.rs`, `src/rules/class.rs`, `src/rules/testing.rs`; `scripts/check-wasm.sh` |
| 03 | [One rule source for every engine](03-rule-source.md) | 01, 02 | `oracles/nemo/owl2rl.rls` → `oracles/rules/<group>.rls`, new `crates/oracle/src/programs.rs`, `crates/oracle/src/{nemo,lib}.rs`, `crates/oracle/tests/programs/`, `crates/bench/src/nemo.rs`, `crates/bench/Cargo.toml`, **`Cargo.lock`** |
| 04 | [Verification by configuration](04-verification-by-configuration.md) | 03 | `crates/reference/` (`src/evaluate.rs` → `src/rules/<group>.rs`, `src/generate.rs` → `src/generate/<group>.rs`, `src/lib.rs`, `tests/`), `crates/oracle/` (`src/cases.rs` → `src/cases/<group>.rs`, `src/{ours,triangulate,w3c}.rs`, `tests/`) |
| 05 | [Rules: the new checks](05-rules-checks.md) | 04 | the `checks` files: `crates/turbowl/src/rules/checks.rs`, `oracles/rules/checks.rls`, and those in `crates/reference/src/{rules,generate}/` and `crates/oracle/src/cases/` |
| 06 | [Rules: identity](06-rules-identity.md) | 04 | the `identity` files, as for 05 |
| 07 | [Rules: the schema](07-rules-schema.md) | 04 | the `schema` files, as for 05 |
| 08 | [Rules: the trivial group and cls-oo](08-rules-trivial-and-enumerations.md) | 04 | the `trivial` files, and for cls-oo `crates/turbowl/src/rules/class/enumerations.rs` and the `core` files; `crates/turbowl/src/reasoner.rs` (eq-ref on read) |
| 09 | [The OWL 2 datatype map](09-datatype-map.md) | 04 | `crates/turbowl/src/rules/datatype.rs`, new `crates/turbowl/src/rules/datatype/values.rs`, `crates/reference/src/datatype.rs`, the `datatypes` files of the generator and the minimal inputs |
| 10 | [Literal values and typing](10-literal-values-and-typing.md) | 08, 09 | the `literal-values` files; the `trivial` files (dt-type2) |
| 11 | [Switches in the CLI and Python](11-switch-adapters.md) | 02 | `crates/cli/`, `crates/python/` |
| 12 | [Competitors: VLog, Soufflé, Nemo (GraphDB if licensed)](12-competitors.md) | 03, 13 | `crates/bench/src/engine.rs`, new `crates/bench/src/{souffle,vlog,graphdb}.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`**, `scripts/fetch-tools.sh` |
| 13 | [Scoreboard data and the 16 GB cap](13-scoreboard-data.md) | 01 | `scripts/fetch-data.sh`, `crates/bench/src/scenario.rs`, new `crates/bench/src/datasets.rs`, `crates/bench/src/machine.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`** |
| 14 | [Published results and the scoreboard report](14-published-results-and-report.md) | 12, 13 | new `crates/bench/src/{published,report}.rs` and the published-results data file, `crates/bench/src/compare.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`** |
| 25 | [Memory: OWL2Bench RL, UOBM-010 U and OpenCyc](25-memory.md) | 13 | `crates/turbowl/` (where the cause is), `crates/turbowl/src/scaling.rs` |
| 26 | [OWL-RL's datatype and identity faults](26-owlrl-faults.md) | 08 | `crates/oracle/src/differences.rs`, `crates/oracle/src/cases/datatypes.rs` |
| 15 | [Close-out: phase 1](15-close-out-phase-1.md) | 01 to 14, 25, 26, version 1's 14 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `CONTEXT.md`, the phase 1 spec, `docs/verification/`, this README |
| 16 | [Fast loading: parsers and the dictionary](16-parsers-and-dictionary.md) | 15 | `crates/turbowl/src/io.rs` and new `src/io/`, **`crates/turbowl/src/term.rs`**, `crates/turbowl/src/scaling.rs` |
| 17 | [All cores: parallel rounds](17-parallel-rounds.md) | 15 | `crates/turbowl/src/engine/`, **`crates/turbowl/src/reasoner.rs`**, **`crates/turbowl/src/rules/mod.rs`**, `crates/turbowl/Cargo.toml`, `scripts/check-wasm.sh` |
| 18 | [All cores: parallel parsing](18-parallel-parsing.md) | 16, 17 | `crates/turbowl/src/io/`, **`crates/turbowl/src/term.rs`** |
| 19 | [Snapshots](19-snapshots.md) | 16, 17 | new `crates/turbowl/src/snapshot.rs`, `crates/turbowl/src/lib.rs`, **`crates/turbowl/src/reasoner.rs`**, `crates/cli/`, `crates/python/` |
| 20 | [The improvement loop](20-improvement-loop.md) | 17, 18, 19 | each experiment's files; **`docs/specs/experiments.md`** (one row per iteration, written by the orchestrator) |
| 21 | [Close-out: phase 2](21-close-out-phase-2.md) | 16 to 20 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `docs/verification/`, this README |
| 22 | [Removals: DRed](22-dred.md) | 21 | `crates/turbowl/src/engine/`, **`crates/turbowl/src/reasoner.rs`**, `crates/turbowl/src/rules/`, `crates/reference/tests/chaos.rs` |
| 23 | [Removals: FBF](23-fbf.md) | 22 | as for 22 |
| 24 | [Close-out: phase 3 and the claim](24-close-out-phase-3.md) | 22, 23 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `docs/verification/`, this README |

## How the lock files stay apart

- **`crates/turbowl/src/rules/mod.rs`** and **`crates/turbowl/src/term.rs`**:
  ticket 02 registers every family and adds every vocabulary IRI at once,
  so no rule ticket edits them. In phase 2, 16 then 18 edit `term.rs` in
  sequence, and 17 alone edits `rules/mod.rs`.
- **Each switch group has its own files** in the core, `oracles/rules/`,
  the reference evaluator, the generator and the minimal inputs. Tickets
  02 and 04 create them empty, and one rule ticket fills each group. The
  `trivial` group is filled by 08 and then 10.
- **`crates/bench/src/main.rs`** and **`crates/bench/README.md`**: 01, 13,
  12 and 14 edit them, one per wave.
- **`Cargo.lock`**: 01 removes `reasonable` and 03 adds the bench's
  dependency on the oracle crate, in different waves.
- **`crates/turbowl/src/reasoner.rs`**: 02, then 08 (alone in its wave),
  then 17, 19, 22 and 23 in sequence.

## Waves

Phase 1:

1. **01, 02.** Version 1's ticket 14 runs whenever the owner gives a quiet
   machine; it blocks only 15.
2. **03, 11, 13.**
3. **04, 12, 25.** 25 (added: ticket 13 found the memory blow-up) lands before wave 4, whose rule tickets edit the core.
4. **05, 06, 07, 08, 09, 14.**
5. **10.**
6. **15.**

Phase 2: **16, 17**, then **18, 19**, then **20**, then **21**.

Phase 3: **22**, then **23**, then **24**.

## Log

Departures from the plan, as they happen.

- **01.** The bench's `Engine` trait went with `reasonable`, since TurbOWL
  was its only other adapter; the competitors of ticket 12 run as
  subprocesses, as Nemo does. `--list` also prints the engines. Results
  files keep schema 4 until ticket 14; `--compare` skips the rows of any
  engine the bench no longer runs, and says so.
- **02.** The vocabulary already named every IRI the new rules need, so
  `term.rs` gained only docs and a test, and the canon's rule constants are
  unchanged. The internal counters list only the families that ran. The
  checks of version 1 moved verbatim to `rules/checks.rs`, and
  `scaling_datatype_rules_linear_in_instances` became
  `scaling_checks_linear_in_instances`.
- **11.** `--complete` is a starting point wherever it comes; a later
  `--switch` for the same name wins. Python's `Reasoner` takes
  keyword-only `switches` and `complete`, and has no setter yet. The CLI's
  help and the Python docs each describe the switches' costs (the CLI owns
  its help text). On review, the `literal-values` text was corrected: dt-diff
  only checks, and lists no `owl:differentFrom` triples.
- **03.** The group files are compiled into the oracle crate
  (`include_str!`), so editing a rule needs a rebuild, which cargo does by
  itself. Nemo's runner reads whatever outputs the program exports, so a
  configuration without `checks` exports no diagnostic files (ticket 04
  replaced `nemo::CODES` with `nemo::codes(switches)`). The Soufflé and VLog programs
  are checked against each engine's grammar but have not run: ticket 12
  runs them. Rulewerk has no `!=`, so the VLog program writes
  `?x != c` as `~sameTerm(?x, c)` over facts; Soufflé reads tab-separated
  terms, not N-Triples. A test reads the spec's switch table and checks
  that every rule sits in exactly one group file.
- **04.** The W3C suites run in the complete configuration only (the
  spec's choice; the ticket's acceptance said both). A case's written
  expectations count only where its group is on, and an expected finding
  only where its code's group is on. Version 1's cases went where their
  rules live: `scm-sco-cycle` and `not-scm-spo` in `schema`, `not-eq-ref` in
  `trivial`, the dt-not-type cases in `datatypes`. The core cases `empty` and
  `no-thing-typing` state the exact closure, so ticket 08 must exempt the
  `trivial` group from them when it fills it. Chaos draws its configuration
  from a stream of its own, so every input is as before. Then, as a
  prefactor on `v2`, the generator gave each file's new shapes their own
  stream, drawn after version 1's: the fingerprint of version 1's part
  stays, and no two rule tickets edit the same value.
- **05, 06: shared files the plan missed.** Ticket 05 stopped before
  writing code. The rules' literals (`"0"` and `"1"` of
  `xsd:nonNegativeInteger`) cannot be named in `oracles/rules/`, because the
  oracle renames every literal for Nemo. So the codec now gives those two
  fixed IRIs (`NAMED_LITERALS`), committed on `v2` and applied to 05 and 06.
  The new finding codes live in shared files (`closure.rs`' codes,
  `diagnostics.rs`' term orders, `reason.py`, `differences.rs`), which ticket
  05 now owns alone. Its term orders were accepted:
  - eq-diff1: x, y;
  - eq-diff2 and eq-diff3: the AllDifferent node, then the two members in
    list order;
  - cax-adc: the individual, then the two classes in list order;
  - prp-adp: the two properties in list order, then subject and object;
  - prp-npa1 and prp-npa2: source, property, target;
  - cls-maxc1, cls-maxqc1 and cls-maxqc2: restriction, individual, value.

  OWL-RL's new faults (it never pairs a list's last member in eq-diff2/3
  and prp-adp, and its messages for the new rules name fewer terms) become
  listed faults with tests. With `trivial` on, eq-ref meets the checks in
  ticket 08. List positions count only along a path that reaches
  `rdf:nil`, as the existing list reader reads them.
- **12.** Soufflé 2.5 (Homebrew's arm64 bottle, unpacked into `tools/`,
  programs compiled with Xcode's compiler) and VLog 1.3.7 (Rulewerk
  0.9.0's client jar driven through its shell, since its `materialize`
  command is broken) run as competitors, single-threaded here. Every
  Datalog competitor reads the input converted once and encoded with the
  oracle's codec (now public), so each matches terms as TurbOWL does.
  Neither engine reports a load/reasoning split, so their rows have a
  total time only. Results files are schema 6. Peak memory is the lifetime
  physical footprint. On brick-small1, lubm-1, uobm-010-l and lubm-uba-10-l
  every engine's closure count equals TurbOWL's.
- **Landing patches twice.** The codec change was committed on `v2` and
  applied, uncommitted, to the worktrees of 05, 06 and 12. Ticket 12's
  cherry-pick then duplicated it, because 12 also changed the lines next
  to it. The fix removed the copy. Before 05 and 06 are committed, the
  orchestrator takes that hunk out of their worktrees.
- **05.** The consistency halves of its eight W3C cases pass. Their
  "entails" halves fail on Nemo and OWL-RL too: five need scm-cls, scm-op
  or scm-dp (ticket 08), and `WebOnt-differentFrom-001` expects
  `owl:differentFrom` to be symmetric, which no OWL 2 RL rule says. OWL-RL's
  messages name fewer terms; `reason.py` recovers the rest from its own
  closure. W3C OWL 2 RL: 95, every case Nemo passes.
- **06.** prp-key groups a class's individuals by hashing their key
  values. A group of k equal terms emits k² `owl:sameAs` triples in its
  first round, as prp-fp and prp-ifp do; with the switches in `Context`
  (ticket 08) a star of k would do, as a later experiment.
  `New-Feature-Keys-006` needs literal values (ticket 10), not schema
  rules. The reference evaluator's prp-key went quadratic over aliases and
  was made faster. W3C OWL 2 RL: 96, every case Nemo passes.
- **09.** Decisions taken:
  - `owl:real` has no lexical forms, so `"1"^^owl:real` is ill-typed.
  - `rdfs:Literal` is a value space only.
  - `xsd:string` accepts any string, as in version 1.
  - `rdf:PlainLiteral` tags follow `xsd:language`'s pattern.
  - An `rdf:XMLLiteral` is identified by its lexical form.

  Three tests that assumed version 1's five datatypes now expect the wider
  check. `WebOnt-I4.6-005-Direct` and `WebOnt-equivalentClass-008-Direct`
  need the Direct Semantics' handling of annotations, not dt-not-type.
  OWL-RL's datatype faults go to ticket 26, added.
- **25.** The cause was prp-trp: a component of k nodes made k³
  derivations for k² pairs, all buffered until the round ended. A
  transitive table now closes in one step past that size, and no round
  buffer is kept after a run. owl2bench-rl-1 went from over 8 GB to 81 MB.
  OpenCyc still passes the cap, for another reason: it makes a term
  `owl:sameAs rdfs:label`, so the generalized rules type every label
  literal through a large class hierarchy (24 M literal-subject tuples).
  The owner decides between dropping literal-subject derivations,
  compacting the round buffer, and lazy diagnostics. scm-sco has the same
  join shape; experiment E2 is planned for it.
- **14.** No RDFox estimate can be made yet, for three reasons:
  - Oxford's `Claros_L.dlog` and `UOBM_L.dlog` axiomatise `owl:sameAs`, but
    replace only the predicates their ontology names. The W3C rules
    replace every predicate, so TurbOWL's comparable count is 3.6% under
    the paper's with `equality` off and 19.8% over with it on. Claros L
    stays context, not an estimate.
  - VLog, running the W3C rules through Rulewerk, passes 16 GB on Claros.
  - LUBM-1K L, whose program has no `owl:sameAs` rules, is the clean
    candidate. It waits for the owner's approval of the large download and
    for phase 2's memory work.

  The equality rules are a property of each program, recorded per row.
  Results files are schema 7.
- **07.** `schema` on by default costs time: Brick's reasoning goes from 7
  ms to 12 ms in a smoke run, past ADR 0005's 5% rule, so ticket 15 will
  likely turn it off by default. F1 widens to every reflexive triple that
  OWL-RL's scm-sco, scm-spo, scm-eqc2 and scm-eqp2 skip. HermiT judges only
  subsumptions between terms the input declares, as OWL 2 DL requires for
  the OWL API to keep the axiom. `WebOnt-I5.8-006`, `-008` and `-009` need
  datatype subsumptions that no OWL 2 RL rule derives. The restriction
  rules in `schema.rls` read helper relations, because Nemo took 31 s on
  one input with five atoms of `T`. The scm-sco cost on deep hierarchies
  (n log² n) is the W3C rule's own cost; experiment E2 addresses it. Chaos
  iteration 8181 (an `owl:sameAs owl:equivalentClass` input) spends 16 s in
  the reference evaluator and passes the 30 s watchdog only below a load
  of about 27.
  On landing: W3C OWL 2 RL 99, every case Nemo passes. RDF-MT went from
  37 to 35, and Nemo's from 30 to 28. The two
  `rdfs-domain-and-range-intensionality` cases are RDFS non-entailments
  that scm-dom1 and scm-rng1 entail by design, so the spec's budget now
  says so.
- **13.** The memory cap first read resident memory, which macOS
  compresses under pressure: one smoke run held 69 GB for about 2.5
  minutes while showing 2 GB, until the agent stopped it. The cap now
  reads the physical footprint (compressed and swapped pages included),
  and later runs stopped near 16.4 GB. Oxford's `Claros.ttl` has 84 lines
  with illegal IRI characters; the fetch script percent-encodes them as it
  unpacks, so every engine reads the same repaired file. The results file
  moved to schema 5 here (the `over-memory` status), not in ticket 14. The
  large zips are pinned by the CRC-32 their zips record until their first
  approved download. The smoke runs found the memory blow-up that ticket
  25, added to phase 1, diagnoses. `claros-le` derives 226 M triples
  against the paper's 555 M: `Claros_LE.owl` uses `owl:hasSelf`, which is
  outside OWL 2 RL, so only the L variants can be the same job.
- **Worktrees.** A copied `crates/python/.venv` runs pytest against the
  main checkout's build. A slice runs `uv sync --reinstall-package turbowl`
  in `crates/python` before pytest, and the briefs say so. Then it runs
  `uv run python -m pytest`: the copied `.venv/bin/pytest` names the main
  checkout's Python in its first line, so plain `uv run pytest` still tests
  the wrong build (found by 08).
- **08.** Departures:
  - None of the five W3C "entails" cases that 05's entry assigned to it
    passes. Their conclusions are `owl:differentFrom`, `owl:AllDifferent`
    or complementOf typings, which no OWL 2 RL rule derives, and Nemo and
    OWL-RL fail them too. `WebOnt-I4.6-003` passes instead.
  - eq-ref is stored once a rule could read it, when `owl:sameAs` is the
    subject or object of a stored triple (`owl:sameAs rdfs:domain c`, a
    restriction, chain or key on it, or a term made equal to it). ADR 0003
    and the spec are amended.
  - Files outside its list, each needed: the tests that build `Context`,
    `lists.rs` (visibility, so cls-oo reuses the list reader),
    `datatype/map.rs` (`const fn`s for the dt-type1 facts), the reference
    crate's `tests/reference.rs` (the cls-oo expectation turned around) and
    its `rules/identity.rs`. prp-key there now compares only the pairs that
    share a key value: a chaos iteration in the complete configuration went
    from over 5 minutes to 3.5 s, with the same result.

  Two fixes landed after it, found on review:
  - dt-type1 typed all 35 datatypes of the map, and OWL-RL's correct list
    was recorded as its fault F7. OWL 2 Profiles (Section 4.2.1) lists 32
    for OWL 2 RL: owl:real and owl:rational must not be used in it, and
    rdf:langString came with RDF 1.1. dt-type1 now types the 32, and F7 is
    gone. Table 8 ranges dt-type2 and dt-not-type over the same 32, so
    ticket 10 now also narrows dt-not-type (ticket 09 checked all 35), and
    ticket 26 drops its owl:real fault: there OWL-RL is right.
  - The oracle's reference test went from 74 s to over 600 s. Nemo spent
    64 of 67 s on one generated input in cls-maxc2, a five-atom join it
    ordered badly once eq-ref made every term a used property. The
    cardinality rules now read helper relations, as the schema rules do,
    and the test takes 63 s with the same closures and findings.

  On landing: W3C OWL 2 RL 100, every case Nemo passes; RDF-MT unchanged.
  Brick's complete closure is contained in OWL-RL's with F1 alone.
