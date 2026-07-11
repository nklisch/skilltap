---
id: epic-harness-observation-adoption-integration-fixtures
kind: story
stage: done
tags: [testing]
parent: epic-harness-observation-adoption-integration
depends_on: [epic-harness-observation-adoption-adopt]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Build Adoption Integration Fixtures

Add deterministic isolated-machine fixture and native-root snapshot helpers for
complete skills, plugin/config/cache examples, symlinks, and scripted harness
processes. Preserve no-follow and secret-safe boundaries.

## Implementation notes

- Added `NativeTreeSnapshot`, `snapshot_tree`, and `snapshot_native_roots` to
  `skilltap-test-support` with deterministic ordering, bytes, link targets,
  entry kinds, and modification times.
- Snapshot traversal uses `symlink_metadata` and never follows symlinks.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap-test-support --all-targets --offline`

## Review

Verdict: Approve - story verified by implement; fast-lane advance.
