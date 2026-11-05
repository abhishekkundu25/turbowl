# 13: Scoreboard data and the 16 GB cap

Status: todo. Blocked by: 01.

`scripts/fetch-data.sh` fetches the public datasets that are still online (`docs/research/competitors-2026-11.md` §8):

- Oxford's 2014 zips (LUBM-01K, UOBM, Claros, DBpedia), with their L, LE and U programs and the OWL 2 RL ontologies they come with, such as `LUBM_L.owl`.
- The official LUBM generator (UBA 1.7), run on the fetched JRE with seed 0 and index 0, for the LUBM sizes Oxford does not ship.

Each download is pinned by checksum. Rerunning the script converges, and an interrupted fetch completes on the next run. The AIJ 2019 datasets (ChEMBL, Reactome, the UniProt and UOBM programs, the 2017 DBpedia subset) are gone and stay out. Every dataset becomes an opt-in scenario. The bench watches each run's resident memory and stops it by PID at 16 GB, recording `over-memory`. LUBM-01K's L closure, 182.4 M triples, is expected to pass the cap at today's memory use.

## Acceptance
- [ ] A test with a small cap: a child that allocates past it is stopped and recorded as `over-memory`.
- [ ] Each new scenario has a smoke run of TurbOWL in the default configuration, which records its fact counts before and after and its peak memory (or `over-memory`).
- [ ] Running the fetch twice changes nothing the second time, and a fetch interrupted halfway completes on the next run.
- [ ] The bench README lists each dataset with its source, licence (the pages state none for the data), size, and whether it fits the cap.
