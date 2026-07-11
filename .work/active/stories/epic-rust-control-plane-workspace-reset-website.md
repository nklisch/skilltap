---
id: epic-rust-control-plane-workspace-reset-website
kind: story
stage: implementing
tags: [content, infra]
parent: epic-rust-control-plane-workspace-reset
depends_on: [epic-rust-control-plane-workspace-reset-workspace]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Isolate and Refresh the Website

## Scope

Implement Unit 4 from the parent feature: retain VitePress as an isolated npm
website, replace its Bun-specific generator, and reduce public documentation to
the current v3 control-plane model. The website must not participate in the
Rust product build or reintroduce retired behavior.

## Acceptance criteria

- [ ] Clean npm install and website build pass.
- [ ] Public pages and generated LLM content contain no retired v2 surface.
- [ ] The website installer copy matches root `install.sh`.
- [ ] Root Cargo commands remain independent of website dependencies.
