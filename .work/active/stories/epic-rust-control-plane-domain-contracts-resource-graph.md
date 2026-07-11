---
id: epic-rust-control-plane-domain-contracts-resource-graph
kind: story
stage: implementing
tags: []
parent: epic-rust-control-plane-domain-contracts
depends_on: [epic-rust-control-plane-domain-contracts-identity-scope-source]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Desired and Observed Resource Graphs

## Scope

Implement Unit 2 from the parent feature: resource/component kinds, provenance,
ownership, desired and observed records, native identity and opaque metadata
namespacing, observation findings, and validated dependency graphs.

## Acceptance criteria

- [ ] Desired and observed records remain distinct and use concrete scopes.
- [ ] Harness-native metadata is namespaced and opaque to general domain code.
- [ ] Graph construction rejects duplicates, dangling/self edges, and cycles.
- [ ] Malformed unmanaged entries can be reported without fabricating resources.
- [ ] Representative graphs serialize deterministically and round-trip.
- [ ] Locked format, clippy, and workspace tests pass.
