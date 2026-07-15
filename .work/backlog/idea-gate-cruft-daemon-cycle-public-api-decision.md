---
id: idea-gate-cruft-daemon-cycle-public-api-decision
created: 2026-04-02
updated: 2026-07-15
tags: [cleanup]
release_binding: null
gate_origin: cruft
---

# Decide whether to remove the unused daemon-cycle API

## Confidence
High

## Relevance
Release-relevant discovery, but unbound because public API removal requires an explicit compatibility decision.

## Location
`crates/core/src/daemon.rs:79` and `crates/core/src/daemon.rs:136`

`DaemonCyclePlan` and `plan_daemon_cycle` have no production caller; only their in-module test uses them. Production daemon execution uses `plan_daemon_native_updates` and `DaemonNativeUpdatePlan` instead.

Confirm external compatibility expectations before deleting the struct, function, and isolated test. Keep `DaemonPendingUpdate` and `DaemonPendingReason`, which remain live.
