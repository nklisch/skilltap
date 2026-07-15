---
id: epic-expanded-harness-support-pi-integration
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-pi
depends_on: [epic-expanded-harness-support-pi-profile, epic-expanded-harness-support-pi-adapter]
release_binding: 3.1.0
research_refs:
  - .research/analysis/campaigns/pi-claude-hook-compatibility/parent.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Integrate Pi Status and Mutation Authorization

## Checkpoint

Register Pi only after one conditional-profile resolver drives status and every
Pi-target mutation boundary. Render core, MCP companion, hook companion, and the
compound profile separately; block all current Pi native/filesystem mutations
before operation construction.

## Composition contract

- Add one `application/conditional_profile.rs` resolver that detects the exact
  configured executable, inspects companions, selects the compiled tuple, and
  applies narrowing. Status and mutation paths do not duplicate this logic.
- Add Pi once to `TargetRegistry::canonical()` as `Managed`; all help,
  enable/disable, list, validation, enabled resolution, and `--target all`
  behavior derive from the registry. First-party bootstrap remains Codex/Claude.
- Status emits static typed rows for Pi core, `pi-mcp-adapter`,
  `@hsingjui/pi-hooks`, and `compound_profile`, including package version,
  scope, activation, compatibility, ownership, requiredness, adoptability, and
  mutation authorization. Plain and JSON share one outcome.
- Companion findings may enter harness findings but companion rows never enter
  `HarnessObservation.resources`; `adopt --from pi` cannot adopt them.

## Mutation guard

Check exact conditional-profile capabilities before any Pi-target:

- global/project standalone skill install, update, or removal;
- project canonical skill publication/link planning;
- plan/sync repair or reconciliation;
- daemon update/application;
- future managed projection dispatch.

Current plugin/marketplace operations also remain blocked because Pi exposes no
native lifecycle or managed projection port. `harness enable pi` and read-only
adoption remain allowed because they do not mutate Pi native state.

A canonical project skill created for another target may be observed as
Pi-loadable, but no Pi target state, ownership, journal, or apply success is
recorded while Pi mutation is blocked. An attention-required Pi sibling must
not prevent unrelated authorized targets from applying.

## Files

- `crates/harnesses/src/registry.rs`
- `crates/cli/src/application/conditional_profile.rs` (new)
- `crates/cli/src/application.rs`
- `crates/cli/src/application/status.rs`
- `crates/cli/src/application/lifecycle.rs`
- `crates/cli/src/application/project_skills.rs`
- `crates/cli/src/application/reconciliation.rs`
- `crates/cli/src/outcome.rs` only if required by a shared typed field

## Acceptance evidence

- Pi appears through registry-derived surfaces; bootstrap remains narrow.
- Core and companion output cannot be conflated or adopted.
- Exact current tuple says known/verified and `mutation_authorized=false`.
- Every current Pi write path blocks before native/filesystem execution and
  leaves inventory/state/native bytes unchanged except explicit skilltap-only
  enable/adoption changes.
- Multi-target safe siblings proceed while Pi reports attention required.
- User-owned package/config changes are fresh health evidence, not drift or
  ownership transfer.
- Next actions explain that current `pi-hooks` is partial; they never promise
  installing it will enable mutation.

## Implementation evidence

- Added the shared CLI conditional-profile resolver and fail-closed scoped mutation guard.
- Registered Pi as a managed target while preserving the Codex/Claude-only bootstrap surface.
- Added typed core, companion, and compound status rows; companion evidence remains outside normalized resources and adoption.
- Guarded lifecycle preview, native lifecycle, project skill publication/link/removal, reconciliation, daemon planning, and managed projection planning before operation construction.
- Added isolated compiled-binary coverage for exact Pi status, plain/JSON output, adoption exclusion, Pi-only mutation immutability, and target-all sibling application.
- Verification: workspace tests, all-feature strict Clippy, format check, and diff check pass.

## Ordering

Depends on both the profile contract and the unregistered Pi adapter. Acceptance
runs only after this guard and output surface are integrated.
