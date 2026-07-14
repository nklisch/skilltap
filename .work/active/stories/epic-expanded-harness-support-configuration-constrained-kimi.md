---
id: epic-expanded-harness-support-configuration-constrained-kimi
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-configuration-constrained
depends_on: [epic-expanded-harness-support-configuration-constrained-source]
release_binding: null
research_refs:
  - .research/attestation/kimi-skills.md
  - .research/attestation/kimi-mcp.md
  - .research/attestation/kimi-plugins.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Kimi Code Adapter

## Checkpoint

Deliver `KimiAdapter` and its private JSON managed-projection/activation codec
against the exact contract locked by the foundation story.

## Design element

Implement Unit 4 from the parent feature:

- validated `KIMI_CODE_HOME` in `PlatformPaths`, with `~/.kimi-code` fallback;
- registry id `kimi`, managed distribution, no interactive native lifecycle;
- exact version profile with `component.skill`, `component.mcp`, and
  `managed.projection` at global/project scope;
- canonical `.agents/skills` destinations while observing Kimi-native skill
  roots for precedence/conflicts;
- global `<kimi-home>/mcp.json` and project `.kimi-code/mcp.json` with project
  same-name override;
- private `KimiMcpDocument` that mutates owned `mcpServers` entries and
  preserves unknown/unmanaged JSON;
- exact stdio/HTTP/SSE, enablement, timeout, and tool-filter mapping;
- fresh-session activation probe so file presence is never mistaken for load.

Kimi's user-only plugin UI remains observable but is not exposed as a native
mutation vector.

## Acceptance evidence

- Known/unknown profile authority, home override/default, both scopes, and
  project precedence match fixtures.
- Complete skills use one canonical tree and project projection reports
  `not_required` with no duplicate Kimi tree.
- Install/update/remove preserves unknown JSON and unmanaged servers; supported
  transports round-trip faithfully.
- Unsupported optional components require acknowledgment; required unsupported
  blocks even with `--yes`.
- Fresh-session probe sees the expected identities and every mutation
  immediately repeats with no change.

## Ordering

Consumes the shared source planner. Implement first among the target stories to
prove the common managed shape before Vibe and Kilo reuse it.
