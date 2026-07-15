---
id: epic-expanded-harness-support-trust-interactive-contract-lock
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-trust-interactive
depends_on: [epic-expanded-harness-support-file-managed-contracts]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/junie-skills.md
  - .research/attestation/junie-mcp.md
  - .research/attestation/junie-extensions.md
  - .research/attestation/amp-manual.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Lock Junie and Amp Native Contracts

## Checkpoint

Implement Unit 1 from the parent feature. Capture exact, bounded native evidence
for one installed Junie release and one installed Amp release before either
adapter receives a mutation-authorized profile or enters the canonical registry.

This story consumes the scope-generic managed/probe/default-binary contract from
`epic-expanded-harness-support-file-managed-contracts`. It must amend that
shared owner if a missing cross-target capability is discovered; it must not
fork a trust-interactive-only runtime or lifecycle port.

## Units

- Add `crates/harnesses/src/adapters/trust_interactive/contracts.rs` with the
  exact `VerifiedTrustInteractiveContract`, `VerifiedMcpContract`, and
  `EffectiveProbeContract` types from the parent design.
- Add isolated bounded version, config, precedence, and effective-state fixtures
  under `crates/harnesses/tests/fixtures/trust_interactive/{junie,amp}/`.
- Pin Junie's exact binary/version contract, scoped MCP schema, and whether any
  deterministic non-TTY effective-state surface exists beyond `/mcp`.
- Pin Amp's exact binary/version contract, selected user settings path, nearest
  project settings precedence, `amp.mcpServers` shape, `mcp doctor` output, trust
  states, and skill-local `mcp.json` behavior.

## Contract constraints

- Do not invent version literals, argv, output grammar, user paths, or trust
  behavior from remembered product behavior.
- A runtime probe is read-only, direct-argv, bounded, explicit-cwd, and
  exact-version decoded. Raw stdout/stderr, config bytes, URLs, secrets, and
  parser text cannot enter findings.
- `InteractiveOnly` is a blocker for MCP mutation/effective support. It is not
  authority to run a pseudo-TTY, parse a cache, or call configured files
  effective.
- Fixtures use inert credential references and test-support-owned HOME/XDG/
  project roots only.

## Acceptance evidence

- Known exact bytes decode to one verified profile; malformed, control-character,
  extra-document, adjacent, and unknown versions cannot grant mutation.
- Both scopes' path/schema/precedence contracts and unknown-field preservation
  are fixture-pinned.
- Effective fixtures are explicitly unavailable for both targets; their
  declaration-managed profiles preserve the unverified boundary.
- Amp skill-local MCP relative-path/lazy behavior is validated separately from
  scoped settings, while doctor/OAuth/login flows remain untouched.
- The relaxed amendment permits registration only with `Unverified`
  component/mutation support and no native/effective lifecycle port.

## Ordering

Depends on the shared file-managed contract checkpoint. Junie and Amp adapter
stories consume this exact evidence and may proceed independently only after it
is complete.

## Evidence-lock checkpoint — 2026-07-14

### Result

**Complete under the relaxed declaration-managed amendment.** Exact Junie and
Amp identities, documented declaration paths, scope precedence, MCP containers,
skill-local boundaries, and interactive/effective-state limitations are locked.
Both targets are now registered only with declaration-managed surfaces and
`Unverified` component/mutation capabilities. Neither target has a native
lifecycle port or effective-state probe. Unknown versions remain observe-only
and perform no native projection writes.

Junie's `/mcp` and extension/cache surfaces, and Amp's doctor, workspace-trust,
OAuth, and login surfaces, remain intentionally unavailable to skilltap. The
finite Amp `mcp list --json` declaration decoder retains only bounded name/source
metadata and is not an effective-health path. No repository source/fixture or
operator settings, cache, trust, approval, OAuth, or auth state was mutated;
all generated configuration and process state stayed in disposable validation
roots.

### Provenance and isolation

The source baseline is the Z.ai source-direct attestation set fetched on
2026-07-12 and already named in `research_refs`. Every moving claim below was
then refreshed from the current official primary URL on 2026-07-14. The
installed artifacts were obtained only from the vendors' current official
install channels and exercised under disposable `/var/tmp/skilltap-*` HOME,
XDG, cache, and project roots with stdin disconnected. Inert values were
`AMP_API_KEY=inert-invalid-key`, `INERT_TOKEN=not-a-secret`, and
`BROWSER=/bin/false`. No approval command was run.

Current official primary sources:

