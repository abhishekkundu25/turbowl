# Tickets: version 2 (the best OWL 2 RL reasoner)

Specs: `docs/specs/v2-1-scoreboard-and-rules.md` (phase 1, tickets 01 to
15), `docs/specs/v2-2-speed.md` (phase 2, 16 to 21),
`docs/specs/v2-3-removals.md` (phase 3, 22 to 24). Decisions: ADR 0005
(switches), ADR 0006 (the scoreboard). Lock files are in **bold**.

| # | Ticket | Status | Blocked by | Files |
|---|---|---|---|---|
| 01 | [Remove `reasonable`](01-remove-reasonable.md) | done | — | `crates/bench/` (`Cargo.toml`, `src/engine.rs`, **`src/main.rs`**, `src/scaling.rs`, `src/compare.rs`, **`README.md`**), **`Cargo.lock`**, `docs/specs/experiments.md`, one doc line in `crates/oracle/src/lib.rs` |
| 02 | [Switches: the seam](02-switches.md) | done | — | `crates/turbowl/`: `Cargo.toml`, `src/lib.rs`, new `src/switch.rs`, `src/reasoner.rs`, **`src/rules/mod.rs`**, **`src/term.rs`**, new `src/rules/checks.rs`, `src/rules/datatype.rs`, `src/rules/property.rs`, `src/rules/property/chain.rs`, new empty `src/rules/{identity,schema,trivial,literal_values}.rs` and `src/rules/class/enumerations.rs`, `src/rules/class.rs`, `src/rules/testing.rs`; `scripts/check-wasm.sh` |
| 03 | [One rule source for every engine](03-rule-source.md) | done | 01, 02 | `oracles/nemo/owl2rl.rls` → `oracles/rules/<group>.rls`, new `crates/oracle/src/programs.rs`, `crates/oracle/src/{nemo,lib}.rs`, `crates/oracle/tests/programs/`, `crates/bench/src/nemo.rs`, `crates/bench/Cargo.toml`, **`Cargo.lock`** |
| 04 | [Verification by configuration](04-verification-by-configuration.md) | done | 03 | `crates/reference/` (`src/evaluate.rs` → `src/rules/<group>.rs`, `src/generate.rs` → `src/generate/<group>.rs`, `src/lib.rs`, `tests/`), `crates/oracle/` (`src/cases.rs` → `src/cases/<group>.rs`, `src/{ours,triangulate,w3c}.rs`, `tests/`) |
| 05 | [Rules: the new checks](05-rules-checks.md) | done | 04 | the `checks` files: `crates/turbowl/src/rules/checks.rs`, `oracles/rules/checks.rls`, and those in `crates/reference/src/{rules,generate}/` and `crates/oracle/src/cases/` |
| 06 | [Rules: identity](06-rules-identity.md) | done | 04 | the `identity` files, as for 05 |
| 07 | [Rules: the schema](07-rules-schema.md) | done | 04 | the `schema` files, as for 05 |
| 08 | [Rules: the trivial group and cls-oo](08-rules-trivial-and-enumerations.md) | done | 04 | the `trivial` files, and for cls-oo `crates/turbowl/src/rules/class/enumerations.rs` and the `core` files; `crates/turbowl/src/reasoner.rs` (eq-ref on read) |
| 09 | [The OWL 2 datatype map](09-datatype-map.md) | done | 04 | `crates/turbowl/src/rules/datatype.rs`, new `crates/turbowl/src/rules/datatype/values.rs`, `crates/reference/src/datatype.rs`, the `datatypes` files of the generator and the minimal inputs |
| 10 | [Literal values and typing](10-literal-values-and-typing.md) | done | 08, 09 | the `literal-values` files; the `trivial` files (dt-type2) |
| 11 | [Switches in the CLI and Python](11-switch-adapters.md) | done | 02 | `crates/cli/`, `crates/python/` |
| 12 | [Competitors: VLog, Soufflé, Nemo (GraphDB if licensed)](12-competitors.md) | done | 03, 13 | `crates/bench/src/engine.rs`, new `crates/bench/src/{souffle,vlog,graphdb}.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`**, `scripts/fetch-tools.sh` |
| 13 | [Scoreboard data and the 16 GB cap](13-scoreboard-data.md) | done | 01 | `scripts/fetch-data.sh`, `crates/bench/src/scenario.rs`, new `crates/bench/src/datasets.rs`, `crates/bench/src/machine.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`** |
| 14 | [Published results and the scoreboard report](14-published-results-and-report.md) | done | 12, 13 | new `crates/bench/src/{published,report}.rs` and the published-results data file, `crates/bench/src/compare.rs`, **`crates/bench/src/main.rs`**, **`crates/bench/README.md`** |
| 25 | [Memory: OWL2Bench RL, UOBM-010 U and OpenCyc](25-memory.md) | done | 13 | `crates/turbowl/` (where the cause is), `crates/turbowl/src/scaling.rs` |
| 26 | [OWL-RL's datatype and identity faults](26-owlrl-faults.md) | done | 08 | `crates/oracle/src/differences.rs`, `crates/oracle/src/cases/datatypes.rs` |
| 15 | [Close-out: phase 1](15-close-out-phase-1.md) | done | 01 to 14, 25, 26, version 1's 14 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `CONTEXT.md`, the phase 1 spec, `docs/verification/`, this README |
| 16 | [Fast loading: parsers and the dictionary](16-parsers-and-dictionary.md) | done; loading met at the close-out on all cores | 15 | `crates/turbowl/src/io.rs` and new `src/io/`, **`crates/turbowl/src/term.rs`**, `crates/turbowl/src/scaling.rs` |
| 17 | [All cores: parallel rounds](17-parallel-rounds.md) | done; the speed-up gap accepted | 15 | `crates/turbowl/src/engine/`, **`crates/turbowl/src/reasoner.rs`**, **`crates/turbowl/src/rules/mod.rs`**, `crates/turbowl/Cargo.toml`, `scripts/check-wasm.sh` |
| 18 | [All cores: parallel parsing](18-parallel-parsing.md) | done; loading met on all cores | 16, 17 | `crates/turbowl/src/io/`, **`crates/turbowl/src/term.rs`** |
| 19 | [Snapshots](19-snapshots.md) | done; 9.2 times on all cores, a gap | 16, 17 | new `crates/turbowl/src/snapshot.rs`, `crates/turbowl/src/lib.rs`, **`crates/turbowl/src/reasoner.rs`**, `crates/cli/`, `crates/python/` |
| 20 | [The improvement loop](20-improvement-loop.md) | done (two gaps accepted) | 17, 18, 19 | each experiment's files; **`docs/specs/experiments.md`** (one row per iteration, written by the orchestrator) |
| 21 | [Close-out: phase 2](21-close-out-phase-2.md) | done (gaps listed) | 16 to 20 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `docs/verification/`, this README |
| 27 | [The full-size scoreboard race](27-full-scoreboard-race.md) | todo, pending the owner | 21 | `docs/verification/` (the scoreboard record) |
| 22 | [Removals: DRed](22-dred.md) | implemented; acceptance open | 21 | `crates/turbowl/src/engine/`, **`crates/turbowl/src/reasoner.rs`**, `crates/turbowl/src/rules/`, `crates/reference/tests/chaos.rs` |
| 23 | [Removals: FBF](23-fbf.md) | implemented; acceptance open | 22 | as for 22 |
| 24 | [Close-out: phase 3 and the claim](24-close-out-phase-3.md) | todo | 22, 23 | `README.md`, `CHANGELOG.md`, `AGENTS.md`, `docs/verification/`, this README |

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
5. **10, 26** (26 ran beside 10 once 08 landed).
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
- **26.** Eight faults, F7 to F14 (the id F7 was free again: ticket 08's
  was withdrawn). Each was checked against the text of XML Schema 1.1, RDF
  Concepts and OWL 2, and against the installed owlrl and rdflib sources,
  before it was listed. Departures:
  - The ticket's cardinality item is a fault only for a double, float or
    boolean cardinality (F14). With dt-eq, the W3C rules too read
    `"1"^^xsd:integer` as the cardinality 1.
  - Top-level CDATA in an `rdf:XMLLiteral` is left out of F11: RDF 2004,
    which OWL 2 cites, requires exclusive canonical XML, where CDATA is
    ill-typed, while RDF 1.1 allows it.
  - Where OWL-RL only derives or reports more (F8, F11, F13, F14, and
    parts of F9 and F10), the fault filters nothing. `CONTEXT.md` now says
    so.
  - `reason.py` now reads OWL-RL's message for an empty lexical form.
  - Only `"a  b"^^xsd:token` returned to the minimal inputs: the
    expectation check allows one known fault per case and check, so each
    other faulted literal has its own fault test.
  - For ticket 10, sent to its agent: OWL-RL skips dt-eq and dt-diff on
    purpose (OWLRL.py:234-237), a fault 10 lists as F15, and the case
    `cls-maxc2-other-numbers` needs a double, not an integer, once dt-eq
    is on.

  On landing: the oracle library has 35 tests (26 before); Brick and the
  W3C counts are unchanged.
- **10.** As built:
  - The datatype map holds only the 32 datatypes of OWL 2 RL, in the core
    and the reference evaluator alike. `owl:real` and `owl:rational`
    literals have no known value, and their lexical readers are deleted;
    a language-tagged literal reads as an `rdf:PlainLiteral`; an untagged
    `rdf:langString` literal is no longer reported. RDF-MT stays at 35:
    `datatypes-intensional-xsd-integer-string-incompatible` passes now,
    and `rdfs-entailment-test002` fails, since it needs `rdf:langString`
    checked. The spec's budget names it.
  - The value rules also range over the two literals the cardinality
    rules name, interned with every dictionary, so `"1"^^xsd:integer`
    fires cls-maxc2 with `literal-values` on. Chaos found that a merge can
    move such a literal's class without a restart; the cardinality rules
    then join every restriction again (`Round::class_moved`, ADR 0003's
    new amendment). Making the two literals win their classes, as
    vocabulary terms do, would remove that path: a follow-up.
  - dt-type2 is stored. dt-eq stores one `owl:sameAs` per literal with
    `equality` on, all k² pairs without it. dt-diff's departure covers its
    eq-rep consequences too.
  - Nemo judges dt-type2, dt-eq and dt-diff through facts the oracle
    computes with the reference evaluator's map (`Codec::literal_facts`);
    the oracle crate now depends on the reference crate. VLog's
    translation gains `?x != ?y`, and the bench's converted input carries
    the same facts. OWL-RL skips dt-eq and dt-diff on purpose: fault F15.
  - The reference evaluator's prp-key went quadratic again on an
    `owl:sameAs owl:sameAs rdf:rest` input and was rewritten with bitsets
    (94 s to 9.7 s, the same result). In the complete configuration a
    chaos iteration costs it about 70 ms, mostly dt-diff's n² unlisted
    triples: 20,000 iterations took 611 to 818 s per shard, so ticket 15's
    100,000 take about an hour per shard.

  On landing, ticket 26's `"a  b"^^xsd:token` left the minimal inputs
  again. The Python tests give the bindings each minimal input as rdflib
  terms, and rdflib rewrites that form (F12), so six of them failed. 26's
  brief had not run pytest. W3C OWL 2 RL 101 (`New-Feature-Keys-006`),
  every case Nemo passes; Brick unchanged; pytest 780.
- **Owner decisions, 2026-11-06.** The W3C OWL 2 RL budget is 101, every
  case the rules can pass, with the other 18 listed. Phase 2's bar is
  raised: at least 5 times on 8 or more cores, at most 50 bytes per
  closure triple, with E2, E8 and E1 first in the loop as before.
  LUBM-1K is fetched now and run after phase 2's memory work. The close-out
  measures at a load of about 1.9 (an animated wallpaper and two idle apps
  kept running), which its record states.
- **15.** Phase 1 closed on 2026-11-06, on a busy machine (load average
  about 2) by the owner's choice
  (`docs/verification/2026-11-06-v2-phase-1.md`). Departures:
  - **The defaults.** ADR 0005's rule turned `schema` and `identity` off
    (`660e3bd`). The research alone would have kept both on: owlrl,
    GraphDB's `owl2-rl` and the OWL-RS Nemo program implement both groups,
    so neither is rarely implemented. The measurement decided. `schema`
    slowed 12 of the 15 scenarios by more than 5%: 22% to 39% on the
    editor-size ones and up to 62% on claros-l. `identity` slowed
    uobm-010-u by 26% and claros-l by 6%. The new default was not measured
    on its own: the closest run has `schema` off and `identity` on, and
    `identity` costs nothing measurable on the editor-size scenarios.
  - **The scoreboard** ran before the defaults were decided, so its
    competitor runs are in the old default (`schema` and `identity` on);
    then the complete configuration at editor size. The owner stopped the
    session after 4 hours ("record what you have measured and move on").
    Missing: the new default against the competitors; the complete
    configuration on claros-l, uobm-010-u, owl2bench-rl-5 and
    lubm-uba-100-l; a quiet machine. No engine runs on all cores yet, and
    no RDFox estimate meets the bridge's conditions. TurbOWL leads every
    scenario and axis where some engine finished: 46 pairs in the old
    default, 22 in the complete configuration.
  - **Two chaos finds**, both on inputs that make a vocabulary term equal
    to `rdf:rest`, both past the 30 s watchdog. Iteration 65821: prp-key
    read a key list whose cells branch, exponentially many paths, for 32 s.
    It now reads such a list by pairs along its paths and keeps the hash
    grouping for chains (`44e411a`, with a unit test and
    `scaling_identity_keys_on_a_branching_list`): 0.16 s. Iteration 27725:
    the reference evaluator's list checks read each alias's pairs anew, for
    29 s; TurbOWL took 18 ms. Each head is now read once (`7a813b4`, with a
    test that pins the findings). The iteration takes 5.4 s.
  - **F1's property test** (`f1_owlrl_scm_spo_skips_reflexive_subproperty`)
    now runs Nemo with `schema` on, since that fault concerns the `schema`
    rules and `schema` is off by default.
  - **The W3C cases.** Of the 18 that fail, 15 expect a conclusion no
    OWL 2 RL rule derives. Three depend on how the runner reads a case:
    `WebOnt-imports-011` needs its imported ontology loaded, and the two
    `-Direct` cases need the annotations dropped from a conclusion under
    the Direct Semantics. The record lists each one.
  - **Open:** VLog's closure on owl2bench-rl-1 differed in one run of
    three (1,445,884 triples against 1,450,868). It needs a reproduction
    and, if it holds, a listed competitor fault. The 16 GB cap overshoots
    by up to 314 MB before it stops a run (16,698 MB, VLog on lubm-10,
    complete).
- **Follow-ups after phase 1.**
  - ADR 0003's choice rule for the two literal constants: make them win
    their classes, as vocabulary terms do, and remove `Round::class_moved`.
  - The reference evaluator's speed in the complete configuration. Its
    prp-key reads each alias of a key list anew: chaos iteration 27725
    still takes 5.4 s of the 30 s watchdog.
  - The oracle crate's `reference` test takes 52 to 100 s of its 120 s
    cap.
  - eq-ref is spread across three places; one would be easier to change.
  - Split `reasoner.rs` (1,336 lines) and `rules/checks.rs` (1,624 lines).
  - Why `schema` costs 35% on LUBM at every size while it adds 99 triples.
    A phase 2 candidate: measure where the time goes before guessing the
    cause.
  - The W3C runner: load a case's imported ontologies from the export
    (`test:importedOntology`), and treat the cases whose only regime is
    the Direct Semantics as the W3C does: run them with annotations dropped
    from the conclusion, or count them as not applicable to the RDF-Based
    rules. A change to an oracle, so it is an iteration of its own
    (`docs/specs/experiments.md`, Rules). It moves every engine's count,
    not only TurbOWL's.
  - VLog's time on the scoreboard is its own report, which leaves out the
    JVM's start, the rulewerk client and the export. Its wall time is 1.1
    to 18 times as long (brick-small1: 187 to 234 ms reported against
    1,659 to 4,311 ms of wall time). Decide whether the scoreboard should
    show wall time for every engine that runs as a process.
- **16, landed (`75fae9f`).** TurbOWL's own N-Triples and Turtle parsers,
  ports of oxttl 0.2.5's lexer and recognizers, with oxiri 0.2.11's IRI
  parser behind a fast scan. N3 and RDF/XML keep Oxigraph's. Departures:
  - The dictionary's layout did not change: it already was one text arena
    with each term's hash stored in its slot. The alternative (a slot that
    points at the text, one read less per lookup) costs 16 to 32 bytes
    more per term, about 35 to 70 MB on lubm-50, against phase 2's memory
    budget, so the speed came from the parser.
  - The ported files keep Oxigraph's MIT notice (`THIRD-PARTY-NOTICES.md`),
    which the slice had left out.
  - The IRI parser is a port only because `oxiri` was not a direct
    dependency and `Cargo.toml` was ticket 17's this wave. `oxiri` is
    already in the build through `oxrdf` and `oxttl`, so depending on it
    directly adds no crate and removes about 600 lines of `io/iri.rs`: do
    it once 17 lands.
  - Loading univ-bench and lubm-50 took 1.36 to 1.42 s at load 2, against
    the Baseline's 3.9 s; the budget is measured at the close-out. About
    0.5 s of it is outside `io`, in `reasoner.rs`: `document_key` hashes
    the whole document twice (FxHash and SipHash), and the base is sorted
    and deduplicated. A follow-up for after 17.
