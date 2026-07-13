---
id: epic-expanded-harness-support-pi-hook-research
kind: feature
stage: done
tags: [research]
parent: epic-expanded-harness-support
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/analysis/campaigns/pi-claude-hook-compatibility/parent.md
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

## Engagement registration

- **Consumer:** `epic-expanded-harness-support-pi`
- **Temporal contract:** `re-engage-on-trigger`
- **Primitives extended:** Pi package identity, hook semantics, capability-profile evidence
- **Primitives opted out:** none
- **Decision relevance:** determine whether the Pi compound adapter may receive mutation authority and the exact companion contract it must enforce; a missing or semantically incomplete extension keeps Pi observe-only.
- **Analytical artifact type:** capability brief

## Decomposition rationale

Autopilot honored the pre-registered scope and full verification rigor. Three
candidate decompositions were considered: one package-centric pass, one facet
per Claude hook event, and three contract facets. A single pass would blur
identity evidence with semantic equivalence; per-event fan-out would duplicate
package and observation work. The selected three-facet decomposition separates
independent admission gates while keeping the cross-join tractable:

1. package identity, source, installation, and version contract;
2. Claude-to-Pi hook event, payload, ordering, blocking, and failure semantics;
3. installed-state health, scope, ownership, update, removal, and interaction
   with `pi-mcp-adapter`.

Self-check: each facet can independently disqualify mutation support, and the
final synthesis must not average a failure in one facet into overall support.

## Engagement completion

- **Fan-out:** three specialists covering package identity, hook semantics, and
  health/ownership, joined in
  `.research/analysis/campaigns/pi-claude-hook-compatibility/parent.md`.
- **Decision:** `@hsingjui/pi-hooks@0.0.2` is the identifiable current package,
  but its best-effort nine-event command-hook subset is not faithful to the
  current Claude hook contract. It cannot grant mutation authority to the Pi
  compound target; the Pi profile remains observe-only until a companion clears
  the semantic and health evidence listed in the capability brief.
- **Verification:** citation lint passed the hard floor for all three specialist
  briefs and the parent synthesis; adversarial semantic-chain review approved;
  isolated full-rigor evaluation approved; lead spot-check found no unresolved
  material finding.
- **Outputs:** parent synthesis, three specialist briefs, eleven new Pi-hook
  attestations, acquisition manifest, verification checklist, and isolated
  campaign evaluation under
  `.research/analysis/campaigns/pi-claude-hook-compatibility/`.
- **Acquisition offgas:** enriching candidates remain research-side only; no
  blocking acquisition and no autonomous `.work/` promotion.