- Junie quickstart/install and headless mode:
  <https://junie.jetbrains.com/docs/junie-cli.html>
- Junie complete CLI reference:
  <https://junie.jetbrains.com/docs/parameters.html>
- Junie skills:
  <https://junie.jetbrains.com/docs/agent-skills.html>
- Junie MCP:
  <https://junie.jetbrains.com/docs/junie-cli-mcp-configuration.html>
- Junie configuration/settings precedence:
  <https://junie.jetbrains.com/docs/junie-cli-configuration.html>
- Junie release channel:
  <https://raw.githubusercontent.com/jetbrains-junie/junie/main/update-info.jsonl>
  at repository HEAD `53df876468373bd6af7ba4d60b7ab3cada1e3b70`
- Junie pinned release:
  <https://github.com/JetBrains/junie/releases/tag/2144.10>
- Amp manual:
  <https://ampcode.com/manual>
- Amp installer:
  <https://ampcode.com/install.sh>
- Amp current-version endpoint:
  <https://static.ampcode.com/cli/cli-version.txt>
- Amp pinned checksum:
  <https://static.ampcode.com/cli/0.0.1784073393-g9a3a12/linux-x64-amp.sha256>
- Amp's official npm distribution cross-check:
  <https://registry.npmjs.org/@ampcode/cli/-/cli-0.0.1784073393-g9a3a12.tgz>

The Junie pages were built/last-modified on 2026-07-14. The final Amp manual
refresh returned HTTP 200 on 2026-07-15 00:04:42 UTC. Amp published a newer CLI
while this checkpoint was running; all final Amp evidence below was repeated
against the later release, so the superseded earlier capture grants no profile.

## Junie contract evidence

### Exact release and profile identity

Official installation is `curl -fsSL https://junie.jetbrains.com/install.sh |
bash`. The current release installer selects the last `linux-amd64` release
entry in `update-info.jsonl`; the pinned entry is:

```json
{"downloadUrl":"https://github.com/JetBrains/junie/releases/download/2144.10/junie-release-2144.10-linux-amd64.zip","marketing":"26.6.29","platform":"linux-amd64","sha256":"c5bbf8adc4c8c0aae0ea1ffda72654dc2f0c590ae276ddc0f336983cb5947eff","size":225566350,"version":"2144.10"}
```

GitHub reports release `2144.10` published at `2026-07-14T14:39:35Z` and not a
prerelease. The installer verified the archive checksum before extraction.

Exact isolated detection:

```text
command: env HOME=/var/tmp/skilltap-junie-home \
  JUNIE_DATA=/var/tmp/skilltap-junie-home/.local/share/junie \
  JUNIE_LOG_DIR=/var/tmp/skilltap-junie-home/.junie/logs \
  /var/tmp/skilltap-junie-home/.local/bin/junie --version
cwd: /var/tmp/skilltap-junie-project
stdin: /dev/null
exit: 0
stdout bytes: Junie version: 26.6.29 (2144.10)\n
stderr bytes: <empty>
```

The exact release identity is the pair `marketing=26.6.29`, `build=2144.10`;
the proposed compiled profile id is
`junie-26-6-29-build-2144-10`. The default binary is `junie`, and version argv
is `['--version']`. The npm package `@jetbrains/junie-cli@1468.30.0` was rejected
as evidence: it is a launcher that refused to run without the product-managed
shim and is not the current release artifact selected by JetBrains' documented
installer.

This closes detection evidence only. Adjacent builds, a changed marketing/build
pair, malformed UTF-8, control characters, missing newline/fields, and any
extra line/document must remain observe-only.

### Skills

Official load roots and precedence are exact:

1. project `<project>/.junie/skills/<skill-name>/`;
2. user `~/.junie/skills/<skill-name>/`.

A same-name project skill wins and the user skill is ignored. `SKILL.md` is
required; scripts, templates, checklists, and other sibling subdirectories are
part of the skill. Third-party skills are executable trust input, but Junie
specifies caution/review rather than a separate project-skill trust-state file.

### MCP files, schema, and user settings

- User MCP: `~/.junie/mcp/mcp.json`.
- Project MCP: `<project>/.junie/mcp/mcp.json`.
- Root key: `mcpServers`.
- Local server fields shown by the official schema: `command`, optional `args`,
  and optional `env`.
- Remote server fields shown by the official schema: `url` and optional
  `headers`.