- **17, landed (`9fec6d3`).** Per-family tasks and buffers, joined in a
  fixed order, with each round's output kept by predicate so that each
  table sorts and advances as its own job (the ticket's "Design" section
  compares this with partitioning by table). Departures:
  - It also edits the hot loops of `rules/class.rs`, `class/lists.rs`,
    `class/restrictions.rs`, `datatype.rs` and `property.rs`, which now
    read in chunks: files the table did not list for 17, and no other
    ticket's.
  - One thread got faster too, since no step sorts a whole round any more:
    lubm-50 reasoning 739 to 803 ms before, 423 to 438 ms after, and 103 to
    114 ms on all cores, at load 2 to 4. All cores against one thread is
    3.7 to 4.3 times: the 5-times budget is decided at the close-out, and
    the spec says against what.
  - `PARALLEL_FROM` (65,536 tuples) was measured on a loaded machine
    (`scaling::parallel::timing_parallel_round_threshold`); rerun it on a
    quiet one.
  - On all cores, large merges and deduplications allocate new vectors, so
    a big table's peak can briefly double: a risk for the memory budget,
    not yet measured.
  - Open: whether `Threads::exactly(n)` should be public, and whether the
    CLI, the bench or Python get a threads option (`RAYON_NUM_THREADS`
    works today). `cargo test -p turbowl --no-default-features` does not
    compile (the rule tests use `io`), which predates 17.
