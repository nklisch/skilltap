---
id: epic-safe-update-automation
kind: epic
stage: done
tags: [infra]
parent: null
depends_on: [epic-native-marketplace-plugin-lifecycle, epic-standalone-skill-lifecycle, epic-cross-harness-materialization]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-10
updated: 2026-07-12
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

## Design decisions

- **What makes an update safe enough for automatic application?** Evaluate the
  fresh semantic plan rather than version magnitude. Auto-apply only when the
  update adds no capability, component, authentication requirement,
  compatibility warning, partial result, drift overwrite, or other new user
  decision. Semver alone neither grants nor removes automatic safety.
- **How does the optional daemon run?** Install a service-manager timer that
  invokes one finite `skilltap daemon run` cycle per interval. Do not keep a
  resident skilltap scheduler or watcher process alive between cycles.
- **How are background results surfaced?** Record applied, pending, failed,
  unreachable, and contended results for `skilltap status` and ordinary
  service logs. Do not send desktop notifications in v3.
- **Does this epic require UI mockups?** No. Daemon lifecycle and update health
  are CLI, JSON, and service-manager surfaces.

## Anticipated child features

- Unified native-plugin and Git-revision update resolution
- Pins, per-resource policy, and compatibility re-evaluation
- Foreground plugin and skill update orchestration
- Safe-update plan filtering and result recording
- `launchd` and `systemd --user` service integration
- Daemon lifecycle, diagnostics, status, and recovery behavior

<!-- The design pass on each child feature will fill in real specifics. -->

## Decomposition

Updates are deliberately downstream of every resource lifecycle and
materialization contract.

### Child features

1. `epic-safe-update-automation-resolution` — resolve Git refs/native versions
   to concrete SHA/revision candidates without mutating — depends on
   `[epic-native-marketplace-plugin-lifecycle,
   epic-standalone-skill-lifecycle]`.
2. `epic-safe-update-automation-policy` — pins, update intent, compatibility
   re-evaluation, and no-new-decision safety classification — depends on
   `[epic-safe-update-automation-resolution,
   epic-cross-harness-materialization]`.
3. `epic-safe-update-automation-foreground` — foreground plugin/skill update
   plans, exact acknowledgments, and state recording — depends on
   `[epic-safe-update-automation-policy]`.
4. `epic-safe-update-automation-service` — finite launchd/systemd user-service
   integration and daemon run cycle — depends on
   `[epic-safe-update-automation-foreground]`.
5. `epic-safe-update-automation-diagnostics` — daemon status, contention,
   recovery, logs, and idempotent lifecycle commands — depends on
   `[epic-safe-update-automation-service]`.

## Decomposition note

Decomposition pre-existed — five child features already define the dependency
chain from revision resolution through policy, foreground application, service
integration, and diagnostics. The ordering is coherent because no automatic
update can be classified before a concrete upstream revision is resolved, and
daemon behavior must reuse the foreground safety boundary.

## Implementation Notes

- Resolution, policy, foreground planning, exact acknowledgment, and verified
  state recording are complete.
- Optional launchd/systemd user-service definitions, bounded manager lifecycle,
  and one finite `daemon run` cycle are complete. Daemon execution delegates to
  the existing native-plugin and Git-backed skill lifecycle paths.
- Typed daemon run records, ordinary/daemon status projections, and advisory
  recovery actions are complete. No daemon path acknowledges partial work,
  overwrites drift, or writes unmanaged resources.

## Review Record

- Inline deep review: **pass**. All five child features are done; the full
  workspace test suite and clippy with `-D warnings` pass.
