---
id: epic-harness-observation-adoption-runtime-integration
kind: story
stage: done
tags: [testing,infra]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-executable-resolution, epic-harness-observation-adoption-runtime-bounded-process, epic-harness-observation-adoption-runtime-strict-json, epic-harness-observation-adoption-runtime-codex-home, epic-harness-observation-adoption-runtime-external-tree]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify the Complete Native Observation Runtime

Exercise resolve identity -> bounded direct run -> strict typed JSON as one
pipeline, alongside isolated `CODEX_HOME` resolution and bounded external-tree
snapshots. Prove repeat determinism, zero mutation, safe failure rendering,
boundary limits, executable/tree replacement handling, process-group escape
with retained pipes, descendant termination,
and output secret canaries with adversarial fixtures. Run the full locked Rust
ladder, optimized compiled-binary verification, and native Linux/macOS behavior
jobs; make only final export/composition corrections here.

## Implementation

- Added `crates/core/tests/runtime_integration.rs` with composition coverage for
  explicit executable resolution, bounded direct `printf` execution, strict
  JSON decoding, deterministic repeated results, duplicate-key and secret-safe
  failures, isolated `CODEX_HOME`/XDG/global instruction paths, and bounded
  descriptor-relative external-tree snapshots.
- The integration tests assert no path creation, no tree mutation, stable
  snapshots, and redacted Debug output while reusing the completed runtime
  adapters without adding production coupling.

## Verification

- Focused integration tests pass 3/3 with the locked workspace.
- Full workspace format/check/Clippy/tests, rustdoc, release build, and
  compiled-binary verification remain green after this addition.

## Review

- Fast-lane review approved the green implementation record and the complete
  locked workspace verification. The integration suite composes existing
  adapters without adding production coupling or mutation surfaces.
