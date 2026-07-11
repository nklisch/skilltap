---
id: epic-rust-control-plane-domain-maintainability-validated-newtypes
kind: story
stage: implementing
tags: [refactor]
parent: epic-rust-control-plane-domain-maintainability
depends_on: [epic-rust-control-plane-domain-maintainability-resource-tests, epic-rust-control-plane-domain-maintainability-operation-tests]
release_binding: null
gate_origin: refactor-design
created: 2026-07-11
updated: 2026-07-11
---

# Consolidate Validated String Newtypes

Introduce crate-private support for repeated validated string-newtype
constructor/display/serde/accessor behavior and migrate only exact matches.
Preserve public APIs, trait sets, validation/normalization order, error text, and
wire forms. Leave custom paths, Git hashes, fingerprints, and leaky exceptions
bespoke. Full golden and locked workspace tests must remain unchanged.
