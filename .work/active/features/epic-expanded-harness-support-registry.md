---
id: epic-expanded-harness-support-registry
kind: feature
stage: drafting
tags: []
parent: epic-expanded-harness-support
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Typed Target Registry and Adapter Contract

## Brief

Replace the closed Codex/Claude target enumerations with one typed registry that
drives harness policy, CLI validation and help, enabled-target resolution,
adapter composition, scoped capability profiles, observation dispatch, and
status rendering. Existing generic `HarnessId`, inventory, target-local state,
and output contracts remain the domain foundation rather than being replaced
with a new hierarchy.

Define the reusable adapter acceptance contract alongside the registry so each
target supplies its own documented paths, codecs, probes, reload behavior, and
native lifecycle capabilities through the same bounded ports. Test support must
derive isolated roots and fake executable profiles from this registry instead
of adding another hard-coded branch for every harness. This feature does not
implement the individual target adapters.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: foundation feature; managed projection and every concrete
  adapter depend on its target and acceptance contracts.

## Simplification opportunity

- Remove repeated Codex/Claude matches from configuration, CLI parsing,
  application composition, status observation, and fixtures while preserving
  the first-party plugin bootstrap as its intentionally narrower distribution
  surface.

## Foundation references

- `docs/SPEC.md` — Harness, Operating Model, Configuration Directory.
- `docs/ARCH.md` — Harness Adapter Contract, Capability Detection, Testing.
- `docs/UX.md` — Target and Scope, Common Flags, Help and Diagnostic Discovery.
- `docs/HARNESS-CONTRACTS.md` — Common Capability Model, Expanded Target Set.

