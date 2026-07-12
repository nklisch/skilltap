---
id: story-reconciliation-observation-drift-repair
kind: story
stage: review
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

## Implementation notes

- Added a bounded native `plugin list --json` observation adapter for Codex and
  Claude lifecycle resources. Valid list output is classified as present or
  missing; malformed, unsupported, or failed observations remain unknown and
  never invalidate the successful journal optimistically.
- Native lifecycle sync now reuses a prior apply result only when fresh list
  evidence does not prove the managed resource missing. Proven missing
  resources are re-planned and repaired through the existing lock, execution,
  and journal path.
- `plan` exposes `fresh_state` and classifies native lifecycle previews as
  `no_change`, `repair`, or `planned` without mutating state.
- Added an isolated compiled regression covering present → externally removed
  → plan repair classification → sync repair → repeat no-change.
- Verification: `cargo fmt --all`, focused harness and CLI unit tests,
  `cargo clippy -p skilltap-harnesses -p skilltap --all-targets --offline -- -D warnings`,
  and all 39 compiled-binary tests pass.
