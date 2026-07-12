---
id: story-skilltap-plugin-package-validation
kind: story
stage: done
tags: [testing, architecture]
parent: epic-skilltap-plugin-distribution-package
depends_on: [story-skilltap-plugin-package-assets]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Validate the canonical plugin package boundary

Add a compiled workspace test that validates the checked-in publication tree
without mutating native harness state or the active sibling publisher. Reuse
the core complete-skill and frontmatter contracts, while keeping Claude and
Codex JSON validation as separate channel branches.

## Acceptance criteria

- `crates/cli/tests/plugin_package.rs` validates both manifests, both native
  catalogs, the shared complete skill tree, identity/version parity, and
  relative source containment.
- Tests pass for the valid package and fail for isolated fixtures with
  malformed JSON, missing channel-required fields, name/version drift,
  `../` traversal, missing/non-regular `SKILL.md`, invalid frontmatter, and
  symlinked skill entries.
- Supporting files beside `SKILL.md` remain included in complete-tree
  validation; the test never treats `SKILL.md` as the entire artifact.
- The expected version is derived from the workspace release identity rather
  than duplicated in test constants.
- Test fixtures are created below test-support temporary roots and no command
  writes to `$HOME`, Codex/Claude caches, or `../skills`.

## Verification

Run the focused compiled test, then `cargo test --workspace --all-targets` and
`cargo clippy --workspace --all-targets -- -D warnings`. The package test's
failure messages should name the channel and relative path that violated the
contract so release failures are actionable.

## Implementation notes

- Added `crates/cli/tests/plugin_package.rs` with channel-specific manifest and
  marketplace validation, Cargo-version parity checks, source containment,
  complete skill-tree observation, and strict Codex/Claude frontmatter checks.
- Isolated fixture copies cover malformed JSON, missing required metadata,
  identity/version drift, traversal and wrong-root sources, missing or
  non-regular `SKILL.md`, invalid frontmatter, and symlink escapes. Supporting
  files remain part of the validated tree.
- Focused verification passes: `cargo test -p skilltap --test plugin_package
  --offline` and `cargo clippy -p skilltap --test plugin_package --offline --
  -D warnings`.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard substrate review. The isolated package fixtures exercise
valid channel-specific metadata, complete skill trees, malformed documents,
version/name drift, traversal, missing/non-regular `SKILL.md`, invalid
frontmatter, and symlink rejection. Focused and full workspace tests pass;
the validator reports channel-relative boundary failures without touching
native state, home directories, or the sibling publisher.
