---
id: epic-expanded-harness-support-file-managed-contracts
kind: story
stage: implementing
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
