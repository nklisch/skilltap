---
id: epic-harness-observation-adoption-contracts-managed-ownership
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-key]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Migrate Managed Artifact Ownership

Use exact `ResourceKey` owners throughout managed records, repository ports,
handles, errors, residuals, serde, and canonical artifact/backup path hashing.
Prove same logical ID in different scopes never aliases and owner mismatch fails
before filesystem I/O.
