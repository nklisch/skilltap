---
id: epic-expanded-harness-support-configuration-constrained-kilo
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-configuration-constrained
depends_on: [epic-expanded-harness-support-configuration-constrained-source]
release_binding: null
research_refs:
  - .research/attestation/kilo-skills.md
  - .research/attestation/kilo-mcp.md
  - .research/attestation/kilo-marketplace.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Kilo Code Adapter

## Checkpoint

Deliver `KiloAdapter`, its profile-bound project document resolver, and private
lossless JSONC projection/activation codec.

## Design element

Implement Unit 6 from the parent feature:

- registry id `kilo`, global native root `<config-home>/kilo`, managed
  distribution, no sidebar/UI lifecycle;
- exact version profile and both-scope managed/skill capabilities;
- canonical `.agents/skills` destination while observing `.kilo/skills`;
- global `kilo/kilo.jsonc` and exactly one effective project document selected
  from `kilo.jsonc`/`.kilo/kilo.jsonc` by the locked precedence;
- block an unmanaged higher-precedence shadow instead of writing both files;
- token/span-preserving `KiloJsoncDocument` that patches only owned MCP members
  and retains comments, trailing commas, order, quote style where unchanged,
  and unknown content;
- exact locked local/remote transport mapping and activation decoding for
  loaded, failed, and authentication-required state.

A serde JSON round-trip, UI automation, or cache mutation is not an acceptable
fallback.

## Acceptance evidence

- Known/unknown versions, global path, both project candidates, precedence, and
  shadow conflict match locked fixtures.
- Project skills use only the canonical `.agents` tree.
- JSONC install/update/remove preserves unrelated bytes/comments and detects
  drift only in owned entries.
- Supported transport maps faithfully; failed/auth-required effective state is
  attention-required rather than drift, and auth material never enters state.
- Every mutation immediately repeats as a byte/inode/plan/state no-op.

## Ordering

Consumes the shared source planner. Independent of Kimi/Vibe in the dependency
graph; normally implemented last among the three because its lossless codec and
dual-path precedence carry the highest target-local complexity.
