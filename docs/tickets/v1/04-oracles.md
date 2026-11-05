# 04: The oracles

Status: todo. Blocked by: 01.

`crates/oracle`: a test crate that runs each oracle on an input and returns a closure and diagnostics as sets of `oxrdf` triples: `reasonable` (dev-dependency, pinned crates.io release `0.5.0-a1`; record any difference from commit `fecee2b` that the parity research read), OWL-RL (`oracles/owlrl/`: a `uv` project pinning `owlrl` and `rdflib`, called as a subprocess), Nemo (`oracles/nemo/owl2rl.rls`: the OWL 2 RL rules as a Nemo program, and a pinned `nmo` binary), a DL reasoner (HermiT or Konclude, pinned, for subsumption checks), and the W3C OWL 2 RL test cases and RDF Semantics tests (fetched). `scripts/fetch-data.sh` (Brick, `reasonable`'s example models and building models, the W3C archives into `data/`) and `scripts/fetch-tools.sh` (the oracle binaries into `tools/`), each pinned by version and checksum. The comparison: sets, the documented-difference filters (D1 to D10), a readable diff. Oracles compared with each other on every input (triangulation).

## Acceptance
- [ ] Each oracle runs on the parity checklist's minimal inputs and the results are listed; where oracles disagree with each other, the report says which and why.
- [ ] A missing oracle reports `skipped`, and with `CI=true` fails.
- [ ] The suite compiles against the stub `Reasoner` and runs fully once ticket 10 lands.
