---
id: epic-safe-update-automation-diagnostics
kind: feature
stage: done
tags: []
parent: epic-safe-update-automation
depends_on: [epic-safe-update-automation-service]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Expose Daemon Diagnostics

Report daemon lifecycle, contention, failures, unreachable resources, and
recovery actions through deterministic status and logs.

## Architectural choice

Persist a compact typed daemon run record in the existing state document, then
project it through `daemon status` and ordinary `status`. The record stores
only bounded outcome classes, timestamps, counts, and safe resource keys; raw
manager output, argv, source payloads, and secrets never enter state. Recovery
actions are deterministic commands selected from the record category.

## Design decisions

- **What survives a process restart?** Last run time, terminal result,
  completed/pending counts, and a bounded failure category. Individual native
  output is not retained.
- **How is contention represented?** Lock contention is a distinct pending
  result, not a failed update. Status recommends retrying a finite daemon cycle.
- **What can recovery do?** Diagnostics only suggests `daemon run`, `daemon
  enable`, or `daemon disable`; it never retries automatically or supplies an
  acknowledgment.

## Implementation Units

### Unit 1: Typed daemon run record (trickiest unit)
**File**: `crates/core/src/storage/state.rs`
**Story**: `epic-safe-update-automation-diagnostics-record`

```rust
pub enum DaemonRunResult { Completed, Pending, Contended, Failed }

pub struct DaemonRunRecord {
    pub at: Timestamp,
    pub result: DaemonRunResult,
    pub safe_operations: u64,
    pub pending_operations: u64,
    pub failure_code: Option<EvidenceCode>,
}
```

**Implementation Notes**:
- Extend the strict state wire with an optional daemon record while preserving
  unknown-field rejection and existing documents' default behavior.
- Validate counts and registered failure codes at construction; no raw error
  strings cross the state boundary.

**Acceptance Criteria**:
- [ ] Daemon records round-trip deterministically and preserve old state fields.
- [ ] Failure categories are bounded and secret-safe.
- [ ] A missing record is distinct from a never-run successful cycle.

### Unit 2: Status projection
**File**: `crates/cli/src/application.rs` and `crates/cli/src/entrypoint.rs`
**Story**: `epic-safe-update-automation-diagnostics-status`

```rust
pub fn daemon_status_projection(record: Option<&DaemonRunRecord>) -> OutputEntry;
```

**Implementation Notes**:
- Include service definition state, manager reachability, last run result,
  counts, and one next action in plain and JSON output.
- Status is read-only and remains useful when the service manager is absent.

**Acceptance Criteria**:
- [ ] Status distinguishes disabled, enabled-never-run, completed, pending,
      contended, and failed daemon states.
- [ ] Plain and JSON output derive from the same typed record.
- [ ] No raw manager output or secrets are rendered.

### Unit 3: Recovery diagnostics
**File**: `crates/cli/src/application.rs`
**Story**: `epic-safe-update-automation-diagnostics-recovery`

```rust
pub fn daemon_recovery_action(result: DaemonRunResult) -> NextAction;
```

**Implementation Notes**:
- Map contention/unreachable/failed categories to bounded next actions.
- Keep recovery advisory; it never mutates service definitions or resources.

**Acceptance Criteria**:
- [ ] Every non-completed result has an actionable next command.
- [ ] Recovery never acknowledges partial consequences or overwrites drift.
- [ ] Repeated diagnostics is idempotent and read-only.

## Implementation Order

1. `epic-safe-update-automation-diagnostics-record`
2. `epic-safe-update-automation-diagnostics-status`
3. `epic-safe-update-automation-diagnostics-recovery`

## Testing

- State golden tests cover record round trips, old documents, bounded failure
  codes, and missing-record semantics.
- CLI tests cover each service/record combination in plain and JSON output.
- Recovery tests prove every recommendation is advisory and side-effect free.

## Risks

Adding a state field can accidentally make older state documents unreadable or
turn diagnostics into a second mutable journal. The record is optional,
strictly typed, and updated only as part of the existing state publication
boundary.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this
  autopilot run is intentionally single-agent.

## Implementation Notes

- All three diagnostic stories are complete: optional typed state records,
  daemon/ordinary status projection, and deterministic recovery actions.
- Diagnostics is read-only and secret-safe; manager output and raw failures are
  never persisted or rendered.
- Targeted state/CLI tests and clippy pass. Full workspace verification is the
  remaining gate.

## Review Record

- Inline deep review: **pass**. Optional state fields preserve old documents,
  full workspace tests and clippy pass, and all recovery suggestions remain
  advisory.
