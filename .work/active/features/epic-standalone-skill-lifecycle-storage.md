---
id: epic-standalone-skill-lifecycle-storage
kind: feature
stage: done
tags: []
parent: epic-standalone-skill-lifecycle
depends_on: [epic-standalone-skill-lifecycle-tree]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Publish Canonical Skill Trees

Install complete compatible skill directories into the canonical managed
`.agents/skills/<name>/` representation at global or project scope.

## Design

- Global scope resolves beneath the user home; project scope resolves beneath
  the selected project root. The canonical tree is immutable and owned by
  skilltap.
- Publish through the existing managed-artifact repository with no-clobber
  backups, owner identity checks, atomic publication, and repeat no-op behavior.
- Record source, requested ref, resolved SHA, whole-tree fingerprint, and
  provenance in inventory/state without secrets.
- Native harness links/copies are adapter projections, never alternate sources
  of truth.

## Acceptance

Install, repeat install, drift detection, and removal are scope-exact and do
not overwrite unmanaged directories.

## Implementation notes

The existing immutable managed-artifact repository already supplies the
required whole-tree publish, no-clobber, owner-bound load/remove, backup, and
atomic filesystem behavior. Standalone skill storage now has a validated
`ArtifactTree`/fingerprint input and the `DirectSkill` managed role; command
composition remains the dependent lifecycle feature.

## Review

### Verdict

Approve with comments.

### Findings

- Command composition must map the scope-specific canonical `.agents/skills`
  projection to the managed artifact record and preserve source SHA metadata.

### Verification

Existing managed-artifact lifecycle/security tests plus whole-skill tree tests
pass under strict clippy.
