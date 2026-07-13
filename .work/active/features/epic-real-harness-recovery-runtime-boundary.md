---
id: epic-real-harness-recovery-runtime-boundary
kind: feature
stage: done
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on: []
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Detect and isolate real harness processes

## Brief

Make the harness process boundary work with current real Codex and Claude
executables. Detection must parse each documented version form, preserve an
explicit minimal environment containing the configured home and harness roots,
resolve `CODEX_HOME` and `CLAUDE_CONFIG_DIR` consistently, and report the actual
failing boundary when detection cannot complete.

This feature owns blocker inventory entries 1, 3, 4, and 8. It keeps newly
recognized versions observe-only until the native-lifecycle feature corrects
the command surface and grants attested capabilities. It does not change native
marketplace/plugin vectors or resource-state semantics.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: foundation feature for real native lifecycle validation.

## Foundation references

- `docs/ARCH.md` — capability detection and native command execution.
- `docs/HARNESS-CONTRACTS.md` — detection and mutation-authority rules.
- `docs/SPEC.md` — validation, status, and mutation safety.

## Design decisions

- **Version command:** invoke the documented `--version` surface and decode
  harness-specific plain text, while retaining strict support for a single JSON
  document with a `version` string for compatible/fake implementations. Never
  scrape arbitrary digit sequences from prose.
- **Mutation authority:** detection returns an opaque validated version only.
  This feature does not authorize newly recognized versions; the native
  lifecycle feature owns the attested capability registry after correcting its
  command vectors.
- **Child environment:** keep `env_clear()` in the bounded runner and construct
  one explicit safe environment from resolved platform paths plus the caller's
  executable search path. Forward `HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`,
  `CODEX_HOME`, `CLAUDE_CONFIG_DIR`, and `PATH`; do not inherit proxy, token,
  shell, or arbitrary caller variables.
- **Root precedence:** `CODEX_HOME` and `CLAUDE_CONFIG_DIR` independently
  override their native roots. Canonical `~/AGENTS.md` remains anchored to
  `HOME`; XDG cache defaults to `$HOME/.cache`.
- **Diagnostics:** preserve a closed detection-failure category through CLI
  projection. Output identifies absent executable, nonzero version command,
  invalid version format, runtime limit, or changed executable without exposing
  argv, native output, or environment values.

## Architectural choice

Extend the existing core runtime path/environment contract and pass one
validated process context into harness adapters. Keep harness-specific version
decoding private to `skilltap-harnesses`, and add one CLI projection from typed
`DetectionError` to registered warning/next-action values.

Alternatives rejected:

- Inheriting the parent environment would restore functionality quickly but
  reintroduce secret leakage and make isolated tests unable to constrain native
  children.
- Letting every lifecycle call rebuild its own environment would duplicate a
  security-sensitive allowlist and allow detection, observation, bootstrap,
  and mutation to drift.
- Accepting any numeric substring as a version would make unrelated human text
  mutation-authorizing input; decoders remain harness-specific and bounded.

## Implementation Units

### Unit 1: Resolved native process context
**Files**: `crates/core/src/runtime/error.rs`,
`crates/core/src/runtime/paths.rs`, `crates/core/src/runtime/observation.rs`,
`crates/core/src/runtime/mod.rs`, `crates/test-support/src/lib.rs`
**Story**: `epic-real-harness-recovery-runtime-boundary-process-context`

```rust
pub enum EnvironmentVariable {
    Home,
    XdgConfigHome,
    XdgCacheHome,
    CodexHome,
    ClaudeConfigDir,
    Path,
}

pub struct PlatformPaths {
    // existing fields
    cache_home: AbsolutePath,
    claude_home: AbsolutePath,
}

impl PlatformPaths {
    pub fn native_process_environment(
        &self,
        search_path: Option<OsString>,
    ) -> Result<BTreeMap<OsString, OsString>, RuntimeError>;
    pub const fn cache_home(&self) -> &AbsolutePath;
}
```

Implementation notes:

- Resolve and validate every configured path once; `CLAUDE_CONFIG_DIR` follows
  the same absolute/non-empty rules as `CODEX_HOME`.
- The process map always contains resolved path values, including default
  roots, and requires a non-empty `PATH` before native execution.
- Extend `IsolatedMachine` with cache, Codex, and Claude roots and expose one
  canonical environment builder used by compiled-binary tests.

Acceptance criteria:

- [ ] Custom and default roots resolve independently and canonical global
  instructions remain under `HOME`.
