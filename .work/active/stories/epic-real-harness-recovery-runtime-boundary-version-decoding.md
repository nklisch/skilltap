---
id: epic-real-harness-recovery-runtime-boundary-version-decoding
kind: story
stage: done
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

## Implementation notes

- Execution capability: focused inline implementation; the decoder and fixture contract are one bounded adapter surface.
- Review weight: standard (project default); the autopilot root owns the independent completion review.
- Files changed: `crates/harnesses/src/lib.rs`, `crates/harnesses/tests/detection.rs`, `crates/harnesses/tests/bootstrap.rs`, `crates/test-support/src/native_process.rs`.
- Tests added: exact real Codex/Claude decoding with observe-only profiles; cross-harness and trailing-document rejection; direct version argv capture; distinct nonzero and timeout errors; exact fake payload coverage.
- Discrepancies from design: the process-context story owns the explicit child-environment parameter and call-site integration, so this story leaves the existing empty environment at the merge seam for that parallel change.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Substrate review at the project-default `standard` weight. The story was escalated to a fresh-context deep lane because it changes a native process contract. Commit `2783a4c` accepts only the attested Codex and Claude forms plus one bounded strict JSON document, preserves typed nonzero/runtime/format failures, and leaves newly recognized versions observe-only. `cargo test --workspace --all-targets`, formatting, and all-target/all-feature clippy completed green.
