---
id: epic-real-harness-recovery-filesystem-instructions-executable-intent
kind: story
stage: review
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-filesystem-instructions
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Preserve normalized executable intent through skill publication

## Scope

Introduce the typed artifact-file contract and carry its executable intent from
descriptor-relative source observation through skill validation,
fingerprinting, managed backup/equality, private publication, reload, rollback,
and destination drift checks.

## Acceptance

- Source files with any execute bit publish as private owner-executable files;
  non-executable files publish private without execute regardless of path or
  shebang.
- Group/world, write, set-id, sticky, and other special metadata never cross
  the managed boundary.
- Mode-only changes affect fingerprints, update/drift detection, backup, and
  rollback; identical repeats are no-ops.
- Whole-directory global/project installs for Codex and Claude preserve all
  contents and normalized intent inside isolated fixture roots.
- Existing no-follow, identity revalidation, cleanup, and unsupported-entry
  tests remain green.

## Implementation notes

- Execution capability: direct inline implementation; the change is cohesive but security-sensitive across the core artifact and descriptor-relative filesystem boundary.
- Review weight: standard (project default); focused security and end-to-end regression coverage exercises the risky mode normalization paths.
- Files changed: `crates/core/src/domain/artifact.rs`, domain exports, external-tree observation, skill validation/fingerprinting, managed artifact trees, directory-tree publication/loading, compatibility consumers, related core tests, and `crates/cli/tests/compiled_binary.rs`.
- Tests added: source execute-bit observation, secret-safe artifact debug rendering, mode-only skill fingerprint changes, exact private `0600`/`0700` publication and reload, managed artifact mode round trips, destination mode drift, source mode-only updates/repeat no-ops, and whole-directory Codex/Claude installs in global/project scopes.
- Discrepancies from design: `ArtifactFile` accepts `Vec<u8>` as non-executable for compatibility at existing byte-only construction sites; explicit observed and published skill paths always construct it with typed intent. No persisted `ArtifactTree` wire exists in the current repository, so wire validation remained at existing domain constructors rather than adding an unused serialized contract.
- Adjacent issues parked: none. One full compiled-binary test currently fails because concurrent harness-detection work changed the default first-use status result; the executable-intent focused test and the other 43 compiled-binary tests pass.

## Review findings (2026-07-12)

- **Blocker — exact publication modes depend on ambient umask**: `write_tree` passes `0700` or `0600` as the `openat` creation mode but never normalizes the already-open file descriptor afterward. POSIX applies the process umask to those bits, so a restrictive umask can silently remove owner execute/read/write permissions and violate the story's exact private-mode contract. Tracked by `epic-real-harness-recovery-filesystem-instructions-umask-independent-modes`.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: `epic-real-harness-recovery-filesystem-instructions-umask-independent-modes`
**Important**: none
**Nits**: none

**Notes**: Substrate review at the project-default `standard` weight using a fresh-context deep lane for the security-sensitive filesystem boundary. Commit `a8084ab` correctly carries typed executable intent through observation, fingerprints, storage, drift, and whole-directory projections, and the full workspace suite is green under the ordinary test umask. The missing descriptor-relative exact-mode normalization remains a correctness blocker; formatting and all-target/all-feature clippy are otherwise green.

## Bounce resolution (2026-07-12)

- The open artifact descriptor is normalized with `fchmod` to exactly `0700`
  or `0600` after content is written and before it is synced and identity-checked.
  No path reopen or followable metadata operation was introduced.
- A restrictive-umask regression runs in an isolated single-test child process,
  restores the prior umask with an unwind-safe guard, and proves exact modes,
  typed reload, and unchanged repeat behavior.
- The focused directory-tree tests, full workspace suite, formatting check, and
  all-target/all-feature Clippy pass. The blocker is ready for re-review through
  `epic-real-harness-recovery-filesystem-instructions-umask-independent-modes`.
