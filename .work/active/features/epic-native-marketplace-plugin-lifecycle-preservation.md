---
id: epic-native-marketplace-plugin-lifecycle-preservation
kind: feature
stage: drafting
tags: []
parent: epic-native-marketplace-plugin-lifecycle
depends_on: [epic-native-marketplace-plugin-lifecycle-codex, epic-native-marketplace-plugin-lifecycle-claude]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Preserve Native Configuration During Lifecycle Edits

Publish only documented native configuration changes while preserving unknown
fields, sibling harness settings, and exact global/project scope.

## Acceptance

Edits are strict, atomic, idempotent, and tested against unknown-field and
concurrent-writer preservation.
