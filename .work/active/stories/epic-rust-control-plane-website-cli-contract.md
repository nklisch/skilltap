---
id: epic-rust-control-plane-website-cli-contract
kind: story
stage: implementing
tags: [content, documentation]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-cli-shell]
release_binding: null
research_refs: []
research_origin: null
gate_origin: docs
created: 2026-07-11
updated: 2026-07-11
---

# Align the Public CLI Result Contract

Update the public website CLI reference to the authoritative foundation and
implemented contract: plain result labels, schema-1 JSON fields, and exit codes
`0`–`3`. Remove obsolete top-level `targets`/`exit_code` claims, regenerate
`website/public/llms-full.txt` through the repository's generator, and verify
the website build and exact generated-output check. Do not change product
behavior or foundation docs.
