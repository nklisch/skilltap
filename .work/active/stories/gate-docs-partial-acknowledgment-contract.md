---
id: gate-docs-partial-acknowledgment-contract
kind: story
stage: implementing
tags: [documentation]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: docs
created: 2026-07-12
updated: 2026-07-12
---

# Implement the documented partial-operation acknowledgment contract

## Drift category

foundation-doc-assertion

## Location

- Docs: `docs/SPEC.md:288-316`, `docs/SPEC.md:339-374`, `docs/UX.md:9-18`, `docs/UX.md:102-123`, `README.md:197-205`, `website/guide/managing-environments.md:72-81`, `website/reference/cli.md:50-63`
- Code: `crates/cli/src/command.rs:403-408`, `crates/cli/src/application.rs:638-646`, `crates/cli/src/application.rs:3585-3589`

## Current doc text

> The docs describe generic `--yes` as permitting a reported partial result
> and use it in plugin and instruction examples.

## Reality

The parser exposes a generic boolean `--yes`; the current plan has no partial
operation to acknowledge, and sync emits `acknowledgment_not_applicable` when
it is set. The product contract requires naming the accepted consequence or
resource (for example, repeatable `--accept-partial <resource-id>`), with
unsupported required components remaining blocked.

## Required implementation

Keep the foundation and website documentation as the source of truth. Make
the implementation accept generic `--yes` as approval for all eligible
partial/lossy consequences in the current operation or plan, while retaining
optional repeatable resource/component selectors for piecewise acceptance.
Required or blocked components remain blocked even with `--yes`.

Update the CLI/application plumbing and core selection logic together, then
add tests for generic acceptance, selector-scoped acceptance, unexpected
selectors, and required-component blocking. Regenerate website/reference
copies only if command syntax changes.

## Product decision

The documentation is correct. The prior gate interpretation incorrectly
treated the exact-selector implementation in `foreground_update.rs` as the
desired public contract; it is the implementation that must change.
