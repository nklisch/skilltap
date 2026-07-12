---
id: story-skilltap-plugin-distribution-bootstrap-harness-contract-coverage
kind: story
stage: implementing
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Complete first-party harness bootstrap contract coverage

Review follow-up for `story-skilltap-plugin-distribution-bootstrap-harness`.

The adapter now has the intended canonical Claude source, qualified plugin
identity, Codex unsupported result, read-first observation, and executable
identity binding. It still needs operation-specific capability authority and
the fake-binary contract suite promised by the feature design.

Acceptance criteria:

- Marketplace registration is attempted only when the selected verified
  profile grants `marketplace.register` for the requested scope; plugin
  installation separately requires `plugin.install`.
- `crates/harnesses/tests/bootstrap.rs` uses isolated fake binaries to assert
  exact Claude marketplace/plugin vectors, user scope, canonical source,
  qualified identity, target isolation, present/missing/unknown observations,
  Codex unsupported behavior, and no cache writes.
- Tests cover capability narrowing, malformed version/list output, and an
  executable replacement between detection and mutation; replacement blocks
  the native mutation.

## Review origin

Fresh-context review of the hardened bootstrap harness commits `c880496` and
`85b56ea` found the marketplace capability check and promised fake-binary
coverage missing.
