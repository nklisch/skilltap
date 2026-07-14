---
id: epic-expanded-harness-support-trust-interactive
kind: feature
stage: drafting
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-project-skill-links]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-14
---

# Trust- and Interactive-State Adapters for Junie and Amp

## Brief

Deliver complete adapters for Junie and Amp while preserving their native trust,
interactive-state, skill-local MCP, and runtime-health semantics. Both targets
provide documented global and project skill and MCP surfaces, but configured
state is not always proof of effective availability; the adapters must expose
that distinction through normalized observation and actionable health.

The adapters consume the shared managed lifecycle, preserve unrelated native
configuration, and keep native extensions or skill-local MCP representations
only when they are the faithful form. Each target ships with isolated native
validation, trust-state cases, and the common adapter acceptance evidence.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: parallel concrete-adapter feature after the registry and
  managed fallback foundations.

## Simplification opportunity

- Reuse normalized declared-versus-effective observation and health findings
  rather than inventing target-specific reconciliation states for trust and
  interactive readiness.

## Foundation references

- `docs/VISION.md` — Observable Ownership, Deep Support Over Broad Claims.
- `docs/ARCH.md` — Observation, Capability Detection, Error Model.
- `docs/HARNESS-CONTRACTS.md` — Expanded Target Set, Adding Another Harness.

