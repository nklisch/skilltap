---
id: gate-docs-plan-sync-contract
kind: story
stage: review
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

The foundation and website contract is intentional and remains the source of
truth. Implement lifecycle candidate resolution and execution for `plan` and
`sync`; do not weaken the documented synchronization behavior to match the
current stub. Regenerate the bundled website documentation only if the
implementation changes the public command behavior or examples.

## Product decision

The current behavior is an implementation gap, not a documentation choice:
populated inventory must produce scope/target-bound operations and `sync` must
execute them through the existing lock, journal, and native lifecycle ports.

## Implementation notes

- `plan` and `sync` now resolve populated desired inventory into concrete,
  scope- and target-bound lifecycle candidates rather than passing an empty
  reconciliation request.
- `plan` renders those candidates through the existing lifecycle preview path
  without mutating native configuration or state.
- `sync` delegates marketplace, plugin, and standalone-skill candidates to the
  existing native and managed-skill adapters, preserving their lock,
  revalidation, journal, and post-observation boundaries. Selection filters
  apply independently to every concrete resource scope.
- Instruction and harness inventory entries remain explicit no-op records until
  their dedicated setup/lifecycle adapters are selected; they are never
  guessed into native mutations.
- Focused verification: `cargo check -p skilltap --offline`, CLI unit tests,
  and the stable compiled-binary channel test pass.
