---
id: epic-harness-observation-adoption-adopt-persistence
kind: story
stage: done
tags: [infra,correctness]
parent: epic-harness-observation-adoption-adopt
depends_on: [epic-harness-observation-adoption-adopt-merge]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Publish Adoption Atomically

Acquire the configuration lock fail-fast, reload inventory, revalidate
selected observation identity/fingerprint evidence, rerun the pure plan, and
publish one atomic inventory replacement. Preserve unrelated entries and leave
native configuration, state.json, and managed artifacts untouched.

## Implementation notes

- Added the generic core `apply_adoption` port with fail-fast configuration
  locking, post-lock inventory reload, selected native identity/fingerprint
  revalidation, pure-plan rerun, and one atomic inventory replacement.
- Empty and repeated plans are logical no-ops; stale evidence fails before any
  publication and native/config/state/managed-artifact boundaries remain
  untouched.
- Added memory-backed tests for one-write publication, repeat idempotence, and
  stale-observation rejection.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap-core adoption::tests --offline`
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings`

## Review

Verdict: Approve - story verified by implement; fast-lane advance.
