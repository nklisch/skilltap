---
id: story-fix-explicit-git-skill-update
kind: story
stage: review
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Preserve Explicit Git Skill Names During Update

## Symptom

Updating a Git-backed skill installed with `--name` can derive the source
directory name instead of the managed resource name and fail with
`inventory_resource_conflict`.

## Root cause

`execute_skill_update` passed `name: None` into the install/update path, so the
source locator was used to derive a new skill identity rather than preserving
the selected inventory resource identity.

## Fix approach

Pass the selected managed skill name through the update request so replacement
uses the existing resource key and destination.

## Regression test

`crates/cli/tests/compiled_binary.rs`
`explicitly_named_git_skill_update_preserves_the_managed_name` installs a
same-source local sibling, installs the Git source under an explicit name,
advances the Git commit, and verifies the named update replaces the complete
tree.

## Implementation notes

- `crates/cli/src/application.rs` preserves the selected managed name during
  updates while keeping direct-install name validation strict.
- Regression coverage passes, including the existing unnamed Git and local
  update suites.
- Full workspace tests and clippy with `-D warnings` pass.
