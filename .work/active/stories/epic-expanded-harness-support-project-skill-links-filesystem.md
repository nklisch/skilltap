---
id: epic-expanded-harness-support-project-skill-links-filesystem
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-project-skill-links
depends_on:
  - epic-expanded-harness-support-project-skill-links-contract
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: operator-request-2026-07-14
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Add a Confined Project Symlink Boundary

## Checkpoint

Implement Unit 2 from the parent feature: descriptor-relative, no-follow
inspection, creation, and identity-checked removal of relative symlinks beneath
a canonical project root. Extend the existing skill adapter port with
adapter-owned compatibility analysis.

This checkpoint supplies the safe primitive only. It does not decide desired
ownership, write inventory/state, or orchestrate a lifecycle.

## Units

- Extend `ConfinedFileSystem` in
  `crates/core/src/runtime/filesystem/directory_tree.rs` with the exact
  `inspect_entry_beneath_no_follow`,
  `create_relative_symlink_beneath_no_follow`, and
  `remove_relative_symlink_beneath_no_follow` signatures from the parent.
- Add `LinkIdentity` and `ConfinedEntryObservation` to the runtime exports.
- Implement Unix operations through the existing descriptor-relative
  `tree_io.rs`/`unix_support.rs` boundary using `O_NOFOLLOW`,
  `AT_SYMLINK_NOFOLLOW`, bounded `readlinkat`, `symlinkat`, `unlinkat`, and
  parent durability.
- Extend `SkillProjectionPort` in `crates/harnesses/src/registry.rs` with the
  conservative default compatibility method. Codex/Claude keep their existing
  path implementations and inherit that default until stronger evidence is
  attested.

## Safety constraints

- Never follow an ancestor or final symlink while deciding kind or ownership.
- Classify absolute and malformed symlink targets; do not convert them into
  managed relative targets.
- Create only missing directory ancestors beneath the project root; preserve
  existing sibling skills and reject non-directory ancestors.
- Removal requires the exact planning-time inode and lexical target. A stale
  observation cannot remove a replacement.
- `LinkIdentity` is ephemeral and non-serializable.

## Acceptance evidence

- Runtime tests distinguish all final-entry kinds and reject absolute,
  malformed, oversized, escaping, or ancestor-symlink cases.
- Race tests replace a link between observation and removal and prove the
  replacement survives.
- Durability/idempotency tests prove correct create/remove behavior on supported
  Unix platforms.
- Registry tests prove Codex and Claude project roots remain adapter-owned and
  default compatibility is conservative.

## Ordering

Depends on the validation/layout contract because link targets and adapter
compatibility use those types. The lifecycle checkpoint consumes this story's
runtime port and identity evidence.
