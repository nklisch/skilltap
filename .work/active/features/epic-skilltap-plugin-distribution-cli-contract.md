---
id: epic-skilltap-plugin-distribution-cli-contract
kind: feature
stage: drafting
tags: [content]
parent: epic-skilltap-plugin-distribution
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Agent-Readable CLI Help and Errors

## Brief

Make the executable CLI a dependable discovery surface for agents. Audit the
root command, command groups, and every public leaf for concise purpose text,
accurate usage, scope/target and acknowledgment guidance, output modes, and
documented exit behavior. Align the website and future skill references to
that executable contract without creating a hand-maintained second grammar.

Improve the plain and JSON failure paths where they do not already identify
the failing boundary, redact sensitive context, or provide a safe next action.
This feature preserves the existing non-interactive semantics and result
classes; it does not add plugin packaging or binary installation behavior.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: independent CLI contract work; the guidance feature uses
  its stable help and diagnostic language.

## Foundation references

- `docs/UX.md` — Help and Diagnostic Discovery, Command Tree, Output, Errors
- `docs/SPEC.md` — Operating Model, Output, Exit Codes, Validation
- `crates/cli/src/command.rs` — command grammar and help metadata
- `crates/cli/src/entrypoint.rs` — parse/error dispatch
- `crates/cli/src/output.rs` and `crates/cli/src/outcome.rs` — render and result
  contracts
- `crates/cli/tests/compiled_binary.rs` — executable CLI contract

## Design decisions

- **Bootstrap discoverability**: The self-setup flow is a first-class,
  help-described skilltap command that the plugin and one-line installer can
  invoke; it is not hidden in undocumented shell behavior.
- **Contract authority**: The executable help and stable output remain the
  exact source of truth, while the plugin and website provide high-level
  guidance and links into that contract.

<!-- Feature design will determine the precise help/error gaps and verification
surface. -->
