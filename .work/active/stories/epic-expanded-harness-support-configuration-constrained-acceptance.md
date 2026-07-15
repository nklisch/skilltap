---
id: epic-expanded-harness-support-configuration-constrained-acceptance
kind: story
stage: done
tags: [testing]
parent: epic-expanded-harness-support-configuration-constrained
depends_on: [epic-expanded-harness-support-configuration-constrained-kimi, epic-expanded-harness-support-configuration-constrained-vibe, epic-expanded-harness-support-configuration-constrained-kilo]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Prove Configuration-Constrained Adapter Acceptance

## Checkpoint

Close the feature with isolated registry, adapter, managed-projection,
project-skill, declaration-status, and compiled CLI evidence for Kimi, Vibe,
and Kilo. This is one integrated acceptance checkpoint, not three new
implementations. Effective activation is deliberately not asserted for this
family.

## Design element

Extend `ManagedProjectionProfile` with the exact Kimi/Vibe/Kilo declaration
locations and a declaration-managed status boundary. Run the existing managed
acceptance matrix for each target twice, covering no-ack versus `--yes`,
daemon pending, unknown versions, repeat, removal, conflict, ownership, and
rollback/recovery evidence. The compiled-binary suite then exercises global and
project installs under isolated home/XDG/project roots, preserves unrelated
native bytes, rejects conflicts, and verifies that unknown versions do not
write.

Exercise the review-ready project skill contract explicitly: because all three
consume `.agents/skills`, projection health is `not_required` and no duplicate
native-root tree appears. No test invokes Kimi MCP commands, Vibe TUI/LLM
flows, Kilo debug/auth commands, or a browser.

## Acceptance evidence

- Registry/help/config enablement expose `kimi`, `vibe`, and `kilo` from the
  canonical registry in stable order; no second production target list exists.
- Each target proves known/unknown detection, both scopes, complete skills,
  declaration precedence, no-ack/`--yes`, daemon pending, conflict, removal,
  target-local sibling preservation, and immediate-repeat idempotency.
- Kimi proves global-only MCP, project MCP `Unsupported`, static/OAuth
  rejection, and no-probe command sentinels; Vibe proves lossless TOML edits,
  OAuth/SSE rejection, and unverified trust; Kilo proves JSONC preservation,
  dual-file precedence/shadowing, unknown-schema rejection, and no-probe
  command sentinels.
- Optional unsupported components are exact acknowledgment-gated omissions;
  required unsupported blocks under `--yes`.
- Plain/JSON outputs agree and expose no raw native document/output/argv/secret.
- `cargo test --workspace --all-targets`, strict Clippy with warnings denied,
  formatting, and `git diff --check` pass.

## Implementation notes

- Execution capability: highest; this checkpoint now uses both the shared
  acceptance matrix and compiled CLI processes with isolated fake binaries.
- The matrix distinguishes `DeclarationStatusPending` from a fresh effective
  load probe. Kimi, Vibe, and Kilo all register no effective probe.
- Removal authorization no longer requires effective activation evidence: it
  retracts only proven skilltap-owned declarations and remains safe for
  declaration-managed profiles.
- Verification: the constrained compiled test passes for all three targets,
  including global/project install, no-ack/`--yes`, daemon pending, unknown
  versions, repeat, removal, conflict, and no-process invocation sentinels.

## Completion

This story is `done` under the relaxed amendment. The parent feature is ready
for its independent review pass.
