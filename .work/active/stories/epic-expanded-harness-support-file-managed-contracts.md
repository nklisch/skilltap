---
id: epic-expanded-harness-support-file-managed-contracts
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-file-managed
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Establish Scope-Aware File-Managed Adapter Contracts

## Checkpoint

Extend the completed target registry and managed projection foundation only as
needed for Gemini, OpenCode, and Kiro: registry-owned default executable names,
global plus project managed projection, control-plane-only marketplace source
registration, exact-version bounded effective-state probes, and data-driven
acceptance layouts. Extract reusable source-side plugin reading without
flattening native destination codecs.

Before registering any new adapter, validate an isolated current binary for each
target against the source-direct research contract and pin its exact
`--version` argv, output bytes, decoded `NativeVersion`, and profile id. The
research artifacts attest paths and behavior but do not contain version pins;
do not invent them. A target that cannot be validated remains unregistered and
observe-only work must not be mislabeled complete.

## Contract surface

- `TargetIdentity` gains `default_binary`; config enablement/detection derive it.
- `HarnessAdapter::supports_managed_projection(CapabilityScope)` replaces the
  project-only boolean. Codex returns project only; the new adapters return
  global and project.
- `ManagedProjectionContext` carries `scope: &Scope` instead of a required
  project root. The shared planner/executor becomes scope-neutral while
  preserving confined writes, ownership, drift, rollback, target-local state,
  pending recovery, and acknowledgment.
- Source-only marketplace actions may return an empty adapter plan and create a
  typed state/control-plane operation. Empty plugin plans remain invalid.
- `EffectiveStateProbePort` supplies direct argv, exact working directory,
  version-pinned decoding, and reload semantics. CLI composition owns executable
  resolution and bounded process execution.
- A private `adapters/file_managed.rs` reads the currently supported selected
  marketplace/plugin source into complete named skill trees, portable MCP server
  values, and required/optional unsupported evidence. Native destination paths
  and MCP encoding remain adapter-owned.
- `KIRO_HOME` becomes a validated `PlatformPaths` input.
- `FakeHarnessProfile` carries its acceptance layout; remove the target-id match.

## Acceptance evidence

- Exact profile fixtures for Gemini, OpenCode, and Kiro are sourced from
  isolated native validation; neighboring and unknown versions cannot mutate.
- A fake managed-only adapter completes marketplace registration and complete
  skill+MCP install/update/remove at global and project scope, then immediately
  repeats to no change.
- Marketplace-only empty plans succeed without fake native files; plugin empty
  plans fail.
- Codex project managed acceptance remains green and Codex global/Claude routing
  is unchanged.
- Probe failures and parse drift are typed, bounded, secret-safe, and never
  interpreted as a healthy empty state.
- Kiro defaults to `kiro-cli`; explicit configured binary overrides still win.

## Ordering

This is the foundation checkpoint. Gemini, OpenCode, and Kiro adapter stories
depend on it; it does not register incomplete adapter placeholders.

## Implementation discovery

The prior partial diff had mixed the shared contract checkpoint with speculative
Gemini, OpenCode, and Kiro adapter files. The exact native validation required
for those adapters was not available in this isolated run: no current binaries,
version output bytes, or source-direct mutation evidence were present. Their
profiles therefore remain unclaimed, and none of those targets is exported or
registered in `TargetRegistry::canonical()`.

That missing evidence belongs to each target adapter story, not to the shared
contract checkpoint. The shared contract can close independently because its
scope, registry, probe, source-reader, operation, and fixture behavior are
covered by existing Codex contracts, a global/project fake managed adapter, and
bounded unit tests. Each target story retains the exact-version evidence gate
before it may add an adapter export or canonical registry entry.

## Implementation

- Added validated `KIRO_HOME` resolution and explicit native-process
  environment propagation in `PlatformPaths`, while keeping canonical global
  instructions at `~/AGENTS.md`.
- Added registry-owned `TargetIdentity::default_binary`; detection and harness
  enablement now use it rather than assuming the target id is an executable.
- Replaced the managed projection context's project-only root with the exact
  concrete `Scope`, and routed both lifecycle paths through the same scope-aware
  planner/executor. Codex remains project-only; the test adapter proves global
  and project roots independently.
- Added the bounded `EffectiveStateProbePort` contract and typed JSON status
  decoder. Probe failures cannot become an empty healthy server set.
- Added a typed no-surface control-plane operation for source-only marketplace
  registration; empty plugin projections still fail at the executable boundary.
- Made `FakeHarnessProfile` acceptance roots profile data for the validated
  Codex/Claude fixtures rather than target-id branching.
- Extracted the complete selected-source plugin reader into
  `crates/harnesses/src/adapters/file_managed.rs` and reused it from Codex
  without moving native destination or MCP encoding into the shared reader.
- Repaired the test-only scope migration: `context_root` carries the inner
  context lifetime and every former `context.project` access derives from
  `context.scope`.

The same feature's untracked draft files remain in the working tree for the
next worker, but they are deliberately excluded from module exports and this
checkpoint's commit: `gemini.rs`, `gemini_managed.rs`, `opencode.rs`,
`opencode_managed.rs`, `kiro.rs`, and `kiro_managed.rs`.

## Verification

- `cargo check --workspace --tests` — passed.
- `cargo test --workspace --all-targets` — 592 passed.
- `cargo test -p skilltap-core source_only_marketplace_registration --lib` —
  passed.
- `cargo test -p skilltap-harnesses file_managed --lib` — 2 passed.
- `cargo test -p skilltap --lib fake_managed_projection_uses_the_exact_global_and_project_scopes` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  passed, and `cargo fmt --all -- --check` plus `git diff --check` passed.
  No target-specific profile is asserted by these shared tests.
