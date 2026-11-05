# 14: Published results and the scoreboard report

Status: done. Blocked by: 12, 13.

A data file in the bench records RDFox's published results, from `docs/research/competitors-2026-11.md` §1.2. For each: the paper, RDFox build, hardware, threads, dataset, program variant, fact counts before and after, time and memory, and the bridge engines' numbers from the same table.

**When an estimate is made.** Only when all of these hold:
- The bridge ran here on the same dataset.
- TurbOWL did the same job. It loads the paper's OWL 2 RL ontology with the data, in the configuration that matches the variant: L with `equality` off, LE with `equality` on.
- Its count of facts about individuals equals the paper's "after" count, or the difference is explained.

The first bridges are VLog on Claros L (VLog AAAI 2016, single-threaded, and CIKM 2019), then LUBM-1K L and DBpedia L once they fit under 16 GB. The ChaseBench LUBM numbers next to RDFox 7.1a (VFLog AAAI 2025) run a different program, so they are recorded as context, not as an estimate.

`--scoreboard` prints, per scenario and axis, each engine's value and the leader, in both configurations and both thread modes. The estimates get their own section, each labelled with its source and bridge. The results file gains what the report needs, as schema 5.

## Acceptance
- [x] Tests: an estimate is refused when the fact counts differ, the variant does not match, or the bridge did not run, and computed correctly when all three hold.
- [x] Tests for the report: the leader per axis, ties, missing engines, and the estimate section.
- [x] `--compare` reads schemas 1 to 5.
