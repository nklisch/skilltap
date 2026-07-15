---
id: gate-patterns-inconsistency-codex-drift-shape
kind: story
stage: drafting
tags: [refactor]
parent: null
depends_on: [gate-patterns-inconsistency-managed-projection-sharing]
release_binding: null
gate_origin: patterns
created: 2026-07-15
updated: 2026-07-15
---

# Normalize Codex managed-projection drift verification

Codex performs skill and MCP drift comparisons inline inside `plan_codex_component_projections` and `plan_codex_mcp_config`, while the other managed adapters route equivalent ownership checks through focused verification helpers.

Once shared projection planning has converged, move Codex onto the same explicit verification boundary or document why its component model requires a distinct shape. Preserve owned-versus-unowned conflict semantics, fingerprint ordering, diagnostics, and all observable lifecycle behavior.
