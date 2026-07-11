---
id: epic-safe-update-automation-foreground-recording
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-foreground
depends_on: [epic-safe-update-automation-foreground-acknowledgment]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Record Verified Foreground Updates

Require fresh target observations before advancing installed revisions, preserve
available/source provenance, and record partial or failed results atomically.

Acceptance criteria:

- Successful updates advance installed revision only after agreement.
- Failed or partial updates remain visible without false success state.
- Independent successful resources share one atomic state publication.

## Implementation Notes

- Added `VerifiedUpdate`, `UpdateRecordingError`, and
  `record_verified_updates` to the pure foreground boundary.
- Recording requires every selected resource's expected revision and exact
  target set to agree; installed revision advances only after verification,
  available revision is cleared, and the returned state marks one application
  timestamp atomically.
- Verification: targeted foreground planner/state tests and core clippy passed.

## Review Record

- Inline review: **pass**. Missing, extra, mismatched, or incomplete
  verification cannot advance installed state.
