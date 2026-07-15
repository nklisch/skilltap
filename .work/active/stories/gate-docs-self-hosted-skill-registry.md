---
id: gate-docs-self-hosted-skill-registry
kind: story
stage: done
tags: [documentation]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: docs
created: 2026-07-15
updated: 2026-07-15
---

# Roll the self-hosted skill forward to the expanded registry

## Drift category
repo-skill-staleness

## Location
- Doc: `plugin/skills/skilltap/SKILL.md:4`
- Contradicting source: `crates/harnesses/src/registry.rs:236`

## Current doc text
> Use skilltap when setting up, inspecting, reconciling, or troubleshooting the local Codex and Claude Code environment.

## Contradiction
The canonical registry contains seventeen targets, and the root, foundation, and website documentation now describe the expanded support tiers. The self-hosted skill and its configuration/diagnostics references still present the overall control plane and target selection as Codex/Claude-only. Instruction bridging remains correctly limited to those harnesses.

## Required edit
Replace the overall Codex/Claude-only framing with supported-agent-harness language. Point target selection to `skilltap harness list` and retain `--target all` as the stable all-target example. Update the same stale assertions in `plugin/skills/skilltap/references/configuration.md` and `diagnostics.md` without adding historical prose.

## Verification

Updated the self-hosted skill and configuration/diagnostics references; verified target-selection language against the canonical registry and preserved Codex/Claude-only instruction-bridge claims. `git diff --check` passes.
