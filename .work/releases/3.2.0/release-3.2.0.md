---
id: release-3.2.0
kind: release
stage: released
tags: []
parent: null
depends_on: []
release_binding: 3.2.0
gate_origin: null
created: 2026-08-15
updated: 2026-08-15
---

# Release 3.2.0

## Summary

- Replace the deprecated `gemini` target with `agy` for Antigravity CLI. No
  compatibility alias or automatic state migration remains.
- Add an exact Antigravity CLI 1.1.13 profile with complete global and project
  skills.
- Add declaration-managed global MCP projection through
  `~/.gemini/config/mcp_config.json` with preservation, ownership drift checks,
  rollback, and repeat idempotence.
- Keep project MCP, native Antigravity plugin lifecycle, and unverified
  transports unsupported until their contracts can be bound and observed.

## Compatibility

This minor release removes the already-deprecated target id `gemini`. Existing configuration and
inventory must be retargeted explicitly to `agy`. Unknown AGY versions remain
observe-only, and managed plugin projection requires foreground `--yes` because
no non-interactive effective-state observer is available.

## Gate runs

- **Security** — no critical, high, or medium findings.
- **Tests** — two medium coverage findings were fixed. Compiled-binary tests now
  prove owned MCP drift refusal and optional unsupported MCP omission.
- **Cruft** — no blocking findings. The redundant AGY fixture was removed and
  stale pattern references were corrected.
- **Docs** — no blocking findings. Foundation, README, website, generated LLM
  reference, changelog, and release identities agree on the AGY contract.
- **Patterns** — no blocking findings and no new pattern. AGY uses the existing
  drift-checked managed-projection pattern.

## Verification

- `cargo test --locked --workspace --all-targets` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- `cargo fmt --all -- --check` and `git diff --check` are clean.
- `cargo test --locked -p skilltap --test plugin_package` passes all package
  channel tests.
- `scripts/verify-installer.sh`, `scripts/verify-install-surfaces.sh`, and
  `scripts/verify-release-contract.sh` pass.
- `npm run build --prefix website` builds all documentation pages and
  regenerates `website/public/llms-full.txt`.
- Research lint resolves nine citations with zero broken, thin, or pattern
  findings.

## Shipment

- **Date shipped:** 2026-08-15
- **Mapping:** tag-based (`v3.2.0`)
- **Implementation commit:** `428c1325`
- **Items shipped:** 2

## Shipped items

| id | title | kind | git ref |
|----|-------|------|---------|
| `feature-replace-gemini-with-agy` | Replace deprecated Gemini support with Antigravity CLI | feature | `428c1325` |
| `gate-tests-agy-plan-branches` | Cover AGY drift and unsupported MCP plan branches | story | `428c1325` |
