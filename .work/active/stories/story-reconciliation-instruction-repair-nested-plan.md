---
id: story-reconciliation-instruction-repair-nested-plan
kind: story
stage: done
tags: [correctness, testing]
parent: null
depends_on: [story-reconciliation-instruction-repair]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Reconcile nested-only project instruction bridges accurately

## Finding

`instructions setup --project` intentionally preserves a supported nested
Claude bridge at `<project>/.claude/CLAUDE.md` when the project-root
`CLAUDE.md` is absent. Reconciliation preview currently always inspects the
root path returned by `instruction_locations`, so `plan --project --target
claude` reports `repair` for a healthy nested-only bridge. `sync` then performs
no change, making plan output inaccurate and violating the instruction bridge
contract.

## Required behavior

- Resolve a nested-only project Claude bridge using the same location policy as
  setup/repair before classifying plan state.
- Report `no_change` for a managed nested bridge and `repair`/`blocked` only
  for its actual state.
- Add isolated compiled coverage for nested-only project plan and sync
  idempotence, including both symlink and import modes if practical.

## Review evidence

The existing setup regression `instruction_setup_preserves_existing_nested_project_claude_bridge`
demonstrates the supported state. `execute_instruction_reconciliation_preview`
currently selects only the root project bridge and has no nested fallback.

## Implementation

Implemented in `1f5f1de`:

- Added shared preview location resolution that recognizes a nested-only
  project Claude bridge when the root `CLAUDE.md` is absent, matching setup's
  preservation policy while retaining the stable bridge resource identity.
- Added isolated compiled coverage for both symlink and import modes. The
  coverage verifies setup preserves the nested bridge, plan reports
  `no_change` with the nested path, sync performs no mutation, and repeated
  sync remains idempotent.

Verification: `cargo fmt --all` and the focused compiled test
`reconciliation_plan_and_sync_preserve_nested_project_claude_bridge` passed.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Substrate review at standard weight in a fresh same-harness
context. Reviewed the implementation and its predecessor/review history,
`docs/SPEC.md`, `docs/UX.md`, and the compiled instruction tests. The shared
preview path resolver matches setup's nested-only project Claude policy for
both symlink and import modes; healthy nested bridges render `no_change`,
sync remains a no-op, and repeated sync is idempotent. Target and project
scope boundaries remain explicit. Focused reconciliation tests and the full
workspace test suite passed; no applicable security, breaking-contract, or
foundation-document findings were identified.
