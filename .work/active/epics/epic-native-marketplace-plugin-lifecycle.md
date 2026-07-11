---
id: epic-native-marketplace-plugin-lifecycle
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

# Native Marketplace and Plugin Lifecycle

## Brief

Manage explicitly selected Codex and Claude Code marketplaces and native
plugins through each harness's verified lifecycle. This epic covers source and
identity normalization, global and personal project scopes, native command and
documented-file operations, enablement, removal, observation after mutation,
and preservation of unknown native configuration.

The capability lists only registered marketplaces and installed or desired
plugins. It never searches, ranks, recommends, or exposes the available plugin
inventory inside a marketplace. Cross-harness conversion and skilltap-owned
materialization are deliberately deferred to their own epic.

## Foundation references

- `docs/VISION.md` — Native First, Non-Goals
- `docs/SPEC.md` — Marketplace Lifecycle, Plugin Lifecycle, Ownership and Removal
- `docs/ARCH.md` — Native Command Execution, Plugin Resolution
- `docs/HARNESS-CONTRACTS.md` — Marketplaces, Plugins, Marketplace Identity, Plugin Identity
- `docs/UX.md` — Marketplace Management, Plugin Management

## Anticipated child features

- Marketplace source and native identity model
- Codex marketplace and plugin lifecycle adapter
- Claude Code marketplace and plugin lifecycle adapter
- Scope-aware native configuration preservation
- Plugin enablement, removal, and post-mutation verification
- Marketplace and plugin list/install/remove command families

<!-- The design pass on each child feature will fill in real specifics. -->
