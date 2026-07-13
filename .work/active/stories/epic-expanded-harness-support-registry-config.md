---
id: epic-expanded-harness-support-registry-config
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-registry
depends_on:
  - epic-expanded-harness-support-registry
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

- [ ] A `config.toml` containing only `[harnesses.codex]` and
      `[harnesses.claude]` deserializes to a `ConfigDocument` equal to today's
      output (round-trip/parse test).
- [ ] Round-trip: `to_string` then `parse` is identity for the defaults document
      and for a two-harness enabled document.
- [ ] `with_harness_policy(&HarnessId::new("gemini").unwrap(), true, None)`
      succeeds at the config layer (membership is a composition concern).
- [ ] `HarnessPolicyMap::enabled()` yields `codex` and `claude` in id order when
      both are enabled, matching today's `enabled_harnesses` output.
- [ ] `defaults()` seeds exactly `codex` and `claude`, both disabled, matching
      today's first-use behavior.

## Out of scope

- CLI dispatch and membership validation (Unit 4).
- The adapter trait and Codex/Claude migration (Unit 1/2), though this story
  only depends on the parent registry feature, not on the adapter story.
