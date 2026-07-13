---
id: epic-expanded-harness-support-registry-test-support
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-registry
depends_on:
  - epic-expanded-harness-support-registry-contract
  - epic-expanded-harness-support-registry-adapters
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Reusable Adapter Acceptance Contract

## Scope

Implement Unit 5 of the registry feature design. Make test-support derive
isolated roots and fake executable profiles from the registry instead of from
per-harness `FakeNativeMode` variants, and codify the shared acceptance matrix
that every registered adapter must pass.

## Units

- `crates/test-support/src/harness_profile.rs` (new):
  - `VersionResponse { TextPrefix, TextSuffix, Json }`.
  - `LifecycleDialect { Codex, Claude, None }`.
  - `FakeHarnessProfile { id, version_response, lifecycle_dialect }` with
    `codex()` / `claude()` constructors and a `build(root, behavior)` that
    composes a harness-specific version/lifecycle script with a generic
    `FakeNativeMode` process-behavior mode.
  - `acceptance_matrix(profile, machine) -> AcceptanceReport` running the
    HARNESS-CONTRACTS "Adding Another Harness" criteria: detection, both
    scopes, complete skills, MCP observation, reload, drift, removal, and
    immediate-repeat idempotency.
- `crates/test-support/src/native_process.rs` (modified):
  - Remove `FakeNativeMode::CodexVersion` and `FakeNativeMode::ClaudeVersion`.
  - Move the lifecycle script block currently gated by
    `matches!(mode, CodexVersion | ClaudeVersion | VersionKnown)` behind
    `LifecycleDialect` so any future Codex-like or Claude-like adapter reuses
    it without a new branch.
  - Keep the generic process-behavior modes (`Hang`, `Flood`, `ProbeNarrow`,
    `ProbeDrift`, `MalformedJson`, `DuplicateJson`, `ExtraJsonDocument`,
    `RetainPipes`, `Exit`, `VersionKnown`, `VersionUnknown`) since they are
    orthogonal to harness identity.
- `crates/test-support/src/lib.rs` (modified): re-export the new types.
- Existing tests that constructed `FakeNativeMode::CodexVersion` /
  `ClaudeVersion` migrate to `FakeHarnessProfile::codex().build(...)` /
  `claude().build(...)`.

## Implementation notes

- `FakeHarnessProfile::codex().build(root, FakeNativeMode::VersionKnown)` must
  produce an executable whose `--version` output is byte-identical to today's
  `FakeNativeMode::CodexVersion` (and likewise `claude()` vs `ClaudeVersion`).
- `acceptance_matrix` is the reusable contract adapter features will invoke
  with their own profile. This story populates and exercises it for Codex and
  Claude only; it does not add profiles for other targets.

## Acceptance criteria

- [ ] `FakeHarnessProfile::codex().build(root, VersionKnown)` produces
      `--version` output byte-identical to today's `FakeNativeMode::CodexVersion`.
- [ ] `FakeHarnessProfile::claude().build(root, VersionKnown)` produces
      `--version` output byte-identical to today's
      `FakeNativeMode::ClaudeVersion`.
- [ ] Every existing test that constructed a Codex or Claude fake passes after
      migrating to the profile constructor.
- [ ] `acceptance_matrix(&FakeHarnessProfile::codex(), machine)` passes the full
      detection/scope/skill/mcp/drift/removal/idempotency suite, and likewise
      for `claude()`.
- [ ] `git grep -n "FakeNativeMode::CodexVersion\|FakeNativeMode::ClaudeVersion"`
      returns no matches.

## Out of scope

- Profiles or acceptance runs for any target other than Codex and Claude.
- Changes to the production registry, adapter, config, or CLI modules beyond
  what migrating the existing fakes requires.
