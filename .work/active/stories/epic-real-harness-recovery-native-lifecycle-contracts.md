---
id: epic-real-harness-recovery-native-lifecycle-contracts
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Attest exact native profiles and command vectors

## Scope

Replace the synthetic shared profile with exact current Codex and Claude
contracts, and build every lifecycle/list vector from operation-specific scope
rules. This story owns blocker 2 and blockers 5-7.

## Acceptance

- Codex `0.144.1` and Claude Code `2.1.201` select only their attested
  capabilities; `3.0.0`, adjacent, cross-harness, and unknown versions remain
  observe-only.
- Claude plugin/marketplace list and marketplace update omit unsupported
  `--scope`; only commands that accept it receive `user` or `local`.
- Codex `0.144.1` never emits `plugin update`; the operation is explicitly
  unavailable unless a future exact profile supplies an attested strategy.
- Fake-native grammar, adapter tests, compiled-binary tests, and
  `docs/HARNESS-CONTRACTS.md` agree with disposable real-CLI evidence.
- All vectors retain direct arguments, bounded execution, explicit environment,
  and the exact global/project working directory.

## Implementation

- Replaced the synthetic shared profile with exact `codex-0-144-1` and
  `claude-2-1-201` profiles. Adjacent, synthetic, cross-harness, and unknown
  versions remain observe-only.
- Codex grants its attested global add/list/remove and marketplace lifecycle,
  while plugin update and every native project mutation remain unverified.
- Claude list observation and marketplace update now omit the unsupported
  scope option. Add/remove and plugin mutations retain their exact user/local
  scope and project working directory.
- Fake native fixtures and compiled-binary lifecycle coverage now identify the
  harness explicitly instead of using one impossible shared version.
- Updated the harness contract with the exact current-version command gaps.

## Verification

- Disposable real `codex 0.144.1` and `claude 2.1.201` help probes under six
  isolated roots for every lifecycle/list command.
- `cargo test -p skilltap-harnesses --all-targets`
- `cargo test -p skilltap --test compiled_binary`

## Review findings (2026-07-12)

- **Blocker — Claude native presence ignores concrete scope**: current Claude `plugin list --json` output includes a `scope` field, but `resource_presence` matches only the resource identity. A global plugin or marketplace with the same name can therefore satisfy a project/local re-observation, causing a drifted or removed project resource to be reported present and skipped. Tracked by `epic-real-harness-recovery-native-lifecycle-scope-aware-presence`.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: `epic-real-harness-recovery-native-lifecycle-scope-aware-presence`
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the project-default `standard` weight for the public native command and scope contract. Commit `8f0b333` correctly attests exact versions, removes unsupported Claude scope flags, and blocks Codex plugin update. An isolated real Claude invocation confirmed list entries carry explicit scope; the observation parser must use it to preserve exact global/project identity. Harness-focused tests otherwise pass.
