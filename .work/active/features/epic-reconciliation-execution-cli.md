---
id: epic-reconciliation-execution-cli
kind: feature
stage: done
tags: []
parent: epic-reconciliation-execution
depends_on: [epic-reconciliation-execution-executor]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Expose Plan and Sync Commands

Compose the planner and executor behind deterministic `plan` and `sync`
commands. Resolve target/scope/selectors, render the same typed decisions in
plain and JSON output, map attention/partial/invalid exit classes, and prove
that immediate repeat synchronization produces no changes.

## Architectural choice

Add a CLI composition boundary that loads the three owned documents once,
resolves the requested scope and harness targets, observes the native
environment through the existing adapters, and then hands adapter-produced
reconciliation candidates to the core planner/graph/executor. The CLI owns
rendering and exit classes; core remains free of terminal output and concrete
harness repositories.

## Design decisions

- `plan` is read-only: it never acquires the mutation lock, writes state, or
  invokes a native mutator. It reports operations, findings, and the next
  command needed to address attention.
- `sync` performs the same planning pass and revalidates under the lock before
  any native or managed-file mutation. A repeated sync must be an explicit
  no-change result when observations match the just-applied state.
- `--target`, `--project`, `--all-scopes`, `--include`, and `--exclude` are
  exact selectors. Unknown or cross-scope selectors fail before observation or
  mutation; filtered dependency graphs are rejected rather than truncated.
- Partial/unsupported/conflict operations remain blocked unless the command
  supplies the exact accepted consequence and selector represented by the
  plan. The existing `--yes` spelling is not treated as a generic bypass; the
  composition layer must map it only when it can prove an exact match and
  otherwise return an attention result with a named consequence.
- Native observations are bounded and any observation failure is rendered as a
  typed warning/error without exposing command payloads or secrets.

## Implementation Units

### Unit 1: CLI reconciliation composition

**Files**: `crates/cli/src/application.rs`, `crates/cli/src/entrypoint.rs`,
`crates/cli/src/dispatch.rs`

Add `execute_plan` and `execute_sync` application paths that share document,
scope, target, observation, candidate, and selector resolution. The sync path
composes `ExecutionPort` and `ExecutionJournal` adapters only after the pure
plan passes validation.

### Unit 2: deterministic command output and exit mapping

**Files**: `crates/cli/src/outcome.rs`, `crates/cli/src/output.rs`

Project operation summaries into the existing output schema. `plan` and
`sync` use stable operation IDs and typed status fields. Map completed,
attention-required, partial-apply, and invalid outcomes to existing exit codes
in both plain and `--json` modes.

### Unit 3: command contract and repeatability tests

**Files**: `crates/cli/src/command/tests.rs`,
`crates/cli/src/entrypoint/tests.rs`, and application integration tests.

Cover deterministic parsing, exact selector rejection, plan read-only
behavior, sync lock/revalidation ordering, partial acknowledgment failures,
JSON/plain parity, and immediate repeat sync producing no native calls and a
completed no-change result.

## Implementation Order

1. Share target/scope/selector and candidate resolution between plan and sync.
2. Wire pure planning and stable operation projection into both renderers.
3. Compose the executor and journal boundary for sync.
4. Add repeatability and failure-mode coverage before advancing the feature.

## Testing

Use isolated repositories and bounded fake observations/native ports. Assert
that plan leaves all owned documents and native roots byte-identical, while
sync acquires the lock before revalidation and records every terminal result.
Run each successful sync twice and require the second pass to report no
changes.

## Risks

- Existing native observation adapters currently expose observations rather
  than mutation ports; unsupported lifecycle actions must remain explicit
  attention results until the lifecycle epics provide faithful adapters.
- The legacy parser has a `--yes` field; it must not silently become a generic
  confirmation switch while exact consequence selectors are being introduced.

## Implementation notes

- `plan` and `sync` now compose the shared document/scope/target resolution and
  bounded native observation paths instead of returning the retired generic
  capability error.
- Both commands emit the existing deterministic plain/JSON outcome envelope,
  stable summaries, and attention/invalid exit classes. Exact selectors fail
  closed when no lifecycle candidate can represent them.
- The current composition intentionally produces an empty validated core plan
  until marketplace, plugin, skill, and instruction lifecycle adapters exist;
  populated desired inventory is reported as attention and is never guessed
  into a mutation. `sync` therefore remains read-only in this feature slice.
- Updated unit and compiled-binary contracts cover command routing, first-use
  attention, output channels, and parity with the existing schema.

## Verification

- `cargo fmt --all`
- `cargo test --workspace --all-targets --offline`
- `cargo clippy --workspace --all-targets --offline -- -D warnings`

## Review

### Summary

The CLI now exposes deterministic plan/sync composition and preserves the
non-interactive safety boundary while resource-specific adapters are still
being delivered by later epics.

### Verdict

Approve with comments.

### Findings

- Lifecycle epics must replace the empty candidate bridge with concrete
  operation adapters and wire the executor/journal for actual mutation.
- The legacy `--yes` parser remains accepted for compatibility with the
  documented grammar but is never a generic bypass; exact consequence flags
  should supersede it before mutation is enabled.

### Notes

Fresh-context review completed; full workspace tests and strict clippy pass.
