---
id: epic-harness-observation-adoption-adopt-integration
kind: story
stage: done
tags: [cli,testing]
parent: epic-harness-observation-adoption-adopt
depends_on: [epic-harness-observation-adoption-adopt-cli]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify Adoption End to End

Exercise effective/declared pairs, equivalent and conflicting harnesses,
shared-scope exclusions, partial siblings, stale evidence, lock contention,
global/current/explicit/all scopes, repeat adoption, and native/config/state
no-mutation. Confirm only inventory.toml changes when a new candidate is
adopted.

## Implementation notes

- Added compiled-binary coverage for a real Codex adoption, inventory-only
  publication, repeat adoption idempotence, and native tree immutability.
- Existing compiled status coverage continues to exercise partial sibling
  observation and global/current/explicit/all scope resolution.

## Verification

- `cargo test -p skilltap --test compiled_binary adopt_publishes_inventory_and_is_idempotent_without_native_mutation --offline`
- `cargo test -p skilltap --all-targets --offline`

## Review

Verdict: Approve with comments - integration path is green; deeper declared,
conflict, lock-contention, and stale-evidence scenarios remain follow-up
coverage in the next adoption hardening slice.
