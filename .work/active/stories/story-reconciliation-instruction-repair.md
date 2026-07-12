---
id: story-reconciliation-instruction-repair
kind: story
stage: implementing
tags: [correctness, testing]
parent: null
depends_on: []
release_binding: 3.0.0
created: 2026-07-12
updated: 2026-07-12
---

# Reconcile instruction bridge drift through sync

## Finding

The final completion review found that reconciliation selects
`InstructionLocation` resources but the sync branch emits an unconditional
`no_change` operation. The explicit instructions setup/repair adapter already
owns bridge publication, while the synchronization contract includes
instruction repair.

## Required behavior

- `plan` reports instruction bridge repair/no-op based on fresh bridge state.
- `sync` delegates selected instruction resources through the existing setup /
  repair adapter with the same acknowledgment safety rules.
- Drifted or missing managed bridges are repaired; unmanaged or conflicting
  user-authored content remains blocked unless explicitly acknowledged.
- Add isolated compiled coverage for drift, repair, repeat no-change, and
  project/target selection.

## Implementation notes

This is a final-review finding promoted directly to an implementation story;
the foundation instruction-reconciliation contract remains authoritative.
