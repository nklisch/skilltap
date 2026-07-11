---
id: epic-rust-control-plane-cli-maintainability-status-phases
kind: story
stage: implementing
tags: [refactor]
parent: epic-rust-control-plane-cli-maintainability
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Decompose Foundation Status Phases

Extract private typed document-load, scope, target, and outcome-projection
phases from `StatusApplication::execute`. Keep its crate-visible signature and
all output bytes, error/resource ordering, early returns, and filesystem effects
unchanged. Add focused phase tests only where they improve contract clarity and
run the locked/binary ladders.
