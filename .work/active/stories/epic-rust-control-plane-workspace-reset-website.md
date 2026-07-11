---
id: epic-rust-control-plane-workspace-reset-website
kind: story
stage: done
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

- [x] Clean npm install and website build pass.
- [x] Public pages and generated LLM content contain no retired v2 surface.
- [x] The website installer copy matches root `install.sh`.
- [x] Root Cargo commands remain independent of website dependencies.

## Implementation notes

- Files changed: `website/**` and `.github/workflows/deploy.yml`.
- Tests added: none; this story is verified through the clean website build,
  deterministic generator comparison, installer comparison, workflow parse,
  and root Cargo metadata check.
- Discrepancies from design: the deployment workflow also moved from Bun to
  Node/npm so the production website path observes the same isolation contract.
- Adjacent issues parked: none.
- Verification: `npm --prefix website ci`, two consecutive
  `npm --prefix website run build` runs with identical `llms-full.txt` hashes,
  `cmp install.sh website/public/install.sh`, Ruby YAML parsing of the deploy
  workflow, `cargo metadata --no-deps --format-version 1`, retired-surface
  searches, and `git diff --check` all passed.

## Review (2026-07-11)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: `idea-vitepress-security-upgrade` tracks upstream VitePress,
Vite, and esbuild development-server advisories for which npm currently offers
no compatible fix.
**Nits**: none

**Notes**: The shipped site is static and the Rust binary is unaffected. Story
verified by implement and integrated clean-install, deterministic-build,
installer-equality, workflow, and stale-surface checks; fast-lane advance.
