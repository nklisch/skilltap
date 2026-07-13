---
id: gate-cruft-unused-catalog-mutation-api
kind: story
stage: review
tags: [cleanup]
parent: null
depends_on: []
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: cruft
created: 2026-07-12
updated: 2026-07-12
---

# Remove abandoned catalog mutation API

Remove `ManagedCodexCatalog::with_local_plugin`, `without_plugin`, and their
private mutable lookup helper. Production now validates and preserves the
selected catalog while projecting effective skills and MCP configuration; it
does not rewrite catalog entries toward a copied plugin directory.

## Implementation Notes

- Removed the unused mutation methods, their self-referential unit assertions,
  and the now-unused `RelativeArtifactPath` import.
- Retained source containment, duplicate detection, unknown-field preservation,
  and `into_bytes` behavior used by production.
- Verification: `cargo test -p skilltap-harnesses managed_codex_project
  --offline` passed (2 focused tests).
