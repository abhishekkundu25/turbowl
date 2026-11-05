# 19: Snapshots

Status: todo. Blocked by: 16, 17.

`Reasoner` gains save and load for a versioned, checksummed binary snapshot. It holds the dictionary, the base, the closure as stored, the canon, the findings and the switches. A version or checksum mismatch is an error. Saving writes a temporary file and renames it, so a crash never leaves half a snapshot. The CLI and Python expose save and load.

## Acceptance
- [ ] A round trip gives the same closure, diagnostics and switches, on the minimal inputs and on Brick.
- [ ] A changed byte, a truncated file or another version is refused with an error.
- [ ] Loading lubm-50's snapshot is at least 10 times faster than parsing and reasoning it, at the close-out.
