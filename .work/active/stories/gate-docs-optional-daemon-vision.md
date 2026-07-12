---
id: gate-docs-optional-daemon-vision
kind: story
stage: review
tags: [documentation]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: docs
created: 2026-07-12
updated: 2026-07-12
---

# Document the optional update daemon

## Drift category

foundation-doc-assertion

## Location

- Doc: `docs/VISION.md:94-108`
- Code: `crates/cli/src/command.rs:327-342`, `crates/cli/src/entrypoint.rs:264-315`, `crates/cli/src/application.rs:649-733`, `crates/core/src/daemon.rs:88-128`

## Current doc text

> Non-Goals says `skilltap does not ... Run a background service.`

## Reality

The CLI exposes `daemon enable`, `daemon disable`, `daemon status`, and
`daemon run`. Enabling installs an optional launchd/systemd-user service and
running it performs a bounded update cycle.

## Required edit

Remove the daemon exclusion or qualify it as an optional user-level service
that is never required and never needs elevated privileges. Keep the limits
aligned with `docs/ARCH.md` and `docs/SPEC.md`.

## Implementation Notes

- Updated the Vision non-goals to distinguish an optional, explicitly enabled
  user-level update daemon from a required background service.
- The wording preserves the daemon safeguards documented by the architecture
  and specification: no elevated privileges and no bypass of acknowledgment,
  drift, or conflict handling.
- Verification: reviewed the corresponding daemon limits in `docs/ARCH.md` and
  `docs/SPEC.md`; `git diff --check` passed.
