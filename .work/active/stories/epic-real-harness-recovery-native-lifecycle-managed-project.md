---
id: epic-real-harness-recovery-native-lifecycle-managed-project
kind: story
stage: review
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

## Implementation notes

- Execution capability: strongest available; the change crosses native
  authorization, managed filesystem publication, catalog parsing, provenance,
  and drift protection.
- Review weight: highest, inherited from the caller's recovery/autopilot scope.
- Files changed: `crates/harnesses/src/managed_codex_project.rs`,
  `crates/harnesses/src/lib.rs`, `crates/harnesses/src/lifecycle.rs`,
  `crates/core/src/lifecycle_operation.rs`,
  `crates/cli/src/application.rs`,
  `crates/cli/src/application/execution.rs`,
  `crates/cli/src/application/lifecycle.rs`, and
  `crates/cli/tests/compiled_binary.rs`.
- Tests added: bounded catalog parsing, contained named local-source
  resolution, duplicate/path-escape rejection, unknown-field preservation,
  and an isolated compiled-CLI scenario covering project marketplace add,
  complete plugin install, executable sibling preservation, repeat no-op,
  materialized ownership/provenance, cache non-mutation, and drift refusal.
- Discrepancies from design: remote Git sources fail closed with an actionable
  local-checkout requirement because the existing source boundary resolves a
  Git revision but does not expose a verified checkout tree. Managed Codex
  project publication therefore consumes an explicit bounded local marketplace
  checkout; it never interprets a remote catalog or mutates a cache.
- Verification: the focused harness adapter suite passes (2 tests),
  `git diff --check` passes, and the CLI crate compiled immediately before the
  concurrent per-target `ResourceState` schema transition. The compiled CLI
  E2E is present but temporarily cannot compile until the overlapping CLI
  constructors are migrated to that new schema; the coordinating worker owns
  that migration and will rerun this scenario afterward.
- Adjacent issues parked: none.
