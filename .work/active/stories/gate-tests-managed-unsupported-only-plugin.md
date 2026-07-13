---
id: gate-tests-managed-unsupported-only-plugin
kind: story
stage: done
tags: [testing]
parent: null
depends_on: []
release_binding: 3.0.2
gate_origin: tests
created: 2026-07-12
updated: 2026-07-12
---

# Keep unsupported-only managed plugins blocked with acknowledgment

## Priority
Critical

## Spec reference
`epic-real-harness-recovery-native-lifecycle-managed-project-load-contract`:
required unsupported behavior blocks and `--yes` cannot make an empty faithful
load surface installable.

## Required test

Use an isolated project plugin containing only an unsupported hook and a
plugin-root-relative MCP declaration. Both the unacknowledged and `--yes`
install attempts must remain blocked and leave project trees, catalog, config,
and skilltap inventory/state byte-for-byte unchanged.

## Implementation

- Added an isolated project plugin with only an unsupported hook and a
  plugin-root-relative MCP declaration.
- Proved both unacknowledged and `--yes` installs remain attention-required and
  leave project trees, catalog/config, skilltap inventory/state, and Codex
  caches unchanged.
- The regression exposed a real lifecycle defect: desired inventory was
  published before a managed planning error returned. The lifecycle now exits
  read-only when planning produced no operation and an error.

## Verification

- `cargo test -p skilltap --test compiled_binary unsupported_only_managed_project_plugin_stays_blocked_with_acknowledgment -- --exact --nocapture`

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none
**Rejected**: none

**Notes**: Substrate review at effective weight `standard` (caller), escalated to the Deep lane because the regression required a production lifecycle correction. Fresh-context review covered planning/inventory ordering, both acknowledgment branches, filesystem/state immutability, successful managed-install regression risk, CLI output, and release-contract alignment. The exact unsupported-only test and the successful managed-project lifecycle test both pass in a detached clean worktree. No foundation-doc or public-contract drift found.
