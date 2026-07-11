---
id: epic-native-marketplace-plugin-lifecycle-claude
kind: feature
stage: done
tags: []
parent: epic-native-marketplace-plugin-lifecycle
depends_on: [epic-native-marketplace-plugin-lifecycle-identity]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Add Claude Code Marketplace and Plugin Adapter

Use Claude Code's verified native marketplace/plugin lifecycle for explicit
operations and observe the resulting declared/effective state.

## Acceptance

Native registration/install/update/remove behavior is target-bound, bounded,
and never substitutes undocumented cache or settings edits.

## Implementation notes

Added verified Claude native argument vectors for marketplace and plugin
registration, install, update, uninstall, and scope mapping (`user` versus
`local`). The vectors are direct arguments and perform no shell expansion.

## Review

### Verdict

Approve with comments.

### Findings

- Execution still requires the compiled profile, bounded process runner, and
  post-mutation observation at the CLI composition boundary.

### Verification

Harness lifecycle vector tests and strict clippy pass.
