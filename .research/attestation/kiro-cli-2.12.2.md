---
source_handle: kiro-cli-2.12.2
fetched: 2026-07-15
source_url: https://prod.download.cli.kiro.dev/stable/latest/manifest.json
provenance: source-direct
substrate_confidence: source-direct-plus-isolated-runtime
---

# Kiro CLI 2.12.2 isolated runtime evidence

## Distribution and exact version

The official stable manifest at
`https://prod.download.cli.kiro.dev/stable/latest/manifest.json` reported:

- `version`: `2.12.2`
- package: `2.12.2/kirocli-x86_64-linux.tar.xz`
- target: `x86_64-unknown-linux-gnu`
- variant: `headless`
- size: `493345216`
- SHA-256: `42c456c08d05f3eaef41d9126f277c9f0d83b8a608ee82b3deacfd561f147370`

The archive was fetched from the corresponding official stable URL and its
streamed SHA-256 matched the manifest exactly. The `kiro-cli` ELF was extracted
without installing the package or writing outside a temporary fixture.

In an isolated `HOME`, `KIRO_HOME`, XDG configuration/cache roots, and project,
`kiro-cli --version` emitted exactly:

```text
kiro-cli 2.12.2
```

The version decoder must therefore strip the `kiro-cli ` prefix and reject
extra lines, whitespace, or adjacent versions. The executable default is
`kiro-cli`.

## Current official CLI and file contract

Primary sources refreshed on 2026-07-15:

- Installation: https://kiro.dev/docs/cli/installation/
- Commands: https://kiro.dev/docs/cli/reference/cli-commands/
- Skills: https://kiro.dev/docs/cli/skills/
- MCP configuration: https://kiro.dev/docs/cli/mcp/configuration/
- Powers: https://kiro.dev/docs/powers/
- Stable package manifest: https://prod.download.cli.kiro.dev/stable/latest/manifest.json

The official docs define:

- global skills at `${KIRO_HOME:-~/.kiro}/skills` and workspace skills at
  `.kiro/skills`, with workspace skills taking precedence;
- global MCP at `${KIRO_HOME:-~/.kiro}/settings/mcp.json` and workspace MCP at
  `.kiro/settings/mcp.json`;
- the MCP document's `mcpServers` object, command and URL server forms,
  environment/header references, `disabled`, and `disabledTools` fields;
- `kiro-cli mcp list [SCOPE]`, with `workspace` and `global` examples;
- file hot reload at idle boundaries without a command, restarting only changed
  servers and preserving session-injected servers; and
- interactive `/mcp` as the documented view of loaded servers and tools.

`KIRO_HOME` was verified in the isolated command environment to relocate the
CLI's global settings root; it does not relocate the project `.kiro` root.

Powers remain a separate IDE/web bundle containing `POWER.md`, MCP
configuration, and steering/hooks. The current Powers source does not establish
CLI Power installation and Powers are not translated by the Kiro adapter.

## Effective-observation blocker

In the isolated fixture, both:

```text
kiro-cli mcp list global
kiro-cli mcp list workspace
```

failed before listing because the official binary requires an authenticated Kiro
account (`You are not logged in, please log in with kiro-cli login`). The public
command reference describes `mcp list` as listing configured servers, while the
public MCP guide identifies interactive `/mcp` as the loaded-server/tool status
surface; it does not publish a stable machine-readable or human-output grammar
for `mcp list` nor a non-interactive effective-load probe.

No host credentials were copied into the fixture, and no login flow was run.
Therefore this evidence authorizes the path/schema/profile research and
version-pinned codec tests, but it does **not** authorize registering Kiro as a
mutation target. The adapter remains unregistered until an authenticated,
non-interactive effective-load contract or an official stable output grammar is
established.
