---
id: epic-expanded-harness-support-registry-config
kind: story
stage: review
tags: []
parent: epic-expanded-harness-support-registry
depends_on:
  - epic-expanded-harness-support-registry-contract
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Registry-Driven Configuration Map

## Scope

Implement Unit 3 of the registry feature design. Replace the closed
`HarnessPolicies { codex, claude }` struct in `crates/core/src/storage/config.rs`
with a registry-driven `HarnessPolicyMap(BTreeMap<HarnessId, HarnessPolicy>)`,
and update `ConfigDocument` accordingly. Core validates structure only; id
membership is enforced at the CLI composition boundary (Unit 4).

## Units

- `crates/core/src/storage/config.rs` (modified):
  - Add `HarnessPolicyMap` with `get`, `iter`, `enabled`, `with_policy`.
  - Change `ConfigDocument.harnesses` from `HarnessPolicies` to
    `HarnessPolicyMap`.
  - `defaults()` still seeds exactly `codex` and `claude`, both disabled with
    PATH-lookup binaries.
  - `with_harness_policy(&HarnessId, ...)` inserts/updates a map entry; works
    for any structurally valid id.
  - `HarnessPolicy` keeps `deny_unknown_fields` per entry.
- `crates/core/src/storage/tests.rs` and any other references to
  `HarnessPolicies` fields (`.codex` / `.claude`) updated to map access.

## Implementation notes

- Wire-compatible: a struct field and a map entry serialize to the same
  `[harnesses.<id>]` TOML table, so existing `config.toml` files round-trip
  unchanged and `CONFIG_SCHEMA_VERSION` stays at 1. Verify before relying on
  this (see acceptance criteria).
- `CONFIG_SCHEMA_VERSION` must not be bumped unless the wire-compatibility
  assumption fails. If it fails, fall back to schema 2 with a one-time loader
  accepting the legacy two-key form; do not assume this is needed.
- This change is additive and wire-compatible, so it must not be cherry-picked
  onto the in-flight `3.0.0` release branch. It targets `main` after `3.0.0`
  ships.

## Acceptance criteria

- [x] A `config.toml` containing only `[harnesses.codex]` and
      `[harnesses.claude]` deserializes to a `ConfigDocument` equal to today's
      output (round-trip/parse test).
- [x] Round-trip: `to_string` then `parse` is identity for the defaults document
      and for a two-harness enabled document.
- [x] `with_harness_policy(&HarnessId::new("gemini").unwrap(), true, None)`
      succeeds at the config layer (membership is a composition concern).
- [x] `HarnessPolicyMap::enabled()` yields `codex` and `claude` in id order when
      both are enabled, matching today's `enabled_harnesses` output.
- [x] `defaults()` seeds exactly `codex` and `claude`, both disabled, matching
      today's first-use behavior.

## Out of scope

- CLI dispatch and membership validation (Unit 4).
- The adapter trait and Codex/Claude migration (Unit 1/2); this story depends
  only on the Unit 1 registry contract, not on the adapter-migration story.

## Implementation record

- Execution capability: highest, selected by the active autopilot caller because
  preserving the schema-1 configuration contract is a compatibility-sensitive
  boundary. Review weight remains the caller's default `standard`.
- Dispatch: direct-read implementation only. Ownership was bounded to core
  configuration storage while the harness crate was being modified concurrently.
- Replaced `HarnessPolicies` with `HarnessPolicyMap`, backed by
  `BTreeMap<HarnessId, HarnessPolicy>`. The map exposes `get`, `iter`, `enabled`,
  and immutable `with_policy` operations; `ConfigDocument` now accepts and
  returns the map.
- Core accepts every structurally valid `HarnessId`. A newly inserted policy
  without an explicit binary uses the id as its PATH-lookup binary; existing
  policy binaries remain unchanged when only enablement changes. Registry
  membership remains a CLI composition concern.
- Schema-1 compatibility was proved before retaining
  `CONFIG_SCHEMA_VERSION = 1`: the pre-change golden fixture parsed as the old
  document, then the migrated document parsed the same fixture, serialized
  defaults to the exact same bytes, and round-tripped both defaults and the
  two-enabled-harness case. Custom map serialization retains the established
  `codex`, `claude` table order; additional ids follow deterministic map order.
- First-use behavior remains exactly two disabled entries (`codex` and
  `claude`) with matching PATH-lookup binaries. Enabled iteration retains the
  established `codex`, `claude` output order and appends additional ids
  deterministically.
- Integration dependency: Unit 4 must migrate CLI imports of
  `HarnessPolicies` and `.codex`/`.claude` accesses to `HarnessPolicyMap` before
  the CLI crate can consume this public-surface change. No CLI files were edited.
  A workspace check was also temporarily blocked earlier by concurrent,
  incomplete Unit 2 harness changes; the core crate is independently verified.

## Verification

- `cargo fmt -p skilltap-core -- --check`
- `cargo test -p skilltap-core` — 346 passed
- `cargo check -p skilltap-core`
- Acceptance evidence includes legacy schema-1 parse and byte-stable default
  serialization, defaults and two-enabled round trips, generic `gemini`
  insertion, deterministic enabled ordering, and exact first-use defaults.
