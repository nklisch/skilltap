---
id: epic-safe-update-automation-foreground-acknowledgment
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-foreground
depends_on: [epic-safe-update-automation-foreground-plan]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Gate Foreground Update Acknowledgments

Select foreground operations through existing domain acknowledgment contracts,
reject missing/extra/cross-scope selectors before mutation, and hand the
selected plan to the shared executor.

Acceptance criteria:

- Partial consequences require exact selector acknowledgment.
- Invalid selection causes zero native or filesystem actions.
- Daemon-style empty acknowledgment cannot apply partial work.

## Implementation Notes

- Added exact selector sets to `UpdateCandidate` and implemented
  `select_foreground_updates` over the pure foreground plan.
- Missing, extra, blocked, and decision-required selections fail before any
  executor/native boundary; partial entries are selectable only with exact
  scope-bearing selectors.
- Verification: targeted foreground planner tests and core clippy passed.

## Review Record

- Inline review: **pass**. There is no generic acknowledgment bypass and the
  daemon-compatible empty selector set cannot apply partial work.
