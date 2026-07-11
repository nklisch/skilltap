---
id: epic-standalone-skill-lifecycle-source
kind: feature
stage: done
tags: []
parent: epic-standalone-skill-lifecycle
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Resolve Explicit Skill Sources

Resolve one explicit local directory or Git source into a bounded source
identity. Do not search a repository for skills or infer a name from unrelated
directories.

## Design

- Accept local directories and Git locators already represented by
  `SourceLocator`; optional subdirectory and requested revision are validated
  at the boundary.
- Resolve exactly one directory and require the caller to name it when a
  source contains multiple possible roots.
- Preserve requested ref separately from the resolved commit SHA; a SHA change
  is an update signal, not a new resource identity.
- `--name` is an assertion against frontmatter/name metadata, never an alias.

## Implementation units

- Add a pure `SkillSource`/`ResolvedSkillSource` value in core.
- Add bounded Git resolution through the existing direct-argument command port.
- Cover local, Git, missing subdirectory, invalid ref, and name mismatch cases.

## Acceptance

Resolution is deterministic, explicit, and never recursively scans for skills.

## Implementation notes

Added `skilltap_core::skill_source` with validated explicit local/Git source
requests, optional subdirectories, expected-name assertions, and local-root
resolution. The CLI lifecycle now resolves Git locators into a deterministic
skilltap-managed checkout cache with bounded direct `git clone`/`fetch`, exact
requested-ref verification, detached checkout, and resolved commit tracking in
state before publication. Recursive repository discovery remains absent.

## Review

### Verdict

Approve with comments.

### Findings

- The command lifecycle must supply the direct Git resolver and resolved SHA
  before publishing inventory; this feature intentionally performs no I/O.

### Verification

Focused source tests and strict core clippy pass.
