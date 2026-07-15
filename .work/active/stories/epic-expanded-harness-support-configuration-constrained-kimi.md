---
id: epic-expanded-harness-support-configuration-constrained-kimi
kind: story
stage: done
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
updated: 2026-07-15
---

# Implement the Kimi Code Adapter

## Checkpoint

Deliver `KimiAdapter` and its private JSON managed-declaration codec against
the exact contract locked by the foundation story. Runtime activation is not
attested and is never probed.

## Design element

Implement Unit 4 from the parent feature:

- validated `KIMI_SHARE_DIR` in `PlatformPaths`, with `~/.kimi` fallback;
- registry id `kimi`, managed distribution, no interactive native lifecycle;
- exact `1.48.0` profile with global `component.mcp: Unverified` and project
  `component.mcp: Unsupported`;
- canonical `.agents/skills` destinations while observing Kimi-native skill
  roots for precedence/conflicts;
- global `<kimi-share-dir>/mcp.json` only; project MCP is an explicit
  unsupported outcome;
- private `KimiMcpDocument` that mutates owned `mcpServers` entries and
  preserves unknown/unmanaged JSON;
- exact static local/remote mapping, enablement, timeout, and tool-filter
  mapping; OAuth and literal credential values fail closed;
- no activation probe, `mcp` command, TUI, browser, or auth flow.

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
- Declaration ownership is reported without claiming runtime load, trust,
  authentication, or reload; every mutation immediately repeats with no change.
- Project MCP is rejected before any project path is read or written.

## Implementation notes

- Execution capability: high; Kimi uses a private JSON codec and the shared
  bounded source/skill planner.
- Verification: exact version, global/project capability, OAuth rejection,
  no-probe, and private-codec tests pass.

## Completion

This story is `done` under the relaxed Kimi contract. Project MCP remains
explicitly unsupported.
