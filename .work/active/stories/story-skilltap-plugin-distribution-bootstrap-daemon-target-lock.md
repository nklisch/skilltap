---
id: story-skilltap-plugin-distribution-bootstrap-daemon-target-lock
kind: story
stage: implementing
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-daemon-binary-policy]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Make daemon binary updates target and lock the running installation

The daemon binary policy now calls the foreground resolver and installer, but
it runs before the shared mutation lock and resolves the destination from a
default path. `daemon enable` records the exact executable used by the user
service, so a custom `SKILLTAP_INSTALL` path (or another current executable)
must remain the daemon's update target. Concurrent foreground and daemon
updates must share one bounded publication lock and must never install a second
unreferenced binary.

## Acceptance criteria

- The daemon derives the binary destination from the executable recorded by its
  service definition (or an equivalent validated persisted identity), while
  foreground bootstrap retains its explicit `SKILLTAP_INSTALL`/default
  behavior.
- Binary policy resolution and publication execute under the same cooperative
  update lock as foreground bootstrap; contention is a deterministic pending
  result and does not fetch or mutate.
- `off` performs no resolver/fetcher/installer call; `check` resolves once and
  never fetches or publishes; `apply-safe` installs/updates only a verified
  same-major artifact, blocks majors without persisted opt-in, and repeats as
  a no-op.
- Isolated daemon-boundary tests inject resolver, fetcher, installer, and
  lock ports to cover off/check/apply-safe, missing/unknown binaries,
  verification failure, major block/opt-in, custom destinations, lock
  contention, no duplicate network, and persisted pending/completed results.
- Existing plugin/skill lifecycle behavior and user-service definitions remain
  unchanged; no ambient environment or native cache is used as a write API.

## Review origin

Fresh-context review of `story-skilltap-plugin-distribution-bootstrap-daemon-binary-policy`
found that `execute_system_daemon_binary_policy` performs binary work before
the shared update lock and that the daemon's default destination can differ
from the `current_exe()` path captured by `daemon enable`. The policy branch is
also only covered by a direct helper test, not by injected daemon-boundary
coverage.

## Implementation notes

- Execution capability: highest; this crosses concurrent publication and the
  self-update security boundary.
- Review weight: standard (source: autopilot).
