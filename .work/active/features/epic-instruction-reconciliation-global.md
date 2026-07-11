---
id: epic-instruction-reconciliation-global
kind: feature
stage: done
tags: []
parent: epic-instruction-reconciliation
depends_on: [epic-instruction-reconciliation-model]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Manage Global Canonical Instructions

Use `~/AGENTS.md` as canonical global instructions and create faithful Codex
and Claude bridges only at documented native locations.

## Acceptance

Missing/global bridge states are observable and setup is non-interactive,
scope-exact, and idempotent.

## Implementation notes

Added canonical global/project path derivation in `skilltap_core::instructions`
for `~/AGENTS.md`, the configured Codex home bridge, and Claude's global
`CLAUDE.md`. The canonical home remains independent of `CODEX_HOME`.

## Review

### Verdict

Approve with comments.

### Findings

- Filesystem probing and actual symlink/import publication remain in the
  repair and command features.

### Verification

Instruction path and scope tests pass under strict clippy.
