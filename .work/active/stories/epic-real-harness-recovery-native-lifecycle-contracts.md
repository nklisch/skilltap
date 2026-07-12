---
id: epic-real-harness-recovery-native-lifecycle-contracts
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Attest exact native profiles and command vectors

## Scope

Replace the synthetic shared profile with exact current Codex and Claude
contracts, and build every lifecycle/list vector from operation-specific scope
rules. This story owns blocker 2 and blockers 5-7.

## Acceptance

- Codex `0.144.1` and Claude Code `2.1.201` select only their attested
  capabilities; `3.0.0`, adjacent, cross-harness, and unknown versions remain
  observe-only.
- Claude plugin/marketplace list and marketplace update omit unsupported
  `--scope`; only commands that accept it receive `user` or `local`.
- Codex `0.144.1` never emits `plugin update`; the operation is explicitly
  unavailable unless a future exact profile supplies an attested strategy.
- Fake-native grammar, adapter tests, compiled-binary tests, and
  `docs/HARNESS-CONTRACTS.md` agree with disposable real-CLI evidence.
- All vectors retain direct arguments, bounded execution, explicit environment,
  and the exact global/project working directory.
