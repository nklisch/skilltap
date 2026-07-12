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

# Align partial-operation acknowledgment documentation

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

## Required edit

Replace generic `--yes` partial-approval grammar, examples, and prose with an
operation/resource-scoped acknowledgment. Update the CLI parser and
application contract in the same change and regenerate website/reference
copies together.

