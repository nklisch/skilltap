---
id: story-skilltap-plugin-distribution-bootstrap-harness
kind: story
stage: implementing
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-contract]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Harness detection and first-party plugin setup

Implement the read-first per-target setup adapter for the canonical skilltap
plugin. Reuse the existing executable detection, verified capability profiles,
native lifecycle argument builders, and conservative JSON observation. This
story must preserve the Codex contract gap rather than inventing a plugin cache
write path.

Scope:

- `crates/harnesses/src/bootstrap.rs` and module exports.
- `crates/cli/src/bootstrap.rs` adapter wiring needed by the application.
- `crates/harnesses/tests/bootstrap.rs` fake-binary tests.

Acceptance criteria:

- `all` probes Claude and Codex independently; absent/unusable targets produce
  bounded results without hiding a successful binary result for another target.
- Claude uses a verified user-scoped native marketplace/plugin lifecycle for
  the canonical skilltap source; existing presence is observed first and a
  healthy repeat is a no-op.
- Codex may register the canonical marketplace only when its verified profile
  grants that operation. The adapter never claims a plugin install when the
  host exposes only an interactive flow and returns an actionable unsupported
  result instead.
- Unknown or malformed native list/version output blocks mutation, raw payloads
  and secrets stay inside the adapter, and no harness cache is written.
- Fake binaries assert exact direct argument vectors, scope isolation, profile
  narrowing, present/missing/unknown observation, and unsupported Codex setup.

Do not add arbitrary third-party selectors, project scope, harness enablement,
or an undocumented post-install hook.

## Implementation notes
- Execution capability: highest available local capability; this crosses native harness contracts and trust boundaries.
- Review weight: standard (source: autopilot project default).
- Files changed: `crates/harnesses/src/bootstrap.rs`, `crates/harnesses/src/lib.rs`.
- Tests added: unknown-version observe-only and unreachable-target bounded-result tests; setup performs presence observation before any native mutation and returns actionable Codex/Claude next actions.
- Discrepancies from design: setup policy accepts the configured executable and canonical source as application-owned inputs, keeping target probing and native command composition in the adapter.
- Adjacent issues parked: none.
