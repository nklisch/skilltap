---
id: idea-gate-cruft-effective-state-probe-decision
created: 2026-04-02
updated: 2026-07-15
tags: [cleanup]
release_binding: null
gate_origin: cruft
---

# Decide whether the effective-state probe port still earns its cost

## Confidence
High

## Relevance
Release-relevant discovery, but unbound because removing it requires an explicit guarantee decision.

## Location
`crates/harnesses/src/effective_state.rs:70`

`EffectiveStateProbePort` has implementations for Copilot, Gemini, OpenCode, and Qwen but no production caller. Removing it would also remove the typed boundary intended for bounded effective MCP status verification.

Decide whether to:

- retain and wire the port into production postcondition/status verification;
- reduce it to the subset with a near-term caller; or
- remove the subsystem, adapter implementations, re-exports, and presence-only tests while explicitly accepting that future verification would need to reintroduce the contract.
