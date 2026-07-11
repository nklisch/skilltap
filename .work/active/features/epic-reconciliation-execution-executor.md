---
id: epic-reconciliation-execution-executor
kind: feature
stage: drafting
tags: []
parent: epic-reconciliation-execution
depends_on: [epic-reconciliation-execution-graph]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Safely Execute Reconciliation Plans

Apply safe operations through generic native/filesystem ports under the
configuration lock. Revalidate affected identities and fingerprints, journal
planned/running/completed/failed results atomically in state, stop dependent
work after failure, preserve independent successes, and return a fresh recovery
plan after partial execution.
