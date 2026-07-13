---
id: epic-expanded-harness-support-configuration-constrained
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

# Configuration-Constrained Adapters for Kimi, Vibe, and Kilo

## Brief

Deliver complete adapters for Kimi Code CLI, Mistral Vibe, and Kilo Code while
preserving their distinct new-session reload, transport/authentication, JSONC,
and configuration-precedence constraints. Each target meets the same
global-and-project skill and MCP admission contract; its limitations appear as
typed capabilities and health evidence rather than being smoothed into generic
support.

The adapters consume shared managed projection and target-local state, preserve
unknown documented native configuration, and classify unsupported transports,
authentication, hooks, agents, or other optional components through the normal
faithful/partial/blocked model. Each target ships with isolated validation and
the common adapter acceptance evidence. This feature does not add target-local
exceptions to the core planner.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: parallel concrete-adapter feature after the registry and
  managed fallback foundations.

## Simplification opportunity

- Express reload, transport, and document-format differences as adapter profile
  data and target-owned codecs instead of duplicating reconciliation policy.

## Foundation references

- `docs/VISION.md` — Faithfulness Before Portability, Explicit Loss.
- `docs/ARCH.md` — Capability Detection, Compatibility Analysis, Error Model.
- `docs/HARNESS-CONTRACTS.md` — Expanded Target Set, MCP Mapping.

