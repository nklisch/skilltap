---
id: epic-cross-harness-materialization-graph-readers
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-graph
depends_on: [epic-cross-harness-materialization-graph-contract]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Read Explicit Native Plugin Sources

Implement `crates/harnesses/src/plugin_graph.rs` for Codex and Claude using
the reader port and existing bounded filesystem/process adapters. Parse only
the documented manifest and component paths of the explicitly supplied source.

Acceptance criteria:

- Fixture plugins produce complete normalized declarations for documented
  skills, MCP servers, hooks, and target-specific components.
- Missing or malformed manifests fail without returning a partial graph.
- Cache and marketplace browsing are not used as write or discovery surfaces.
- Tests assert bounded native arguments, explicit source scoping, and unknown
  field preservation at the adapter boundary.

## Implementation notes

- Files changed: `crates/harnesses/src/plugin_graph.rs`,
  `crates/harnesses/src/lib.rs`.
- Tests added: Codex/Claude fixture reader tests for complete skills, known
  components, malformed manifests, and remote catalog rejection.
- Discrepancies from design: readers take an explicit checked-out root so the
  composition layer can resolve Git sources without granting the reader any
  browse or checkout authority; only documented component directories/files
  are normalized and skills are required while other component kinds default
  optional pending manifest-level requiredness evidence.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-harnesses --offline` — passed.
- `cargo clippy -p skilltap-harnesses --all-targets --offline -- -D warnings`
  — passed.

## Review

Verdict: Approve — story verified by implement; fast-lane advance.

## Review follow-up

The feature-level completeness pass identified MCP coverage as an acceptance
gap. Added strict bounded parsing for documented MCP manifest files and a
fixture assertion for named MCP servers; targeted harness tests and clippy
remain green.
