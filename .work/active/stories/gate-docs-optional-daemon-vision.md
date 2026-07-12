---
id: gate-docs-optional-daemon-vision
kind: story
stage: implementing
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
