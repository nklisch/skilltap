---
id: epic-expanded-harness-support-project-skill-links-contract
kind: story
stage: implementing
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