- **19, landed (`628edad`).** Snapshot format version 1: a header, nine
  sections and an XXH64 trailer, documented in `snapshot.rs`. lubm-50's
  snapshot is 312 MB, about 30 bytes per closure triple, and loaded in
  0.109 s against 1.69 s to parse and reason, at load 3.5. Departures:
  - Its `impl Dictionary` block sits just before `term.rs`'s test module,
    not at the end of the file, because clippy's `items_after_test_module`
    forbids that. It reads the dictionary's private internals, which
    ticket 18 is editing in the same wave; 18 was told what they are.
  - **pytest in the worktrees tested the main checkout.** The `.venv` that
    `slice.sh new` clones keeps console scripts that point at the main
    checkout's environment, so `uv run pytest` in a worktree imported the
    main checkout's package. Ticket 19 found it. Every pytest run on a
    merged tree, after each landing, ran in the main checkout and tested
    the landed code; the slices' own pytest results did not. From now on a
    new worktree runs `uv sync --reinstall` in `crates/python` first.
  - `Error` gained a variant and is not `#[non_exhaustive]`: worth deciding
    before anything is published.
  - Open: a snapshot from the same TurbOWL version but other rules (during
    development) is not detected; a rules fingerprint would close it. The
    base takes 79 MB of the 312, mostly a copy of stored pairs. A crash
    mid-save can leave a `.NAME.PID-N.tmp` file beside the snapshot.
