# 03: Close-out

Status: done. Blocked by: 01, 02.

- `README.md` (a short Cancellation part in the library and Python
  sections), `CHANGELOG.md` (Unreleased: cancellation), the crate doc
  (`Cancel`, `reason_cancellable`, the explain forms, `profile::cancel_after`).
- The verification record `docs/verification/<date>-cancellation.md`, its raw
  data under `docs/verification/data/`, and the decisions taken without the
  owner.

## Acceptance
- [ ] The bench against `main` on a quiet machine (load under 1): reasoning times and peak memory within `main`'s spread on brick-small1, brick-soda-hall, lubm-1 and lubm-50, default and complete, one thread and all cores. If the machine is not quiet, the run moves to a ticket and the record says so.
- [ ] Chaos with and without `CHAOS_CANCEL=0`, the time difference recorded against the 10% budget.
- [ ] Every suite in AGENTS.md passes on the merged tree.
