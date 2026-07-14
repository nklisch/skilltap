---
id: epic-expanded-harness-support-project-skill-links-observation
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-project-skill-links
depends_on:
  - epic-expanded-harness-support-project-skill-links-contract
  - epic-expanded-harness-support-project-skill-links-lifecycle
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: operator-request-2026-07-14
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Observe and Report Project Skill Health

## Checkpoint

Implement Unit 4 from the parent feature: semantic desired/unmanaged project
skill observation, stable status and JSON distinctions, explicit canonical
adoption candidates, and source-less desired reconciliation from a validated
canonical tree.

This checkpoint consumes the lifecycle; it must not create a second mutation
path or persist read-only snapshots.

## Units

- Implement `ProjectSkillObservation`,
  `CanonicalProjectSkillObservation`,
  `TargetProjectSkillObservation`, and `observe_project_skill` in
  `crates/cli/src/application/project_skills.rs` as designed in the parent.
- Integrate exact desired comparison and canonical direct-child adoption
  candidates in `crates/cli/src/application/status.rs`.
- Register authored project-skill finding codes/summaries in
  `crates/core/src/domain/resource/finding.rs`.
- Update `application/reconciliation.rs` so an explicitly adopted or manually
  desired source-less project skill reconciles only from its present validated
  canonical tree.

## Observation constraints

- Observe desired resources by exact name. Enumerate unmanaged canonical skills
  only as bounded direct children of `.agents/skills`; never search a repository
  or marketplace.
- Inspect native destinations no-follow, resolve relative targets lexically,
  and fingerprint the canonical tree separately.
- Render strict conformance, compatibility, loadability, and projection as
  independent fields. Use stable codes for format invalid, target incompatible,
  link missing/broken/divergent, and unmanaged destination conflict.
- Observation alone never grants ownership. Adoption writes desired inventory
  through the existing adoption path; sync then revalidates and seeds state.
- Noncanonical regular trees are conflicts, not implicit adoption sources.
- Preserve the generic comparison-unavailable warning for resource kinds still
  lacking semantic comparison; remove it only for project standalone skills now
  compared exactly.

## Acceptance evidence

- Status/JSON tests distinguish missing canonical, malformed canonical,
  incompatible target, missing link, broken link, divergent link, unmanaged
  conflict, healthy link, and canonical `not_required`.
- Plain and JSON outcomes derive from the same fields/codes and preserve exit
  class semantics.
- `status` and `adopt` do not mutate project/native/state files.
- Explicit adoption/source-less desired sync creates only selected links from a
  valid canonical tree; disappearance or drift before apply blocks safely.
- Final removal of adopted targets preserves the canonical tree and returns it
  to unmanaged status.

## Ordering

Depends on lifecycle because status and adoption must describe the exact
ownership/repair behavior that execution implements. The acceptance checkpoint
closes both together.
