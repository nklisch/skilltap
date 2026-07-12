---
id: story-fix-target-aware-skill-lifecycle
kind: story
stage: review
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Preserve unselected harnesses during targeted skill lifecycle operations

## Symptom

Targeting one harness while removing a skill removes the inventory entry for
all harnesses. Targeting one harness while updating a skill either conflicts
with the existing multi-target resource or loses the other harness's native
provenance.

## Root cause

The CLI treats the selected target set as the complete desired resource and
removes the complete resource key after executing a target-scoped operation.
State seeds are also rebuilt from only the selected adapter's native IDs.

## Fix approach

Treat targeted operations as projections: preserve unselected targets and their
native IDs, replace the desired resource with the remaining target set, and
remove it only when no targets remain.

## Regression test

`crates/cli/tests/compiled_binary.rs` covers targeted remove and update across
Codex and Claude.

## Implementation notes

- Added target projection helpers and core resource replacement APIs.
- Skill and native lifecycle operations now preserve unselected desired targets
  and native IDs, and only publish removals after successful operations.
- Added compiled-binary regressions for targeted skill remove/update and native
  marketplace removal.