- Manually added servers are imported and enabled by default.
- Discovery inputs are `--mcp-default-locations <true|false>` and repeatable
  `--mcp-location <path>`.
- User settings are `~/.junie/settings.json`. For ordinary Junie configuration,
  documented precedence is command-line flags, user settings, project
  `<project>/.junie/config.json`, then user `~/.junie/config.json`.

Unknown top-level MCP fields and unknown fields inside unowned server entries
must be preserved. The official MCP source does **not** state what happens when
user and project `mcpServers` contain the same name; ordinary `config.json`
precedence is not authority for the separate MCP loader.

### Effective state, trust, and authentication

The official MCP state vocabulary is exact:

```text
Starting / Active / Inactive / Disabled / Failed / Authorization required
```

`Inactive` means enabled and correctly configured but not currently running.
`Failed` includes connection/start/crash, invalid configuration, authentication
failure, missing dependencies, or runtime crash. Remote OAuth servers appear as
`Authorization required`; authorization, enable, and disable are actions inside
interactive `/mcp`. Invoking MCP tools remains subject to Junie's command
approval/Action Allowlist/Brave behavior, but the sources do not define an Amp-
style workspace-declaration trust state.

The installed release's complete `junie --help` MCP section is:

```text
MCP:
  --mcp-default-locations=<text>  Enable or disable adding MCP servers from default locations (per user / per project).
  --mcp-location=<text>           Additional folders where MCP servers should be found. Can be specified multiple times.
```

The current official complete CLI reference contains the same two MCP inputs and
no list/status/doctor/export command. Junie's documented non-interactive mode is
agent task execution, not MCP observation. The MCP page repeatedly directs the
user to the interactive `/mcp` screen to verify active state. Therefore:

> **Junie has no currently supported deterministic non-TTY MCP observer beyond
> interactive `/mcp`.**

A pseudo-TTY, headless agent prompt, session/cache parser, extension cache, or
configured-file inference is not an acceptable substitute.

### Junie blockers

1. `JUNIE-EFFECTIVE-OBSERVER`: no supported finite non-TTY effective MCP state
   command/API exists in the current docs or installed `2144.10` help surface.
2. `JUNIE-MCP-PRECEDENCE`: same-name project/user MCP resolution is neither
   documented nor observable without the prohibited interactive surface.

The Junie adapter story remains dependency-blocked and Junie remains outside the
canonical registry.

## Amp contract evidence

### Exact release and profile identity

Official installation is `curl -fsSL https://ampcode.com/install.sh | bash`.
The final current-version response was exactly
`0.0.1784073393-g9a3a12\n`. The installer selected `linux-x64`, verified the
published checksum, and installed an ELF x86-64 binary with SHA-256:

```text
628bd7520993eeeb9037f928d2a7ab6943bb2742a2bf02e4c08353a2c8f034d6
```

The official npm cross-check returned the same version with integrity:

```text
sha512-Mr9Xvh8JFPBHAQy+y6Yr/sEs2P4ldOQjKOEBx89JTk0EvAjL2pH6ue6EbvN48hSW1OzhNzsvVIxmWyPJy8f4cw==
```

Exact isolated detection capture:

```text
command: env HOME=/var/tmp/skilltap-amp-user \
  XDG_CONFIG_HOME=/var/tmp/skilltap-amp-user/.config \
  AMP_SKIP_UPDATE_CHECK=1 NO_COLOR=1 TERM=dumb \
  /var/tmp/skilltap-amp-home/bin/amp --version
cwd: /var/tmp/skilltap-amp-project
stdin: /dev/null
exit: 0
stdout bytes: 0.0.1784073393-g9a3a12 (released 2026-07-14T23:56:33.000Z, 8m ago)\n
stderr bytes: <empty>
```

`amp version` and `amp -V` returned the same shape. Amp has no JSON version
option, and the trailing relative age changes over time; it is display text,
not identity. The exact profile identity is version
`0.0.1784073393-g9a3a12` plus release timestamp
`2026-07-14T23:56:33.000Z`; proposed profile id
`amp-0-0-1784073393-g9a3a12`. The default binary is `amp`, and version argv is
`['--version']`. A decoder must require exactly one bounded line and the exact
version/timestamp grammar while excluding the changing age from profile
identity. Adjacent hashes/versions, malformed/control/extra data remain
observe-only.

### Skills and precedence

The official path list says precedence is first-wins:

