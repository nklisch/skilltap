---
id: epic-expanded-harness-support-pi-profile
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-pi
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/campaigns/pi-claude-hook-compatibility/parent.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Establish Conditional Compound-Profile Contracts

## Checkpoint

Add the normalized, ephemeral contract needed to represent a harness runtime
and required capability-provider companions without collapsing them into one
identity or exposing user-owned packages to adoption.

Core owns `ProfileComponentObservation`, `ProfileComponentSet`,
`ConditionalComponentReport`, and `ConditionalProfileObservation` with distinct
MCP/hook roles, declaration scope, presence, version, activation,
compatibility, requiredness, and ownership. Harnesses owns an optional
`ConditionalProfilePort` with separate component inspection and compiled tuple
selection. Existing adapters default to no conditional port.

## Safety contract

- Effective capability selection is always
  `compiled_profile.narrow(runtime_health_narrowing)`. Runtime package/config
  evidence can preserve or reduce support but cannot grant it.
- Missing/unknown components cannot produce mutation authority.
- Companion evidence is not `ObservedResource`, never enters desired/observed
  resource graphs, and cannot become an adoption candidate or state record.
- Findings use registered codes, authored summaries, and bounded typed fields;
  no raw settings, package JSON, paths, argv, stdout/stderr, or parser text is
  accepted.
- The contract remains ephemeral and adds no config/inventory/state schema.

## Files

- `crates/core/src/domain/conditional_profile.rs` (new)
- `crates/core/src/domain/mod.rs`
- `crates/core/src/domain/resource/finding.rs`
- `crates/harnesses/src/conditional_profile.rs` (new)
- `crates/harnesses/src/registry.rs`
- `crates/harnesses/src/lib.rs`

## Acceptance evidence

- Duplicate component ids and finding scope mismatches reject.
- MCP and hook companions remain independently queryable.
- Narrowing tests reject every attempted capability widening.
- Unknown compiled tuples remain observe-only under apparently healthy runtime
  fixtures.
- Serialization/output tests reject arbitrary native payload channels.
- Codex and Claude behavior remains unchanged through the absent-port default.

## Ordering

Foundation checkpoint. The Pi adapter depends on this contract; no Pi adapter is
registered here.
