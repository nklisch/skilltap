---
id: epic-skilltap-plugin-distribution-bootstrap
kind: feature
stage: review
tags: [infra, security]
parent: epic-skilltap-plugin-distribution
depends_on: [epic-skilltap-plugin-distribution-package]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Verified Skilltap Binary Bootstrap

## Brief

Provide the platform-aware bootstrap flow that makes a published skilltap
plugin useful on supported macOS and Linux systems. Bootstrap ensures the
skilltap binary is installed or repaired, selects and verifies the current
release artifact, detects installed Claude Code and Codex executables, and
installs or repairs the skilltap skill/plugin resources those harnesses can
load. It reports binary availability and harness resource setup separately.
Repeating a healthy setup must be a no-op, while failed downloads or
verification leave no misleading partial installation.

The feature must account for the native contract gap: Codex does not have an
attested non-interactive plugin mutation or post-install hook, and Claude's
trust/consent remains native. It therefore supplies a deterministic
agent-invocable path and uses native hooks only when capability evidence makes
that safe. The same boundary is callable by the one-line online installer
after binary verification. Binary updates default to the latest compatible
release, can be opted out of, and never auto-apply a major version unless the
user opts in. It never edits harness caches, requires root, or turns arbitrary
plugin installation into code execution.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: consumer of the package contract; release integration and
  final guidance depend on its verified bootstrap behavior.

## Foundation references

- `docs/SPEC.md` — Self-Hosted Plugin Distribution, Mutation Safety, Platform
  Contract, Validation
- `docs/ARCH.md` — Plugin Publication Boundary, Native Command Execution,
  Testing
- `install.sh` — existing platform and checksum protocol
- `.github/workflows/release.yml` — release assets, checksums, and attestations
- `crates/test-support/` — isolated machine and native-process fixtures

## Design decisions

- **Bootstrap responsibility**: The first-class bootstrap flow installs or
  repairs the latest binary, detects installed Claude Code and Codex binaries,
  and installs or repairs the skilltap resources those harnesses can load. It
  does not adopt environments, enable ordinary harness management, or mutate
  unrelated resources.
- **Online installer parity**: `install.sh` invokes the same bootstrap flow
  after binary verification, so marketplace installation and the one-line
  website install are equivalent setup methods.
- **Update policy**: Bootstrap/manual update fetches the latest release. The
  ongoing auto-update policy is enabled by default but opt-out; it applies
  within the current major version and requires explicit opt-in for major
  upgrades.

## Architectural choice

Use one deterministic bootstrap application boundary with three ports: a pure
release/policy planner in `skilltap-core`, a bounded artifact transport and
atomic binary installer at the runtime boundary, and harness adapters that
observe and invoke only attested native lifecycle commands. The CLI composition
root wires those ports for `skilltap bootstrap`; the online installer invokes
the same boundary after it has installed and verified a binary. The plugin
contains no cache-writing or arbitrary post-install executable path.

The alternatives were (1) keep all download and harness setup in `install.sh`,
which would make plugin setup and online setup diverge, and (2) let each native
plugin manager run an opaque post-install script, which would be unsafe and is
not an attested Codex contract. The shared application boundary keeps policy,
verification, output, and idempotency identical while allowing each channel to
use only its documented capabilities.

The highest-risk unit is native setup capability: Claude has a documented
non-interactive lifecycle on verified versions, while current Codex contracts
do not establish a deterministic non-interactive plugin install. Bootstrap
therefore probes the selected executable and profile first. Claude may receive
the native marketplace/plugin operation; Codex receives marketplace/source
registration only when attested and otherwise a structured `unsupported`
result with the exact agent next action. Neither path writes a cache or enables
ordinary harness reconciliation.

## Command contract

`skilltap bootstrap` is a global, non-interactive command. It installs or
repairs the latest platform artifact, then detects `claude` and `codex` and
attempts the first-party plugin setup independently for each reachable target.
Binary availability and harness resource setup are separate result entries.

The command has these flags:

