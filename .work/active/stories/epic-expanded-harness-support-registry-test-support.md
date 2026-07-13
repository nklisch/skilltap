---
id: epic-expanded-harness-support-registry-test-support
kind: story
stage: review
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

- [x] `FakeHarnessProfile::codex().build(root, VersionKnown)` produces
      `--version` output byte-identical to today's removed Codex version mode.
- [x] `FakeHarnessProfile::claude().build(root, VersionKnown)` produces
      `--version` output byte-identical to today's removed Claude version mode.
- [x] Every existing test that constructed a Codex or Claude fake is migrated
      to the profile constructor; affected test-support and harnesses suites pass.
- [x] `acceptance_matrix(&FakeHarnessProfile::codex(), machine)` passes the full
      fixture-level detection/scope/skill/mcp/reload/drift/removal/idempotency
      suite, and likewise for `claude()`.
- [x] `git grep` over `crates/` returns no removed identity-specific fake-mode
      call sites. Repository-wide matches are historical references in the
      parent/sibling work-item designs, which this story was explicitly barred
      from editing.

## Out of scope

- Profiles or acceptance runs for any target other than Codex and Claude.
- Changes to the production registry, adapter, config, or CLI modules beyond
  what migrating the existing fakes requires.

## Implementation record

- Execution capability: highest, selected by the active autopilot caller because
  the fixture API is shared by every future adapter. Review weight remains the
  caller's `standard`.
- Dispatch: direct implementation only, as required by the caller. No delegated
  agent or peer mechanism was used.
- Added dependency-neutral `VersionResponse`, `LifecycleDialect`, and
  `FakeHarnessProfile`. `test-support` stores a validated static id rather than
  importing production `HarnessId`: core and harnesses already dev-depend on
  test-support, so reversing that dependency would create a Cargo package cycle.
  Codex and Claude constructors are the test-side projection of the canonical
  registry identities and compose identity with orthogonal process behavior.
- Removed the Codex/Claude identity branches from `FakeNativeMode`. Profiled
  `VersionKnown` responses preserve exact version bytes; the existing generic
  JSON `VersionKnown` behavior remains unchanged for non-profiled boundary tests.
  Lifecycle emulation is gated by `LifecycleDialect`, while retaining the
  existing command/list state machine so current lifecycle tests remain stable.
- `FakeHarnessProfile::build` materializes under a caller-owned isolated root.
  Executable publication prefers the prior hard-link behavior and falls back to
  an exact copy only when the build artifact and isolated root cross filesystems.
- Added a proportional fixture-level `acceptance_matrix` rather than duplicating
  the compiled CLI suite or adding a forbidden production dependency. It creates
  documented Codex/Claude global and project skill/MCP surfaces, observes all
  files in complete skills, performs fresh MCP reload reads, detects supporting-
  file drift, removes both resource types, and proves immediate repeat produces
  no writes. Existing harness detection tests and compiled CLI tests remain the
  production-bound adapter/command evidence.
- Migrated every existing identity-specific fake call site in harness detection
  and compiled CLI tests to `FakeHarnessProfile`. The staged `HarnessKind` seam
  and all production CLI/harness/core code remain untouched for the next story.

## Verification

- `cargo test -p skilltap-test-support -p skilltap-harnesses` — passed, 75 tests
  across 8 suites.
- `cargo clippy -p skilltap-test-support -p skilltap-harnesses --all-targets -- -D warnings`
  — passed.
- `cargo fmt --all -- --check` — passed.
- `git grep -n -E 'FakeNativeMode::CodexVersion|FakeNativeMode::ClaudeVersion' -- crates`
  — no matches.
- `cargo test -p skilltap --test compiled_binary` — expected Unit 3 → Unit 4
  integration blocker: 16 CLI production compile errors still access removed
  `HarnessPolicyMap.codex` / `.claude` fields. This story did not alter those
  out-of-scope production sites; the ready CLI integration story owns that seam.
