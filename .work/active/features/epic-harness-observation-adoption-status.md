---
id: epic-harness-observation-adoption-status
kind: feature
stage: review
tags: [cli]
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-normalization]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Harness Management and First-Use Status

Replace CLI placeholders with `harness list`, `harness enable`, `harness
disable`, and observation-backed `status`. Missing config remains explicit:
status detects both known harnesses but reports neither user-enabled, while
enable creates config with only the named harness enabled and never touches
native state. Expand only requested global/project/inventory-recorded scopes,
report reachability/version/profile/capabilities/resources/findings and partial
sibling success, preserve JSON/plain/exit contracts, and prove every list/status
path creates or writes nothing.

## Design

Status composes persisted skilltap policy with the normalized ephemeral
Codex/Claude environment. Bare scoped commands remain global by default;
`--project` targets the current/explicit project and `--all-scopes` expands only
managed scopes. `--target` selects one or both harnesses. Missing config is a
read-only attention report; `harness enable` creates only skilltap config and
never native files. `disable` edits only policy and fails clearly for unknown
or already-disabled names.

Plain and `--json` output derive from one typed result with stable channels and
exit classes. Partial sibling success is visible without turning healthy
observations into failure. Status never scans marketplaces, writes caches,
creates paths, or mutates native settings.

## Design decisions

- **First use**: status detects known harnesses but reports none enabled and
  creates nothing; enable is the explicit write operation.
- **Scope expansion**: status resolves only requested global/current/explicit
  project scopes plus inventory-recorded scopes under `--all-scopes`.
- **Output**: every plain/JSON field is derived from one redacted result model;
  exit code depends only on result class.

## Implementation notes

- Harness policy commands now load missing config as read-only defaults, create
  config only on explicit enable, preserve binary overrides, and make repeated
  transitions byte/mtime stable.
- Status resolves exact scopes and enabled targets, detects configured native
  binaries, selects observe-only or verified profiles, and reports bounded
  native-tree observations with sibling-local warnings.
- First-use status now performs read-only detection of both known harnesses and
  reports their disabled/reachable state without creating configuration.
- Project observations are limited to documented `.agents`, `.codex`, and
  `.claude` roots; status never recursively walks arbitrary project content or
  writes native/state files.
- Compiled CLI coverage verifies plain/JSON output, scope/target selection,
  first-use no-create, partial sibling success, idempotence, and native
  byte/type/symlink/mtime preservation.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --offline -- -D warnings`
- `cargo test -p skilltap --all-targets --offline`
- `cargo test -p skilltap-harnesses --all-targets --offline`

## Review findings

Deep review requested changes. The remaining implementation is intentionally
not marked complete:

- Status now composes `ObservationBatch`/`HarnessObservation` outcomes through
  `normalize_observations`, emitting stable typed surface identities, kinds,
  health, and capability findings while preserving failed siblings.
- Canonical observation roots are named and bounded for Codex `.agents/skills`,
  Codex plugin/skill roots, project `.agents`/`.codex`, and Claude plugin/skill
  roots; unrelated parent files are excluded. Top-level instruction/config
  details remain typed surfaces rather than copied payloads.
- Desired-vs-observed resource counts now produce conservative attention
  warnings without mutation. Remaining harness-list health detail and deeper
  native config/settings parsing are follow-up work for lifecycle features.

Local policy fixes in this pass make already-disabled harness changes explicit
errors, preserve command names on storage failures, and isolate application
unit tests from the host native environment. The feature remains implementing
until the normalized observation and comparison work is delivered.

## Implementation units

1. `epic-harness-observation-adoption-status-policy` — implement strict
   harness policy load, enable/disable/list writes, and first-use semantics —
   depends on `[epic-harness-observation-adoption-normalization]`.
2. `epic-harness-observation-adoption-status-observation` — compose exact scope,
   target, normalized observation, reachability, capabilities, resources, and
   findings — depends on `[epic-harness-observation-adoption-status-policy]`.
3. `epic-harness-observation-adoption-status-integration` — verify plain/JSON,
   scopes/targets, partial success, first-use no-create, idempotence, and safe
   diagnostics — depends on
   `[epic-harness-observation-adoption-status-policy,
   epic-harness-observation-adoption-status-observation]`.
4. `epic-harness-observation-adoption-status-normalized` — complete typed
   normalized resource/finding projection and desired-vs-observed comparison —
   depends on `[epic-harness-observation-adoption-status-integration]`.

## Acceptance criteria

- Harness list/enable/disable and status are deterministic, non-interactive,
  scope/target exact, and preserve JSON/plain/exit contracts.
- First-use status is read-only; enable/disable touch only skilltap policy.
- Status exposes normalized observations and partial sibling findings without
  marketplace discovery or native writes; repeated reads are no-op.
