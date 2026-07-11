---
id: epic-rust-control-plane-cli-shell-integration
kind: story
stage: implementing
tags: [cli, testing]
parent: epic-rust-control-plane-cli-shell
depends_on: [epic-rust-control-plane-cli-shell-composition]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify the Compiled CLI Contract

Add compiled-binary integration coverage for the full grammar, no-subcommand,
help/version, first-use no-create status, project/all-scope and target forms,
malformed storage, unavailable handlers, one-document JSON, safe plain output,
and exit codes. Run the full locked ladder plus release-binary smoke checks.
