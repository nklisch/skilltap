---
id: epic-instruction-reconciliation
kind: epic
stage: drafting
tags: []
parent: null
depends_on: [epic-reconciliation-execution]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Instruction Reconciliation

## Brief

Make `AGENTS.md` shared infrastructure across Codex and Claude Code while
preserving user-authored content. This epic manages canonical global
`~/AGENTS.md`, project-root and explicitly adopted nested instructions, Codex
and Claude native bridges, Claude symlink and import modes, fingerprints,
ownership, and health.

Setup and repair must distinguish missing resources from divergent files,
broken links, effective overrides, and unmanaged content. Conflicts block
mutation until explicitly reconciled, and approved replacements receive
recoverable backups.

## Foundation references

- `docs/VISION.md` — Instructions as Shared Infrastructure
- `docs/SPEC.md` — Instruction Lifecycle
- `docs/ARCH.md` — Instruction Management
- `docs/HARNESS-CONTRACTS.md` — Global Instructions, Codex Instructions, Claude Instructions
- `docs/UX.md` — Instructions

## Anticipated child features

- Instruction location, bridge, ownership, and fingerprint model
- Global canonical setup and native bridges
- Project and explicitly adopted nested instruction handling
- Claude symlink and import modes
- Conflict detection, adoption, and recoverable backups
- Instruction status, setup, and managed repair commands

<!-- The design pass on each child feature will fill in real specifics. -->
