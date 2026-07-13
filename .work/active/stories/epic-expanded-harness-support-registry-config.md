---
id: epic-expanded-harness-support-registry-config
kind: story
stage: done
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

## Review

Cross-model review (GLM reviewer of an OpenAI host implementation),
review_weight `standard`, risk-escalated to focused Deep because the change
rewrites persisted configuration internals while claiming schema-1 wire and
byte compatibility. Re-read the story, parent feature design (Unit 3), full
`crates/core/src/storage/config.rs` diff at `43464e1c`, `storage/mod.rs`,
`storage/tests.rs`, `storage/repository/tests.rs`, the golden fixture, the
prior `HarnessPolicies` shape, `domain/identity.rs`, and the `validate_identifier`
contract. Re-ran `cargo test -p skilltap-core` (346 passed), `cargo fmt -p
skilltap-core -- --check` (clean), and `cargo check -p skilltap-core` (clean).
The other worker's Unit 2 adapter work landed as `256189a8` during the review
and does not touch `crates/core`; the config commit `43464e1c` is intact and the
review verdict is unchanged. `.pi/` and the (now-committed) harness crate
changes were left untouched.

Verdict: **approve**. Every acceptance criterion is met with explicit tests,
and the design's riskiest assumption (wire/byte compatibility) is pinned by the
golden fixture assertion.

Verified dimensions:

- Legacy TOML parsing: `config_defaults_are_explicit_strict_and_golden` parses
  `fixtures/config.toml` (codex/claude only) into a `ConfigDocument` equal to
  `defaults()`. The derived `Deserialize` for `HarnessPolicyMap(BTreeMap<…>)`
  consumes the same `[harnesses.<id>]` tables the old struct produced.
- Unknown-field behavior: `HarnessPolicy` retains `#[serde(deny_unknown_fields)]`,
  so extra keys within a `[harnesses.<id>]` table still error. Top-level
  `deny_unknown_fields` on `ConfigWire` is unchanged.
- Unknown-harness behavior at the core boundary: an extra `[harnesses.<id>]`
  table is now accepted at the config layer. This is the intended design — core
  validates structure only and id membership is a CLI composition concern — and
  is correctly paired with the Unit 4 membership-validation handoff noted in the
  implementation record. The CLI still references `HarnessPolicies`/`.codex`/
  `.claude` and will not compile against this change until Unit 4 lands; that is
  the declared dependency-chain state, not a defect in this story.
- Duplicate tables: rejected by the TOML parser itself, so behavior is unchanged
  from the prior struct shape.
- Serialization order/bytes: `stable_iter()` emits `codex` then `claude` in fixed
  order before any additional ids in BTreeMap order, and the byte-equality
  assertion `toml::to_string_pretty(&defaults()) == fixtures/config.toml` holds
  against the pre-change golden file — proving byte stability across the
  struct→map migration, not just within the new shape.
- Defaults: `defaults()` seeds exactly `codex` and `claude`, both disabled, with
  PATH-lookup binaries `"codex"`/`"claude"`; asserted explicitly.
- Generic insertion: `with_harness_policy(&HarnessId::new("gemini"), true, None)`
  succeeds and the new entry appears in the map; asserted.
- Binary default derivation: a newly inserted entry without an explicit binary
  falls back to the existing entry's binary when present, otherwise to the id as
  a PATH-lookup name; the gemini case asserts `binary == "gemini"` and the codex
  enable/disable case asserts the prior `/opt/bin/codex` binary is retained.
  The `expect("validated harness id is a PATH name")` is sound on this crate's
  Linux target: `validate_identifier` restricts `HarnessId` to `[a-z0-9._:-]`
  starting with `[a-z0-9]`, so every valid id is a single `Component::Normal`
  PATH name and `HarnessBinary::new` accepts it.
- Independent bootstrap/update policy: `with_harness_policy`/`with_harness_enabled`
  operate by cloning the whole `ConfigDocument`, so `bootstrap` survives
  harness edits. `harness_policy_updates_preserve_binary_update_policy` pins
  this across enable, disable, and binary replacement.
- API compatibility: the public-surface break (`HarnessPolicies` removed,
  `harnesses()` now returns `&HarnessPolicyMap`, `.codex`/`.claude` field access
  gone) is deliberate and the consuming migration is owned by Unit 4. The
  breaking change is contained to core and does not affect persisted state.
- Boundary split: `crates/core` has no dependency on `skilltap-harnesses`; the
  registry is never referenced. Membership validation is deferred to the CLI
  composition boundary exactly as the parent design specifies.

Non-blocking observations (below the material current-cycle bar; no backlog
item warranted):

- No explicit TOML round-trip test for a three-harness document (the suite pins
  defaults and the two-enabled case). The inverse `stable_iter` serialize /
  derived-BTreeMap deserialize makes a three-harness round trip mechanically
  identity, so this is coverage polish rather than a gap.
- `stable_iter` is O(n²) over the harness table. The map is bounded to a handful
  of registered targets, so this is a clarity-for-byte-stability trade that
  earns its keep; revisit only if adapter count grows into the dozens.
