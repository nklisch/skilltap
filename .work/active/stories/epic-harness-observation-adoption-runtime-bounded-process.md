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
per-stream and combined caps through nonblocking owned readers. On timeout or
overflow, terminate the dedicated Unix process group and always reap. Apply a
hard post-kill drain deadline and close parent read descriptors so even a
`setsid`-escaped descendant retaining pipe handles cannot block completion.
Return non-zero exit as a bounded result, revalidate executable identity just
before spawn, and keep all errors/output Debug-safe in native Linux and macOS
behavior suites.
