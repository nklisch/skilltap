---
id: epic-expanded-harness-support-configuration-constrained-contract-lock
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-configuration-constrained
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/kimi-mcp.md
  - .research/attestation/mistral-mcp.md
  - .research/attestation/kilo-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Lock Kimi, Vibe, and Kilo Native Contracts

## Checkpoint

Close the bounded evidence gap before any new adapter gains mutation authority.
Capture exact version command/output, global/project MCP documents and
precedence, and deterministic non-interactive effective-state probes for one
installed Kimi Code, Mistral Vibe, and Kilo Code release. Store only bounded,
non-secret fixtures and profile constants under the harnesses crate.

This is a contract-validation checkpoint, not a broad harness survey. Use the
existing source-direct URLs and isolated installations. Do not guess version
literals, parse unversioned human text, drive an interactive UI as an API, or
write production adapter registration in this story.

## Design element

Implement the feature's `VerifiedManagedTargetContract` and exact fixture set in:

- `crates/harnesses/src/adapters/configuration_constrained/contracts.rs`
- `crates/harnesses/tests/fixtures/configuration_constrained/{kimi,vibe,kilo}/`

Pin Kilo's precedence when both project `kilo.jsonc` and
`.kilo/kilo.jsonc` exist. Pin Vibe's exact named `[[mcp_servers]]` wire forms
and trust response. Pin Kimi's fresh-session probe and user/project override.

## Acceptance evidence

- Exact version bytes decode once and select only the matching compiled profile;
  malformed/extra/control/unknown output cannot authorize mutation.
- Exact scoped config fixtures cover supported transport/auth fields, unknown
  fields/comments, and precedence without credentials.
- Probe fixtures distinguish loaded, reload-required, trust-required,
  authentication-required, and failed state through bounded decoding.
- If any target has no deterministic non-interactive probe or reproducible
  write/reload boundary, record the blocker in the parent feature and leave
  that target unverified. Dependent work must not manufacture a workaround.

## Ordering

Foundation checkpoint. `projection-scope` is blocked until this evidence is
complete because its profile/probe interfaces must be proven by real contracts.

## Evidence lock run — 2026-07-15

### Method and safety boundary

- Current official documentation, release repositories, registry metadata, and
  exact released distributions were read directly. This delegated harness
  context did **not** expose the requested Z.ai research tools, so this run does
  not claim a Z.ai retrieval pass; that process requirement remains open below.
