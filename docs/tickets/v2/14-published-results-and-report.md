# 14: Published results and the scoreboard report

Status: todo. Blocked by: 12, 13.

A data file in the bench records RDFox's published results with their sources: the paper, RDFox version, hardware, threads, dataset, program variant, fact counts before and after, time and memory. It also records the bridge engines' numbers from the same tables. An estimate is computed only when the dataset's fact counts here match the paper's and the bridge ran here on the same data, and it is labelled an estimate. `--scoreboard` prints, per scenario and axis, each engine's value and the leader, in both configurations and both thread modes, with the estimates in their own section. The results file gains what the report needs, as schema 5.

## Acceptance
- [ ] Tests: an estimate is refused when fact counts differ or the bridge did not run, and computed correctly when both hold.
- [ ] Tests for the report: leaders per axis, ties, missing engines, and the estimate section.
- [ ] `--compare` reads schema 1 to 5.