- **18, landed (`8cee6d4`).** Documents of 384 KiB or more are cut at line
  starts and parsed in pieces, each into its own term table, then merged in
  order; a piece that started in the wrong state is parsed again.
  Departures:
  - **Turtle is split too**, at statement ends, since the bench's lubm-50
    is a Turtle file: N-Triples alone would not have moved the budget.
  - `oxiri` became a direct dependency (it was already in the build), and
    about 750 lines of ported IRI code went. The sequential parse got 2 to
    4% slower with it, bisected to that change alone; code layout is the
    suspect.
  - A reasoner set to one thread still parsed on every core:
    `reasoner.rs`, ticket 19's file, called `io::parse`. The orchestrator
    made it call `io::parse_on` with the reasoner's own workers.
  - The merge is now the bound: interning about 2.2 M terms on one thread
    takes 110 to 150 ms while the workers parse. A sharded dictionary
    would let it run in parallel: a loop candidate.
  - Open: a parallel parse briefly holds every piece's terms and triples
    (about 80 MB on lubm-50); a cut inside a long multi-line Turtle string
    can cost up to one more sequential parse (cap the overrun); a
    `Workers::every_round()` getter would be cleaner than detecting it
    with `engage(0)`.
- **Owner decision, 2026-11-06: the memory budget measures the process's
  footprint**, on one thread and on all cores, as the bench records it for
  every engine. Iteration 6's probe had shown that on all cores the
  reasoner never holds more than about 461 MB at once (under the 517.3 MB
  budget) while the footprint reaches about 602 MB, mostly memory the
  allocator keeps after the run frees it. With this answer the all-cores
  budget stays open; the loop works on what the run allocates.
