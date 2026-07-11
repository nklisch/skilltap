---
id: epic-standalone-skill-lifecycle
kind: epic
stage: drafting
tags: []
parent: null
depends_on: [epic-reconciliation-execution]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Standalone Skill Lifecycle

## Brief

Deliver complete-directory standalone skill management from explicit local or
Git sources. The capability validates a top-level `SKILL.md`, preserves every
sibling resource, fingerprints the whole tree, records provenance and resolved
Git revisions, and installs the canonical managed representation at the
appropriate global or project scope.

Harness projections use native paths, links, or copies only where required.
The lifecycle includes compatibility evidence, drift-safe removal and update,
Git SHA comparison, and pins without recursively discovering skills or
separating `SKILL.md` from its directory.

## Foundation references

- `docs/SPEC.md` — Standalone Skill Model, Standalone Skill Lifecycle, Skill Compatibility
- `docs/ARCH.md` — Standalone Skills, Compatibility Analysis, Updates
- `docs/HARNESS-CONTRACTS.md` — Standalone Skill Contract, Version and Update Contract
- `docs/UX.md` — Standalone Skills, Skill Updates

## Anticipated child features

- Explicit local and Git skill source resolution
- Whole-directory validation and deterministic fingerprinting
- Canonical `.agents/skills/` storage and harness projections
- Evidence-bearing compatibility classification
- Install, list, remove, and drift handling
- Ref-to-SHA update resolution, pins, and safe replacement

<!-- The design pass on each child feature will fill in real specifics. -->
