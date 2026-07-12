---
id: epic-real-harness-recovery-bootstrap-transport
kind: feature
stage: drafting
tags: [correctness, security, infra, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Repair bootstrap release transport

## Brief

Correct release HTTP status/redirect parsing so successful non-redirected
responses are accepted while every redirect hop retains the existing host,
checksum, size, and atomic-publication protections. Bootstrap and the binary
update lifecycle must work in an isolated home against the current release
manifest and repeat without change.

This feature owns blocker inventory entry 11 and the direct clean-room
bootstrap failure. Harness plugin setup remains in the runtime/native lifecycle
features.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: independent release-distribution repair.

## Foundation references

- `docs/SPEC.md` — self-hosted plugin distribution and update daemon.
- `docs/ARCH.md` — plugin publication and verified artifact boundary.

