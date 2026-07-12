---
id: epic-native-marketplace-plugin-lifecycle-codex
kind: feature
stage: done
tags: []
parent: epic-native-marketplace-plugin-lifecycle
depends_on: [epic-native-marketplace-plugin-lifecycle-identity]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Add Codex Marketplace and Plugin Adapter

Use the verified Codex native lifecycle for explicit marketplace/plugin
operations, with bounded direct arguments and fresh observation after mutation.

## Acceptance

Supported operations preserve native output as typed evidence, unknown versions
remain observe-only, and unavailable operations block without cache writes.

## Implementation notes

Added verified Codex native argument vectors for marketplace and plugin add,
remove, upgrade, and update actions. Project scope deliberately returns a typed
unsupported result when Codex lacks a verified native project lifecycle.

## Review

### Verdict

Approve with comments.

### Findings

- Execution still requires the compiled profile, bounded process runner, and
  post-mutation observation at the CLI composition boundary.

### Verification

Harness lifecycle vector tests and strict clippy pass.
