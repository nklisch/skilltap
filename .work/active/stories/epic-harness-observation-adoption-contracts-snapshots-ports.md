---
id: epic-harness-observation-adoption-contracts-snapshots-ports
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-storage-wires, epic-harness-observation-adoption-contracts-findings, epic-harness-observation-adoption-contracts-installation-profiles]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Ephemeral Snapshot and Adapter Ports

Add one-concrete-scope requests, harness observations, partial environment
snapshots, safe adapter errors, and behavior ports for harness adapters and the
shared coordinator. Bind installation/profile evidence to one executable and
expose normalized resources/findings only; no native DTO, I/O, persistence, or
CLI dependency enters core.
