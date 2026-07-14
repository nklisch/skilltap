---
id: epic-expanded-harness-support-native-coexistence-acceptance
kind: story
stage: implementing
tags: [testing]
parent: epic-expanded-harness-support-native-coexistence
depends_on: [epic-expanded-harness-support-native-coexistence-factory, epic-expanded-harness-support-native-coexistence-qwen, epic-expanded-harness-support-native-coexistence-copilot]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Prove Integrated Native-Managed Coexistence

## Checkpoint

Close the feature with reusable and compiled-binary acceptance evidence for
Factory Droid, Qwen Code, and GitHub Copilot CLI. Exercise each adapter's native
contract and managed fallback independently, then prove mixed representations,
target-local ownership, effective-state constraints, partial failure recovery,
and immediate-repeat idempotency across all three targets.

## Files

- `crates/test-support/src/harness_profile.rs`
- `crates/test-support/src/managed_acceptance.rs`
- `crates/test-support/src/integration.rs`
- `crates/harnesses/tests/detection.rs`
- `crates/harnesses/tests/lifecycle_scope.rs`
- `crates/harnesses/tests/normalization.rs`
- `crates/harnesses/tests/native_coexistence.rs` (new)
- `crates/cli/src/application/tests.rs`
- `crates/cli/tests/compiled_binary.rs`

## Required behavior

- Extend `FakeHarnessProfile` with `droid`, `qwen`, and `copilot` profiles that
  declare version response, lifecycle dialect, native roots, global/project
  skill roots, MCP documents, reload/effective probe, and managed projection
  profile. Do not create another target registry or target-id branch in generic
  snapshots.
- Run the existing `acceptance_matrix` and `managed_acceptance_matrix` for all
  three adapters. Add a focused coexistence matrix for representation selection,
  marketplace inheritance, target-local state, and native/managed collision
  behavior.
- Use only test-support-owned temporary homes, configuration roots, projects,
  source checkouts, fake executables, and credentials-as-references. Never read
  or mutate the operator's real harnesses, HOME, XDG state, trust files, caches,
  or repository.
- Snapshot native trees without following symlinks. Every mutating scenario
  repeats immediately and expects no operation, artifact rewrite, state entry,
  native invocation, or link inode change.

## Acceptance evidence

- `TargetRegistry::canonical()` and compiled help/config/`--target all` include
  `droid`, `qwen`, and `copilot`; `first_party_targets()` remains exactly Codex
  and Claude.
- Exact validated profiles pass detection and scoped capabilities; nearby and
  unknown versions remain observe-only.
- Every adapter passes both scopes for complete skills, MCP, structured/effective
  observation, reload/session behavior, drift, update, removal, and repeat.
- Droid/Qwen project standalone skills are relative links to canonical
  `.agents/skills`; Copilot is `NotRequired`. Complete siblings, modes, and
  unmanaged native-only skills survive.
- A selected plugin installed natively on Droid, managed on Qwen, and natively
  on Copilot retains three distinct native ids/revisions or managed manifests,
  ownership classes, fingerprints, and journals through update/removal.
- Equal native/managed names never coalesce. Removing one representation leaves
  the other and all unselected target bindings unchanged.
- Factory user-over-project MCP shadowing, Qwen restart-required state, and
  Copilot trust/enterprise blocks produce distinct stable findings and matching
  plain/JSON results.
- Unsupported required components block even with acknowledgment. Optional loss
  remains operation-scoped and requires foreground acknowledgment; the daemon
  never acknowledges or applies it.
- A native mutation failure records completed/failed operations, skips only
  dependents, re-observes exact state, and emits a deterministic recovery plan.
- `cargo test --workspace --all-targets`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo fmt --all -- --check`, and `git diff --check` pass.

## Ordering

Depends on all three adapter checkpoints. It is the integrated feature evidence,
not an independent implementation worker or a review stage. On green evidence,
this child advances directly to done and the parent feature becomes eligible
for its one standard independent review pass.
