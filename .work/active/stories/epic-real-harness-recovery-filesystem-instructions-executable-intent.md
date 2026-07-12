---
id: epic-real-harness-recovery-filesystem-instructions-executable-intent
kind: story
stage: implementing
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-filesystem-instructions
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Preserve normalized executable intent through skill publication

## Scope

Introduce the typed artifact-file contract and carry its executable intent from
descriptor-relative source observation through skill validation,
fingerprinting, managed backup/equality, private publication, reload, rollback,
and destination drift checks.

## Acceptance

- Source files with any execute bit publish as private owner-executable files;
  non-executable files publish private without execute regardless of path or
  shebang.
- Group/world, write, set-id, sticky, and other special metadata never cross
  the managed boundary.
- Mode-only changes affect fingerprints, update/drift detection, backup, and
  rollback; identical repeats are no-ops.
- Whole-directory global/project installs for Codex and Claude preserve all
  contents and normalized intent inside isolated fixture roots.
- Existing no-follow, identity revalidation, cleanup, and unsupported-entry
  tests remain green.

