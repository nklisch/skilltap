---
id: epic-expanded-harness-support-native-coexistence
kind: feature
stage: drafting
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Native-Coexistence Adapters for Droid, Qwen, and Copilot

## Brief

Deliver complete adapters for Factory Droid, Qwen Code, and GitHub Copilot CLI.
These targets combine documented skill and MCP files with native plugin,
extension, marketplace, conversion, or policy behavior that must coexist with
skilltap-managed fallback rather than being flattened into it.

For every resource, prefer and independently track a faithful native
distribution when one exists; use managed component projection only for the
target lacking a native equivalent. Each adapter owns its native identity,
scope, precedence, enterprise or trust constraints, structured observation,
and verified version profiles while sharing target-neutral execution and state
machinery. The feature includes isolated native validation and complete
acceptance-contract evidence for all three targets.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: parallel concrete-adapter feature after the registry and
  managed fallback foundations.

## Simplification opportunity

- Consolidate native-versus-managed selection into capability-driven adapter
  composition instead of accumulating target-name branches in application code.

## Foundation references

- `docs/VISION.md` — Native First, Faithfulness Before Portability.
- `docs/SPEC.md` — Plugin Lifecycle, Marketplace Lifecycle, Ownership and Removal.
- `docs/HARNESS-CONTRACTS.md` — Contract Rules, Expanded Target Set.

