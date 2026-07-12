---
id: epic-real-harness-recovery-native-lifecycle-managed-project
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Materialize unsupported Codex project lifecycle safely

## Scope

Resolve Codex project marketplace/plugin operations to the documented managed
load-path lifecycle when native project commands are unavailable. This story
owns blocker 9.

## Acceptance

- Codex project operations use validating project marketplace edits and owned
  plugin/skill/MCP load-path publications without invoking an unverified native
  command or writing a cache.
- Explicit sources are bounded and validated before planning; complete required
  components are faithful, optional omissions are disclosed/acknowledged, and
  missing required behavior remains blocked.
- Materialized state records skilltap ownership and source/fingerprint evidence,
  not native provenance.
- Update and removal preserve unknown native fields and fail closed on drift,
  foreign ownership, or changed destinations.
- Successful install/update/remove operations repeat as zero-change; authorized
  global Codex and Claude operations remain native.
