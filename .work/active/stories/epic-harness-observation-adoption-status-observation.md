---
id: epic-harness-observation-adoption-status-observation
kind: story
stage: review
tags: [cli,infra]
parent: epic-harness-observation-adoption-status
depends_on: [epic-harness-observation-adoption-status-policy]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Compose Observation-Backed Status

Replace CLI status placeholders with exact scope/target resolution and
normalized Codex/Claude environment results. Report reachability, version,
profile authority, capabilities, resources, findings, and partial sibling
success while expanding only requested scopes and never scanning or writing.

## Implementation notes

- Status now resolves every requested scope and enabled target into a bounded,
  read-only native observation attempt. It reports target/scope entries with
  reachability, native version, profile authority, capability support counts,
  and bounded native-tree entry counts.
- Configured harness binaries are honored for both `PATH` lookup and absolute
  paths through the harness detection adapter. Detection and observation
  failures remain sibling-local warnings and do not prevent other targets from
  being reported.
- Global native trees use the documented Codex/Claude observation adapters.
  Project status deliberately does not recursively scan arbitrary project
  content; it observes only documented `.agents`, `.codex`, and `.claude`
  project roots through dedicated bounded adapters.
- Status never writes state or native configuration and no longer emits the
  `native_observation_unavailable` placeholder.

## Verification

- `cargo fmt --all`
- `cargo check -p skilltap --all-targets --offline`
- `cargo test -p skilltap --all-targets --offline`
- `cargo test -p skilltap-harnesses --test detection --offline`
- `cargo clippy -p skilltap --all-targets --offline -- -D warnings`

Stage: review
