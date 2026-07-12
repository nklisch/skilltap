---
id: epic-skilltap-plugin-distribution
kind: epic
stage: drafting
tags: [infra, content, architecture]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Skilltap Self-Hosted Plugin Distribution

## Brief

Publish one first-party skilltap plugin from this repository for both Claude
Code and Codex. The plugin is deliberately small: one complete `skilltap`
skill that gives an agent high-level orientation, command-family guidance,
configuration-layout knowledge, and debugging/status workflows. It is a
discovery surface for the skilltap binary, not a marketplace browser,
recommendation engine, or second configuration control plane.

The plugin must be released alongside the Rust binary and provide a reliable
path to a usable `skilltap` executable. That path must respect each harness's
native plugin contract, verify platform-specific release artifacts, and make
binary availability observable rather than claiming that plugin installation
alone completed setup. The same release must publish distinct Claude and Codex
manifests and marketplace catalogs from one canonical source in this
repository.

This epic also performs the publication cutover. The sibling `../skills`
repository is a temporary migration mirror, not a second source of truth. Once
the skilltap plugin is published and verified, the old skilltap-related skills
(including the former `claude-code-marketplace` surface) are retired there and
the repository is archived according to its own repository controls.

## Foundation references

- `docs/VISION.md` — Core Idea, Agent Forward, Non-Goals
- `docs/SPEC.md` — Self-Hosted Plugin Distribution, Plugin Lifecycle, Output,
  Platform Contract, Validation
- `docs/ARCH.md` — Plugin Publication Boundary, Plugin Resolution, Native
  Command Execution, Testing
- `docs/UX.md` — Help and Diagnostic Discovery, Command Tree, Output, Errors
- `docs/HARNESS-CONTRACTS.md` — Codex and Claude plugin/marketplace contracts
- `.research/analysis/briefs/current-agent-extension-standards.md` — current
  cross-harness extension boundary
- `.research/analysis/campaigns/marketplace-standards/specialists/codex.md`
- `.research/analysis/campaigns/marketplace-standards/specialists/claude.md`
- `.research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md`

## Strategic decisions

- **Which repository owns the plugin?** This skilltap repository is the sole
  canonical publisher. `../skills` may mirror the plugin only long enough to
  cut users over; it must not remain a bidirectional publishing source.
- **How are the two harnesses represented?** One public plugin identity is
  expressed through separate native Claude and Codex manifests and marketplace
  catalogs. The shared unit is the complete skill directory; manifests,
  catalogs, cache state, and lifecycle semantics remain harness-specific.
- **What does “install the binary automatically” mean?** The released plugin
  must provide a deterministic bootstrap path that an agent can invoke during
  setup and that installs a verified platform artifact without elevation. A
  native post-install hook may be used only when the host documents and
  exposes that behavior. Otherwise the plugin supplies one explicit bootstrap
  command and reports the binary as a separate setup result; it must never
  mutate a native plugin cache or silently claim success.
- **What belongs in the skill?** The skill teaches purpose, command selection,
  scopes and targets, state/config locations, status/plan/sync interpretation,
  updates/daemon behavior, and recovery decisions. Exact flags, schemas, and
  exit classes remain in the executable `--help` surface and stable CLI
  output, not a hand-maintained duplicate reference.
- **What is the primary install scope?** The plugin and binary are personal,
  user-level tooling for the managed computer. Project-scoped declarations may
  be documented if a native channel requires them, but project installation is
  not a separate product requirement for this epic.
- **Does this introduce discovery?** No. The plugin may explain how to use
  explicitly selected marketplaces and resources, but it must not search,
  rank, recommend, or inventory marketplace contents.

## Scope boundaries

In scope:

- Native Claude and Codex plugin manifests with contract-valid component paths.
- Native marketplace catalog definitions that point to the canonical release
  source without flattening channel-specific schemas.
- One complete `skilltap` skill directory with strict frontmatter and bundled
  references only where they improve high-level diagnosis.
