---
id: epic-instruction-reconciliation
kind: epic
stage: done
tags: []
parent: null
depends_on: [epic-reconciliation-execution]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-10
updated: 2026-07-12
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

## Design decisions

- **What wins when canonical and native instruction content diverge?** The
  canonical `AGENTS.md` wins only when the operation includes explicit
  `--yes` acknowledgment. Without acknowledgment the plan blocks. An approved
  replacement backs up the divergent native file before establishing the
  bridge. Effective Codex override content is treated as the divergent native
  source under the same rule.
- **Which Claude project instruction location is managed?** Preserve whichever
  single supported bridge already exists: root `CLAUDE.md` or
  `.claude/CLAUDE.md`. When neither exists, default to root `CLAUDE.md`. When
  both exist, report a warning and block ordinary repair; with explicit
  approval, back up divergent content and consolidate to the root bridge so a
  project ends with only one managed Claude instruction entry point.
- **Does this epic require UI mockups?** No. Instruction health and
  reconciliation decisions are non-interactive CLI and JSON surfaces.

## Anticipated child features

- Instruction location, bridge, ownership, and fingerprint model
- Global canonical setup and native bridges
- Project and explicitly adopted nested instruction handling
- Claude symlink and import modes
- Conflict detection, adoption, and recoverable backups
- Instruction status, setup, and managed repair commands

<!-- The design pass on each child feature will fill in real specifics. -->

## Decomposition

Instruction work is split into canonical location/bridge modeling, global and
project materialization, conflict-safe repair, and CLI composition.

### Child features

1. `epic-instruction-reconciliation-model` — scope-aware instruction
   locations, fingerprints, ownership, bridge modes, and health findings
   — depends on `[]`.
2. `epic-instruction-reconciliation-global` — canonical `~/AGENTS.md` setup
   and Codex/Claude global bridges — depends on
   `[epic-instruction-reconciliation-model]`.
3. `epic-instruction-reconciliation-project` — project-root and explicitly
   adopted nested instruction handling, including one-entry-point detection
   — depends on `[epic-instruction-reconciliation-model]`.
4. `epic-instruction-reconciliation-repair` — canonical-wins conflict plans,
   backups, symlink/import modes, and locked atomic repair — depends on
   `[epic-instruction-reconciliation-global,
   epic-instruction-reconciliation-project]`.
5. `epic-instruction-reconciliation-commands` — setup/status/repair command
   output, exact acknowledgment, and idempotent health checks — depends on
   `[epic-instruction-reconciliation-repair]`.

## Design review

### Verdict

Approved for implementation.

### Notes

Canonical `AGENTS.md` is the source of truth. A divergent native file blocks
ordinary repair; approved replacement creates a recoverable backup first.

## Children complete

All instruction model, global, project, repair, and command features are now
done. The command surface covers canonical setup, bridge health, nested-project
location preservation, duplicate consolidation with backups, and deterministic
plain/JSON outcomes.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Deep aggregate review completed inline in degraded fresh-context
mode because this run intentionally uses no sub-agents. The realized children
cover the epic brief and its foundation decisions, including global canonical
location, project bridge choice, explicit conflict acknowledgment, backup
safety, and idempotent output. Full workspace clippy and tests pass.
