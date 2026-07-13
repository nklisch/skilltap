---
id: epic-real-harness-recovery-bootstrap-transport
kind: feature
stage: done
tags: [correctness, security, infra, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on: []
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Repair bootstrap release transport

## Brief

Correct release HTTP status/redirect parsing so successful non-redirected
responses are accepted while every redirect hop retains the existing host,
checksum, size, and atomic-publication protections. Bootstrap and the binary
update lifecycle must work in an isolated home against the current release
manifest and repeat without change.

This feature owns blocker inventory entry 11 and the direct clean-room
bootstrap failure. Harness plugin setup remains in the runtime/native lifecycle
features.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: independent release-distribution repair.

## Foundation references

- `docs/SPEC.md` — self-hosted plugin distribution and update daemon.
- `docs/ARCH.md` — plugin publication and verified artifact boundary.

## Design decisions

- **Response framing:** Keep curl's portable explicit
  `%{http_code}\n%{redirect_url}` write-out contract and parse it in forward
  order with a required first separator. This preserves the empty redirect
  field produced by a successful non-redirected response without depending on
  newer curl-only JSON write-out support.
- **Abstraction boundary:** Add a private validated frame type inside the
  production artifact adapter rather than changing the public
  `ArtifactFetcher` port. HTTP status and redirect metadata are curl adapter
  details; release resolution and bootstrap policy continue to consume only
  `Result<(), ArtifactError>`.
- **Implementation dispatch:** Direct-read only and one cohesive implementation
  stride. The parser, its use by `SystemArtifactFetcher`, and the regression
  tests share one file and one failure mode, so child stories would not create
  useful parallel ownership or resume points.
- **Foundation timing:** No foundation edit is required. `docs/SPEC.md` and
  `docs/ARCH.md` already describe the intended verified, redirect-attested
  transport; this feature brings the implementation back into conformance.

## Architectural choice

Parse curl's existing two-field stdout into a small private validated value,
then pass that value to the existing redirect policy. This keeps three concerns
separate inside the runtime adapter: curl owns transport execution, the frame
parser owns unknown stdout validation, and `redirect_target` owns HTTP/host
policy. The chosen approach fixes the empty-final-field bug without weakening
the current per-hop URL allowlist, process limits, private temporary files,
bounded reads, checksum verification, or atomic publication.

Alternatives considered:

1. Switch curl to `%json` and deserialize the response. That gives named fields
   but commits bootstrap to a newer curl feature than the current explicit
   write-out contract requires.
2. Keep reverse line iteration and special-case a one-line `200`. That repairs
   the observed symptom but leaves framing ambiguous and accepts malformed
   output through position-dependent heuristics.
3. Introduce a new public HTTP transport port. That is disproportionate for a
   private curl framing defect; `ArtifactFetcher` is already the correct core
   boundary and remains easy to fixture in resolver/bootstrap tests.

## Implementation units

### Unit 1: Validate and consume curl response frames

**Files**:

- `crates/core/src/runtime/artifact.rs`
- `crates/cli/src/bootstrap_commands.rs` (test assertions only, if the existing
  isolated bootstrap matrix does not already prove every outcome field)

The production-only representation remains private to the artifact adapter:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CurlResponseFrame<'a> {
    status: u16,
    redirect: &'a str,
}

fn parse_curl_response_frame(
    response: &[u8],
) -> Result<CurlResponseFrame<'_>, ArtifactError>;

