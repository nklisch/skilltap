---
id: epic-expanded-harness-support-project-skill-links-lifecycle
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-project-skill-links
depends_on:
  - epic-expanded-harness-support-project-skill-links-contract
  - epic-expanded-harness-support-project-skill-links-filesystem
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: operator-request-2026-07-14
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Reconcile Canonical Project Skills and Target Links

## Checkpoint

Implement Unit 3 from the parent feature: route project-scoped standalone skill
install, update, remove, plan, and sync through one dependency-ordered lifecycle
that publishes a single canonical complete tree and reconciles registry-derived
per-target relative links.

Global skill behavior remains unchanged. This checkpoint owns the mutation and
state semantics; it does not own final status/adoption rendering.

## Units

- Add `crates/cli/src/application/project_skills.rs` and the exact
  `ProjectSkillPlan`/planning context from the parent design.
- Add `ProjectSkillLinkEntry`, `ProjectSkillLinkAction`,
  `ProjectSkillLinkPort`, and `ProjectSkillLifecyclePort` to
  `crates/cli/src/application/execution.rs`.
- Route project branches in `application/lifecycle.rs` and
  `application/reconciliation.rs` through the new service; retain current
  global `ManagedSkillPort` behavior.
- Reuse `faithful_file_operation_with_dependencies`, `Plan`,
  `StateExecutionJournal`, `refresh_resource_state`, and target-local state.
  Do not introduce a second operation graph or state manifest.

## Lifecycle constraints

- Validate/snapshot canonical content once and gather each selected adapter's
  compatibility before any mutation.
- Publish or replace canonical content before dependent link operations.
- Collapse equal/duplicate native roots without losing target-local state
  bindings.
- Repair only desired, skilltap-owned relative links; preserve absolute,
  untracked, file, directory, and special-entry conflicts.
- Block canonical-byte repair/update unless selected targets cover every desired
  target for the project resource. Link-only plans remain target-selectable.
- Union an explicitly installed target into an existing desired resource only
  when canonical content matches. Source changes still require `skill update`.
- Remove selected links first. Remove a skilltap-owned canonical tree only after
  the last desired target is removed and every required link removal succeeds.
  Preserve adopted canonical trees.
- Bind every request to one operation id, revalidate link/tree identities under
  the configuration lock, journal pending/terminal outcomes, and restore only
  captured owned link representations on replacement failure.

## Acceptance evidence

- Planner/port tests prove canonical-before-link and reverse removal order,
  dependency skipping, no redundant canonical projection, duplicate-root
  collapse, target-local sibling preservation, partial-target content-update
  blocking, conflict preservation, rollback residual reporting, and immediate
  repeat no-op.
- Project install produces one canonical tree and one link per distinct
  noncanonical root; no duplicate complete target tree remains.
- Targeted remove preserves canonical content while another desired target
  remains; final direct remove clears owned links then canonical.
- Plan/apply races reject stale tree or link identity without deleting a
  replacement.

## Ordering

Consumes the contract and filesystem checkpoints. The observation checkpoint
uses its planning/state semantics; the acceptance checkpoint verifies the
integrated lifecycle.

## Implementation notes

- Added `application/project_skills.rs` as the project-only lifecycle planner.
  It validates the canonical source once, asks each selected registry adapter
  for compatibility and destination evidence, publishes one `.agents/skills`
  tree, and derives links from the pure project layout contract.
- Added a composite execution port that binds canonical tree operations and
  no-follow link operations to one dependency graph and configuration lock.
  Canonical publication precedes links; final canonical removal depends on all
  selected link removals.
- Link repair is limited to desired skilltap-owned relative links. Absolute,
  regular, directory, special, missing-ownership, and divergent unmanaged
  destinations remain preserved conflicts. Replacement rollback restores only
  the captured relative target when the pathname is still absent.
- Project updates enforce the shared-content target gate; target-local state
  seeds preserve unselected sibling bindings while the canonical fingerprint is
  shared. Global skill lifecycle remains on the existing copied-tree path.
- Explicit project install, update, and remove now route through this service;
  source-backed reconciliation follows the same path.

## Verification

- `cargo check -p skilltap` — passed with warnings denied by later workspace checks.
- `cargo test -p skilltap --test compiled_binary` — 53 passed.
- `cargo fmt --all -- --check` — passed before this checkpoint transition.
- `git diff --check` — passed before this checkpoint transition.
