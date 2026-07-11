---
id: epic-harness-observation-adoption-detection-fixtures
kind: story
stage: review
tags: [testing,infra]
parent: epic-harness-observation-adoption-detection
depends_on: []
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Build Detection Fixtures

Extend test support with deterministic fake-native modes and isolated fixture
helpers for missing and unexecutable binaries, known and unknown version
responses, strict JSON/probe payloads, output floods, hangs, and executable
identity replacement. Preserve exact argv/environment/cwd capture, avoid
inherited environment, and coordinate all races with barriers rather than
timing. Keep fixtures harness-neutral so Codex and Claude adapters use the same
safe process/runtime support.

## Implementation

- Added deterministic fake-native payload modes for known and unknown
  versions, valid probe narrowing, probe drift, malformed JSON, and duplicate
  keys. Existing barrier-driven hang, flood, replacement, and executable
  identity fixtures remain reusable by detection adapters.
- Added exact-output regression coverage for every payload mode with no
  inherited environment or timing assumptions.

## Verification

- Test-support Clippy and all 16 fixture tests pass under the locked workspace.
