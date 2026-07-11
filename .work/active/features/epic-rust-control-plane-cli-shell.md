---
id: epic-rust-control-plane-cli-shell
kind: feature
stage: review
tags: []
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-storage, epic-rust-control-plane-runtime-primitives]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Non-Interactive CLI Shell

## Brief

Deliver the runnable `skilltap` composition root and deterministic command tree
for the v3 public surface. Centralize scope, target, and selector argument
validation; render concise plain results or exactly one stable JSON document;
and map completed, invalid, attention-required, and partial-apply outcomes to
the documented exit codes.

Handlers compose core repositories and runtime ports without containing domain
or native-format business logic. Commands whose later capability epics have not
landed must fail explicitly or expose a deliberate foundation-only behavior;
this feature does not simulate harness observation or reconciliation.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: integration consumer — depends on storage and runtime
  primitives and establishes the executable contract used by later epics

## Foundation references

- `docs/SPEC.md` — Operating Model, Output, Exit Codes
- `docs/UX.md` — Command Tree, Target and Scope, Common Flags, JSON Output,
  Errors, Exit Codes
- `docs/ARCH.md` — `skilltap-cli`, Dependency Direction, Error Model

## Design

### Command and argument boundary

`skilltap-cli` owns one Clap-derived command model covering the complete v3
tree in `docs/UX.md`. Reusable flattened argument groups encode scope, target,
selection, acknowledgment, and output dimensions, but each command includes
only meaningful groups. Parsing uses `try_parse_from`; Clap never exits the
process. Help/version complete with exit `0`, a missing subcommand prints
concise help and exits `1`, and every invalid combination is normalized into
the skilltap error contract. `--project` accepts no value for the current
directory or one path, and conflicts with `--all-scopes`. Target and selector
values convert immediately into validated core types.

### Application outcome and rendering

The command layer has one typed outcome with result class `completed`,
`invalid`, `attention_required`, or `partial_apply`; the class alone maps to
exit codes `0`, `1`, `2`, and `3`. A stable schema-1 JSON envelope always emits
exactly one object containing command, result, optional scope, summary,
resources, operations, warnings, errors, and next actions. Plain rendering is
concise and derived from the same outcome. Rendering never includes debug
values, raw native output, ANSI under JSON, or incidental text before/after the
document.

### Foundation composition

`main` composes system paths, filesystem, command runner/Git-root resolution,
and the three typed repositories. `status` is the deliberate foundation-only
operation: it resolves requested scope/targets, validates all present owned
documents, reports missing/present storage surfaces, and creates nothing. Until
native observation lands, it returns a typed attention result explicitly
naming that unavailable capability rather than claiming harness health.

Every other syntactically valid command returns a stable pre-mutation
`capability_unavailable` result for its owning later epic. It does not create
configuration, emulate native observation, or mutate desired state. This
temporary boundary is tested but not added to rolling foundation docs as
product behavior; later epics replace handlers behind the unchanged grammar,
outcome, and renderer contracts.

### Error boundary

CLI errors carry a stable safe code, summary, optional contextual fields, and
next actions. Storage/runtime/domain failures are classified at the command
boundary without exposing nested source/debug content. A malformed owned
document is invalid (`1`); status drift/unavailable observation is attention
(`2`); no foundation command can produce partial apply (`3`), but renderer and
mapping contract tests cover it for later mutation services.

### Pre-mortem

- **Clap writes or exits before JSON normalization.** Use `try_parse_from`,
  disable implicit process exit, and capture help/version separately.
- **Optional project value consumes another option or positional.** Lock the
  exact forms with parser tests for omitted/current, explicit path, conflicts,
  and representative nested commands.
- **Placeholder handlers look successful.** Unavailable capability is always
  an explicit invalid result with a stable code and next action.
- **Status creates first-use files.** Repository reads and path resolution are
  read-only; integration tests assert the configuration root remains absent.
- **Plain and JSON semantics diverge.** Both render the same typed outcome and
  share exit mapping tests.

## Implementation units

1. `epic-rust-control-plane-cli-shell-command-model` — full command tree,
   reusable argument groups, validated conversions, and parse normalization —
   depends on `[]`.
2. `epic-rust-control-plane-cli-shell-output` — typed outcomes, plain/JSON
   renderers, safe errors, and exit mapping — depends on `[]`.
3. `epic-rust-control-plane-cli-shell-composition` — system composition,
   read-only foundation status, and explicit unavailable handlers — depends on
   `[epic-rust-control-plane-cli-shell-command-model,
   epic-rust-control-plane-cli-shell-output]`.
4. `epic-rust-control-plane-cli-shell-integration` — compiled-binary grammar,
   first-use, scope/target, malformed-storage, output, and exit-code contracts —
   depends on `[epic-rust-control-plane-cli-shell-composition]`.

## Acceptance criteria

- The complete documented v3 command tree parses non-interactively; irrelevant
  flags and invalid scope/target/selector forms fail before execution.
- No-subcommand, help, version, plain, JSON, and all four exit classes obey the
  exact contract; JSON is one stable document with no incidental output.
- Foundation status composes real path/scope/target/storage adapters, works
  before configuration exists, and creates nothing.
- Unimplemented capability handlers fail explicitly before mutation and cannot
  be mistaken for native observation or reconciliation.
- Core remains CLI-independent and handlers contain no native-format or domain
  business logic.
- Full locked format/check/Clippy/test/rustdoc and release-binary smoke ladders
  pass on supported local targets.

## Implementation summary

All four planned children and one integration-discovered correction are
complete. The runnable Rust binary exposes the full non-interactive v3 grammar,
validated scope/target/selector boundaries, stable schema-1 JSON and concise
plain outcomes, exact exit mapping, read-only first-use status over real
storage/runtime adapters, and explicit pre-mutation failures for later
capabilities. Bare invocation prints normalized concise usage with exit `1`.
The optimized compiled-binary suite covers every leaf command, scope/target and
malformed-storage cases, output channels, JSON purity, no-create behavior, and
exits `0`/`1`/`2`; renderer contracts cover `3`. The locked workspace passes
191 tests, and CI/release runners test the optimized binary before publication.
