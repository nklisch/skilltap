---
id: epic-standalone-skill-lifecycle-commands
kind: feature
stage: done
tags: []
parent: epic-standalone-skill-lifecycle
depends_on: [epic-standalone-skill-lifecycle-storage, epic-standalone-skill-lifecycle-compatibility]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Expose Skill Lifecycle Commands

Compose explicit skill install/list/remove/update commands with exact scopes,
target projections, compatibility gates, and Git SHA update tracking.

## Design

- Install and update resolve a fresh tree, compare resolved SHA and whole-tree
  fingerprint, then plan before mutation.
- Pins suppress automatic update but never hide drift; foreground operations
  can override only with an explicit operation-scoped acknowledgment.
- Remove requires skilltap ownership and leaves unmanaged or drifted content
  untouched unless the plan is explicitly accepted.
- List reports desired/installed state only; it never searches sources or
  marketplace contents.

## Acceptance

All lifecycle commands are non-interactive, deterministic in plain/JSON mode,
and an immediate repeat is a no-op.

## Implementation notes

Added the pure SHA/fingerprint lifecycle decision model and exposed an
inventory-backed `skill list` command. Explicit local-directory install now
validates the complete tree, checks target frontmatter compatibility, publishes
the canonical `.agents/skills/<name>` tree through the core lock/plan/journal
path, and is idempotent. Local `skill remove` now verifies the skilltap
ownership record and current complete-tree fingerprint, removes only an exact
owned tree through descriptor-checked deletion, and drops the desired inventory
entry. Git-backed install now uses the bounded direct Git resolver, supports
requested refs and subdirectories, and records the resolved commit SHA in
state. Explicit `--yes` replacement and `skill update <name>` now back up the
complete prior tree into skilltap-managed storage, replace it through the
descriptor-checked directory boundary, refresh the recorded fingerprint, and
remain idempotent. A changed installed tree is treated as local drift and is
never overwritten by `--yes`; only an explicit update may replace an intact
managed tree. Named and unnamed `skill update` now plan each selected managed
source independently, preserving exact project/global scope and continuing
past unrelated source gaps. Git metadata is excluded from the complete skill
tree before fingerprinting, so refreshing an unchanged commit remains
idempotent. Operation-scoped consequence flags and pin enforcement remain open.
An explicit Git commit supplied through `--ref` is recorded as a pinned desired
resource, so update-all does not reinterpret it as a tracking branch.
Explicit Git subdirectories are persisted in the source record and reused by
named and unnamed updates. The `.agents/skills/<name>` canonical tree is now
created even for Claude-only targeting, with the Claude-native complete-tree
projection managed alongside it and removed atomically on skill removal.
Missing managed destinations are re-published even when an earlier apply record
exists, so status drift can be repaired by repeating the explicit install.

The standalone lifecycle is ready for review; partial-operation selectors remain
owned by the downstream cross-harness materialization epic rather than this
faithful standalone path.

## Review (2026-07-11)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: none
**Nits**: native compatibility projections beyond complete skill directories remain downstream materialization work.

**Notes**: Deep substrate review completed inline in degraded fresh-context
mode because this run intentionally uses no sub-agents. The review checked
whole-directory integrity, Git ref/SHA and subdirectory persistence, canonical
`.agents/skills` publication, Claude projection/removal, ownership and drift
safety, scope/target selection, idempotency, and CLI output. Full workspace
clippy and tests pass.

## Review

### Verdict

Interim approve with comments; the feature remains implementing until all
source and lifecycle verbs are wired.

### Findings

- Native projections and mutating install/update/remove paths are still
  required before the standalone-skill epic can be closed.

### Verification

Full workspace tests, compiled CLI contracts, and strict clippy pass.
