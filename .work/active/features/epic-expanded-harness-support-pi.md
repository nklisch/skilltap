---
id: epic-expanded-harness-support-pi
kind: feature
stage: drafting
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-pi-hook-research]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Pi Compound Target Adapter

## Brief

Deliver Pi as a conditional compound target whose mutable profile requires the
Pi runtime plus separately observed, compatible user-installed MCP and Claude
Code hook-compatibility extensions. Their identities, versions, capabilities,
health, and ownership remain distinct; skilltap neither attributes extension
behavior to Pi core nor silently adopts existing companion packages.

The adapter consumes the shared whole-skill and MCP lifecycle, and uses the
attested hook-extension contract to classify concrete hook components. Missing,
incompatible, or unknown companions keep Pi observe-only with actionable health
output. The feature includes global/project scope, update and drift behavior,
complete isolated validation, and the common adapter acceptance evidence.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: conditional concrete adapter after the registry, managed
  fallback, and Pi hook-extension research.

## Simplification opportunity

- Represent the compound target through ordinary profile selection and health
  evidence instead of adding a Pi-only execution or ownership system.

## Foundation references

- `docs/VISION.md` — Deep Support Over Broad Claims.
- `docs/ARCH.md` — Capability Detection, Observation, Mutation Safety.
- `docs/HARNESS-CONTRACTS.md` — Expanded Target Set, Hook Mapping.

