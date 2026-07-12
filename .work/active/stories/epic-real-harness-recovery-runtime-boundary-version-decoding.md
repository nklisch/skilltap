---
id: epic-real-harness-recovery-runtime-boundary-version-decoding
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-runtime-boundary
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Decode real Codex and Claude versions

## Scope

Replace the JSON-only synthetic detection contract with bounded
harness-specific decoding for the current plain-text forms plus strict JSON
fallback, and update fake harness fixtures to model real version output.

## Acceptance

- Exact current Codex and Claude fixture outputs produce reachable opaque
  versions but do not automatically grant mutation authority.
- Cross-harness, malformed, extra-document, nonzero, timeout, and over-limit
  outputs remain distinct safe failures.
- Detection uses the explicit child environment and direct argument vector.

