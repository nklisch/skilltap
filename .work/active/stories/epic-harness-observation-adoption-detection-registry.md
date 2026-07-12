---
id: epic-harness-observation-adoption-detection-registry
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-detection
depends_on: [epic-harness-observation-adoption-runtime, epic-harness-observation-adoption-contracts, epic-harness-observation-adoption-detection-fixtures]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Detect Harness Installations

Implement the Codex and Claude adapter registry and common installation
detection boundary. Resolve configured binaries through the canonical
executable resolver, run bounded version commands with explicit arguments and
environment, retain opaque native version text only in the typed evidence
envelope, and classify missing, inaccessible, non-runnable, timeout, overflow,
non-zero, malformed, and replacement failures without exposing raw payloads.
Detect siblings independently and do not observe resources, mutate native
state, or expose CLI commands.

## Implementation

- Added the Codex/Claude `HarnessKind` registry boundary and
  `detect_installation` API in `skilltap-harnesses`. Detection resolves a
  configured PATH binary, reuses bounded direct execution and strict JSON,
  preserves opaque version text, and returns safe typed failures without
  observing resources or writing state.
- Added explicit unreachable installation construction and sibling detection
  tests for known/unknown versions, duplicate JSON, and missing binaries.
- Extended fake-native support with detection payload modes and an alias
  publisher that preserves the companion behavior file across filesystem
  boundaries.

## Verification

- Harness detection Clippy and all three detection integration tests pass in
  the locked/offline workspace.

## Review

- Fast-lane review approved the green implementation record. The registry is
  read-only, uses the completed runtime boundaries, and keeps failure output
  closed and sibling-safe.
