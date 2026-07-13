---
id: epic-real-harness-recovery-native-lifecycle-managed-project-journal-recovery
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on: []
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Recover the exact managed Pending journal shape

## Finding

The managed retry special case does not accept the state that
`StateExecutionJournal` actually publishes before a first install. The Pending
attempt retains the desired `managed_projections` and resolved revision while
leaving the aggregate fingerprint empty, but ownership validation requires both
the fingerprint and projection manifest to be empty. After files become
effective and terminal state publication fails, the next invocation therefore
reports `managed_project_drifted` instead of recovering the exact completed
attempt.

Updates have the parallel failure: Pending publication preserves the previous
installed fingerprint and manifest, so a successfully applied new projection
cannot satisfy either normal ownership or the first-install-only recovery
special case after terminal publication fails.

The compiled regression does not exercise either real shape. It manually
removes `managed_projections`, `installed_revision`, and `fingerprint` from a
successful state before setting Pending, creating the one representation the
recovery predicate accepts rather than the representation the journal emits.

## Required fix

- Model a managed Pending attempt explicitly enough to distinguish previous
  effective evidence from the exact desired attempt without claiming terminal
  success.
- For first install and update, recover only when the exact operation is
  Pending and fresh locked observation proves every desired skill/MCP surface
  matches the attempted manifest/revision. Publish the desired binding as a
  verified no-change without repeating filesystem mutation.
- Keep failed/indeterminate/mismatched attempts fail-closed; never let one
  operation or scope authorize another.
- Replace the hand-edited recovery fixture with executable journal/failure
  injection that uses the actual Pending representation produced before apply
  and fails terminal state publication after the managed surfaces changed.

## Acceptance

- First install followed by terminal journal failure retries as a verified
  no-change, performs no duplicate publication, and records the exact manifest
  and revision.
- Update followed by terminal journal failure does the same while reconciling
  previous-versus-attempted projection evidence correctly.
- Missing, partial, drifted, different-revision, different-operation, target,
  and scope cases remain blocked before mutation.
- The regression fails if the fixture deletes or rewrites fields that the real
  Pending journal retains.
- Full workspace tests and Clippy pass in isolated roots without touching the
  operator environment or a harness cache.
