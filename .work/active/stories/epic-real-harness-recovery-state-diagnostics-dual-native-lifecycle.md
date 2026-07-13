---
id: epic-real-harness-recovery-state-diagnostics-dual-native-lifecycle
kind: story
stage: done
tags: [correctness, testing]
parent: epic-real-harness-recovery-state-diagnostics
depends_on:
  - epic-real-harness-recovery-state-diagnostics-target-evidence
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Reconcile dual-native lifecycle without losing siblings

## Scope

Union target sets for sequential identical installs and drive install, update,
and removal from exact target bindings. Preserve unselected desired and
observed siblings, and prove that a plugin native to both harnesses never
falls back to a managed copy.

## Acceptance

- A Codex-only install followed by Claude-only install widens the same desired
  resource to both targets and mutates only the missing target.
- Repeating narrowed and target-all operations is a no-op.
- Target-all install/update/remove records and acts on separate native target
  evidence with no managed plugin artifact.
- Narrowed update/removal preserves the sibling inventory, installation,
  revision, provenance, ownership, and applicable journal evidence.
- A same-key definition conflict fails before inventory publication or native
  mutation.

## Implementation

- Sequential identical installs now union the existing and selected harness
  sets before inventory publication; conflicting definitions still fail through
  the existing strict inventory boundary.
- Per-target state seeds and journals mutate only selected bindings, preserving
  every unselected sibling field introduced by the target-evidence migration.
- Exact Codex/Claude fixtures prove sequential widening, two native bindings,
  and a repeated `--target all` no-op without a managed artifact.
- Existing narrowed update/removal and dual lifecycle coverage verifies sibling
  preservation. Codex `0.144.1` remains correctly update-unavailable; Claude
  update does not erase its Codex sibling.

## Verification

- `cargo test -p skilltap --test compiled_binary sequential_native_plugin_installs_widen_targets_and_preserve_bindings`
- Full compiled lifecycle suite.
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the project-default `standard` weight,
escalated for cross-target inventory and journal correctness. The exact
sequential widening regression passes at native integration commit `29afee5`,
including both target bindings and target-all repeat no-op. Existing narrowed
lifecycle coverage establishes sibling preservation, while the state model
keeps native ownership per target and creates no managed plugin artifact.
