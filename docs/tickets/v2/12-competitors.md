# 12: Competitors: VLog, Soufflé, Nemo (GraphDB if licensed)

Status: done. Blocked by: 03, 13.

Each competitor is fetched and pinned into `tools/` by version and checksum, with no system install. `docs/research/competitors-2026-11.md` §2 to §6 says how each can run on this machine:

- **VLog** through Rulewerk's published jars, whose VLog libraries are universal (x86_64 and arm64), on the fetched JRE. VLog has no prebuilt arm64 command line, and its source build does not pass on CMake 4.
- **Soufflé** 2.5 from the Homebrew bottle, unpacked into `tools/`. It is single-threaded on Apple silicon: upstream turns OpenMP off there because of crash issue #2476.
- **Nemo**, already fetched.
- **GraphDB Free** only if the owner obtains a licence, which comes by e-mail. It is capped at one core, and its numbers are not published without Graphwise's written consent.

RDFox and Stardog are not run (§1, §5). Jena's OWL rules predate OWL 2 RL, so it does not enter the same-rules race.

Each competitor runs the composed program for a configuration as a subprocess. Its closure is checked against TurbOWL's, and its times and peak memory are recorded as Nemo's are. A competitor that is not fetched is `skipped`.

Three fixes to the bench, found by ticket 13:
- The bench converts each scenario's inputs once (RDF/XML, Turtle) to N-Triples, and to tab-separated terms for Soufflé, with Oxigraph, and caches them, so that Nemo and the competitors run the dataset scenarios. Today Nemo reads only `.nt`, `.ttl` and `.n3`.
- Each run reports its peak physical footprint, as the memory cap reads it, not `ru_maxrss`, which reads low under memory pressure.
- If ticket 25 finds that the bench's closure check holds the expanded closure in memory, the check counts without holding it. An engine gets all-cores rows only where it can run in parallel on this machine; none of the competitors can today.

## Acceptance
- [x] `--list` shows each engine, its pinned version, and whether it is fetched.
- [x] On brick-small1 and lubm-1, every fetched competitor's closure count equals TurbOWL's, in the default and the complete configuration.
- [x] Nemo runs `claros-l` and `uobm-010-l` (converted inputs) and its closure count equals TurbOWL's.
- [x] Unit tests for each adapter's output parsing, and for its error and timeout paths.
- [x] The bench README says, for each competitor, its version, how it runs, what its time includes, and why it has no all-cores row.
