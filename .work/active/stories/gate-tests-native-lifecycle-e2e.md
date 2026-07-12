---
id: gate-tests-native-lifecycle-e2e
kind: story
stage: implementing
tags: [testing]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: tests
created: 2026-07-12
updated: 2026-07-12
---

# Cover both-harness plugin and marketplace lifecycle

## Priority

High

## Spec reference

Items `epic-native-marketplace-plugin-lifecycle-commands`,
`epic-native-marketplace-plugin-lifecycle-claude`,
`epic-native-marketplace-plugin-lifecycle-codex`, and
`epic-native-marketplace-plugin-lifecycle-preservation`.

## Gap type

Missing target/action integration coverage for plugin and marketplace install,
update, remove, observation, repeat no-op, and state journaling.

## Suggested test

Use fake Codex and Claude binaries that record argv and update fixture config;
cover all lifecycle actions, Claude/project scope, immediate repeats, native
unknown-field preservation, post-observation, and state journal records.

## Test location (suggested)

`crates/cli/tests/compiled_binary.rs` and harness lifecycle integration tests.
