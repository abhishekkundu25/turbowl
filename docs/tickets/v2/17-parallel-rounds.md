# 17: All cores: parallel rounds

Status: todo. Blocked by: 15.

A native-only cargo feature, on by default, runs rounds in parallel. At least two designs are sketched before one is built (per-family output buffers against partitioning by table). Each family of a round writes to its own buffer, each table's step runs independently, and a large recent part splits into chunks. Below a measured delta size, rounds stay sequential. The WebAssembly build has no threads.

## Acceptance
- [ ] Chaos runs each iteration at 1 thread and at all cores and finds the same closure and diagnostics.
- [ ] `scaling_*` tests of a round at 1 and at 12 threads; reasoning on lubm-50 is at least twice as fast on 12 cores at the close-out, and no slower on 1.
- [ ] `scripts/check-wasm.sh` passes without the feature.
