# 06: Explanation speed in the complete configuration

Status: todo. Blocked by: none. A follow-up the close-out found; the
owner decides whether and when.

In the complete configuration (every switch on, which an editor does not
use by default), explaining on the editor-size Brick scenarios misses the
budget (median under 1 ms, 99th percentile under 10 ms):

| scenario | median ms | p99 ms | max ms | cache MB |
|---|---|---|---|---|
| brick-small1 | 2.2 | 157 | 3,910 | 2,608 |
| brick-soda-hall | 0.27 | 161 | 999 | 1,402 |

(Load 1.5 to 2.3.) A profile puts the time in level-0 fact joins, the
helper-call bookkeeping of the complete configuration's schema rules
(`schema`, `trivial`), not in equality. A cache of 2.6 GB for a closure of
this size is the sign to start from.

## Acceptance
- [ ] The limiter named, then fixed in `crates/turbowl/src/explain/`.
- [ ] brick-small1 and brick-soda-hall within the budget in the complete configuration, and the cache under 100 MB.
- [ ] A test that holds it (`scaling_explain` or a timed unit test).
- [ ] Default-configuration latencies, chaos time and every proof check unchanged.
