---
id: epic-native-marketplace-plugin-lifecycle-commands
kind: feature
stage: drafting
tags: []
parent: epic-native-marketplace-plugin-lifecycle
depends_on: [epic-native-marketplace-plugin-lifecycle-preservation]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Expose Marketplace and Plugin Lifecycle Commands

Expose explicit add/remove/update/list and install/remove/update/list command
families with deterministic plans, ownership, target/scope selectors, and
post-mutation verification.

## Acceptance

List reports registered/desired/installed identities only; it never browses or
searches marketplace contents. Immediate repeats are no-ops.