- **Loop process, from iteration 4:** two candidates that touch disjoint
  files may be built in parallel, but each is evaluated and decided alone,
  against the version kept before it. From iteration 6 the all-cores peak
  is judged on 10 alternating single runs per side, by medians and middle
  halves: the bench's largest of 3 runs is noise on all cores.

- **Owner decision, 2026-11-07: the dictionary may drop its lookup table
  during a large full run** and rebuild it at the next change (about 30 to
  60 ms once on a lubm-50-sized input, microseconds on editor-sized ones),
  to bring the all-cores footprint under its budget (iteration 8's
  candidate 2, run as iteration 9).
- **Owner decision, 2026-11-07: the all-cores memory gap is accepted.**
  After loop iterations 1 to 9, lubm-50 on all cores peaks at about 547 MB
  in the largest run against 517.3 MB (5.8% over); one thread meets the
  budget. Closing it needed a large structural change (sharing the base
  with the tables, or splitting IRIs), each a few hours at medium to high
  risk. Phase 2's remaining effort goes to the all-cores speed-up and the
  close-out; the two candidates stay open in the loop's list.
- **Owner decision, 2026-11-07: the all-cores speed-up gap is accepted** at
  about 4.1 to 4.2 times against 5. Iteration 11 showed round 1 nearly
  saturates the 12-core pool (6 Efficiency cores), with about four times the
  CPU of one thread for the same work.