fn redirect_target(
    status: u16,
    redirect: &str,
) -> Result<Option<String>, ArtifactError>;
```

Implementation notes:

- Decode stdout as UTF-8 and split once, in forward order, at the required
  newline emitted by `--write-out`. Do not use `str::lines()` because it omits
  the trailing empty record that carries the no-redirect case.
- Parse a trimmed first field as an HTTP status and reject a missing separator,
  an empty or nonnumeric status, and any additional newline-delimited field as
  `ArtifactError::DownloadFailed`. A CR before the separator may be trimmed so
  the parser remains deterministic if curl emits CRLF on a supported host.
- Preserve an empty second field as `redirect == ""`. Trim only framing
  whitespace; never infer a redirect from the status field.
- Replace the current reverse-line extraction in
  `SystemArtifactFetcher::fetch` with `parse_curl_response_frame`, then call the
  unchanged `redirect_target(frame.status, frame.redirect)`.
- Keep the direct curl argument vector, `--max-redirs 0`, HTTPS-only protocol
  restrictions, six-hop bound, output limits, and `validate_release_url` call
  before every request. A `3xx` with an empty or unapproved redirect continues
  to fail closed; a `2xx` succeeds; every other status remains a download
  failure.
- Do not add a compatibility fallback for malformed curl output and do not
  expose raw response content in errors.

Acceptance criteria:

- [ ] Exact production framing `b"200\n"` parses as status `200` plus an empty
  redirect and completes the fetch successfully.
- [ ] Exact redirected framing such as
  `b"302\nhttps://objects.githubusercontent.com/asset"` advances to the next
  attested hop.
- [ ] Missing separators, malformed statuses, extra fields, empty `3xx`
  redirects, and redirects to unapproved hosts fail closed.
- [ ] Existing bounds, checksum, executable identity, destination race, and
  atomic-publication tests remain unchanged and green.
- [ ] The isolated bootstrap matrix installs into an empty destination and an
  immediate identical rerun returns `no-op`, with no warning, pending work, or
  attention-required result.
- [ ] A clean-room invocation using disposable `HOME`, `XDG_CONFIG_HOME`, and
  `XDG_CACHE_HOME` resolves the current canonical GitHub release, completes
  bootstrap, and repeats without binary change.

## Implementation order

1. Add the private frame value/parser and table-driven malformed-frame tests in
   `crates/core/src/runtime/artifact.rs`.
2. Route `SystemArtifactFetcher::fetch` through the parser while retaining the
   existing redirect policy and direct bounded process request.
3. Run the focused core runtime/bootstrap tests, strengthen the existing CLI
   install/no-op assertions only if needed, then run the clean-room canonical
   release check and full workspace verification.

## Testing

### Adapter unit tests: `crates/core/src/runtime/artifact.rs`

- Parse the exact non-redirect stdout bytes (`200` followed by the required
  separator and an empty second field) and assert `redirect_target` returns
  `None`.
- Parse an allowed `302` frame and assert the next URL is returned; preserve the
  existing hostile-host and empty-redirect rejection assertions.
- Use a compact malformed-frame table covering no separator, empty status,
  nonnumeric status, and an additional line. Every case must produce
  `DownloadFailed` without including raw bytes in a rendered error.
- Keep the write-out string assertion close to the parser test (or assert the
  constructed `NativeProcessRequest` when a local fake runner is cheaper during
  implementation) so the tested framing and production argument cannot drift.

### Resolver integration tests: `crates/core/tests/bootstrap_integration.rs`

- Retain the `ArtifactFetcher` fixture seam to prove resolver payload bounds,
  private workspace cleanup, manifest parsing, and symlink rejection. These
  tests must not contact the network or mutate a host cache.

### Bootstrap policy tests: `crates/cli/src/bootstrap_commands.rs`

- Reuse `isolated_matrix_covers_install_noop_update_major_block_and_opt_in` as
  the deterministic install/rerun seam. For the first result assert `changed`,
  no attention, and no pending work; for the identical rerun assert no change,
  no attention, no pending work, and no warnings/next actions.

### Verification commands

```text
cargo test -p skilltap-core runtime::artifact
cargo test -p skilltap-core --test bootstrap_integration
cargo test -p skilltap-cli bootstrap_commands
cargo test --workspace --all-targets
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

After deterministic tests pass, build the binary once and run `bootstrap
--json` twice with a new disposable home/config/cache root and a disposable
`PATH` destination. Capture that the first canonical-release run succeeds and
the second reports no change; remove the disposable root afterward. This is a
release-contract verification step, not an ordinary network-dependent Cargo
test.

## Risks

- **Riskiest assumption:** Supported curl versions honor the documented
  explicit write-out order and emit the requested separator even when
  `redirect_url` is empty. The exact `b"200\n"` regression is the observed real
  output; forward `split_once` makes that contract explicit.
- **Production failure condition:** A future curl changes or contaminates
  write-out stdout. Strict frame validation fails closed as a download error
  and preserves any installed binary rather than guessing.
- **Fallback:** If portability evidence later disproves newline framing, change
  only the private write-out/parser pair to an attested delimiter supported by
  all minimum curl versions; the `ArtifactFetcher` port and redirect policy do
  not change.
- **Residual uncertainty:** The live canonical-release check depends on network
  and GitHub availability, so deterministic regression coverage remains the
  merge gate while the clean-room run supplies release evidence.

## Implementation discovery

The clean-room canonical-release run exposed two release-boundary facts that
were not visible in the original `200\n` failure:

- Current GitHub release downloads redirect to the exact host
  `release-assets.githubusercontent.com`, not only
  `objects.githubusercontent.com`. The production allowlist now attests that
  exact HTTPS host while rejecting lookalike neighbors.
- HTTP download metadata does not carry a trustworthy executable mode; curl
  creates the private payload as `0600`. Bootstrap now requires a bounded
  regular file, verifies its signed checksum, and only then changes the private
  payload to `0700` for version probing and atomic publication. A checksum
  mismatch remains non-executable and is never run.

The live binary phase installed release `3.0.1` into a disposable destination
and the immediate rerun returned `no-op`, with no binary warnings or pending
work. The overall command still reported attention because the separate Codex
first-party plugin setup lane classified the current native lifecycle as
unsupported; that outcome belongs to the runtime/native lifecycle features as
declared in this feature's brief.

## Implementation notes

- Execution capability: direct inline implementation; the parser, verified
  payload preparation, and bootstrap assertions form one bounded release
  transport surface.
- Review weight: standard (project default).
- Files changed: `crates/core/src/runtime/artifact.rs`,
  `crates/cli/src/bootstrap_commands.rs`, and this feature item.
- Tests added: exact success and redirect frame parsing, malformed frame and
  hostile-host rejection, install/no-op result semantics, verified promotion
  of a non-executable HTTP payload, and checksum-before-execution behavior.
- Discrepancies from design: the live canonical release required the exact
  `release-assets.githubusercontent.com` redirect host and checksum-then-mode
  preparation described above; the public ports and atomic publication
  boundary remain unchanged.
- Adjacent issues parked: none; the Codex plugin setup attention is already
  owned by sibling epic features.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Substrate review at the project-default `standard` weight using a fresh-context deep lane for the feature's network, checksum, and executable-publication boundaries. Commit `b0cd79c` parses the exact curl frame without losing an empty redirect field, preserves strict per-hop host validation, recognizes GitHub's exact release-asset host, and verifies checksum before making a private payload executable. The deterministic bootstrap matrix and full workspace tests are green; the implementation record also captures a successful disposable live install/no-op repeat. Formatting and all-target/all-feature clippy are green.