- Platform-aware binary bootstrap, release artifact verification, idempotency,
  failure cleanup, and explicit installed-binary status.
- Version and provenance parity between Cargo releases, plugin metadata,
  marketplace entries, checksums, and attestations.
- A CLI help/error quality pass for the root, command groups, and every public
  leaf command, with plain and JSON tests for actionable, secret-safe output.
- Release workflow, website/install documentation, and Homebrew story updates
  needed to make the plugin and binary installation path coherent.
- A cutover/deprecation record for the sibling `../skills` publication, with
  the old skilltap skills retired and the repository archived after verification.

Out of scope:

- A universal plugin manifest or a new marketplace/search service.
- Automatic installation of arbitrary third-party plugins or skills.
- Project-collaborator setup, shared skilltap state, or repository metadata.
- Native cache editing, undocumented Codex plugin mutation, or bypassing
  Claude trust and consent boundaries.
- A background watcher for plugin installation; the existing optional daemon
  remains an update mechanism, not a plugin bootstrap service.

## Anticipated child features

- `epic-skilltap-plugin-distribution-package` — define the canonical plugin
  tree, complete shared skill, channel manifests, marketplace catalogs, and
  version/identity validation.
- `epic-skilltap-plugin-distribution-guidance` — author the high-level skill,
  command discovery map, configuration/debug references, and deprecation of
  the obsolete skill surface.
- `epic-skilltap-plugin-distribution-bootstrap` — design and implement the
  platform-aware verified binary bootstrap with explicit native-hook
  capability handling and idempotent isolated tests.
- `epic-skilltap-plugin-distribution-cli-contract` — audit and improve
  `--help`, errors, JSON envelopes, next actions, and compiled-binary
  coverage so agents can learn the CLI directly.
- `epic-skilltap-plugin-distribution-release` — integrate plugin packaging,
  checksums, attestations, website/install/Homebrew documentation, and release
  validation into one versioned publication path.
- `epic-skilltap-plugin-distribution-cutover` — migrate the canonical skill
  from `../skills`, retire `claude-code-marketplace` there, add the archival
  notice, and verify no duplicate publisher remains.

## Decomposition constraints

The package contract must precede release wiring, and the CLI contract must
precede final skill prose so the skill can link agents to real help output. The
bootstrap design must be reviewed against both native plugin contracts before
it promises automatic execution. Release integration depends on package,
guidance, bootstrap, and CLI contract completion. The sibling cutover is last:
it cannot remove the old publisher until the canonical marketplace entries,
binary bootstrap, and release verification are live.

Every mutating bootstrap test runs in an isolated fixture root and repeats the
operation to prove idempotency. Tests must distinguish stale documentation or
fixtures from real production bugs; real bootstrap or release correctness
failures become tracked work rather than being hidden in test changes.

## Acceptance criteria

- Claude and Codex load the published plugin from their own valid manifest and
  marketplace formats, with one matching public identity and release version.
- The installed plugin contains the complete skill directory with a top-level
  `SKILL.md`; supporting files are preserved as part of the skill artifact.
- A clean supported macOS or Linux environment can follow the documented
  plugin/bootstrap path to obtain a verified `skilltap` binary, and repeat
  setup without changing an already healthy installation.
- Unsupported native automatic-hook behavior is detected and reported with a
  safe next action; no implementation writes harness caches or requires root.
- Root, group, and leaf `--help` output is concise and descriptive, and plain
  and JSON errors identify the boundary, redact secrets, provide next actions,
  and preserve the documented exit classes.
- Plugin metadata, binary assets, checksums, attestations, website guidance,
  and Homebrew/install instructions cannot silently drift in version or source
  identity.
- `../skills` no longer publishes the canonical skilltap plugin after cutover;
  its duplicate skilltap surfaces are retired and the repository has an
  explicit archival/deprecation record.
- No part of the plugin or skill adds marketplace search, ranking,
  recommendation, or broad inventory discovery.

<!-- The epic-design pass will turn these seams into dependency-ordered child
features and resolve implementation-level native/bootstrap details. -->
