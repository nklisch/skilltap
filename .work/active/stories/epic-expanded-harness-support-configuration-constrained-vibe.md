---
id: epic-expanded-harness-support-configuration-constrained-vibe
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-configuration-constrained
depends_on: [epic-expanded-harness-support-configuration-constrained-source]
release_binding: null
research_refs:
  - .research/attestation/mistral-skills.md
  - .research/attestation/mistral-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Mistral Vibe Adapter

## Checkpoint

Deliver `VibeAdapter` and its private lossless TOML projection/activation codec,
preserving project trust and OAuth limitations as typed behavior.

## Design element

Implement Unit 5 from the parent feature:

- registry id `vibe`, native root `~/.vibe`, managed distribution, no native
  package lifecycle;
- exact version profile and both-scope managed/skill capabilities;
- canonical `.agents/skills` destination while observing `.vibe/skills`;
- user `~/.vibe/config.toml` and project `.vibe/config.toml` precedence;
- private lossless `VibeConfigDocument` editing only owned named
  `[[mcp_servers]]` entries while preserving comments/order/unknown tables;
- exact locked stdio/HTTP/streamable-HTTP mapping;
- explicit OAuth unsupported classification;
- project-cwd activation probe mapping untrusted state to `trust.required`.

Correct declared config remains owned when project trust prevents effective
load; it is not drift and repeat does not rewrite it.

## Acceptance evidence

- Known/unknown versions, both scopes, precedence, and trusted/untrusted states
  match the locked contract.
- Project skills consume the canonical root without duplicate links/copies.
- Lossless edits preserve comments, unknown fields, filters, and unmanaged
  servers; removal deletes only owned named tables.
- OAuth optional/required outcomes obey partial/block policy, and every
  supported transport maps exactly.
- Immediate repeats are document/tree/state no-ops while untrusted status stays
  attention-required.

## Ordering

Consumes the shared source planner. Independent of Kimi/Kilo in the dependency
graph; normally implemented after Kimi by the same feature owner.