1. `~/.config/agents/skills/`;
2. `~/.agents/skills/`;
3. `~/.config/amp/skills/`;
4. project `.agents/skills/`;
5. project `.claude/skills/`;
6. `~/.claude/skills/`;
7. plugins, legacy toolbox directories, and built-in skills.

The same manual later says project skills take priority over user-wide skills,
which contradicts that ordered list. Isolated same-name validation on the pinned
release put one skill-local MCP marker in
`~/.config/agents/skills/collision` and another in project
`.agents/skills/collision`. `amp mcp doctor skill-collision` returned exit 0,
empty stderr, labeled the selected server `(user settings)`, and only the user
marker ran (`winner=user`). The pinned current profile therefore follows the
ordered first-wins list; the contrary general sentence is not authority.

Each skill is a complete directory with required `SKILL.md`; sibling resources
are reachable by the agent relative to the skill file. The managed portable
user destination may be `~/.agents/skills`, but the adapter must observe the
higher-precedence `~/.config/agents/skills` root and report a same-name conflict
rather than claim the managed skill is effective. Project `.agents/skills` is
the canonical no-copy destination, but a same-name user skill shadows it on this
profile.

### Settings, MCP schema, and precedence

Selected managed user settings path on Linux/macOS:
`~/.config/amp/settings.json`. `~/.config/amp/settings.jsonc` is a separately
supported alternative; because the source says “or” but gives no both-present
precedence, an existing alternate is a conflict, not authority to write both.
The Windows alternatives are `%USERPROFILE%\.config\amp\settings.json` and
`%USERPROFILE%\.config\amp\settings.jsonc`.

Workspace settings are the nearest `.amp/settings.json` or
`.amp/settings.jsonc`, searched upward from cwd to the repository root (or only
the current directory outside a repository). Workspace settings override user
settings. Enterprise managed settings override both; Linux uses
`/etc/ampcode/managed-settings.json`.

Scoped MCP lives under the literal flat settings key `amp.mcpServers`:

- local: `command`, optional `args`, optional `env`;
- remote: `url`, optional `headers`;
- `${VAR_NAME}` preserves environment references.

MCP source precedence is:

1. CLI `--mcp-config`;
2. user/workspace `amp.mcpServers` (workspace wins a same-name collision);
3. skill-local `mcp.json` only when not already configured above.

The current deterministic declared-state command is
`amp mcp list --json`. In a valid temporary Git repository with user, root, and
nearer nested settings, executing it from `sub/child` returned exit 0, empty
stderr, selected `sub/.amp/settings.json` rather than the repository-root file,
and emitted the following exact decoded JSON value and order:

```json
[
  {"name":"user-only","type":"command","source":"global","spec":{"command":"/bin/true","args":[],"env":{"TOKEN":"${INERT_TOKEN}"}}},
  {"name":"same","type":"command","source":"global","spec":{"command":"/bin/echo","args":["user"]}},
  {"name":"nested-only","type":"command","source":"workspace","spec":{"command":"/bin/false"}},
  {"name":"same","type":"command","source":"workspace","spec":{"command":"/bin/echo","args":["nested"]}},
  {"name":"remote-auth","type":"url","source":"workspace","spec":{"url":"https://mcp.example.invalid/v1","headers":{"Authorization":"Bearer ${INERT_TOKEN}"}}}
]
```

The native output is pretty-printed; the compact rendering above preserves the
exact value/order while avoiding incidental whitespace. `mcp list --json` is
useful declared-state evidence but does not report effective health.

### Deterministic doctor states

Current supported effective command:

```text
amp mcp doctor [name]
```

`--help` says: “Wait for MCP service initialization and display the status of
configured servers.” It has human output, no JSON flag, and returns exit 0 for
healthy, failed, and trust-required outcomes; parsing must therefore be exact-
version and output-state based.

A trusted CLI-flag server using official
`@modelcontextprotocol/server-everything@2026.7.4` (npm integrity
`sha512-ydMW/M6rk9tK23b+U38trsNLHhd5eF+ntiv2Vr+RPMDhbiKY/IKrZU25ukvSXVPUBvy7TxTPWpeV4KcYcXg72w==`)
returned exit 0, empty stderr, and:

```text
User settings: /var/tmp/skilltap-amp-user/.config/amp/settings.json
Workspace settings: /var/tmp/skilltap-amp-healthy/.amp/settings.json

everything (--mcp-config flag): connected (13 tools: echo, get-annotated-message, get-env, get-resource-links, get-resource-reference, get-structured-content, get-sum, get-tiny-image, gzip-file-as-resource, toggle-simulated-logging, toggle-subscriber-updates, trigger-long-running-operation, simulate-research-query)
```

