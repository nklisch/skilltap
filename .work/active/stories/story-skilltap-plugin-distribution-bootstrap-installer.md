---
id: story-skilltap-plugin-distribution-bootstrap-installer
kind: story
stage: review
tags: [infra, content, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-command]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Online installer and plugin bootstrap parity

Align the curl/wget installer and the self-hosted plugin's agent guidance with
the first-class bootstrap command. After checksum verification and binary
installation, the online installer invokes the same Rust bootstrap boundary;
it detects Claude/Codex read-only and reports binary availability separately
from plugin setup. No second shell implementation of harness mutation is
allowed.

Scope:

- `install.sh` and a static `scripts/verify-installer.sh` check.
- Website getting-started/updates installation guidance and generated copies
  where applicable.
- Compiled/isolated installer contract tests.

Acceptance criteria:

- Clean isolated Linux/macOS fixtures can install a verified binary, detect
  fake harness executables, and invoke the shared bootstrap boundary without
  touching the operator's HOME, active sibling `../skills`, or native caches.
- Installer and binary choose the same latest asset/checksum protocol and are
  idempotent on a healthy repeat; unsupported platforms, malformed metadata,
  checksum failure, missing dependencies, and unsafe destinations fail safely.
- Website presents marketplace plugin installation and one-line installation
  as equal first-class paths, links to executable `skilltap bootstrap --help`,
  and does not reproduce a second command grammar.
- Missing or unsupported harness setup appears as a separate actionable result,
  never a false complete binary status. No arbitrary post-install command,
  shell interpolation, root escalation, or cache write is introduced.

If a documented native hook is later available, package it only through the
attested channel contract; otherwise preserve the explicit agent-invocable
bootstrap path and record the limitation in the plugin guidance.

## Implementation notes
- Execution capability: highest available local capability; installer parity and plugin handoff affect security and release trust.
- Review weight: standard (source: autopilot project default).
- Files changed: `install.sh`, `scripts/verify-installer.sh`, `website/guide/getting-started.md`, `website/guide/updates.md`, compiled bootstrap coverage.
- Tests added: POSIX syntax/static installer contract checks and isolated compiled binary install/no-op/major acknowledgment scenarios.
- Discrepancies from design: the shell entrypoint delegates post-install harness setup to the verified binary and accepts attention exit 2 while failing closed on pre-mutation errors.
- Adjacent issues parked: none.

## Review readiness
- `install.sh` validates absolute non-symlink destinations, verifies checksums before publication, and invokes the verified Rust bootstrap with a direct fixed argument vector.
- Unsupported harness setup is accepted as attention (exit 2) while binary/pre-mutation failures remain fatal.
- `scripts/verify-installer.sh`, `sh -n install.sh`, and the VitePress website build pass; generated `website/public/llms-full.txt` is synchronized.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: the shell installer publishes the downloaded bytes with plain
`mv` before invoking Rust bootstrap, then accepts any bootstrap exit `2` as
success (`install.sh:161-186`). A checksum-valid but wrong-version executable,
an unknown existing binary, or a blocked major upgrade can therefore leave a
broken/unverified binary while the installer reports installation complete;
this also bypasses the command's atomic publication and major-version policy.
The installer must verify the artifact identity before publication, preserve a
prior binary on failure, and distinguish binary attention from optional
harness attention (this item)

**Important**: release metadata/version parsing and shell downloads are not
bounded or host/redirect-attested like the Rust resolver (`install.sh:48-65,
128-141`), and `VERSION` is not validated as a release tag. The static script
check has no isolated installer fixture for malformed metadata, checksum
failure, unsafe destination, missing dependency, or repeat behavior; the
compiled bootstrap tests do not execute `install.sh`. Add bounded validation
and an offline shell contract harness (or explicitly constrain the installer
to a verified handoff) before approval (this item)

**Nits**: `validate_install_dir` checks only the final directory component;
existing destination symlinks and symlinked parent components should be
rejected before `mv`.

**Notes**: Standard substrate review at highest implementation capability with
standard review weight. `sh -n install.sh`, `scripts/verify-installer.sh`, and
the VitePress build pass, and website/package parity is present. The installer
does correctly delegate native harness setup through a fixed direct argument
vector and keeps unsupported harness attention visible, but its pre-bootstrap
binary publication remains a security and parity gap. Keep the story at
`stage: implementing` until that boundary is closed.

## Review (2026-07-12, installer hardening follow-up)

**Verdict**: Request changes

**Blockers**: the shell transport does not attest every redirect hop (this
item)

**Important**: the static installer fixture does not exercise redirect-host
rejection, unsupported platforms, or missing runtime dependencies; the Rust
installed-version probe accepts any semver token in arbitrary output rather
than requiring the exact `skilltap <version>` identity (this item)

**Nits**: temporary metadata files can remain when an early pre-trap download
fails

**Notes**: Standard fresh-context substrate review of `edcaf74` and `13d2558`,
with a focused security/correctness pass over the installer boundary. The
installer now verifies release tags, bounds metadata/checksum/artifact sizes,
checksums and candidate identity before invoking the Rust bootstrap, preserves
the prior binary on binary attention, rejects symlinked/foreign-owned
destinations, and delegates publication through the atomic Rust installer.
`sh -n install.sh`, `scripts/verify-installer.sh`, the core artifact suite,
and the CLI bootstrap matrix pass offline; website guidance is aligned.

The remaining blocker is redirect safety in the shell fallback: curl follows
up to three arbitrary HTTPS redirects and only checks the final effective URL,
while wget does not attest the effective URL at all. An intermediate
cross-host hop can therefore supply the metadata or executable before the
final URL returns to an allowed GitHub host, violating the same per-hop
release-host contract enforced by the Rust resolver. The installer must either
validate each hop before fetching it or constrain the shell path to a verified
handoff that provides that guarantee, and add a fixture proving a hostile
intermediate redirect fails closed. Keep this story at `stage: implementing`.
