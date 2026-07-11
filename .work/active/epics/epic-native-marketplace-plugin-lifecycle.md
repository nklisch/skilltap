---
id: epic-native-marketplace-plugin-lifecycle
kind: epic
stage: done
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

## Design decisions

- **How are cross-harness native installations associated?** One explicitly
  selected source may be one logical desired resource with separate
  harness-native marketplace names, plugin identities, scopes, versions, and
  apply results. Matching names alone never create that association.
- **What happens when a native lifecycle operation is unavailable?** Report
  the missing capability and hand the semantic request to the later
  cross-harness materialization planner when an accessible source exists.
  Never substitute an undocumented cache or configuration mutation.
- **Does marketplace management expose remote inventory?** No. List commands
  report registered marketplaces and installed or desired plugins only; they
  never enumerate, search, rank, or recommend marketplace contents.
- **Does this epic require UI mockups?** No. Its lifecycle is exposed through
  deterministic CLI and JSON operations.

## Anticipated child features

- Marketplace source and native identity model
- Codex marketplace and plugin lifecycle adapter
- Claude Code marketplace and plugin lifecycle adapter
- Scope-aware native configuration preservation
- Plugin enablement, removal, and post-mutation verification
- Marketplace and plugin list/install/remove command families

<!-- The design pass on each child feature will fill in real specifics. -->

## Decomposition

Native lifecycle work is split by source identity, harness adapters, document
preservation, and command composition. No child may enumerate marketplace
contents or write undocumented caches.

## Children complete

Identity, Codex, Claude, native-preservation, and command features are done.
The realized lifecycle invokes only verified native vectors, preserves desired
inventory and state safely across failure, supports explicit and update-all
selectors, and exposes fresh post-mutation observation without browsing
marketplace contents.

### Child features

1. `epic-native-marketplace-plugin-lifecycle-identity` — validate explicit
   marketplace locators, plugin selectors, source association, and scope-aware
   logical resource identity — depends on `[]`.
2. `epic-native-marketplace-plugin-lifecycle-codex` — compose verified Codex
   native marketplace/plugin lifecycle commands and post-mutation observation
   — depends on `[epic-native-marketplace-plugin-lifecycle-identity]`.
3. `epic-native-marketplace-plugin-lifecycle-claude` — compose verified Claude
   Code marketplace/plugin lifecycle commands and post-mutation observation
   — depends on `[epic-native-marketplace-plugin-lifecycle-identity]`.
4. `epic-native-marketplace-plugin-lifecycle-preservation` — edit documented
   config surfaces while preserving unknown fields and exact scope boundaries
   — depends on `[epic-native-marketplace-plugin-lifecycle-codex,
   epic-native-marketplace-plugin-lifecycle-claude]`.
5. `epic-native-marketplace-plugin-lifecycle-commands` — expose explicit
   add/remove/update/list/install commands, operation plans, ownership, and
   idempotent verification without discovery — depends on
   `[epic-native-marketplace-plugin-lifecycle-preservation]`.

## Design review

### Verdict

Approved for implementation.

### Notes

Native harness lifecycle commands remain authoritative when available. A
missing native capability produces a typed attention result and never falls
back to cache mutation or guessed configuration edits.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: exact native identity correlation can be strengthened by a future observation adapter without changing lifecycle authority.

**Notes**: Deep aggregate review completed inline in degraded fresh-context
mode because this run intentionally uses no sub-agents. The children match the
native-first, no-discovery brief and preserve the documented separation between
native lifecycle and later cross-harness materialization. Full workspace clippy
and tests pass.
