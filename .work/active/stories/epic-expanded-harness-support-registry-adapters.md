---
id: epic-expanded-harness-support-registry-adapters
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-registry
depends_on:
  - epic-expanded-harness-support-registry-contract
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Codex and Claude Adapter Migration

## Scope

Implement Unit 2 of the registry feature design. Move the existing Codex and
Claude detection, version-decode, capability-profile, observation, native-
lifecycle-vector, instruction-bridge, and skill-projection logic out of the
closed `HarnessKind` match sites and onto `CodexAdapter` / `ClaudeAdapter`
structs that implement the `HarnessAdapter` trait defined in the parent
feature. Register both in `TargetRegistry::canonical()`.

This story proves the contract is concrete and that Codex/Claude behavior is
preserved byte-for-byte. It does not implement any new target adapter.

## Units

- `crates/harnesses/src/adapters/mod.rs` (new): adapter module root and any
  shared `adapter_helpers` relocation target.
- `crates/harnesses/src/adapters/codex.rs` (new): `CodexAdapter` plus
  `CodexLifecycle`, `CodexInstructionBridge`, `CodexSkillProjection`.
- `crates/harnesses/src/adapters/claude.rs` (new): `ClaudeAdapter` plus the
  corresponding Claude port structs.
- `crates/harnesses/src/lib.rs` (modified): relocate the existing
  `observe_codex_*` / `observe_claude_*` / `decode_native_version` /
  `select_profile` / `compiled_capabilities` / `unknown_capabilities` free
  functions behind a private `adapter_helpers` module; re-export the registry
  and adapter modules; remove the `HarnessKind` enum after all call sites are
  converted.
- `crates/harnesses/src/lifecycle.rs` (modified): drop `harness: HarnessKind`
  from `NativeLifecycleRequest`; the owning adapter's `NativeLifecycleVector`
  supplies the per-harness argument vector, including the Codex
  project-scope-unsupported constraint.

## Implementation notes

- Relocate, do not rewrite: the adapter structs delegate to the existing
  functions unchanged so behavior is preserved. The capability matrix (Codex
  `0.144.1` with no plugin.update and no project-scope marketplace/plugin
  lifecycle; Claude `2.1.201` with the asymmetric update/project matrix) is
  reproduced exactly.
- `CodexAdapter` and `ClaudeAdapter` are stateless singletons exposing
  `static_ref() -> &'static dyn HarnessAdapter`, satisfying the registry's
  `&'static` storage.
- After migration, `git grep -n "HarnessKind" crates/` must return no matches.

## Acceptance criteria

- [ ] Every existing Codex and Claude detection, capability, observation,
      lifecycle, instruction, and skill-projection test passes without
      modification to its assertions.
- [ ] A capability-matrix table test asserts `CodexAdapter::select_profile`
      and `ClaudeAdapter::select_profile` reproduce today's `select_profile`
      output for the verified version and for an unknown version.
- [ ] `git grep -n "HarnessKind" crates/` returns no matches.
- [ ] `git grep -n '"codex"\|"claude"' crates/cli/src/` returns no behavior-
      dispatching match arms (display labels only, if any).
- [ ] `TargetRegistry::canonical().ids()` yields exactly `codex` and `claude`.

## Out of scope

- The CLI parser/help/config changes (Unit 3/4) and the test-support contract
  (Unit 5) land in their own stories.
- Any new target adapter.
