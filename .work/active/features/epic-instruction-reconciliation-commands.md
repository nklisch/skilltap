---
id: epic-instruction-reconciliation-commands
kind: feature
stage: done
tags: []
parent: epic-instruction-reconciliation
depends_on: [epic-instruction-reconciliation-repair]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Expose Instruction Commands

Expose deterministic setup/status/repair commands with project/global scope,
bridge mode, exact acknowledgment, and structured health output.

## Acceptance

Status is read-only; setup/repair fail fast on conflicts and emit actionable
next steps in plain and JSON modes.

## Design

- `instructions status` is read-only and scope-aware.
- Setup/repair must use canonical `~/AGENTS.md`, exact project paths, bridge
  mode, lock/revalidation, and recoverable backups before publication.
- Divergence requires an exact acknowledgment; generic confirmation must not
  bypass the repair plan.

## Implementation notes

`instructions status` now probes canonical and bridge paths and reports
managed/missing/divergent/broken health deterministically. Global/project setup
now creates a missing
canonical `AGENTS.md` and missing Codex/Claude bridges through the core
plan/lock/journal path, records instruction resources in inventory/state, and
blocks divergent existing files. `instructions repair --yes` now accepts only
divergent regular bridges, creates a recoverable backup under skilltap-managed
storage before replacement, and journals the repair operation; symlink and
special-file conflicts remain blocked. Project status now reports both
supported Claude locations when `.claude/CLAUDE.md` is present, distinguishing
a nested managed bridge from a duplicate root/nested configuration so repair
does not silently choose one. Setup preserves a supported nested bridge when
the project root bridge is absent and validates its `../AGENTS.md` relative
target or import form.
Repair with explicit acknowledgment now consolidates a duplicate root/nested
project Claude setup to the root bridge, backing up and removing the nested
entry through the same locked journal path.
Consolidation refuses broken directory or special-file entries rather than
recursively deleting them, even with acknowledgment.

## Review (2026-07-11)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: none
**Nits**: status uses one nested-bridge warning code for both nested-only and duplicate states; the structured resource status distinguishes them.

**Notes**: Deep substrate review completed inline in degraded fresh-context
mode because this run intentionally uses no sub-agents. Correctness, tests,
design alignment, filesystem safety, CLI contract, and foundation-doc lenses
were checked. Full workspace clippy and tests pass. The active parent remains
open for its other lifecycle command work.