- [ ] The child map contains exactly the six allowed variables and rejects a
  missing/invalid search path before process execution.
- [ ] Bounded-runner tests prove allowed values arrive byte-exact and an
  unrelated canary environment variable does not.

### Unit 2: Harness-specific version decoding
**Files**: `crates/harnesses/src/lib.rs`,
`crates/harnesses/tests/detection.rs`, `crates/test-support/src/native_process.rs`,
`crates/harnesses/tests/bootstrap.rs`
**Story**: `epic-real-harness-recovery-runtime-boundary-version-decoding`

```rust
fn version_arguments(harness: HarnessKind) -> Vec<OsString>;

fn decode_native_version(
    harness: HarnessKind,
    stdout: &[u8],
    json_limits: JsonLimits,
) -> Result<NativeVersion, DetectionError>;

pub fn detect_configured_installation(
    harness: HarnessKind,
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    environment: &BTreeMap<OsString, OsString>,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
) -> Result<HarnessInstallation, DetectionError>;
```

Implementation notes:

- Accept one strict JSON document containing a string `version`, Codex's
  attested `codex-cli <version>` form, and Claude's attested
  `<version> (Claude Code)` form. Reject trailing documents, missing tokens,
  control characters, and cross-harness formats.
- Update fake modes to represent real Codex and Claude version payloads rather
  than one shared synthetic JSON version.
- Do not change `select_profile` here; real decoded versions remain
  observe-only until the dependent lifecycle contract lands.

Acceptance criteria:

- [ ] Codex 0.144.1 and Claude 2.1.201 fixtures are reachable with their exact
  native versions.
- [ ] JSON, malformed, cross-harness, over-limit, nonzero, and timeout cases
  retain distinct typed results.
- [ ] Detection child argv and environment are captured exactly and contain no
  inherited canary.

### Unit 3: Typed detection diagnostics and real-root regression
**Files**: `crates/cli/src/entrypoint.rs`,
`crates/cli/src/application/status.rs`, `crates/cli/src/application.rs`,
`crates/cli/tests/compiled_binary.rs`
**Story**: `epic-real-harness-recovery-runtime-boundary-diagnostics`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DetectionFailureKind {
    ExecutableUnavailable,
    VersionCommandFailed,
    VersionFormatInvalid,
    RuntimeLimit,
    ExecutableChanged,
}

fn project_detection_error(
    harness: HarnessKind,
    error: &DetectionError,
) -> (DetectionFailureKind, Warning, NextAction);
```

Implementation notes:

- Use the same projection in `harness list`, first-use status, normal status,
  planning, and lifecycle capability lookup.
- Context contains only registered scalar labels such as harness and failure
  kind. Suggested actions distinguish installing/fixing the binary from
  upgrading an unsupported format or inspecting harness help.
- Add compiled scenarios with isolated `HOME`, XDG, Codex, and Claude roots;
  assert no real user tree changes before and after.

Acceptance criteria:

- [ ] All public detection surfaces agree on reachability/version and safe
  failure category.
- [ ] Invalid version output is not described as an absent executable.
- [ ] Plain and JSON output derive from the same result and expose no raw output,
  argv, paths outside declared resources, or environment values.

## Implementation Order

1. Process context and isolated roots.
2. Version decoding in parallel with the process-context unit where practical.
3. CLI diagnostic projection after both contracts are available.

## Testing

- Core unit tests cover path precedence, explicit environment construction,
  missing `PATH`, and canary non-inheritance.
- Harness contract tests cover exact real version forms, strict JSON fallback,
  invalid shapes, limits, and process capture.
- Compiled-binary tests cover harness list, first-use status, configured status,
  planning, custom roots, and read-only tree snapshots.
- A manually gated isolated-real test records current `codex --version` and
  `claude --version` behavior without authentication or mutation.

## Risks

- Native CLIs may need certificate/proxy variables for network mutations. This
  feature intentionally does not inherit them; later lifecycle design must add
  any required variable as an explicit, reviewed contract rather than widening
  the environment implicitly.
- Real version presentation can change. Unknown forms remain a typed observable
  failure and cannot grant mutation authority until attested.

## Children complete (2026-07-12)

All direct stories completed review; the feature advanced through its own
aggregate review.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep aggregate review at the project-default
`standard` weight. The four direct stories collectively implement the intended
plain/JSON version decoding, explicit process environment, root precedence,
and closed actionable diagnostic projection. Detached focused regressions at
`29afee5` cover all failure categories and healthy sibling preservation; no
foundation drift was found.
