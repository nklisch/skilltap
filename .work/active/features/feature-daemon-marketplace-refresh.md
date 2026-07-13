---
id: feature-daemon-marketplace-refresh
kind: feature
stage: drafting
tags: [infra]
parent: null
depends_on: [epic-safe-update-automation]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Refresh Marketplaces During Daemon Updates

## Brief

Close the omitted first step in the approved daemon update lifecycle. A finite
`skilltap daemon run` currently updates tracked native plugins and Git-backed
skills, but it does not refresh the registered marketplace metadata those
plugin updates resolve through. Build marketplace refresh tasks from managed
inventory and execute them before dependent plugin updates through the same
bounded native lifecycle, lock, postcondition, journal, and status boundaries
used by foreground commands.

The daemon still has no acknowledgment authority. A refresh that is
unsupported, unreachable, drifted, or otherwise requires judgment remains
visible as pending or failed; it cannot silently broaden plugin update
authority. Independent marketplaces may progress incrementally, while a plugin
whose required marketplace refresh did not complete must not update from stale
metadata.

## Strategic decisions

- **Was marketplace refresh intentionally part of daemon updates?** Yes. It is
  the first step in `docs/SPEC.md` and was omitted from the current task
  assembly rather than removed from product scope.
- **What ordering is required?** Refresh each plugin's registered marketplace
  before resolving or applying that plugin update; unrelated resources retain
  incremental progress.
- **Does the daemon gain new authority?** No. It reuses foreground lifecycle
  capabilities with no `--yes` or partial acknowledgment.

## Foundation references

- `docs/SPEC.md` — marketplace lifecycle, plugin update, and daemon interval order.
- `docs/ARCH.md` — shared foreground/daemon application services and locking.
- `docs/UX.md` — unattended update diagnostics and pending decisions.

## Acceptance direction

- A tracked plugin update issues one verified marketplace refresh before its
  native plugin update when the target supports both operations.
- Refresh failure or indeterminate postcondition prevents dependent plugin
  mutation and is recorded for `status` without blocking unrelated resources.
- Duplicate plugins sharing a marketplace do not refresh it redundantly in one
  cycle.
- Repeating a no-change daemon cycle is idempotent and reports no changes.
- JSON and plain output identify refresh, update, pending, and failure results
  without leaking native process output.
