---
id: epic-skilltap-plugin-distribution
kind: epic
stage: implementing
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

## Design decisions

- **How is the epic split?** The package contract, CLI contract, and binary
  bootstrap are separate capability features because they have different
  ownership and verification surfaces. Guidance consumes those contracts;
  release integration consumes all of them; sibling cutover is deliberately
  last. This avoids a layer-shaped split while keeping the critical path
  visible.
- **Is there a UI surface?** No. The deliverable is native plugin metadata,
  terminal help/errors, release automation, and portable skill prose. No
  mockups or dashboard work apply.
- **Where does native uncertainty live?** The bootstrap feature owns the
  Codex/Claude post-install capability gap and must preserve explicit
  agent-invocable setup. The package and release features may not assume an
  undocumented hook or cache mutation path.
- **What is the version source?** Cargo release version, plugin channel
  metadata, marketplace entries, checksums, and attestation references are one
  release identity. The release feature owns parity checks; no independent
  sibling version stream is introduced.

## Decomposition

The realized decomposition follows capability boundaries and keeps package and
CLI work parallel. Bootstrap depends on the package shape; guidance waits for
the package, stable help contract, and verified bootstrap instructions. Release
publication then consumes every artifact contract, and the sibling cutover is
the final dependent handoff.

### Child features

- `epic-skilltap-plugin-distribution-package` — canonical plugin tree,
  complete shared skill boundary, native manifests/catalogs, and identity
  validation — depends on: `[]`.
- `epic-skilltap-plugin-distribution-cli-contract` — executable help, plain and
  JSON diagnostics, next actions, and compiled-binary contract — depends on:
  `[]`.
- `epic-skilltap-plugin-distribution-bootstrap` — verified macOS/Linux binary
  bootstrap with explicit native-hook capability handling — depends on:
  `[epic-skilltap-plugin-distribution-package]`.
- `epic-skilltap-plugin-distribution-guidance` — high-level portable skill and
  diagnostic references — depends on:
  `[epic-skilltap-plugin-distribution-package,
  epic-skilltap-plugin-distribution-cli-contract,
  epic-skilltap-plugin-distribution-bootstrap]`.
- `epic-skilltap-plugin-distribution-release` — versioned plugin/binary
  publication, checksums, attestations, website, install, and Homebrew
  alignment — depends on:
  `[epic-skilltap-plugin-distribution-package,
  epic-skilltap-plugin-distribution-cli-contract,
  epic-skilltap-plugin-distribution-bootstrap,
  epic-skilltap-plugin-distribution-guidance]`.
- `epic-skilltap-plugin-distribution-cutover` — sibling publisher retirement,
  superseded-skill removal, and archive handoff — depends on:
  `[epic-skilltap-plugin-distribution-release]`.

## Decomposition risks

- Codex's public contract does not guarantee a non-interactive plugin install
  or post-install hook. Treating native installation as binary setup would
  create a false-success path; bootstrap must remain explicitly observable.
- The existing `website/public/install.sh` has drifted from the checksum-
  verifying root installer. Release design must establish one generated or
  parity-checked installer path before publishing plugin guidance.
- The sibling repository currently has no skilltap marketplace entry, so the
  cutover must verify the canonical publication rather than assume a mirror is
  already equivalent. Repository archival authority is external and requires
  an explicit handoff record.
- Package, guidance, and release all touch publication assets. Their ownership
  boundaries must stay explicit so version parity is generated or validated
  from one source rather than maintained by repeated manual edits.

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
