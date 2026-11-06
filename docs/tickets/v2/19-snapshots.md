# 19: Snapshots

Status: done on 2026-11-06 (`628edad`); the load budget is measured at the
phase close-out. Blocked by: 16, 17 (done).

`Reasoner` gains save and load for a versioned, checksummed binary snapshot. It holds the dictionary, the base, the closure as stored, the canon, the findings and the switches. A version or checksum mismatch is an error. Saving writes a temporary file and renames it, so a crash never leaves half a snapshot. The CLI and Python expose save and load.

## Acceptance
- [x] A round trip gives the same closure, diagnostics and switches, on the minimal inputs and on Brick (`crates/oracle/tests/snapshot.rs`: 206 minimal-case runs in both configurations, all 17 finding codes seen), and later changes match a reasoner that was never saved.
- [x] A changed byte, a truncated file or another version is refused with an error: every single-byte change and truncation of a small snapshot, and about 76,000 crafted changes with the checksum fixed up, all refused without a panic.
- [ ] Loading lubm-50's snapshot is at least 10 times faster than parsing and reasoning it, at the close-out: 9.2 times on all cores (0.10 against 0.92 s, CLI wall time), a gap; parsing and reasoning got faster after this ticket measured 15 times. At load 3.5: 0.109 s against 1.69 s, about 15 times.
