---
id: epic-rust-control-plane-runtime-maintainability
kind: feature
stage: done
tags: [refactor]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-runtime-primitives]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Runtime Maintainability

## Brief

Reduce structural pressure introduced by the runtime foundation without
changing behavior, public names, error ordering, test identities (except the
explicit publication scenario split), or platform semantics.

## Evidence

- `runtime/filesystem.rs` is 1,397 lines, with 908 production lines spanning
  basic filesystem operations, publication recovery, Unix identity/open
  primitives, and locking.
- Filesystem and scope contain 947 inline test lines; established domain modules
  already use private sidecar test modules.
- Runtime command, filesystem, and scope tests each implement their own unique
  temporary-root ownership and cleanup.
- Five production call sites repeat the same role-aware `Path` to validated
  `AbsolutePath` conversion and error mapping.
- One 105-line publication test contains six independent recovery scenarios.

## Design

This is pure refactoring. First move the two largest test modules to sidecars,
then establish the shared test-only temporary root and split the recovery
scenario test. Independently extract the exact path conversion helper. Finally
split filesystem production internals in dependency order: Unix descriptor/path
identity primitives, publication state machine, then configuration locking.

`runtime/filesystem.rs` remains the public parent module and basic `FileSystem`
adapter owner. Private children are `filesystem/unix_identity.rs`,
`filesystem/publication.rs`, and `filesystem/locking.rs`. `runtime/mod.rs`
re-exports the same public API. `RuntimeError` remains the exhaustive error
single source of truth. No macros, generic error framework, public API cleanup,
or new behavior enters this feature.

## Pre-mortem

- **Test moves silently rename filters.** Keep the original private module names
  and verify `cargo test -- --list` before/after, except for the declared split.
- **Shared test support creates a dependency cycle.** `skilltap-core` uses
  `skilltap-test-support` only as a dev-dependency; test support remains unaware
  of core domain types.
- **Private module moves alter cfg behavior.** Keep Unix/non-Unix implementation
  pairs adjacent and run Linux behavior plus warnings-clean cross-platform cfg
  compilation where locally available.
- **Filesystem split changes error precedence.** Move code mechanically first;
  existing adversarial tests and public compile consumers must remain unchanged.

## Implementation units

1. `epic-rust-control-plane-runtime-maintainability-sidecar-tests` — move
   filesystem and scope tests to private sidecars — depends on `[]`.
2. `epic-rust-control-plane-runtime-maintainability-temp-roots` — centralize the
   three test temp-root owners — depends on
   `[epic-rust-control-plane-runtime-maintainability-sidecar-tests]`.
3. `epic-rust-control-plane-runtime-maintainability-publication-tests` — split
   the six-scenario recovery matrix — depends on
   `[epic-rust-control-plane-runtime-maintainability-sidecar-tests]`.
4. `epic-rust-control-plane-runtime-maintainability-path-conversion` — extract
   the repeated role-aware validated path conversion — depends on `[]`.
5. `epic-rust-control-plane-runtime-maintainability-unix-identity` — move Unix
   no-follow and identity internals — depends on
   `[epic-rust-control-plane-runtime-maintainability-sidecar-tests]`.
6. `epic-rust-control-plane-runtime-maintainability-publication-module` — move
   recoverable publication and rollback internals — depends on
   `[epic-rust-control-plane-runtime-maintainability-unix-identity]`.
7. `epic-rust-control-plane-runtime-maintainability-locking-module` — move the
   configuration lock port/adapter — depends on
   `[epic-rust-control-plane-runtime-maintainability-unix-identity,
   epic-rust-control-plane-runtime-maintainability-publication-module]`.

## Acceptance criteria

- Public runtime exports and compile consumers are unchanged.
- Existing validation, errors, ordering, filesystem effects, lock behavior, and
  platform cfg behavior remain byte/semantically equivalent.
- Test identities remain unchanged except the explicitly decomposed recovery
  test; its scenarios and assertions are preserved individually.
- No production runtime module exceeds roughly 400 lines after the split.
- Full locked format/check/Clippy/test/rustdoc ladder passes after every step.

## Implementation summary

All seven children are complete. Filesystem and scope tests live in sidecars;
three suites share the domain-agnostic `TempRoot`; publication recovery has six
focused scenario tests; five identical path conversions share one private
helper; and filesystem production is separated into a 370-line parent plus
269-line publication, 166-line Unix identity, and 151-line locking children.
Public runtime exports and behavior remain unchanged. The workspace passes 99
tests plus doctests and warnings-clean rustdoc.

## Review finding

Fresh-context review found one observable pure-refactor regression: re-exporting
the four public lock declarations preserved import paths but changed their
canonical rustdoc and `type_name` identities to the private child module. A
corrective child restores the declarations/storage to `filesystem.rs` while
keeping implementations and acquisition helpers split.

8. `epic-rust-control-plane-runtime-maintainability-lock-identities` — restore
   canonical parent-module identities for public lock types — depends on
   `[epic-rust-control-plane-runtime-maintainability-locking-module]`.

The corrective child is complete. Canonical lock identities match the baseline
while implementation remains separated; the parent is 386 lines and the
locked workspace remains green at 99 tests.

## Final review

Approved in fresh context after one correction. The same-toolchain rustdoc JSON
snapshot is byte-identical to the pre-refactor baseline across all 380 paths,
and external lock `type_name` values match. Test-list changes are exactly the
declared one-to-six publication split plus the new `TempRoot` lifecycle test;
no unrelated identity changed. Locked format/check, warnings-denied Clippy, 99
tests and doctests, and warnings-denied rustdoc pass.
