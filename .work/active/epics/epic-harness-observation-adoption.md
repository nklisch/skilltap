---
id: epic-harness-observation-adoption
kind: epic
stage: drafting
tags: []
parent: null
depends_on: [epic-rust-control-plane]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Harness Observation and Adoption

## Brief

Make Codex and Claude Code observable without changing their environments.
This epic delivers harness detection, runtime-versioned capability profiles,
native configuration and resource observation, normalized identities, health
findings, first-use status, and explicit adoption into desired inventory.

Observation remains read-only and reports malformed, unmanaged, conflicting,
or unknown-version state instead of hiding it. Adoption records non-conflicting
native resources and provenance but does not transfer or mutate them.

## Foundation references

- `docs/SPEC.md` — Harness Commands, Adoption, Status
- `docs/ARCH.md` — Harness Adapter Contract, Capability Detection, Observation, Adoption Flow
- `docs/HARNESS-CONTRACTS.md` — Common Capability Model, Codex Contract, Claude Code Contract
- `docs/UX.md` — First Use, Enabling Harnesses, Adoption, Status

## Anticipated child features

- Harness detection and capability-profile selection
- Codex observation adapter
- Claude Code observation adapter
- Native-to-normalized identity and health mapping
- Harness list/enable/disable and first-use status
- Conflict-aware adoption and provenance recording

<!-- The design pass on each child feature will fill in real specifics. -->
