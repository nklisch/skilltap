---
id: epic-expanded-harness-support-project-skill-links-observation
kind: story
stage: done
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

## Implementation notes

- Execution capability: direct feature-owner implementation, matching the
  caller's sequential continuation posture; observation and source-less
  reconciliation share the existing project lifecycle port rather than adding a
  second mutation path.
- Review weight: standard, caller default; this child stops at `done` and the
  parent owns the independent feature review.
- Files changed: `crates/cli/src/application/project_skills.rs`,
  `crates/cli/src/application/status.rs`,
  `crates/cli/src/application/reconciliation.rs`,
  `crates/core/src/runtime/filesystem/directory_tree.rs`,
  `crates/core/src/skill.rs`, `crates/core/src/skill_compatibility.rs`, and
  `crates/core/src/domain/resource/finding.rs`.
- Tests added/removed: no acceptance fixtures were changed in this checkpoint;
  the existing core and compiled lifecycle suites were rerun. The acceptance
  checkpoint owns the expanded compiled scenarios.
- Simplification: project status now uses one canonical observation model and
  removes the blanket comparison-unavailable result when all compared state is
  project standalone skills; source-less adoption reuses the existing link
  execution port.
- Discrepancies from design: the bounded direct-child listing and validated
  artifact-tree constructor were added to the existing core runtime/skill
  boundaries because status cannot safely enumerate canonical children or
  revalidate an already-loaded tree through CLI-local filesystem code. Finding
  vocabulary is registered in core while CLI output remains derived from the
  same stable codes.
- Adjacent issues parked: none.

## Verification

- `cargo check -p skilltap` — passed.
- `cargo test -p skilltap-core --all-targets` — 363 passed.
- `cargo test -p skilltap --test compiled_binary` — 53 passed.
- `cargo fmt --all` — passed.
