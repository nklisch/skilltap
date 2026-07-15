---
id: epic-expanded-harness-support-project-skill-links-acceptance
kind: story
stage: done
tags: [testing]
parent: epic-expanded-harness-support-project-skill-links
depends_on:
  - epic-expanded-harness-support-project-skill-links-lifecycle
  - epic-expanded-harness-support-project-skill-links-observation
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: operator-request-2026-07-14
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Prove the Project Skill Link Lifecycle

## Checkpoint

Implement Unit 5 from the parent feature: isolated integration and compiled-CLI
coverage proving the canonical-tree/link contract, safety behavior, output
semantics, and immediate-repeat idempotency across multiple registry-derived
native roots.

This is an acceptance checkpoint, not a separate test framework. Reuse
`IsolatedMachine`, fake harness profiles, `snapshot_tree`, and production
application entry points.

## Coverage

- Update project-scope assertions in `crates/cli/tests/compiled_binary.rs` from
  duplicate complete trees to one canonical tree plus relative per-skill links.
  Keep global copy assertions unchanged.
- Cover nested project roots, Codex canonical no-op, Claude link projection,
  and a throwaway adapter with another project skill root.
- Prove complete siblings and executable intent exist only in canonical content
  and remain reachable through a link after first asserting the native entry is
  the exact expected relative symlink.
- Exercise correct repeat, missing link repair, canonical-restored broken link,
  owned divergent relative-link repair, and preservation of unmanaged relative,
  absolute, regular-file, directory, and special-entry conflicts.
- Exercise targeted remove, final direct remove, final adopted remove, all-target
  content update, partial-target content-update block, state sibling
  preservation, and dependency failure/partial apply.
- Assert stable plain/JSON format, compatibility, loadability, projection codes,
  next actions, and exit 0/2/3 classes.

## Acceptance evidence

- Every mutating scenario immediately repeats and reports zero changes without
  rewriting a correct link inode.
- No test touches the operator's HOME, XDG roots, native binaries, or repository
  project paths.
- Runtime race tests from the filesystem checkpoint and full compiled lifecycle
  tests both pass on supported macOS/Linux CI.
- `cargo test --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`, and `git diff --check` are green before the
  parent feature advances to review.

## Ordering

Runs after lifecycle and observation semantics exist. Green evidence advances
this child directly to done; independent review remains at the parent feature
level under the caller's standard review weight.

## Implementation notes

- Execution capability: direct feature-owner acceptance pass, matching the
  caller's host-owned continuation posture; the compiled binary suite exercises
  production install, update, remove, status, adoption, and reconciliation
  entry points inside `IsolatedMachine` roots.
- The project-scope copy assertion now proves one complete canonical tree plus
  the exact relative Claude projection while leaving global copied-tree checks
  unchanged. Added scenarios cover nested paths, canonical no-op behavior,
  missing/broken/divergent link repair, unmanaged conflict preservation,
  read-only status, malformed unmanaged candidates, source-less adoption, and
  the all-desired-target content update gate.
- Acceptance fixes: unmanaged status observation now uses the selected target
  set rather than constructing an invalid empty `HarnessSet`; project-skill
  warnings retain `attention_required` after status observation completes; and
  the partial shared-content update response reports `changed: false`.
- Adjacent verification hardening: retained checked Unix link-identity
  conversions with a platform rationale for strict Clippy, and removed one
  needless borrow in the existing link-race fixture.
- Discrepancies from the design: the compiled registry currently exposes only
  Codex and Claude, so no throwaway third adapter is available in this binary
  surface; Unix filesystem tests cover the lower-level entry-kind and race
  matrix. Resource-ID assertions use the stable rendered project-scope form.

## Verification

- `cargo test -p skilltap --test compiled_binary project_skill -- --nocapture` —
  passed (4 tests).
- `cargo test -p skilltap --test compiled_binary -- --nocapture` — passed (58
  tests).
- `cargo test --workspace --all-targets` — passed (586 tests).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  passed.
- `cargo fmt --all -- --check` and `git diff --check` — passed.
