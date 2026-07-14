---
id: epic-expanded-harness-support-configuration-constrained-acceptance
kind: story
stage: implementing
tags: [testing]
parent: epic-expanded-harness-support-configuration-constrained
depends_on: [epic-expanded-harness-support-configuration-constrained-kimi, epic-expanded-harness-support-configuration-constrained-vibe, epic-expanded-harness-support-configuration-constrained-kilo]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Prove Configuration-Constrained Adapter Acceptance

## Checkpoint

Close the feature with isolated registry, adapter, managed-projection,
project-skill, activation, and compiled CLI evidence for Kimi, Vibe, and Kilo.
This is one integrated acceptance checkpoint, not three new implementations.

## Design element

Extend `FakeHarnessProfile` and `ManagedProjectionProfile` with scope-specific
load locations and exact activation responses. Register profile runners without
adding target switches outside profile constructors/the acceptance dispatcher.
Run the existing `acceptance_matrix` and `managed_acceptance_matrix`, then add
compiled-binary scenarios for each target under isolated home/XDG/project roots.

Exercise the review-ready project skill contract explicitly: because all three
consume `.agents/skills`, projection health is `not_required` and no duplicate
native-root tree appears.

## Acceptance evidence

- Registry/help/config enablement expose `kimi`, `vibe`, and `kilo` from the
  canonical registry in stable order; no second production target list exists.
- Each target proves known/unknown detection, both scopes, complete skills, MCP
  precedence, effective probing, drift, removal, pending recovery, target-local
  sibling preservation, and immediate-repeat idempotency.
- Kimi proves fresh-session visibility; Vibe proves trust and OAuth outcomes;
  Kilo proves JSONC preservation, dual-file precedence/shadowing, and
  failed/auth-required health.
- Optional unsupported components are exact acknowledgment-gated omissions;
  required unsupported blocks under `--yes`.
- Plain/JSON outputs agree and expose no raw native document/output/argv/secret.
- `cargo test --workspace --all-targets`, Clippy with warnings denied,
  formatting, and `git diff --check` pass.

## Ordering

Depends on all three target adapters. Green verification advances this child
directly to `done`; the parent feature then receives one standard independent
feature-level review pass.
