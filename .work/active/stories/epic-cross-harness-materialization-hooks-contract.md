---
id: epic-cross-harness-materialization-hooks-contract
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-hooks
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Normalize Hook Contracts

Add validated hook contract types in core and bounded Codex/Claude readers in
the harness adapters.

Acceptance criteria:

- Event, payload, failure, cwd, environment references, and executable
  permission semantics are normalized without raw secret values.
- Malformed or unsafe hook declarations fail before mapping.
- Reader tests cover both harness fixtures and observation-only behavior.
