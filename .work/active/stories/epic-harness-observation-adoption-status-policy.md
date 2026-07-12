---
id: epic-harness-observation-adoption-status-policy
kind: story
stage: done
tags: [cli,infra]
parent: epic-harness-observation-adoption-status
depends_on: [epic-harness-observation-adoption-normalization]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Manage Harness Policy

Implement strict skilltap harness policy load and deterministic list/enable/
disable operations. Missing policy remains explicit and read-only until enable;
enable creates only skilltap config with the named harness, disable edits only
policy, and unknown/duplicate/disabled selections fail safely without touching
native state.

## Implementation notes

- Added a typed `ConfigDocument::with_harness_policy` update that changes only
  the selected harness and optionally records its validated executable override.
- Implemented deterministic `harness list`, `harness enable`, and
  `harness disable` dispatch paths. Missing config is read as defaults for
  listing and is created only by a real enable transition.
- Enable/disable writes are limited to skilltap's config repository; native
  harness files and resources are never opened for mutation.
- Repeated transitions are idempotent and unknown harness identifiers remain a
  typed invalid result.

## Verification

- `cargo test -p skilltap --all-targets --offline`
- `cargo test -p skilltap-core --all-targets --offline`
- Compiled-binary coverage verifies successful JSON list/enable/disable leaf
  commands alongside the existing unavailable-command contract.

## Review notes

## Review

Verdict: Approve - story verified by implement; fast-lane advance.

Observation-backed status remains in the dependent status-observation story.
