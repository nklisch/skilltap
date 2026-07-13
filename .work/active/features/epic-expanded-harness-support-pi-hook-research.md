---
id: epic-expanded-harness-support-pi-hook-research
kind: feature
stage: drafting
tags: [research]
parent: epic-expanded-harness-support
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
research_dials:
  scope_authority: pre-registered
  verification_rigor: full
  intent: verify-contract
  output_kind: capability-brief
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Verify Pi Claude-Hook Compatibility Extension

## Brief

Identify and attest the exact currently supported Pi extension that provides
Claude Code hook compatibility for the user's compound Pi target. Establish its
source, installation and version identity, global/project behavior, supported
hook events and failure semantics, health and observation surfaces, update and
ownership boundaries, and interaction with `pi-mcp-adapter`.

The research must determine whether the extension can satisfy skilltap's hook
equivalence contract and which capabilities remain partial or blocked. It must
not infer mutation authority from package presence or treat similar event names
as equivalent timing, payload, working-directory, environment, permission, or
blocking behavior.

## Research questions

- Which active Pi package implements Claude Code hook compatibility, and what
  official or source-direct contract identifies it?
- How can skilltap observe its installed version, enablement, scope, and health
  without reading or writing opaque caches?
- Which Claude hook events and semantics are faithful, partial, or unsupported?
- Does it have independent update/removal ownership from Pi core and the MCP
  adapter?
- What exact evidence may grant or narrow a compiled Pi compound profile?

## Completion

- Produce source-direct attestations and a current capability brief.
- Record contradictions and disconfirming evidence.
- Define revisit triggers and the acceptance evidence required by the Pi
  adapter feature.
- Run citation and attestation verification with no unresolved findings.

