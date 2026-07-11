---
id: epic-safe-update-automation
kind: epic
stage: drafting
tags: [infra]
parent: null
depends_on: [epic-native-marketplace-plugin-lifecycle, epic-standalone-skill-lifecycle, epic-cross-harness-materialization]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Safe Update Automation

## Brief

Unify foreground and optional background updates for managed native plugins,
materialized plugins, and Git-backed skills. This epic resolves upstream
versions and revisions, honors resource pins and policy, re-evaluates changed
artifacts for compatibility, and applies only plans that require no new user
decision.

The optional daemon is a user-level `launchd` or `systemd --user` entry point to
the same update application service used by foreground commands. It never
supplies acknowledgment, overwrites drift, resolves conflicts, modifies
unmanaged resources, or gains additional mutation authority.

## Foundation references

- `docs/SPEC.md` — Update Daemon, Mutation Safety, Platform Contract
- `docs/ARCH.md` — Updates, Optional Daemon, Concurrency
- `docs/HARNESS-CONTRACTS.md` — Version and Update Contract, Unknown Harness Versions
- `docs/UX.md` — Updates, Daemon

## Anticipated child features

- Unified native-plugin and Git-revision update resolution
- Pins, per-resource policy, and compatibility re-evaluation
- Foreground plugin and skill update orchestration
- Safe-update plan filtering and result recording
- `launchd` and `systemd --user` service integration
- Daemon lifecycle, diagnostics, status, and recovery behavior

<!-- The design pass on each child feature will fill in real specifics. -->
