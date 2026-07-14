---
id: feature-daemon-marketplace-refresh-acceptance
kind: story
stage: implementing
tags: [infra, testing]
parent: feature-daemon-marketplace-refresh
depends_on: [feature-daemon-marketplace-refresh-execution]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Verify Daemon Marketplace Refresh End to End

## Checkpoint

Extend only the test-support-owned fake native process and isolated CLI fixtures
to verify the complete daemon marketplace-refresh contract. Record ordered,
bounded native invocations and provide deterministic action-specific failure and
revision controls without touching real harness, HOME, or XDG state.

Keep native dialects distinct: Claude marketplace update and Codex marketplace
upgrade may use different vectors while satisfying the same normalized ordering,
deduplication, dependency, target-locality, status, and idempotency behavior.

## Expected implementation surface

- `crates/test-support/src/native_process.rs`
- `crates/test-support/src/harness_profile.rs`
- `crates/cli/tests/compiled_binary.rs`
- `crates/cli/tests/native_postconditions.rs`

## Acceptance evidence

- Each supported native target refreshes the exact marketplace before updating
  its tracked plugin through direct argument vectors.
- Two plugins sharing one exact marketplace refresh it once per daemon cycle.
- Injected refresh command failure and indeterminate postcondition both prevent
  the dependent plugin invocation, persist actionable status, and allow an
  unrelated branch to finish.
- A multi-target fixture proves one target's failure does not mutate or block a
  sibling target.
- Plain and JSON output distinguish refresh, update, no-change, failure, and
  dependency-pending states without leaking raw native data.
- An immediate repeated clean cycle reports no resource changes while still
  performing the scheduled marketplace freshness check.
- Existing foreground lifecycle, postcondition, daemon policy, drift,
  source-failure, and lock-contention suites remain green.

## Ordering

Runs after the task graph and execution checkpoints and completes integrated
feature verification.
