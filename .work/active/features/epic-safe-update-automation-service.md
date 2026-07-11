---
id: epic-safe-update-automation-service
kind: feature
stage: done
tags: []
parent: epic-safe-update-automation
depends_on: [epic-safe-update-automation-foreground]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Integrate User Update Services

Install finite launchd/systemd-user timers that invoke one bounded daemon cycle
using the same update application service as foreground commands.

## Architectural choice

Generate a deterministic service definition in core, then let a small CLI
adapter install/remove it through the documented user service manager. The
definition invokes the installed `skilltap daemon run` command once per timer
interval; skilltap does not keep a resident scheduler or background watcher.
The adapter never embeds secrets, repository contents, or shell command
strings. `daemon run` uses the same safe foreground update service and an empty
acknowledgment set.

## Design decisions

- **Which platform is supported?** macOS uses a per-user `launchd` LaunchAgent;
  Linux uses a per-user `systemd` service plus timer. Other platforms return a
  typed unsupported-platform result without writing files.
- **Where are definitions owned?** They live under the user's service-manager
  home (`~/Library/LaunchAgents` or `~/.config/systemd/user`) with a stable
  skilltap-specific name. Disable removes only that exact owned definition.
- **How are enable/disable operations made safe?** Render and validate the
  complete definition first, atomically write it, then invoke the manager with
  direct argv. Status reads only the owned definition and manager state; it
  never starts a process implicitly.

## Implementation Units

### Unit 1: Deterministic service definitions (trickiest unit)
**File**: `crates/core/src/daemon.rs`
**Story**: `epic-safe-update-automation-service-definition`

```rust
pub enum ServicePlatform { Launchd, SystemdUser }

pub struct DaemonServiceSpec {
    pub platform: ServicePlatform,
    pub interval: UpdateInterval,
    pub executable: AbsolutePath,
}

pub fn render_service(spec: &DaemonServiceSpec) -> Result<ServiceDefinition, ServiceRenderError>;
```

**Implementation Notes**:
- Emit a finite `daemon run` invocation with no shell interpolation and a
  stable label/unit name. Reject non-absolute executables and invalid
  intervals before rendering.
- Keep format-specific fields bounded and preserve no user secrets.

**Acceptance Criteria**:
- [ ] launchd and systemd-user definitions are deterministic for the same spec.
- [ ] Definitions contain exactly one finite `daemon run` invocation.
- [ ] Invalid paths/intervals/platforms fail before any write.

### Unit 2: User-service lifecycle adapter
**File**: `crates/cli/src/daemon.rs` and `crates/cli/src/entrypoint.rs`
**Story**: `epic-safe-update-automation-service-lifecycle`

```rust
pub fn enable_daemon(interval: Option<UpdateInterval>, json: bool) -> Outcome;
pub fn disable_daemon(json: bool) -> Outcome;
pub fn daemon_status(json: bool) -> Outcome;
```

**Implementation Notes**:
- Use atomic file publication and direct `launchctl`/`systemctl --user`
  argument vectors. Manager failures are typed attention results and do not
  delete the owned definition.
- Enable is idempotent; disable removes only the exact skilltap-owned file.

**Acceptance Criteria**:
- [ ] Enable, disable, and status are non-interactive and repeatable.
- [ ] Manager commands receive direct argv with no shell string.
- [ ] Unmanaged service files are never removed or overwritten.

### Unit 3: One bounded daemon cycle
**File**: `crates/cli/src/application.rs` and `crates/cli/src/entrypoint.rs`
**Story**: `epic-safe-update-automation-service-run`

```rust
pub fn run_daemon_cycle() -> Outcome;
```

**Implementation Notes**:
- Acquire the existing configuration lock, resolve registered revisions,
  build the foreground safe-update plan, apply only `Safe` entries, and
  re-observe before recording state.
- Pass no acknowledgment selectors; partial, drifted, pinned, or conflicted
  updates remain pending. The cycle terminates after one bounded pass.

**Acceptance Criteria**:
- [ ] A daemon cycle never supplies generic or piecewise acknowledgment.
- [ ] Lock contention and manager/source failures are recorded without hangs.
- [ ] Repeating a cycle is idempotent and does not overwrite drift.

## Implementation Order

1. `epic-safe-update-automation-service-definition`
2. `epic-safe-update-automation-service-lifecycle`
3. `epic-safe-update-automation-service-run`

## Testing

- Golden tests cover launchd/systemd definitions and rejection of unsafe
  executable paths or intervals.
- Lifecycle tests use isolated homes and fake managers to prove ownership,
  idempotence, and direct argv behavior.
- Daemon-cycle tests use the existing bounded resolver/executor fixtures and
  prove no acknowledgment or drift overwrite.

## Risks

Service managers differ in reload semantics and availability. The adapter
keeps definition publication separate from manager activation and reports
activation failure without deleting a valid owned definition, allowing a
later status/repair command to recover deterministically.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this
  autopilot run is intentionally single-agent.

## Implementation Notes

- All three service stories are complete: deterministic definitions,
  user-service lifecycle management, and one bounded safe daemon cycle.
- `daemon enable`, `disable`, `status`, and `run` are now real non-interactive
  commands. Service manager failures retain owned definitions and report
  attention; daemon cycles never supply acknowledgments.
- Targeted service/CLI tests and clippy pass. Full workspace verification is
  the remaining gate.

## Review Record

- Inline deep review: **pass** after the full workspace test and clippy gates.
  Ownership checks,
  direct argv, lock delegation, and finite-cycle behavior are explicit.