An unapproved workspace server returned exit 0, empty stderr, did not execute the
server command, and emitted:

```text
User settings: /var/tmp/skilltap-amp-user/.config/amp/settings.json
Workspace settings: /var/tmp/skilltap-amp-project/sub/.amp/settings.json

remote-auth (workspace: untrusted, server: untrusted): awaiting approval
```

No `amp mcp approve` command was run. Global settings and `--mcp-config` do not
require workspace approval. A user server that immediately exited returned
exit 0, empty stderr, and the state sequence:

```text
user-only (user settings): connecting...
user-only (user settings): reconnecting (attempt 2, retry in 1500ms)...
user-only (user settings): error - MCP server connection was closed unexpectedly.
```

A same-name `--mcp-config` `/bin/false` server produced the same finite failure
sequence labeled `same (--mcp-config flag)`, proving CLI precedence over both
workspace and user definitions.

Without Amp account authentication, `amp mcp doctor` started the login flow and
did not exit. The 10-second isolated bound produced exit 124 with stable stdout
prefix `No API key found. Starting login flow...\nIf your browser does not open
automatically, visit:\n\n`, followed by a per-run URL/token that is deliberately
not retained, then `When prompted, paste your code here: `. With the inert API
key present, doctor skipped that account login and produced the state evidence
above; no real account request or credential was used.

For remote MCP OAuth, `https://mcp.linear.app/mcp` produced stable doctor stdout:

```text
linear (--mcp-config flag): authenticating (waiting for OAuth)...
```

It then printed a per-run authorization URL, PKCE challenge, state, localhost
callback port, and prompt to stderr and remained running until the 25-second
bound killed it. Those dynamic credential-bearing bytes are intentionally not
stored. Even the narrower command
`amp --settings-file <isolated> mcp oauth status linear` printed
`No OAuth credentials found for linear` and remained running until timeout.
OAuth tokens, when authorized by the user, are stored outside settings under
`~/.amp/oauth/` and refreshed by Amp.

### Skill-local MCP behavior

A skill-local file is `<skill>/mcp.json` whose root directly maps server names
to the same local/remote server objects; it does **not** use the scoped
`amp.mcpServers` wrapper. `includeTools` accepts tool names/globs. Official
behavior is precise: skill-local servers start when Amp launches, but their
tools remain hidden until the skill is loaded. The server is eager; tool
exposure is lazy.

Isolated current-binary validation disproved an assumed skill-directory working
directory. A project skill-local entry with
`"command":"./probe-server.sh"` produced exit 0 and:

```text
probe-relative (user settings): reconnecting (attempt 2, retry in 1500ms)...
probe-relative (user settings): error - ENOENT: no such file or directory, posix_spawn './probe-server.sh'
```

The executable marker remained absent. Replacing it with absolute command
`/bin/sh` and an argument that recorded `pwd` ran immediately at doctor startup
and recorded exactly `/var/tmp/skilltap-amp-skill-project`, the explicit process
cwd, before reporting the expected closed-connection error. Therefore a relative MCP command resolves from Amp's selected process cwd, not
from the skill directory, and the child process inherits that cwd. The adapter
must preserve this behavior and classify a source that assumes
skill-directory-relative execution as partial/blocked rather than silently
rewrite or duplicate it into scoped settings.

The current doctor labels this project skill-local server `(user settings)`, so
that human source label is not reliable proof of native placement. Inventory
identity plus the owned skill tree must retain placement; doctor contributes
health only.

### Amp blocker

`AMP-AUTH-EFFECTIVE-STATE`: current release
`0.0.1784073393-g9a3a12` has deterministic finite doctor output for connected,
failed, and awaiting-approval states, but its authentication-required path is
interactive and non-terminating. The shared `EffectiveStateProbePort` decoder
receives stdout only after successful bounded process completion;
`SystemBoundedNativeProcess` returns `ProcessDeadlineExceeded` without partial
stdout, and `EffectiveServerHealth` cannot represent authentication-required.
Parsing dynamic OAuth stderr would also violate the secret/raw-output boundary.

This is a cross-target shared-contract gap, but the operator allowed writing only
this story, so the completed shared owner was not amended. The Amp adapter story
remains dependency-blocked and Amp remains outside the canonical registry until
a finite native auth observer is attested or the shared probe contract is
explicitly extended to carry a safe typed pre-timeout authentication signal.
