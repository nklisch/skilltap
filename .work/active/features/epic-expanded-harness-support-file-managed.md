---
id: epic-expanded-harness-support-file-managed
kind: feature
stage: drafting
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# File-Managed Adapters for Gemini, OpenCode, and Kiro

## Brief

Deliver complete adapters for Gemini CLI, OpenCode, and Kiro CLI using their
documented global and project skill roots, MCP configuration, effective-state
observation, and reload or status mechanisms. Each adapter exposes its own
verified version profile and target semantics while consuming the shared
managed projection lifecycle for complete skills, MCP entries, ownership,
drift, update, and removal.

These targets form the direct file-managed group because their supported write
and observation boundaries are explicit and do not require a native marketplace
to be useful. The feature includes each target's isolated native validation,
agent-facing help/status exposure, and shared acceptance-contract evidence. It
does not broaden first-party plugin bootstrap or treat project trust as proof of
effective load.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: parallel concrete-adapter feature after the registry and
  managed fallback foundations.

## Simplification opportunity

- Reuse one managed skill/MCP transaction and one acceptance harness; delete
  adapter-local copies of ownership, rollback, and idempotency logic.

## Foundation references

- `docs/VISION.md` — Native First, Deep Support Over Broad Claims.
- `docs/ARCH.md` — Harness Adapter Contract, Observation, Plugin Resolution.
- `docs/HARNESS-CONTRACTS.md` — Expanded Target Set, Adding Another Harness.

