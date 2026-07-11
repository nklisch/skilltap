---
id: epic-native-marketplace-plugin-lifecycle-commands
kind: feature
stage: implementing
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

## Design

- All mutating command forms first produce exact scope/target operation
  previews. Native execution is allowed only through a verified harness profile,
  bounded direct arguments, the configuration lock, and post-mutation
  observation.
- Native command failures remain typed operation failures; no cache or
  undocumented configuration fallback is permitted.
- The current CLI preview intentionally remains non-mutating until state
  journaling and adapter execution are composed; it reports the exact pending
  operation rather than returning a generic capability error.

## Implementation notes

Marketplace/plugin list commands are inventory-backed and read-only. Add and
install now expose deterministic operation previews with scope, target, source,
and name fields. The harness crate supplies bounded native lifecycle vectors
and an execution boundary. Core now has one validated constructor for faithful
native operations, and state journaling is atomic and resource-exact; the
remaining gap is composing these pieces into the mutating CLI adapter with
fresh post-mutation observation. The harness layer now exposes a typed
`NativeLifecyclePort` that enforces exact operation/request identity and maps
bounded native failures to operation failures.
