---
id: epic-standalone-skill-lifecycle-tree
kind: feature
stage: done
tags: []
parent: epic-standalone-skill-lifecycle
depends_on: [epic-standalone-skill-lifecycle-source]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Validate Complete Skill Trees

Treat a skill as its entire directory, with a required top-level `SKILL.md`.
Validate and fingerprint every included sibling through the bounded no-follow
tree substrate.

## Design

- Reject a file source, missing top-level `SKILL.md`, nested-only `SKILL.md`,
  symlink escape, special file, path collision, or tree over configured limits.
- Preserve relative paths and exact bytes in a deterministic `ManagedArtifact`
  tree; the markdown file is metadata, never the installed payload by itself.
- Compute a stable whole-tree fingerprint and retain frontmatter parse status
  as evidence without rewriting authored content.

## Implementation units

- Add a core skill tree validator/fingerprint service over `ExternalTreeSnapshot`.
- Extend test support with complete-tree fixtures and sibling-byte assertions.
- Cover replacement, symlink, special-file, and deterministic ordering cases.

## Acceptance

Every accepted installed directory contains top-level `SKILL.md` and all
accepted sibling files; repeated fingerprinting is byte-stable.

## Implementation notes

Added `skilltap_core::skill::ValidatedSkillTree`, which requires a regular
top-level `SKILL.md`, rejects symlinks, preserves all sibling file bytes in an
`ArtifactTree`, and computes a deterministic SHA-256 fingerprint over typed
paths and length-delimited contents.

## Review

### Verdict

Approve with comments.

### Findings

- Filesystem observation remains bounded by the existing no-follow tree
  observer; frontmatter compatibility and publication are dependent features.

### Verification

Focused skill-tree tests and strict core clippy pass.
