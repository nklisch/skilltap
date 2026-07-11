---
id: epic-rust-control-plane-storage-managed-artifacts
kind: story
stage: implementing
tags: [infra]
parent: epic-rust-control-plane-storage
depends_on: [epic-rust-control-plane-storage-schemas]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Publish Managed Artifact Trees

Implement immutable, owner-bound complete-directory publication and recoverable
backup storage beneath the resolved `managed/` root.

## Acceptance criteria

- `ArtifactTree` deterministically owns every file in the directory and rejects
  empty, duplicate, absolute, traversal, or non-normal paths; nested bytes
  round-trip exactly.
- Publication derives a unique owner/fingerprint path under `managed/`, writes
  and syncs every file before success, never overwrites, and returns exact owned
  residual context on cleanup failure.
- Live/dangling links at the root or any owned ancestor are rejected without
  following; paths cannot escape through spelling or races supported by the
  runtime boundary.
- Load and remove require matching owner/path; removal never touches an
  unowned/replaced inode. Backup paths are generated uniquely and never replace.
- A representative standalone skill preserves its top-level `SKILL.md` and all
  sibling content as one tree; storage adds no skill discovery/validation.
- Repeated publish reports existing immutable content rather than rewriting;
  full locked verification passes.
