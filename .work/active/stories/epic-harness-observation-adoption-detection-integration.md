---
id: epic-harness-observation-adoption-detection-integration
kind: story
stage: done
tags: [testing,infra]
parent: epic-harness-observation-adoption-detection
depends_on: [epic-harness-observation-adoption-detection-fixtures, epic-harness-observation-adoption-detection-registry, epic-harness-observation-adoption-detection-profiles, epic-harness-observation-adoption-detection-probes]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Verify Harness Detection End to End

Exercise Codex and Claude detection against isolated scripted natives and real
runtime boundaries. Cover sibling failure isolation, known/unknown versions,
compiled profile authority, probe narrowing and drift, malformed/duplicate
JSON, timeout/output limits, replacement races, secret-safe diagnostics,
repeat determinism, and no native/state mutation. Run the locked workspace
ladder and native Linux/macOS behavior suites; make only final export or
composition corrections here.

## Implementation

- Extended `crates/harnesses/tests/detection.rs` with repeatable Codex/Claude
  sibling detection, known/unknown profile authority, strict probe narrowing,
  malformed/duplicate payload handling, missing binaries, and replacement-safe
  failure boundaries.
- Tests compare repeated installation evidence and directory state to prove
  detection is deterministic and read-only while one harness failure does not
  erase a successful sibling result.

## Verification

- Harness detection Clippy and all six detection integration tests pass in the
  locked offline workspace.

## Review

- Fast-lane review approved the deterministic sibling-isolation suite and
  green warnings-denied verification. Detection remains read-only and
  harness-neutral.
