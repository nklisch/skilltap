---
id: story-fix-drifted-skill-removal
kind: story
stage: done
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Fail Closed on Partial Drifted Skill Removal

## Symptom

Removing a multi-target skill with one drifted target removed the clean target
and desired inventory without an explicit partial-operation acknowledgment.

## Root cause

The skill removal planner accumulated safe operations before reporting drift
and published inventory before execution, allowing a warning-bearing partial
plan to mutate the clean sibling.

## Fix approach

Treat any preflight warning as a blocked foreground removal when no
acknowledgment is available. Return without executing operations or changing
desired inventory.

## Regression test

`crates/cli/tests/compiled_binary.rs`
`skill_remove_blocks_all_targets_when_one_target_is_drifted` verifies both
target trees and inventory remain unchanged.

## Implementation notes

- `crates/cli/src/application.rs` now fails closed before publishing inventory
  or executing any sibling removal when preflight warnings exist without an
  acknowledgment.
- Regression coverage confirms drift leaves every target and desired entry
  intact.
- Full workspace tests and clippy with `-D warnings` pass.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fast-lane substrate review. The drift regression and green full
workspace verification were confirmed; no lens walk was needed for this story.
