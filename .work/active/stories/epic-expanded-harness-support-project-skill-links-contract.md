---
id: epic-expanded-harness-support-project-skill-links-contract
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-project-skill-links
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
  - .research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md
research_origin: operator-request-2026-07-14
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Define Project Skill Validation and Layout Contracts

## Checkpoint

Implement Unit 1 from the parent feature: one strict YAML-backed Agent Skills
validation result, a separate target loadability/compatibility result, and the
pure project layout model that decides whether a target consumes the canonical
path directly or needs a relative per-skill link.

This checkpoint replaces the current line-oriented frontmatter probe. It does
not perform filesystem mutation, CLI rendering, or adapter-specific link
reconciliation.

## Units

- Add workspace `serde_yaml` dependency in `Cargo.toml` and
  `crates/core/Cargo.toml`.
- Replace `crates/core/src/skill_compatibility.rs` with the exact
  `AgentSkillName`, `AgentSkillMetadata`, `AgentSkillValidation`,
  `AgentSkillFormatFinding`, `SkillLoadability`, and revised
  `SkillCompatibility` contracts from the parent design.
- Add `crates/core/src/project_skill.rs` with
  `TargetProjectSkillProjection`, `ProjectSkillLinkSpec`,
  `ProjectSkillLinkHealth`, path derivation, and pure classification/planning
  decisions.
- Export the new public surface from `crates/core/src/lib.rs` without leaking
  parser-native YAML values.

## Contract constraints

- Require exact top-level `SKILL.md` and retain complete-tree/no-internal-link
  validation in `ValidatedSkillTree`.
- Enforce the normative one-to-64 skill name grammar and equality with the
  canonical directory name.
- Validate required and optional portable field types/limits exactly; preserve
  unknown source files and report unknown frontmatter keys as extension
  evidence rather than rewriting them.
- Invalid YAML or absent required metadata is malformed and blocked.
  Parseable nonconformance is never silently promoted to conforming or known
  loadable.
- Derive target paths only from a canonical project root and adapter-provided
  native root. Reject roots outside the project and collapse path equality to
  `Canonical`.

## Acceptance evidence

- Unit tests cover conforming metadata; each material malformed/nonconforming
  class; name/directory mismatch; extension-field preservation; and strict
  conformance versus compatibility/loadability independence.
- Layout tests cover canonical equality, Claude-style and arbitrary future
  descendant roots, nested project depths, lexical target resolution, and
  outside-project rejection.
- No test asserts parser implementation details or unstable error text.

## Ordering

Foundation checkpoint. The filesystem and lifecycle checkpoints consume these
exact types and must not duplicate name, path, conformance, or link-health
rules.

## Implementation notes

- Added bounded `serde_yaml` parsing over the already captured `SKILL.md`
  bytes, with validating `AgentSkillName`, typed portable metadata, extension
  field preservation, and independent strict conformance/loadability results.
- Added `project_skill` layout derivation. It validates native roots beneath
  the project, treats `.agents/skills` as canonical by path equality, and
  computes normalized relative targets for all other descendant roots without
  harness-specific branching.
- Retained the existing non-project compatibility helper as a compatibility
  surface for plugin bootstrap callers, mapping strict metadata to the new
  `CompatibilityClass`/`SkillLoadability` contract. Unterminated frontmatter
  remains explicitly nonconforming but may be conservatively loadable, matching
  the established client-tolerance behavior.
- Updated the existing lifecycle caller and plugin-package fixture to consume
  the separate compatibility/loadability results; no mutation or status
  orchestration was introduced in this checkpoint.

## Verification

- `cargo test -p skilltap-core --all-targets` — 361 passed.
- `cargo test -p skilltap --test compiled_binary skill_install_requires_generic_yes_for_loadable_partial_frontmatter` — passed.
- `git diff --check` — passed.
