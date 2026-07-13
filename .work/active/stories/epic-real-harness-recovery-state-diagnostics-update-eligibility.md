---
id: epic-real-harness-recovery-state-diagnostics-update-eligibility
kind: story
stage: done
tags: [correctness, testing]
parent: epic-real-harness-recovery-state-diagnostics
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Count only actionable available updates

## Scope

Give update decisions one authoritative actionable-summary predicate and use
it for status counts. Keep blocked and unresolved diagnostics visible while
preventing local instructions and non-resolvable local skills from appearing
as available updates.

## Acceptance

- Unresolved, blocked, disabled, and unchanged candidates contribute zero to
  `available_updates` while retaining their status and reason.
- Resolved changed safe and decision-required candidates are counted.
- Local instructions and local-path skills do not create phantom updates.
- Target-specific revision disagreement yields exact entries and identical
  plain/JSON summary counts.

## Implementation

- Added `UpdateDecision::is_actionable_available` as the single summary
  predicate: only safe and decision-required changed revisions count.
- Status retains blocked resolution, drift, and policy entries and their
  warnings, but no longer inflates `available_updates` for them.
- Disabled, unchanged, local-path, and instruction candidates remain zero-count
  because their decisions are no-update or blocked.

## Verification

- Core coverage exercises every safety class plus unresolved and disabled
  candidates.
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context substrate review at the project-default `standard` weight. Commit `62dadc7` makes safe and decision-required changed revisions the sole authoritative actionable-update predicate; unresolved, disabled, unchanged, policy-blocked, and drift-blocked decisions remain visible without inflating the count. Core update coverage passed.
