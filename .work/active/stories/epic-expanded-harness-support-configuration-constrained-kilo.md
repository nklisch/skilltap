---
id: epic-expanded-harness-support-configuration-constrained-kilo
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-configuration-constrained
depends_on: [epic-expanded-harness-support-configuration-constrained-source]
release_binding: 3.1.0
research_refs:
  - .research/attestation/kilo-skills.md
  - .research/attestation/kilo-mcp.md
  - .research/attestation/kilo-marketplace.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Implement the Kilo Code Adapter

## Checkpoint

Deliver `KiloAdapter`, its profile-bound project document resolver, and private
lossless JSON/JSONC managed-declaration codec. Runtime activation is not
probed.

## Design element

Implement Unit 6 from the parent feature:

- registry id `kilo`, global native root `<config-home>/kilo`, managed
  distribution, no sidebar/UI lifecycle;
- exact `7.4.7` version profile and both-scope managed/skill capabilities;
- canonical `.agents/skills` destination while observing `.kilo/skills`;
- global `kilo/kilo.json`/`kilo.jsonc` and exactly one project document selected
  from the documented JSON/JSONC locations by locked precedence;
- block an unmanaged higher-precedence shadow instead of writing both files;
- token/span-preserving `KiloJsoncDocument` that patches only owned MCP members
  and retains comments, trailing commas, order, quote style where unchanged,
  and unknown content;
- exact locked local/remote transport mapping, with unsupported schema keys
  and authentication outcomes failing closed;
- no `debug config`, `mcp list`, `mcp auth`, cache, `.kilo`, or `.gitignore`
  creation as an observation side effect.

A serde JSON round-trip, UI automation, or cache mutation is not an acceptable
fallback.

## Acceptance evidence

- Known/unknown versions, global path, both project candidates, precedence, and
  shadow conflict match locked fixtures without invoking Kilo.
- Project skills use only the canonical `.agents` tree.
- JSONC install/update/remove preserves unrelated bytes/comments and detects
  drift only in owned entries.
- Supported transport maps faithfully; effective load/authentication is not
  inferred from file bytes, and auth material never enters state.
- Every mutation immediately repeats as a byte/inode/plan/state no-op.

## Implementation notes

- Execution capability: high; Kilo keeps a private span patcher and resolves
  valid global/project documents through the bounded filesystem port.
- Verification: JSONC comment/unrelated-field preservation, unknown-schema and
  precedence rejection, exact version, and no-probe tests pass.

## Completion

This story is `done` under the relaxed Kilo contract. Effective human output
and side-effectful native probes remain explicitly unused.