- Executables ran only under `/tmp/skilltap-contract-probes` with isolated
  `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, and `XDG_CACHE_HOME`. No operator
  harness configuration or credentials were read or changed. The temporary
  roots were removed after capture.
- No interactive UI was driven. Only version/help commands and documented
  non-TTY commands were invoked. No source or adapter files were written.
- Registry/release identity was checked before execution. Artifact digests and
  byte outputs below are the bounded fixtures from this run; dynamic paths are
  shown as `<probe>` where appropriate.

### Kimi Code CLI

**Disposition: blocked for project scope; global evidence is independently
usable.**

Official evidence:

- Current MCP documentation:
  <https://www.kimi.com/code/docs/en/kimi-code-cli/customization/mcp.html>
- Release-pinned MCP document:
  <https://github.com/MoonshotAI/kimi-cli/blob/1.48.0/docs/en/customization/mcp.md>
- Release-pinned MCP command reference:
  <https://github.com/MoonshotAI/kimi-cli/blob/1.48.0/docs/en/reference/kimi-mcp.md>
- Release-pinned CLI/version implementation:
  <https://github.com/MoonshotAI/kimi-cli/blob/1.48.0/src/kimi_cli/cli/__init__.py>
- Release-pinned MCP implementation:
  <https://github.com/MoonshotAI/kimi-cli/blob/1.48.0/src/kimi_cli/cli/mcp.py>
- Release: <https://github.com/MoonshotAI/kimi-cli/releases/tag/1.48.0>
  at commit `2c34efbbc6c7cfe40770623281e87c138ff8eb6c`.
- Current PyPI metadata: <https://pypi.org/pypi/kimi-cli/json>.

Version contract:

- Current release and PyPI identity: `1.48.0`.
- Official Linux x86_64 artifact:
  <https://github.com/MoonshotAI/kimi-cli/releases/download/1.48.0/kimi-1.48.0-x86_64-unknown-linux-gnu.tar.gz>
- Observed SHA-256:
  `202fefb5ac2b1b43993bd4889467622a69676124cd09dc1813daa1d4552fc32e`,
  equal to the GitHub release digest.
- `argv = ["--version"]` and `argv = ["-V"]` both exited `0` with exact
  `stdout = b"kimi, version 1.48.0\n"`, `stderr = b""`.

Configuration and precedence contract:

- The `1.48.0` release and current docs name only global
  `~/.kimi/mcp.json` (relocatable through `KIMI_SHARE_DIR`). The exact
  documented shape is:

  ```json
  {
    "mcpServers": {
      "context7": {
        "url": "https://mcp.context7.com/mcp",
        "headers": { "CONTEXT7_API_KEY": "your-key" }
      },
      "chrome-devtools": {
        "command": "npx",
        "args": ["chrome-devtools-mcp@latest"],
        "env": { "SOME_VAR": "value" }
      }
    }
  }
  ```

- `kimi --mcp-config-file /path/to/mcp.json` and
  `kimi --mcp-config '<json>'` are explicit invocation overrides and may be
  repeated. They are not ambient project configuration and do not establish a
  project path or user/project precedence.
- The release implementation's `get_global_mcp_config_file()` and all
  `kimi mcp` lifecycle commands address only that global file. The older attestation
  naming `$KIMI_CODE_HOME/mcp.json` and `.kimi-code/mcp.json` is not valid for
  `1.48.0` and must not authorize a profile.
- No current official source establishes an ambient project MCP file, a
  project-over-user collision rule, or project trust behavior. Kimi therefore
  fails the required two-scope contract for this release.

Non-TTY probes and outcomes:

- Declared global state grammar: `argv = ["mcp", "list"]`. The isolated
  fixture exited `0`, wrote only to stdout, and emitted:

  ```text
  MCP config file: <probe>/kimi-share/mcp.json
    loaded (stdio): /usr/bin/python <probe>/mock_mcp.py
    broken (stdio): /definitely/not/a/program
    oauth (http): https://example.invalid/mcp [authorization required - run: kimi mcp auth oauth]
  ```

- Connection grammar: `argv = ["mcp", "test", NAME]`. A bounded local MCP
  server produced exit `0` and:

  ```text
  Testing connection to 'loaded'...
  ✓ Connected to 'loaded'
    Available tools: 1
    Tools:
      - ping: deterministic probe tool
  ```

  An absent executable produced exit `1`, the same initial stdout line, and
  stderr beginning `✗ Connection failed: RuntimeError: Client failed to
  connect:`. Error detail is dynamic and must not be accepted as a stable
  grammar.
- OAuth-required state is exposed by the literal bracketed suffix from
  `mcp list`; `kimi mcp auth NAME` opens a browser and
  `kimi mcp reset-auth NAME` clears tokens. Merely listing an OAuth entry
  created metadata under the
  isolated `~/.kimi/mcp-oauth/` store, so that auth check is not a strictly
  side-effect-free operator-state probe.
- Default MCP configuration is read once before the CLI instance is created.
  No current documented in-session reload command exists. A fresh process is
  the only reproducible load boundary found; it still cannot prove missing
  project scope.

### Mistral Vibe

**Disposition: blocked for effective-state observation and contradictory auth
contract; scoped file evidence is independently usable.**

Official evidence:

- Current MCP documentation:
  <https://docs.mistral.ai/vibe/code/cli/mcp-servers>
- Current configuration documentation:
  <https://docs.mistral.ai/vibe/code/cli/configuration>
- Release: <https://github.com/mistralai/mistral-vibe/releases/tag/v2.19.1>
  at commit `30792a4cac2c2e5173c6b5a98739fbbf36324545`.
- Release-pinned CLI parser:
  <https://github.com/mistralai/mistral-vibe/blob/v2.19.1/vibe/cli/entrypoint.py>
- Release-pinned layer selection:
  <https://github.com/mistralai/mistral-vibe/blob/v2.19.1/vibe/core/config/default_orchestrator.py>
- Release-pinned TOML persistence:
  <https://github.com/mistralai/mistral-vibe/blob/v2.19.1/vibe/core/config/layers/_base.py>
- Release-pinned MCP models:
  <https://github.com/mistralai/mistral-vibe/blob/v2.19.1/vibe/core/config/models.py>
- Current PyPI metadata: <https://pypi.org/pypi/mistral-vibe/json>.

Version contract:

- Current release and PyPI identity: `2.19.1`.
- Official wheel:
  <https://files.pythonhosted.org/packages/94/6d/c75c141bda47fad5dd3ab71ed727d64803d8670fae26ee4e66e440ab137d/mistral_vibe-2.19.1-py3-none-any.whl>
- Observed SHA-256:
  `4e2632b800ded75e1c058ae144b203c18b5732c786e7ea656454a7d1e2e67298`,
  equal to PyPI metadata.
- `argv = ["--version"]` and `argv = ["-v"]` both exited `0` with exact
  `stdout = b"vibe 2.19.1\n"`, `stderr = b""`.

Configuration and precedence contract:

- User file: `~/.vibe/config.toml` (relocatable through `VIBE_HOME`).
- Project file: `./.vibe/config.toml`, loaded only for a trusted working
  directory. The `2.19.1` implementation selects the trusted project TOML
  *instead of* the user TOML; it does not merge both TOML layers. Higher layers
  are command-line/runtime overrides, environment, the selected TOML, then
  defaults/discovered configuration.
- Exact named wire forms accepted by the release include:

  ```toml
  [[mcp_servers]]
  name = "my_http_server"
  transport = "http"
  url = "http://localhost:8000"
  headers = { "Authorization" = "Bearer my_token" }
  api_key_env = "MY_API_KEY_ENV_VAR"
  api_key_header = "Authorization"
  api_key_format = "Bearer {token}"

  [[mcp_servers]]
  name = "my_streamable_server"
  transport = "streamable-http"
  url = "http://localhost:8001"
  headers = { "X-API-Key" = "my_api_key" }

  [[mcp_servers]]
  name = "fetch_server"
  transport = "stdio"
  command = "uvx"
  args = ["mcp-server-fetch"]
  env = { "DEBUG" = "1", "LOG_LEVEL" = "info" }
  ```

- Release source also accepts OAuth as:

  ```toml
  [[mcp_servers]]
  name = "linear"
  transport = "streamable-http"
  url = "https://mcp.linear.app/mcp"

  [mcp_servers.auth]
  type = "oauth"
  scopes = []
  ```

  However, the current official web MCP page explicitly says OAuth MCP is not
  yet supported and directs users to static credentials. The release source,
  release README, and release tests implement OAuth and `/mcp add|login`.
  This unresolved official-source contradiction prevents an OAuth mutation
  profile; static-auth entries remain the only non-contradicted subset.

Trust, probe, and reload outcomes:

- Interactive startup offers trust choices. Non-interactive programmatic mode
  does not show that dialog: without prior trust it ignores project
  configuration and emits `Warning: <cwd> is not trusted; project
  configuration (...) will be ignored. Re-run with --trust to trust this
  folder temporarily.`
- `argv = ["--trust", "--prompt", TEXT, ...]` grants only session trust. It
  then requires a configured model/provider and is therefore not a
  deterministic MCP observation probe.
- Exact `2.19.1 --help` exposes no MCP subcommand. Official docs expose only
  `/mcp` and `/mcp NAME` **inside a CLI session**. Programmatic mode
  (`--prompt`, with text/JSON/streaming output) executes an LLM turn and does
  not report the effective MCP registry independently of model credentials or
  model behavior.
- Release internals can refresh configuration and reconcile registry states,
  including `NEEDS_AUTH`, but no documented non-TTY command invokes that
  boundary or emits it. A fresh process reloads the selected TOML; it still
  cannot provide the required deterministic effective-state evidence.

Lossless TOML boundary:

- File layers parse through `tomllib` into a permissive raw model, so unknown
  TOML values are retained in the raw layer. Native patch persistence then
  writes the complete semantic document through `tomli_w.dump(...)` to an
  atomic replacement.
- Consequently, Vibe's native write path preserves representable values but
  loses comments, original ordering, whitespace, and lexical choices. Unknown
  top-level values are retained in the raw file model but ignored when absent
  from the effective schema.
- A lossless skilltap boundary would have to fingerprint and syntax-patch only
  the selected file's named `[[mcp_servers]]` entry while preserving every
  other token; it cannot delegate writes to Vibe's serializer. Because no
  deterministic non-TTY post-write MCP probe exists, that direct-write
  boundary does not yet gain mutation authority.

### Kilo Code CLI

**Disposition: blocked for side-effect-free observation/reload; version,
precedence, JSONC, and isolated status evidence are independently usable.**

Official evidence:

- Current MCP documentation:
  <https://kilo.ai/docs/automate/mcp/using-in-cli>
- Current npm metadata: <https://registry.npmjs.org/@kilocode%2fcli>
- npm provenance attestation:
  <https://registry.npmjs.org/-/npm/v1/attestations/@kilocode%2fcli@7.4.7>
- Release tag: <https://github.com/Kilo-Org/kilocode/releases/tag/v7.4.7>
  at `afda9794582d164694d01a7c2179492295a08c3b`.
- npm provenance resolves the published package to
  `c99db83f8974c0f75ca3cda142bd621850d32092`. The contract-sensitive source
  files below are byte-identical between that provenance commit and tag.
- Effective-config command:
  <https://github.com/Kilo-Org/kilocode/blob/v7.4.7/packages/opencode/src/cli/cmd/debug/config.ts>
- MCP commands/status grammar:
  <https://github.com/Kilo-Org/kilocode/blob/v7.4.7/packages/opencode/src/cli/cmd/mcp.ts>
- Layer ordering:
  <https://github.com/Kilo-Org/kilocode/blob/v7.4.7/packages/opencode/src/config/config.ts>
- Kilo config selection/update rules:
  <https://github.com/Kilo-Org/kilocode/blob/v7.4.7/packages/opencode/src/kilocode/config/config.ts>
- JSONC format boundary and tests:
  <https://github.com/Kilo-Org/kilocode/blob/v7.4.7/packages/opencode/src/kilocode/cli/cmd/mcp.ts>
  and
  <https://github.com/Kilo-Org/kilocode/blob/v7.4.7/packages/opencode/test/kilocode/cli/cmd/mcp.test.ts>.

Version contract:

- Current npm identity: `@kilocode/cli@7.4.7`; binary aliases are `kilo` and
  `kilocode`.
- Official Linux x64 package:
  <https://registry.npmjs.org/@kilocode/cli-linux-x64/-/cli-linux-x64-7.4.7.tgz>
- Observed SHA-1 `699aaf1b5e205bde8ed7bcd356e4fdb3f073b504` and SHA-512
  `tRUbzp+oNniBj/cB5bTHXiODJLLIc7fIJqL9mC3SOrqYjgcKVTOjTgPFXLkC8Hqrd/acCyAcT+phyklW49i2/A==`
  equal npm metadata.
- `argv = ["--version"]` and `argv = ["-v"]` both exited `0` with exact
  `stdout = b"7.4.7\n"`, `stderr = b""`.

Configuration and precedence contract:

- Recommended global file: `~/.config/kilo/kilo.json`; global
  `kilo.jsonc` and `config.json` are also documented. The XDG config root is
  honored.
- Recommended project files: `./kilo.json` or `./.kilo/kilo.json`; matching
  `kilo.jsonc` is supported. The exact MCP forms are:

  ```json
  {
    "mcp": {
      "local": {
        "type": "local",
        "command": ["npx", "-y", "my-mcp-command"],
        "environment": { "API_KEY": "value" },
        "enabled": true
      },
      "remote": {
        "type": "remote",
        "url": "https://example.com/mcp",
        "headers": { "Authorization": "Bearer {env:MY_API_KEY}" },
        "enabled": true
      }
    }
  }
  ```

- Project values deep-merge over global values. In one directory,
  `kilo.jsonc` overrides `kilo.json`. For the dual project locations, root
  config loads first and `.kilo` config loads later; therefore
  `./.kilo/kilo.jsonc` overrides `./kilo.jsonc` on the same leaf fields while
  retaining unrelated nested fields.
- Isolated `kilo --log-level ERROR --pure debug config` with all three scopes
  proved that rule: the same-named `shared` MCP used the nested URL and
  `enabled = true`, while headers from global, root, and nested files all
  survived the deep merge. The effective `username` was the nested value and
  global-only/root-only MCP entries remained present. All three source config
  hashes were unchanged by this invocation.

Non-TTY probes and outcomes:

- Effective declared state grammar is `argv = ["--log-level", "ERROR",
  "--pure", "debug", "config"]`: exit `0`, one pretty-printed JSON document
  plus LF on stdout, empty stderr after isolated first-run state exists.
- Connection grammar is `argv = ["--log-level", "ERROR", "--pure", "mcp",
  "list"]`. It exited `0` and distinguished literal states `connected`,
  `disabled`, and `failed`; the local bounded server was `connected`, an absent
  executable was `failed`, and `enabled = false` entries were `disabled`.
  Output is human-oriented Unicode with ANSI dim sequences even under
  `NO_COLOR=1`; error details are dynamic. No JSON mode is exposed.
- Auth grammar is `argv = ["--log-level", "ERROR", "--pure", "mcp", "auth",
  "list"]`. It exited `0` and reported `not authenticated` for remote servers.
  Release source also defines `authenticated` and `expired`; remote MCP is
  OAuth-capable unless `oauth: false`.
- Kilo does not require a workspace trust prompt. Instead, project configuration
  is treated as untrusted input: environment substitution is rejected and
  `{file:...}` reads are confined to the project root. That is a parsing policy,
  not a `trust-required` runtime state.

Reload and observation safety:

- A fresh process deterministically re-reads global and project configuration.
  The release also checks for external global-config changes on subsequent
  config reads. No current public contract proves equivalent in-process reload
  of externally edited project MCP state.
- More importantly, the documented probe commands are not read-only on a clean
  environment. In the isolated run, help/debug/list created Kilo databases,
  logs, telemetry identity, lock metadata, npm cache, and `.gitignore` in both
  the global config directory and the project's `.kilo/` directory. The project
  `.kilo/.gitignore` creation is an operator-visible project mutation that a
  read-only skilltap observation did not plan.
- Therefore `debug config` and `mcp list` cannot be invoked against operator
  state as skilltap's status probe without a separately proven isolation
  mechanism that preserves actual project/global resolution. Direct file
  parsing can observe declarations but does not prove runtime connection or
  reload state.

Lossless JSONC boundary:

- The MCP mutator uses `jsonc-parser.modify/applyEdits` for the exact
  `mcp.<name>` path. For `.jsonc`, the patched token stream is retained; for
  strict `.json`, the whole semantic document is reserialized with
  `JSON.stringify(..., null, 2)`.
- Valid JSONC comments and unrelated known fields therefore have a supported
  targeted-preservation path. A read-only `debug config` run left comments and
  all config hashes unchanged.
- Arbitrary unknown keys are not tolerated by `7.4.7`. An isolated global
  `unknownGlobal` key made `debug config` exit `1` with
  `Configuration is invalid at <path> ↳ Unrecognized key: unknownGlobal` and
  left the file hash unchanged. Mutation must fail closed on such a document;
  preserving its bytes is not evidence that Kilo will load it.
- The lossless authority is consequently limited to targeted edits of valid,
  version-known JSONC documents. Strict JSON has semantic preservation only,
  not comment/format preservation. This boundary is proven, but the unsafe
  effective-state probe still blocks target authorization.

## Blocker

This checkpoint remains at `stage: implementing`; no target profile may gain
mutation authority from this run.

1. **Required research channel unavailable.** The delegated harness exposed no
   Z.ai search/fetch/repository tools. The evidence above is source-direct and
   current, but the operator-required Z.ai pass is unfulfilled.
2. **Kimi Code CLI 1.48.0:** current official docs and release source provide
   only global `~/.kimi/mcp.json`. There is no ambient project MCP file,
   user/project precedence, project trust outcome, or project reload contract.
3. **Mistral Vibe 2.19.1:** no deterministic non-TTY command reports effective
   MCP servers/connections; `/mcp` is TUI-only and programmatic mode requires an
   LLM turn. Current web docs reject OAuth while the exact release implements
   it. Native TOML writes are lexically lossy, and no non-TTY post-write probe
   closes that boundary.
4. **Kilo Code CLI 7.4.7:** `debug config`, `mcp list`, and auth listing expose
   the needed effective/status states, but invoking the CLI creates native and
   project files, including `.kilo/.gitignore`; `mcp list` is also unstructured
   human output. No side-effect-free isolation grammar has been proven against
   the real global/project resolution, and externally edited project MCP reload
   remains undocumented.

The older attestations are insufficient to clear these release-specific gaps.
Dependent work must remain blocked rather than manufacture project paths,
parse interactive UIs, ignore documentation contradictions, or treat
side-effectful probes as read-only observation.
