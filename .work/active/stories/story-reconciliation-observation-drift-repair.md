---
id: story-reconciliation-observation-drift-repair
kind: story
stage: implementing
tags: [correctness, testing]
parent: null
depends_on: []
release_binding: 3.0.0
created: 2026-07-12
updated: 2026-07-12
---

# Reconcile fresh native drift instead of trusting stale journal state

## Finding

The final completion review found that `execute_reconciliation` observes fresh
native state but does not compare it against desired inventory and recorded
state before delegating lifecycle work. `plan` emits generic preview operations,
and sync lifecycle adapters can suppress operations through a prior successful
journal entry even after an external removal or corruption.

## Required behavior

- `plan` compares desired inventory, recorded state, and fresh native
  observations to classify no-op, drift, missing, or repair operations.
- `sync` repairs an externally removed or drifted managed resource when the
  fresh observation proves the previous journal entry is stale.
- A genuinely healthy repeated sync remains a no-op.
- Add isolated compiled coverage for external removal/drift followed by plan,
  sync repair, and an immediate no-change repeat.

## Implementation notes

This is a final-review finding promoted directly to an implementation story;
the foundation synchronization contract remains authoritative.
