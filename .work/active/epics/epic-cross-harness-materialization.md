---
id: epic-cross-harness-materialization
kind: epic
stage: drafting
tags: []
parent: null
depends_on: [epic-native-marketplace-plugin-lifecycle, epic-standalone-skill-lifecycle]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Cross-Harness Materialization

## Brief

Allow an explicitly selected plugin to cross harness boundaries only when its
components can be represented faithfully or its exact partial result is
acknowledged. This epic builds source component graphs, evaluates target
capabilities and dependencies, maps portable skills, MCP servers, hooks, and
other native components, and renders skilltap-owned target artifacts through
documented load paths.

Unsupported required components block the resource. Optional omissions remain
visible, selectors provide piecewise control, and managed artifacts retain
source provenance and ownership. Native caches are never used as undocumented
write surfaces.

## Foundation references

- `docs/VISION.md` — Native First, Faithfulness Before Portability, Explicit Loss
- `docs/SPEC.md` — Synchronization, Plugin Lifecycle, Skill Compatibility
- `docs/ARCH.md` — Plugin Resolution, Compatibility Analysis
- `docs/HARNESS-CONTRACTS.md` — Cross-Harness Component Matrix, MCP Mapping, Hook Mapping
- `docs/UX.md` — Plugin Management, partial-result examples

## Design decisions

- **Can users define their own faithful-equivalence mappings?** No in v3.
  Harness adapters own equivalence rules and their evidence. Users may accept
  an exact partial result and select included or excluded components, but
  configuration cannot force skilltap to classify unsupported behavior as
  faithful.
- **What happens when a materialized component cannot preserve its identity?**
  Block that component when its source identity or a documented target
  namespace collides. Do not rename skills, invocations, frontmatter, or other
  behavior-bearing identifiers. An unsupported required collision blocks the
  resource; an optional collision can appear only as an acknowledged omission.
- **Does this epic require UI mockups?** No. Compatibility evidence,
  omissions, and acknowledgments are expressed through plans and structured
  CLI output.

## Anticipated child features

- Source plugin component graph and dependency model
- Cross-harness compatibility evidence and classification
- Portable skill and conditional MCP materialization
- Hook and harness-specific component mapping
- Managed plugin rendering, registration, and ownership
- Partial plans, required-component blocking, and selector acknowledgments

<!-- The design pass on each child feature will fill in real specifics. -->
