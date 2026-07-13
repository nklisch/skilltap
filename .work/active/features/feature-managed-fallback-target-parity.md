---
id: feature-managed-fallback-target-parity
kind: feature
stage: drafting
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-cross-harness-materialization, epic-expanded-harness-support-registry]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Complete Managed Fallback Target Parity

## Brief

Complete the cross-harness promise for an explicitly selected plugin when the
target has no faithful native distribution or native plugin lifecycle. The
current production publication path covers Codex project skills and portable
MCP configuration; extend the adapter model so every supported target can use
its documented skill and MCP load surfaces without requiring that target to
provide a marketplace or plugin manager.

Native dual distributions remain preferred and independently tracked. Managed
fallback owns acquisition, revision, projection, drift, update, and removal
only for the target that lacks a native distribution. Complete skill
directories remain indivisible resources, unsupported required components
remain blocked, optional loss remains visible and acknowledgment-gated, and no
adapter writes undocumented caches.

## Strategic decisions

- **Is native distribution still preferred?** Yes. When the same plugin is
  published for both targets, track both native installations instead of
  materializing either one.
- **What qualifies a target without marketplace lifecycle?** Documented,
  observable global and project skill and MCP load surfaces are required;
  hooks and other capabilities are optional and compatibility-gated.
- **What is the immediate parity gap?** Preserve the existing Codex project
  projection and add equivalent managed fallback wherever another supported
  adapter exposes faithful documented destinations.

## Scope integration

This feature is the shared managed-projection foundation for
`epic-expanded-harness-support`. Harness-specific adapters consume this work;
they do not each invent a separate acquisition, ownership, drift, update, or
removal lifecycle.

## Foundation references

- `docs/VISION.md` — native first, faithfulness before portability, explicit loss.
- `docs/SPEC.md` — plugin installation preference and target eligibility.
- `docs/ARCH.md` — compatibility analysis and managed projection ownership.
- `docs/HARNESS-CONTRACTS.md` — documented target load surfaces.

## Acceptance direction

- A dual-native plugin uses both native lifecycles and creates no managed fallback.
- A one-sided plugin can project complete compatible skills and portable MCP
  configuration into documented global and project destinations for the other
  supported target.
- Required unsupported behavior blocks even with broad acknowledgment;
  optional loss requires explicit foreground approval.
- Update, drift, status, and removal operate from recorded managed ownership
  and repeat safely without duplicate artifacts.
