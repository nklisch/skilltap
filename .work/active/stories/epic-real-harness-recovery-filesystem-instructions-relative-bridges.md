---
id: epic-real-harness-recovery-filesystem-instructions-relative-bridges
kind: story
stage: done
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-filesystem-instructions
depends_on: []
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Compute and validate canonical instruction bridges

## Scope

Replace fixed-depth link strings with one core bridge specification derived
from the actual canonical and native paths. Project effective link targets into
typed observations so status, plan, setup, repair, and sync share one health
classifier.

## Acceptance

- Arbitrary supported `HOME`/`CODEX_HOME` relationships compute a relative link
  whose effective target is the actual canonical `$HOME/AGENTS.md`.
- Health requires that exact resolved canonical path and an existing regular
  destination; dangling, absolute, escaping, wrong-target, and conflicting
  entries fail closed.
- Default global, root project, and nested Claude symlink/import layouts retain
  documented behavior and repeat as no-ops.
- The known custom-home fixed `../AGENTS.md` bridge is reported unhealthy and
  repairable, never managed.
- Unit and compiled-binary coverage uses only isolated roots.

## Implementation notes

- Execution capability: strongest available; this is a security-sensitive
  filesystem identity change shared by setup, status, plan, repair, and sync.
- Review weight: standard, inherited from the autopilot caller.
- Files changed: `crates/core/src/instructions.rs`,
  `crates/cli/src/application.rs`,
  `crates/cli/src/application/instructions.rs`,
  `crates/cli/src/application/execution.rs`, and
  `crates/cli/tests/instruction_bridges.rs`.
- Tests added: core unit coverage for arbitrary relative computation, lexical
  observation, root escape rejection, and exact live regular-target health;
  compiled-binary coverage for a sibling custom `CODEX_HOME`, the historical
  fixed-depth wrong link, acknowledged symlink repair, and absolute-link
  rejection in an isolated machine.
- Discrepancies from design: the validated `RelativeSymlinkTarget` remains
  exported by the runtime filesystem module for workspace compatibility; the
  single bridge specification and classifier live in core instructions and
  are consumed by every CLI instruction path. Divergent symlinks are
  repairable without backup because removing the link itself cannot follow or
  alter its target; divergent regular files retain recoverable backups.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context substrate review at the project-default `standard` weight, escalated to the deep lane for the filesystem identity and repair boundary. Commit `2757575` computes bridge targets from actual paths, classifies only the exact live regular canonical destination as healthy, rejects absolute/escaping targets, and safely replaces divergent symlinks without following them. Core and isolated compiled bridge coverage passed; the exact second-wave baseline's only full-suite failures belong to the separately bounced diagnostics story.