```text
skilltap bootstrap [--target codex|claude|all] [--allow-major] [--json]
```

`--target` narrows harness setup only; the binary operation still runs once.
`--allow-major` is required when an existing binary would cross a major
version. A missing binary installs the latest release, including its current
major. Without the flag, an existing binary can update only within its current
major while still reporting that a newer major exists. A successful healthy
repeat is a no-op. Harness absence, unsupported native lifecycle, or an
unavailable optional target is reported as a bounded warning/attention result,
not mistaken for binary failure.

The command never asks for input, never enables a harness, never changes
project scope, and never installs arbitrary marketplace contents. Claude trust
and consent remain Claude's responsibility. Codex plugin setup remains
agent-invocable when native mutation is not attested.

## Implementation Units

### Unit 1: Pure bootstrap release and update policy

**Files**:

- `crates/core/src/bootstrap.rs`
- `crates/core/src/lib.rs`
- `crates/core/src/storage/config.rs`
- `crates/core/src/bootstrap/tests.rs`

**Story**: `story-skilltap-plugin-distribution-bootstrap-contract`

Define the platform/release value objects and pure decision functions. The
domain must not know HTTP, shell commands, concrete harnesses, or terminal
rendering:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapTarget { Codex, Claude, All }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReleaseVersion { major: u64, minor: u64, patch: u64 }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactKey { pub platform: SupportedPlatform, pub arch: ArtifactArch }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseArtifact {
    pub version: ReleaseVersion,
    pub key: ArtifactKey,
    pub asset_name: String,
    pub sha256: String,
    pub download_url: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryDecision { Install, Update, Noop, MajorUpgradeBlocked }

pub fn choose_binary_decision(
    installed: Option<&ReleaseVersion>,
    available: &ReleaseVersion,
    allow_major: bool,
) -> BinaryDecision;
```

Add a typed `BootstrapPolicy` to the config document for automatic binary
updates (`off`, `check`, `apply-safe`) and an explicit `allow_major` opt-in;
the default remains safe same-major automatic updates. The existing resource
update mode continues to govern plugins and skills. Config decoding remains
strict and preserves no secrets. Manual `bootstrap` always resolves latest;
the policy only controls unattended daemon/update-cycle application.

**Acceptance Criteria**:

- [ ] Release versions reject malformed, missing, or non-numeric semver fields
      and compare major/minor/patch numerically.
- [ ] A missing binary selects `Install`; a same-major newer version selects
      `Update`; an equal version selects `Noop`; a newer major is blocked unless
      `allow_major` is true.
- [ ] Artifact keys cover only supported macOS/Linux x64/arm64 combinations.
- [ ] Checksum and download locators are represented as validated values and
      cannot contain control characters or option-like command values.
- [ ] Config round-trips the binary update policy with existing unknown-field
      and schema guarantees; defaults are deterministic and read-only status
      does not persist them.

### Unit 2: Bounded release transport and atomic binary installation

**Files**:

- `crates/core/src/runtime/artifact.rs`
- `crates/core/src/runtime/mod.rs`
- `crates/core/src/runtime/filesystem.rs`
- `crates/cli/src/bootstrap.rs`
- `crates/core/tests/bootstrap_integration.rs`

**Story**: `story-skilltap-plugin-distribution-bootstrap-artifacts`

Define infrastructure ports and a system adapter that downloads only the
selected GitHub release asset and checksum document, verifies SHA-256 before
installation, and publishes the executable atomically into the configured
user install directory. The port shape is:

```rust
pub trait ReleaseResolver {
    fn latest(&self) -> Result<ReleaseManifest, ArtifactError>;
}

pub trait ArtifactFetcher {
    fn fetch(&self, url: &str, destination: &AbsolutePath) -> Result<(), ArtifactError>;
}

pub trait BinaryInstaller {
    fn inspect(&self, path: &AbsolutePath) -> Result<Option<InstalledBinary>, ArtifactError>;
    fn install_verified(
        &self,
        artifact: &AbsolutePath,
        destination: &AbsolutePath,
        expected: &ReleaseArtifact,
    ) -> Result<(), ArtifactError>;
}
```

The system fetcher uses bounded direct argument vectors for `curl` or `wget`
and never invokes a shell; it rejects redirects or URLs outside the release
allowlist, limits response size/time, and records no response body in output.
The installer writes a private temporary file beside the destination, checks
checksum and executable type, fsyncs/renames atomically, preserves an existing
healthy binary on failure, and cleans temporary artifacts. It never uses
`sudo`, modifies PATH profiles, or overwrites a binary whose identity changed
between observation and publish.

**Acceptance Criteria**:

- [ ] Latest manifest parsing requires one supported asset and a valid
      64-character SHA-256 entry; duplicate/missing assets fail closed.
- [ ] Download, checksum mismatch, unsupported platform, permission, and
      interrupted-install paths leave no destination or temporary-file
      corruption and retain the prior binary when one existed.
- [ ] A verified artifact is installed with user-private permissions and an
      executable bit; a post-install identity/version probe confirms it before
      reporting success.
- [ ] Fetching is direct and bounded; no shell interpolation, arbitrary URL,
      native cache write, root escalation, or secret-bearing output is possible.
- [ ] Isolated tests cover first install, same-major update, no-op repeat,
      checksum failure, replacement race, and cleanup using test-support roots.

### Unit 3: Harness detection and first-party plugin setup

**Files**:

- `crates/harnesses/src/bootstrap.rs`
- `crates/harnesses/src/lib.rs`
- `crates/harnesses/tests/bootstrap.rs`
- `crates/cli/src/bootstrap.rs`

**Story**: `story-skilltap-plugin-distribution-bootstrap-harness`

Add a read-first adapter that resolves configured `claude` and `codex`
executables, invokes their JSON version/capability probes, and returns a
bounded per-target setup result. Reuse `detect_configured_installation`,
`select_profile`, `NativeLifecyclePort`, and the package's canonical source
selector rather than adding a second native command builder.

```rust
pub enum HarnessSetupResult {
    Installed { harness: HarnessKind, version: NativeVersion },
    AlreadyPresent { harness: HarnessKind, version: NativeVersion },
    Unavailable { harness: HarnessKind, reason: SetupReason },
    Unsupported { harness: HarnessKind, next_action: String },
    Failed { harness: HarnessKind, reason: SetupReason },
}

pub fn setup_first_party_plugin(
    target: HarnessKind,
    policy: &HarnessBootstrapPolicy,
) -> HarnessSetupResult;
```

For Claude, a verified profile may register the canonical marketplace source
and install/update `skilltap` through its native user-scoped lifecycle. For
Codex, the adapter may register the canonical marketplace source when its
profile grants that operation, but never treats the interactive plugin browser
as a successful non-interactive install. The result tells the agent to run the
documented native flow or use the complete standalone skill path. Existing
native resources are observed before mutation; repeats are no-op and unknown
list output blocks rather than reinstalling. The adapter never enables ordinary
skilltap harness management and never writes cache directories.

**Acceptance Criteria**:

- [ ] `--target all` probes both known harnesses independently; one missing or
      unsupported target does not hide a successful binary result for the other.
- [ ] Claude native lifecycle vectors use user scope and the canonical source;
      Codex receives only attested marketplace operations and an actionable
      unsupported result for plugin mutation gaps.
- [ ] Existing `skilltap` plugin presence is observed before setup and a
      healthy repeat emits `AlreadyPresent` without a native mutation.
- [ ] Unknown/malformed native JSON is conservative, secrets and raw payloads
      stay inside the adapter, and cache trees are never written.
- [ ] Fake binaries prove command vectors, target isolation, unsupported
      capability handling, and per-target result classification.

### Unit 4: First-class CLI application and result rendering

**Files**:

- `crates/cli/src/command.rs`
- `crates/cli/src/dispatch.rs`
- `crates/cli/src/entrypoint.rs`
- `crates/cli/src/application/bootstrap.rs`
- `crates/cli/src/application.rs`
- `crates/cli/src/command/tests.rs`
- `crates/cli/src/entrypoint/tests.rs`
- `crates/cli/tests/compiled_binary.rs`

**Story**: `story-skilltap-plugin-distribution-bootstrap-command`

Expose `bootstrap` as a public leaf with shared help/exit guidance and
`--target`, `--allow-major`, and `--json`. The application composes the core
decision, artifact adapter, and harness adapter, then emits one result with
separate binary and harness entries, warnings, and next actions. It maps
partial setup to the existing attention/partial exit classes without allowing
`--yes` to bypass unsupported required native behavior.

```rust
pub struct BootstrapArgs { pub target: TargetSelection, pub allow_major: bool, pub output: OutputArgs }

pub struct BootstrapApplication<P, R, H> {
    policy: P,
    release: R,
    harnesses: H,
}

impl<P, R, H> BootstrapApplication<P, R, H> {
    pub fn execute(&self, args: &BootstrapArgs) -> Outcome;
}
```

`bootstrap --json` uses the existing schema-1 result envelope. Plain output
names the installed path only as a safe role/path and reports the next action
for unavailable or unsupported targets. No invocation creates a project state
or enables a harness.

**Acceptance Criteria**:

- [ ] Root and leaf help describe bootstrap purpose, target semantics, major
      acknowledgment, JSON output, and all exit classes.
- [ ] Plain and JSON results distinguish binary install/update/no-op from each
      harness setup result and include actionable unsupported-target next steps.
- [ ] Existing binary/config state remains untouched on failed pre-mutation
      planning or verification; no prompt or arbitrary source argument exists.
- [ ] Compiled tests cover first install, repeat no-op, target narrowing,
      missing harnesses, blocked major upgrade, and mixed success/attention.

### Unit 5: Online installer parity and plugin bootstrap handoff

**Files**:

- `install.sh`
- `scripts/verify-installer.sh`
- `website/guide/getting-started.md`
- `website/guide/updates.md`
- `crates/cli/tests/compiled_binary.rs`

**Story**: `story-skilltap-plugin-distribution-bootstrap-installer`

After the shell installer verifies and installs the release binary, it invokes
that binary's bootstrap boundary with the detected harness set. Detection is
read-only and optional; the installer never assumes either harness is present,
never edits their caches, and reports binary success separately from plugin
setup. The plugin's skill points agents at `skilltap bootstrap` and the
one-line installer as equivalent first-party setup paths. Where a native host
documents an automatic hook, release packaging may register the fixed
bootstrap entrypoint; otherwise the skill remains the explicit agent-invocable
fallback. No arbitrary post-install script is added to either manifest.

The script remains POSIX shell for the curl/wget entry point, but delegates
artifact policy, checksum parsing, harness detection, and output semantics to
the verified Rust binary wherever possible. Its fallback errors are stable,
secret-safe, and point to the release URL or `skilltap bootstrap --help`.

**Acceptance Criteria**:

- [ ] A clean isolated macOS/Linux fixture can run the installer, verify the
      binary, detect fake Claude/Codex executables, and invoke the same setup
      boundary without touching the real home or native caches.
- [ ] The installer and `skilltap bootstrap` select the same latest asset and
      checksum protocol; healthy repeats are no-op.
- [ ] Website installation guidance presents plugin marketplace installation
      and the one-line installer as equal first-class paths and links executable
      help instead of copying the full grammar.
- [ ] Harness absence or unsupported Codex mutation is visible as a separate
      setup result with an agent next action, never a false complete status.
- [ ] Shell checks reject unsupported platforms, malformed release metadata,
      checksum mismatch, missing dependencies, and unsafe install destinations
      without leaking environment values.

## Implementation Order

1. `story-skilltap-plugin-distribution-bootstrap-contract` — pure release
   identity, policy, and config contract (depends on the package feature).
2. `story-skilltap-plugin-distribution-bootstrap-artifacts` — bounded fetch and
   atomic binary installation (depends on the contract).
3. `story-skilltap-plugin-distribution-bootstrap-harness` — target detection and
   native first-party setup (depends on the contract and package feature).
4. `story-skilltap-plugin-distribution-bootstrap-command` — CLI composition,
   output, and compiled contract (depends on artifacts and harness).
5. `story-skilltap-plugin-distribution-bootstrap-installer` — shell/website
   parity and plugin handoff (depends on the command).

## Testing

### Pure contract tests

`crates/core/src/bootstrap/tests.rs` covers semver and major-policy decisions,
artifact identity validation, config defaults/round trips, and malformed
release metadata. These tests use no filesystem, network, or native process.

### Boundary and integration tests

`crates/core/tests/bootstrap_integration.rs` and
`crates/harnesses/tests/bootstrap.rs` use `skilltap-test-support` temporary
homes, fake release manifests, fake binaries, and snapshot assertions. They
prove bounded transport, checksum-before-publish, cleanup, executable identity
revalidation, native argument vectors, target isolation, and unsupported
Codex behavior.

`crates/cli/tests/compiled_binary.rs` exercises every bootstrap help/error and
result path using isolated environment variables. No test uses the operator's
HOME, Claude/Codex installation, network, or active sibling `../skills` repo.

`scripts/verify-installer.sh` statically checks shell quoting, dependency
guards, checksum delegation, and that the installer calls the compiled
bootstrap boundary rather than duplicating harness mutation logic.

## Risks

- **Codex lifecycle gap**: the public contract may remain interactive-only.
  Bootstrap must report that boundary rather than write caches or claim a
  plugin install. The standalone skill and installer remain usable fallbacks.
- **Self-update replacement race**: an executable can change after inspection;
  publish must revalidate identity immediately before atomic replacement and
  retain the previous file on any failure.
- **Latest-major ambiguity**: latest is always resolved, but automatic policy
  cannot silently cross a major. Fresh installs take the latest major;
  existing installs report a blocked major with `--allow-major` as the explicit
  user decision.
- **Installer drift**: the shell entry point must not become a second policy
  implementation. Compiled parity tests and static checks keep it delegated to
  the Rust boundary.
- **Native trust/consent**: Claude and Codex may require user approval or
  interactive flows outside skilltap's authority. Results must preserve those
  boundaries and give an exact next action.

## Implementation notes

- Execution capability: highest available local capability; this feature spans release artifacts, security-sensitive installation, and native harness contracts.
- Review weight: standard (autopilot project default).

## Review findings (2026-07-12)

- **Blocker — unattended binary update policy is inert**: `BinaryUpdatePolicy`
  is persisted and round-trips, but `StatusApplication::execute_daemon_cycle`
  reads only the ordinary resource update policy and never resolves or applies
  the skilltap binary. The documented default same-major self-update,
  opt-out/check modes, and explicit major opt-in therefore do not execute.
  Follow-up: `story-skilltap-plugin-distribution-bootstrap-daemon-binary-policy`.
- **Important — CLI rollback can clobber a replacement**:
  `restore_previous_binary` checks the destination identity and then performs
  an overwrite-capable `rename`/`write`. A replacement between those steps can
  be overwritten during post-install identity recovery. Follow-up:
  `story-skilltap-plugin-distribution-bootstrap-cli-rollback-safety`.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: daemon/update-cycle binary policy is unimplemented ->
`story-skilltap-plugin-distribution-bootstrap-daemon-binary-policy`

**Important**: CLI post-install rollback remains identity-unsafe ->
`story-skilltap-plugin-distribution-bootstrap-cli-rollback-safety`

**Nits**: none

**Notes**: Deep substrate review at standard weight with fresh context. All
direct child stories were done and the full offline workspace test, clippy,
format, installer contract, shell syntax, website build, and diff checks were
green. Foundation-doc and product-contract review found the two follow-ups
above; the feature remains implementing until both are closed.
