---
id: epic-rust-control-plane-cli-shell-composition
kind: story
stage: implementing
tags: [cli, infra]
parent: epic-rust-control-plane-cli-shell
depends_on: [epic-rust-control-plane-cli-shell-command-model, epic-rust-control-plane-cli-shell-output]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Compose the Foundation CLI

Compose system runtime and typed storage adapters behind command dispatch.
Implement read-only first-use `status` with real scope/target/storage
validation and explicit native-observation attention. Route all later-capability
commands to stable pre-mutation unavailable outcomes. Keep handlers free of
native-format/domain business logic and run the locked ladder.