- **20, done.** Loop iterations 1 to 11 on phase 2's budgets: kept P0, the
  windowed read, relations sized to their contents, dictionary slots to three
  quarters, the full run from exact batches, the object index in place, and
  the base by predicate; reverted the one-vector merge, the dictionary's
  dropped lookup table and the first index in its job; counted and did not
  build one split per property rule. lubm-50: reasoning 735 ms (version 1) to
  about 387 ms on one thread and 93 ms on all cores; peak memory 1,483 MB to
  about 482 MB on one thread and 544 MB on all cores.
- **Owner decision, 2026-11-07: phase 2's close-out runs a short race**, the
  editor-size scenarios only, without GLog or a multi-core Soufflé. The
  full-size race is ticket 27, pending.

- **Owner decision, 2026-11-07: a competitor run stops once it passes 3 times
  TurbOWL's time** on the same scenario, recorded as more than 3 times
  slower, instead of waiting for the 600 s timeout. Applied by hand in phase
  2's short race (VLog on lubm-50 with every rule on, stopped after about
  9 minutes against TurbOWL's 2.1 s); the bench should gain it as an option
  before ticket 27's full race.
- **21.** Phase 2 closed on 2026-11-07, at commit `1faa436` (the code of
  `b654de6`), on a busy machine (load average 3 to 5)
  (`docs/verification/2026-11-07-v2-phase-2.md`). Correctness ran in full:
  every suite, the slow tier, and 100,000 chaos iterations of each kind,
  every iteration also comparing one thread with all cores. Departures:
  - **The short race**, at the owner's request: the editor-size scenarios
    only, against Nemo, Soufflé and VLog, in both configurations, TurbOWL
    on one thread and on all cores. The data-size scenarios ran with
    TurbOWL alone, one run each. The full-size race is ticket 27.
  - **The 3-times stop rule**, the owner's, applied by hand once: VLog on
    lubm-50 in the complete configuration, stopped after about 9 minutes.
    The bench does not stop a run itself yet.
  - **The snapshot gap**, new: lubm-50's snapshot loads 9.2 times as fast
    as parsing and reasoning on all cores (0.10 against 0.92 s), against
    10, and about 19 times on one thread. Ticket 19 measured 15 times;
    parsing and reasoning got faster since. The snapshot is now
    285 MB, against ticket 19's 312 MB at format version 1. It waits for
    the owner, as does one-thread loading (1.52 s against 1.31 s).

- **22 and 23, 2026-11-09.** Incremental removals implemented on
  `feat/incremental-deletions`. The shared rule parser moved from the
  explainer into `rules/`; maintenance records no explanation provenance.
  Indexed dependent queries and survivor checks replaced the first native
  DRed design for closures with no equality classes. Bounded backward
  proofs protect supported facts. Snapshot format 3 preserves pending
  deletions. Public regressions cover lifecycle behavior and the two
  reduced correctness failures. See
  `docs/verification/2026-11-09-removals.md` for checks and measurements.
  Departures and open acceptance:
  - `trivial` and `literal-values` configurations use full reasoning.
  - Nonempty findings are recomputed globally. Empty findings need no
    work on the positive, empty-canon path.
  - Batch, affected-set and query limits are bounded heuristics; their
    crossover has not been tuned on a workload sweep.
  - The Brick 10% budget is not met. The lubm-50 deletion budget, constant
    scaling, 100,000 iterations per chaos shard, and the removal scoreboard
    close-out remain open. The exhaustive per-family alternate-proof and
    cascade matrix also remains open. Phase 3 is not closed.
