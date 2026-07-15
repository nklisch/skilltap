---
id: epic-expanded-harness-support-declaration-managed-daemon-safety
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-declaration-managed
depends_on: [epic-expanded-harness-support-declaration-managed-planner-acknowledgment, epic-expanded-harness-support-declaration-managed-execution-status]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-15
gate_origin: null
created: 2026-07-15
updated: 2026-07-15
---

# Keep Declaration-Managed Work Out of the Daemon

## Checkpoint

Classify declaration-managed updates as pending before the daemon constructs a
managed entry, state seed, executable request, or execution acknowledgment.
Supported independent work remains eligible.

## Design element

- Add a typed daemon pending reason that distinguishes declaration-managed/
  acknowledgment-required work from drift, conflict, and ordinary unsupported
  work.
- Resolve the exact profile and component authority before daemon route
  construction.
- Return declaration-managed work as pending and create no checkout mutation,
  managed entry, state seed, operation journal, or effective probe.
- Keep the daemon on `execute_plan` with empty acknowledgments. It cannot call
  the foreground acknowledgment constructor and its CLI continues to reject
  `--yes`.
- Preserve dependency behavior: a declaration prerequisite keeps its dependents
  pending/skipped while independent Supported siblings may apply.
- Render stable target/scope/resource/reason output without raw paths, argv,
  settings bytes, or secret material.

## Acceptance evidence

- Daemon install/update/remove candidates requiring declaration acknowledgment
  never alter target files, inventory, target bindings, or journals.
- Supported independent marketplace/plugin/skill updates still run.
- Dependents do not bypass a pending declaration prerequisite.
- Unknown versions, drift, collisions, required unsupported components, trust,
  auth, and native `Unverified` never become daemon-safe.
- Repeated daemon runs are byte-for-byte no-ops for pending declaration work and
  retain actionable status.

## Ordering constraint

Depends on the executor/status contract so daemon classification and output use
the same authority and declared/effective semantics.

## Implementation notes

- Added typed `DaemonPendingReason` and `DaemonPendingUpdate` evidence in core.
- Daemon planning checks exact managed projection authority before lifecycle
  route selection. Declaration-managed work is recorded as pending and does
  not resolve a checkout, create an entry/seed, invoke an effective probe, or
  enqueue an executor request.
- Pending marketplace prerequisites propagate to their dependent plugin work;
  unrelated supported updates continue through the existing empty-acknowledgment
  executor path.
- Verification: `cargo fmt --all && cargo test --workspace --all-targets`
  (703 passed).

## Completion

Implemented and verified. Migration and integrated acceptance remain.
