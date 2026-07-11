---
id: epic-harness-observation-adoption-runtime-external-tree
kind: story
stage: implementing
tags: [infra,correctness]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits, epic-harness-observation-adoption-runtime-adversarial-fixtures]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Observe External Trees Without Following Links

Add a bounded descriptor-relative external tree observer separate from managed
artifact APIs. Traverse directories deterministically, read bounded regular
files, and report symlinks with bounded opaque targets without following them.
Reject FIFO/socket/device, non-UTF-8, raced, over-depth, over-entry, per-file,
and total-byte cases while walking. Verify parent/name/file identity before and
after open/read and preserve portable Linux/macOS errno behavior.
