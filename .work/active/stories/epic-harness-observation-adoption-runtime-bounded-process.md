---
id: epic-harness-observation-adoption-runtime-bounded-process
kind: story
stage: implementing
tags: [infra,correctness]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits, epic-harness-observation-adoption-runtime-adversarial-fixtures, epic-harness-observation-adoption-runtime-executable-resolution]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Run Bounded Native Processes

Replace unbounded observation command execution with direct `OsString` argv,
null stdin, explicit cleared environment, optional canonical cwd, and the
resolved absolute executable. Concurrently drain stdout/stderr while enforcing
per-stream and combined caps. On timeout or overflow, terminate the dedicated
Unix process group and always reap even when descendants retain pipe handles.
Return non-zero exit as a bounded result, revalidate executable identity just
before spawn, and keep all errors/output Debug-safe on Linux and macOS.
