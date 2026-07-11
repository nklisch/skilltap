---
id: epic-standalone-skill-lifecycle-commands
kind: feature
stage: implementing
tags: []
parent: epic-standalone-skill-lifecycle
depends_on: [epic-standalone-skill-lifecycle-storage, epic-standalone-skill-lifecycle-compatibility]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
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
path, and is idempotent. Git-backed resolution, safe replacement/update, and
ownership-checked remove remain open.

## Review

### Verdict

Interim approve with comments; the feature remains implementing until all
source and lifecycle verbs are wired.

### Findings

- Native projections and mutating install/update/remove paths are still
  required before the standalone-skill epic can be closed.

### Verification

Full workspace tests, compiled CLI contracts, and strict clippy pass.
