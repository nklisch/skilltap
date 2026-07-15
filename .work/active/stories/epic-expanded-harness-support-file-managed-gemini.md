---
id: epic-expanded-harness-support-file-managed-gemini
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-file-managed
depends_on: [epic-expanded-harness-support-file-managed-contracts]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Gemini CLI Adapter

## Checkpoint

Implement and register a distinct `GeminiAdapter` only after the contracts
checkpoint pins its exact native version/output profile. The adapter is a
managed distribution target, not a first-party bootstrap target and not a
Gemini extension/marketplace lifecycle adapter.

## Native contract

- Default executable: `gemini`.
- Complete skills: choose portable `~/.agents/skills` globally and
  `<project>/.agents/skills` in projects; also observe `.gemini/skills` as a
  native precedence/unmanaged surface.
- MCP: merge only `mcpServers` in `~/.gemini/settings.json` and
  `<project>/.gemini/settings.json`; preserve unrelated settings and servers.
- Status: bounded `gemini mcp list`, using project root as cwd at project scope.
- Reload: `/mcp reload` is interactive and becomes an actionable next step, not
  a subprocess invocation.
- Trust: project files are declared but not effective until positive workspace
  trust/status evidence exists.

## Implementation boundary

Add `crates/harnesses/src/adapters/gemini.rs` and `gemini_managed.rs`.
`GeminiSkillProjection` supplies roots to the existing standalone project-link
planner. `GeminiManagedProjection` consumes the shared complete source plugin
and owns only Gemini JSON mapping/path decisions. `GeminiEffectiveStateProbe`
owns exact-version status decoding and trust evidence. No extension cache or
native package directory is written.

## Acceptance evidence

- Known exact profile is mutation-authorized in both scopes; unknown versions
  are observe-only.
- Complete skills require no project link because Gemini consumes the canonical
  root directly.
- Stdio/HTTP/SSE definitions map only when semantically faithful; incompatible
  required fields block and optional loss requires acknowledgment.
- Global/project precedence, preservation of unrelated JSON/native skills,
  drift, pending recovery, owned removal, rollback, and immediate-repeat no-op
  pass through shared matrices.
- Trusted MCP status verifies load. Untrusted/unknown status is attention
  required and does not create successful ownership evidence.
- Gemini extension and cache paths remain untouched.

## Implementation

- Added and registered the distinct managed `GeminiAdapter` with default binary
  `gemini`, exact profile `gemini-0-50-0`, and verified native version `0.50.0`.
  It exposes managed projection, skill projection, and the bounded effective
  MCP probe, but no native extension or marketplace lifecycle.
- Added global and project skill observation for `.agents/skills` plus the
  unmanaged `.gemini/skills` surface. Standalone project skills target the
  canonical `.agents/skills` root directly and therefore do not create links.
- Added the Gemini managed codec for complete source skill trees and
  `.gemini/settings.json` MCP projection at both scopes. It preserves unrelated
  settings and unowned servers, rejects same-name ownership conflicts and
  drift, validates stdio/SSE/HTTP/httpUrl mappings, rejects source-root paths
  and literal credential values, and records optional omissions or required
  incompatibilities through the shared projection contract.
- Added exact-version parsing for the 0.50.0 human-readable `gemini mcp list`
  report, including its stderr output, Connected/Disconnected/Disabled health
  markers, and untrusted-folder warning. Connected status is positive
  effective evidence; untrusted or indeterminate status remains attention
  required. Interactive `/mcp reload` is surfaced as a next action only.
- Updated the registry contract assertions and the compiled CLI registry
  fixture assertions for the newly registered target. No Gemini extension
  directory or cache is accessed or written.

## Official evidence

Validated against current official sources and an isolated temporary HOME,
XDG/config root, project, package cache, and npm installation:

- Official Gemini skills contract:
  https://geminicli.com/docs/cli/using-agent-skills/
  — user `~/.gemini/skills` or `~/.agents/skills`, workspace `.gemini/skills`
  or `.agents/skills`, with workspace precedence and complete skill directories.
- Official Gemini MCP contract:
  https://geminicli.com/docs/tools/mcp-server/
  — `mcpServers` in settings, command/url/httpUrl transports, environment and
  header references, and MCP list behavior.
- Official settings paths and schema:
  https://geminicli.com/docs/reference/configuration/
  — `~/.gemini/settings.json` and project `.gemini/settings.json`, with project
  settings overriding user settings.
- Official trust contract:
  https://geminicli.com/docs/cli/trusted-folders/
  — untrusted workspaces ignore project settings; headless trust uses
  `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true`; persisted rules use
  `~/.gemini/trustedFolders.json`.
- Official extension contract, intentionally not used for materialization:
  https://github.com/google-gemini/gemini-cli/blob/main/docs/extensions/reference.md
  — extension install/update/cache semantics remain outside this adapter.
- Official npm registry package metadata:
  https://registry.npmjs.org/@google/gemini-cli/0.50.0
  — package repository is `google-gemini/gemini-cli`, tarball is
  `https://registry.npmjs.org/@google/gemini-cli/-/gemini-cli-0.50.0.tgz`,
  registry SHA-1 is `c908be1813d7a612921bedbf99d8fc631bd3e159`, and registry SRI
  is `sha512-Z2HSR+UWgOP11WYomOymglYk2AcwEdFfQKytzVla9tkqMIYGVgIvXobmdxI4cgfPQ54XKE9V9uQBP2PrDhvljQ==`.
  Downloaded tarball hashes matched both values before installation.

The isolated official package produced exact `--version` stdout `0.50.0\n`.
The exact status argv is `gemini mcp list`; `mcp list` exits zero and writes its
human report to stderr. Empty status is `No MCP servers configured.`. Configured
status uses `Configured MCP servers:` and rows such as `✓ name: command
(stdio) - Connected`, `✗ ... - Disconnected`, or `○ ... - Disabled`. An
untrusted project additionally reports that MCP servers are disabled because
the folder is untrusted. Setting `GEMINI_CLI_TRUST_WORKSPACE=true` in the
isolated project produced connected status for the fixture MCP server. The
isolated run created only normal Gemini history/project metadata; no extension
or extension-cache path was written.

## Root cause, fix, and evidence

The blocker was real: managed routing checked only the adapter's managed
surface flag before planning, so it could construct a managed mutation without
binding the request to the exact detected executable, native version, compiled
profile, and concrete scope. That also allowed an unknown Gemini version to
reach managed planning and state seeding.

The shared lifecycle now resolves a `ConfiguredAdapterProfile` before managed
planning, stores it with the planned entry, and revalidates executable identity,
version, profile, capability, target, and scope after the configuration lock is
held. Unknown and adjacent versions remain observe-only before planning, and
failed lock-time revalidation performs no target writes or new state
publication. The managed capability is separate from native Codex project
lifecycle support, preserving Codex managed routing and source-only marketplace
behavior.

Compiled regressions prove that Gemini 0.50.0 materializes complete skill and
MCP projections in both global and project scopes, while 0.50.1 and 0.49.0
return attention with unchanged target surfaces and no target state seed. A
version flip between planning and lock-time revalidation is rejected before
writes and preserves prior state. OpenCode and Kiro draft adapters remain
unregistered and uncompiled.

## Verification

- `cargo test -p skilltap-harnesses --lib` — 39 passed.
- `cargo test -p skilltap --lib` — 70 passed.
- `cargo test -p skilltap --test compiled_binary` — 60 passed.
- `cargo check --workspace --tests` — passed.
- `cargo test --workspace --all-targets` — 602 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `cargo fmt --all -- --check` and `git diff --check` — passed.
