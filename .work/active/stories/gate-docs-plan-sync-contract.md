---
id: gate-docs-plan-sync-contract
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

# Align plan and sync documentation with lifecycle execution

## Drift category

foundation-doc-assertion

## Location

- Docs: `docs/SPEC.md:264-318`, `docs/UX.md:255-318`, `website/guide/managing-environments.md:51-81`, `website/guide/getting-started.md:34-54`, `website/reference/cli.md:73-107`
- Code: `crates/cli/src/application.rs:634-646`, `crates/cli/src/application.rs:3528-3598`

## Current doc text

> The foundation and website docs say that `plan` computes lifecycle
> operations and that `sync` computes and applies the current reconciliation
> plan, including marketplace, plugin, and skill lifecycle actions.

## Reality

`execute_reconciliation` currently passes `ReconciliationRequest::default()`
to planning, yielding no candidates for populated inventory and returning a
`reconciliation_candidates_unavailable` attention. It never composes the
lifecycle executor for `plan` or `sync`; lifecycle-specific commands remain
separate paths.

## Required edit

Either implement lifecycle candidate resolution and execution for `plan` and
`sync`, or revise all listed foundation and website language and examples to
state that populated-inventory lifecycle support is pending. Regenerate the
bundled website documentation after the source edit.

