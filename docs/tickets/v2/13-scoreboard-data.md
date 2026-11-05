# 13: Scoreboard data and the 16 GB cap

Status: todo. Blocked by: 01.

`scripts/fetch-data.sh` fetches or generates the public datasets the research names (the official LUBM generator run on the fetched JRE, UOBM, Claros, ChEMBL, Reactome, a DBpedia subset), pinned by checksum, at sizes where every engine stays under 16 GB. Rerunning the script converges, and it resumes after a crash. They are opt-in scenarios. The bench watches each run's resident memory and stops it by PID at 16 GB, recording `over-memory`.

## Acceptance
- [ ] A test with a small cap: a child that allocates past it is stopped and recorded as `over-memory`.
- [ ] Each new scenario runs TurbOWL once (a smoke run) and prints its fact counts before and after; counts that the papers publish are compared and recorded.
- [ ] Running the fetch twice changes nothing the second time, and a fetch interrupted halfway completes on the next run.
- [ ] The bench README lists each dataset with its source, licence, size and the cap it fits.
