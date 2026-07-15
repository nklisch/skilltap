---
id: epic-expanded-harness-support-configuration-constrained-vibe
kind: story
stage: done
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
updated: 2026-07-15
---

# Implement the Mistral Vibe Adapter

## Checkpoint

Deliver `VibeAdapter` and its private lossless TOML declaration codec,
preserving project trust and OAuth limitations as typed, non-probed behavior.

## Design element

Implement Unit 5 from the parent feature:

- registry id `vibe`, native root `~/.vibe`, managed distribution, no native
  package lifecycle;
- exact `2.19.1` profile and both-scope managed/skill capabilities;
- canonical `.agents/skills` destination while observing `.vibe/skills`;
- user `~/.vibe/config.toml` and project `.vibe/config.toml` declaration
  precedence;
- private lossless `VibeConfigDocument` editing only owned named
  `[[mcp_servers]]` entries while preserving comments/order/unknown tables;
- exact locked stdio/HTTP/streamable-HTTP mapping and static references;
- explicit OAuth unsupported classification;
- no `/mcp`, TUI, LLM, trust approval, browser, or effective-state probe.

Correct declared config remains owned when project trust prevents effective
load; it is not drift and repeat does not rewrite it.

## Acceptance evidence

- Known/unknown versions, both scopes, and declaration precedence match the
  locked contract; trust remains unverified without approval.
- Project skills consume the canonical root without duplicate links/copies.
- Lossless edits preserve comments, unknown fields, filters, and unmanaged
  servers; removal deletes only owned named tables.
- OAuth optional/required outcomes obey partial/block policy, and every
  supported transport maps exactly.
- Immediate repeats are document/tree/state no-ops; effective load, trust, and
  reload are not inferred from the declaration.

## Implementation notes

- Execution capability: high; Vibe uses `toml_edit` to preserve syntax while
  sharing only the bounded source and skill planner.
- Verification: exact version, lossless comment/unknown-table preservation,
  OAuth rejection, and no-probe tests pass.

## Completion

This story is `done` under the relaxed Vibe contract. OAuth and effective trust
remain explicitly unsupported/unverified.
