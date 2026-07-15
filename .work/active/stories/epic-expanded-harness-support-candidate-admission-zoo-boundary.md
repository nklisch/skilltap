---
id: epic-expanded-harness-support-candidate-admission-zoo-boundary
kind: story
stage: done
tags: [testing]
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-gate]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/zoocode-skills.md
  - .research/attestation/zoocode-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Validate Zoo Code Boundaries

## Checkpoint

Resolve Zoo Code's highest-risk extension boundary before any production path or
adapter is added. Record the complete source-bound and isolated evidence table in
this story body and finish with exactly one disposition: `admitted`,
`observe_only`, or `blocked`.

Validation must establish:

- exact Zoo extension identity, installed version, and deterministic supported
  host/CLI command that can run through the bounded process port;
- every supported global and project skill root in the documented `.roo` and
  `.agents` families, mode-specific precedence, complete sibling access,
  executable intent, update visibility, and project-over-global collision;
- the stable global `mcp_settings.json` path on macOS and Linux and project
  `.roo/mcp.json`, both as supported direct-write files rather than editor
  databases or caches;
- exact `mcpServers` schema, stdio/HTTP/SSE, enablement/tool policy, scope
  precedence, preservation of unknown/unowned entries, and owned update/removal;
- a deterministic reload/effective server/tool observation after direct edits;
- isolated profile redirection proving the operator's real HOME/editor profile,
  credentials, extension storage, and caches stay untouched.

A candidate integration test at
`crates/harnesses/tests/candidate_zoo_boundary.rs` is created only if all native
roots and processes can be isolated safely. UI automation or screen scraping may
identify a lead but cannot satisfy a gate check.

## Acceptance evidence

Checked items were verified; unchecked items are explicit blockers rather than
unfinished story work.

- [x] Every admission check has exact official source, fetched date, native
      version/output bytes or an exact unavailable result, isolated action,
      observed result, and disconfirming result where relevant.
- [x] No path is accepted solely because the settings UI opened it.
- [ ] Complete skill and MCP updates are effectively observed after the
      documented reload/restart behavior.
- [ ] Removal deletes only proven owned entries and preserves native/unmanaged
      siblings and unknown fields.
- [ ] Immediate repeats produce no file, identity, or effective-state change.
- [x] The disposition names every missing check. Missing platform-independent
      global MCP or cache-independent effective observation prevents
      `admitted`.

## Ordering

Runs after the shared gate and before Zoo's admission checkpoint. It does not
edit the canonical registry or production adapter modules.

## Boundary validation — 2026-07-15

### Isolation stop

The prerequisite gate was at `stage: done`. Z.ai tools were not exposed to this
worker, so current material was fetched only from official Zoo Code HTTPS
endpoints with non-interactive `curl --disable` GET requests. Downloads and
probes wrote only beneath `/tmp/skilltap-zoo-*`.

Before any extension-host action, a non-interactive command probe ran with fresh
temporary `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, and
`XDG_STATE_HOME` roots. Exact stdout was 190 bytes
(`sha256:c32935d98781c86a7ed0c64095baa76e694ebc96d7a508a01a214d392b84064`);
stderr was 0 bytes
(`sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`):

```text
code	absent	command-v-exit=1
code-insiders	absent	command-v-exit=1
codium	absent	command-v-exit=1
cursor	absent	command-v-exit=1
windsurf	absent	command-v-exit=1
zoo	absent	command-v-exit=1
```

Zoo is an editor extension and no documented compatible host command was
available. Consequently no installed extension version or safe effective-state
observation command existed. Per the checkpoint's stop rule, validation stopped
before installing or launching any editor, extension host, extension command,
MCP server, or skill. No browser, OAuth/auth/login, UI, screen scraping, editor
profile, operator HOME, credentials, extension storage, or cache was used or
changed.

### Current official source set

All sources below were fetched on 2026-07-15.

| Handle | Official source and immutable revision | Retrieved bytes / SHA-256 | Relevant contract |
| --- | --- | --- | --- |
| `zoo-install` | [Installing Zoo Code](https://github.com/Zoo-Code-Org/Zoo-Code-Docs/blob/9bc5317cebacf5c7dfa674b81265b85893f2a85a/docs/getting-started/installing.mdx) | 4,168 / `720015ef8fac6ae388b594e8517734b6b3d9abc4545be0ade2a215eb58c7cbb7` | Zoo is a VS Code extension installed through editor UI or VSIX; reload is editor-driven. No Zoo CLI is documented. |
| `zoo-skill-tool` | [skill tool](https://github.com/Zoo-Code-Org/Zoo-Code-Docs/blob/9bc5317cebacf5c7dfa674b81265b85893f2a85a/docs/advanced-usage/available-tools/skill.md) | 3,911 / `3861dfd10701a0f178ca9e934df12b07093ad3095e11699846138c15e871e523` | Enumerates eight project/global, `.roo`/`.agents`, generic/mode-specific roots and linked-file loading. |
| `zoo-skills` | [Skills](https://github.com/Zoo-Code-Org/Zoo-Code-Docs/blob/9bc5317cebacf5c7dfa674b81265b85893f2a85a/docs/features/skills.mdx) | 13,555 / `df70be5f8590967b32e7e87c838d3dc09a73b941a3b99377e614a04527457248` | Complete directories may contain scripts, templates, references, and assets; documents roots, priority, symlinks, and `SKILL.md` watchers. Skills do not register executable tools. |
| `zoo-mcp` | [Using MCP in Zoo Code](https://github.com/Zoo-Code-Org/Zoo-Code-Docs/blob/9bc5317cebacf5c7dfa674b81265b85893f2a85a/docs/features/mcp/using-mcp-in-roo.mdx) | 25,363 / `728e5985234abc1962740175b9fac097b8fb32e935a8ac7e0dc7dc0c6c7d65a8` | Documents global `mcp_settings.json` only through the settings view, project `.roo/mcp.json`, project precedence, schema fields, transports, and UI refresh/tool observation. |
| `zoo-transports` | [MCP transports](https://github.com/Zoo-Code-Org/Zoo-Code-Docs/blob/9bc5317cebacf5c7dfa674b81265b85893f2a85a/docs/features/mcp/server-transports.md) | 11,794 / `b3bb41ff920483cd8ec60e3773b013babb05968d4ed99fb3c45bf9113370bd05` | Describes stdio, streamable HTTP, and legacy SSE. |
| `zoo-release` | [Zoo Code v3.68.0](https://github.com/Zoo-Code-Org/Zoo-Code/releases/tag/v3.68.0), commit `1833f5a9ec84f4138a7598a853895326cf38312a` | Release API: 6,209 / `82d2ffe82227f8e705bc5125d6b09cf90b64795bfce6fdaf8d00e35a85ece5d9`; source tarball: 122,886,110 / `60479e537e0ed87b64744dd13b8f09cbf48438bb13e93b5bab150c08eed9d40a` | Latest stable official release at fetch time. |
| `zoo-vsix` | [Official v3.68.0 VSIX](https://github.com/Zoo-Code-Org/Zoo-Code/releases/download/v3.68.0/zoo-code-3.68.0.vsix) | 34,153,931 / `2ae6c85a695ad352680da0b29abfcfbba70de73417656fbb4017d4597671bb5b`; embedded manifest: 14,820 / `e9d3a93b868595b7fa4248c53c06c9011728002958fa4928569e136107af1e10` | Manifest identity is `ZooCodeOrganization.zoo-code`, version `3.68.0`, main `./dist/extension.js`, VS Code engine `^1.100.0`, and no `bin`. This is artifact identity, not an installed-version observation. |
| `zoo-source` | [v3.68.0 source](https://github.com/Zoo-Code-Org/Zoo-Code/tree/v3.68.0) | Per-file hashes recorded below | Establishes implementation paths/watchers and disconfirming gaps; source inspection is not effective runtime evidence. |

The official Open VSX endpoint linked by `zoo-install` reported verified
`ZooCodeOrganization.zoo-code` version `3.69.100247` (5,458 response bytes,
`sha256:7c4bebf8856f831105a260436b67ad503431ddfdd713ebf70c0b140e446fc88e`).
That distribution-channel value differs from the latest stable GitHub release,
reinforcing that a controller must observe the installed version rather than
infer it from a registry.

Relevant stable source files were checked exactly:

| Source file | Bytes / SHA-256 | Source result |
| --- | --- | --- |
| [`src/package.json`](https://github.com/Zoo-Code-Org/Zoo-Code/blob/v3.68.0/src/package.json) | 14,820 / `e9d3a93b868595b7fa4248c53c06c9011728002958fa4928569e136107af1e10` | Exact extension identity/version; extension commands only; no standalone binary. |
| [`src/services/roo-config/index.ts`](https://github.com/Zoo-Code-Org/Zoo-Code/blob/v3.68.0/src/services/roo-config/index.ts) | 13,061 / `7b2eb22decc1ba3cf03577c51d466050a152bdeb59548e6264b0345fdec33b88` | Global skill bases derive from `os.homedir()` as `~/.roo` and `~/.agents`; project bases derive from workspace cwd. |
| [`src/services/skills/SkillsManager.ts`](https://github.com/Zoo-Code-Org/Zoo-Code/blob/v3.68.0/src/services/skills/SkillsManager.ts) | 24,072 / `5b2a7053af91dcc65573229c8bcddfdc5f8cfeacad68e9337e4c7a89a96e2e1c` | Follows directory/skill symlinks, reads `SKILL.md`, resolves project over global and mode-specific over generic, and watches only `**/SKILL.md`. |
| [`src/services/mcp/McpHub.ts`](https://github.com/Zoo-Code-Org/Zoo-Code/blob/v3.68.0/src/services/mcp/McpHub.ts) | 87,466 / `c19f79b0d89899bd9c4ee56c6a7b2db9d7e11215cbffe43f97638f87597c8930` | Global path is extension storage plus `settings/mcp_settings.json`; project path is workspace `.roo/mcp.json`; both are watched; runtime server/tool state is sent to the webview. |
| [`src/utils/storage.ts`](https://github.com/Zoo-Code-Org/Zoo-Code/blob/v3.68.0/src/utils/storage.ts) | 4,671 / `95b2477e07ff36a4ab3e1f0a1875ba1677824bf97fbd1acf2b564ff4961af00d` | Global settings base is either host-provided `globalStorageUri.fsPath` or the configured `zoo-code.customStoragePath`; configuration of that override itself belongs to the editor profile. |
| [`src/shared/globalFileNames.ts`](https://github.com/Zoo-Code-Org/Zoo-Code/blob/v3.68.0/src/shared/globalFileNames.ts) | 298 / `0899ab51711478743f970592fd542cbc164e6ba3ea70e6a1498e4740fa95f972` | Confirms filename `mcp_settings.json`. |

### Admission-check evidence

`Native result` means observed from an installed Zoo extension. Source-only
findings are deliberately not promoted to native results.

| Admission check | Official-source contract | Isolated action | Native result | Disconfirming result or missing check |
| --- | --- | --- | --- | --- |
| Extension identity, installed version, bounded command | `zoo-install`, `zoo-release`, `zoo-vsix`, `src/package.json` | Downloaded and inspected the official stable VSIX under `/tmp`; probed documented compatible host commands under redirected roots. | None. No host or Zoo executable exists; therefore installed version output is unavailable. | Stable artifact is `ZooCodeOrganization.zoo-code` 3.68.0, while the linked Open VSX channel reports 3.69.100247. The manifest has no `bin`, docs expose editor/UI installation, and no deterministic Zoo list/status/version command is documented. |
| Global and project skill roots | `zoo-skill-tool`, `zoo-skills`, `roo-config/index.ts` | Source-bound enumeration only; no files were placed in operator or project roots after the host probe failed. | None. | Official roots are `{project,~}/{.roo,.agents}/skills[-<mode>]/<name>/SKILL.md`, but `os.homedir()` redirection was not exercised inside a real extension host. |
| Mode and family precedence; project/global collision | `zoo-skill-tool`, `zoo-skills`, `SkillsManager.ts` | Compared docs with stable source. | None. | Docs list `.roo` generic above `.agents` mode-specific at a scope, while the same docs also summarize mode-specific above generic. Stable source resolves project over global, mode-specific over generic, and `.roo` over `.agents` only for the same scope/mode key. Exact cross-family/cross-mode collision behavior remains contradictory and unobserved. |
| Complete siblings, executable intent, symlinks, update visibility | `zoo-skills`, `SkillsManager.ts` | Inspected docs and source only. | None. | Docs permit on-demand bundled scripts/templates/references/assets and say skills do not register executable tools. Source follows symlinks but watches only `SKILL.md`. Sibling reads, executable-bit intent, sibling-only updates, and effective reload were not observed. |
| Stable global MCP path on macOS and Linux | `zoo-mcp`, `McpHub.ts`, `storage.ts`, `globalFileNames.ts` | Resolved the stable source expression without opening settings UI. | None. | The expression is `<editor globalStorageUri or customStoragePath>/settings/mcp_settings.json`; docs do not name macOS/Linux absolute defaults, and compatible editors/profiles choose `globalStorageUri`. No platform-independent global path is established. |
| Project MCP direct-write file and precedence | `zoo-mcp`, `McpHub.ts` | Source-bound path/schema inspection only. | None. | `<project>/.roo/mcp.json` and project-name precedence are documented and implemented, but direct edit, collision, and effective state were not run. |
| `mcpServers` schema, stdio/HTTP/SSE, enablement/tool policy | `zoo-mcp`, `zoo-transports`, `McpHub.ts` | Compared docs with the stable Zod schema. | None. | Source accepts stdio `command/args/cwd/env`, URL transports with explicit `sse` or `streamable-http`, plus `disabled`, `timeout`, `alwaysAllow`, `watchPaths`, and `disabledTools`. No installed runtime validated any transport or policy. |
| Direct edits, reload, effective server/tool observation | `zoo-mcp`, `McpHub.ts` | Stopped before launch. | None. | Source has 500 ms file-watch debounce and internal/webview server/tool state; docs offer UI refresh. No documented cache-independent non-interactive list/status surface exists, and UI use was prohibited. |
| Preserve unknown/unowned entries; owned update/removal | `McpHub.ts` | No mutation attempted after isolation stop. | None. | Required preservation/update/removal checks are missing. Disconfirmingly, native `updateServerConfig` and `deleteServer` rebuild the top-level document as only `{mcpServers}`, so native UI mutation is not evidence that unknown top-level fields survive. |
| Immediate repeat/idempotence | No native command contract found. | No mutation attempted after isolation stop. | None. | File, identity, server/tool, update, and removal repeat checks are all missing. |
| Fully redirected HOME/profile/storage/cache/process tree | `zoo-install`, `roo-config/index.ts`, `storage.ts` | Redirected shell probe roots only; launched no host. | None. | Skill roots depend on process home, MCP depends on editor global storage or an editor-profile setting, and no installed host was available to prove all roots and child processes redirect together. |

### Missing checks

The boundary is missing all evidence that requires an installed, safely
redirectable extension host:

1. exact installed extension version bytes and a documented deterministic
   list/status/version command suitable for the bounded process port;
2. native discovery at every global/project `.roo` and `.agents` generic and
   mode-specific skill root, exact collision precedence, linked sibling reads,
   executable intent, sibling-only update visibility, and reload;
3. official absolute global MCP paths for macOS and Linux independent of editor
   cache/profile inference;
4. effective direct-edit observation for global and project MCP, same-name
   precedence, stdio/streamable-HTTP/SSE behavior, enablement, `alwaysAllow`,
   `disabledTools`, and actual server/tool lists;
5. unknown/unowned-field and sibling-server preservation, ownership-scoped
   update/removal, and immediate no-op repeats; and
6. proof that HOME, editor user data, extensions, credentials, global storage,
   caches, all child processes, and project roots can be redirected together.

No candidate integration test was created because the required native roots and
processes could not all be isolated and no effective observation boundary was
available. No production adapter or registry entry was added.

## Disconfirming analysis

The current source closes some prior documentation leads but does not close the
boundary. It confirms the global MCP filename and its extension-storage-relative
expression, not stable macOS/Linux absolute paths. It implements file watchers,
not a deterministic external observation command. It also exposes two risks
that require native checks: documentation and source do not state the same
cross-family/cross-mode skill priority, and native MCP update/removal helpers
reconstruct the top-level JSON object. Neither risk can be resolved by treating
source inspection or the webview as effective-state evidence.

## Original mutation disposition

**blocked**

The exact installed extension identity, host isolation, global storage path,
effective observation, preservation, ownership, removal, and repeat checks
remain unavailable under the mandated isolation and non-UI constraints.

## Relaxed registry disposition — 2026-07-15

The relaxed gate requires reliable target identity plus a safe documented read
surface, not an exact installed mutation profile. Zoo's source-attested
extension identity and documented `.roo`/`.agents` skill roots plus project
`.roo/mcp.json` provide that narrow read boundary. The adapter deliberately
omits the editor global-storage MCP file because its host-provided storage
location is unresolved.

The registry disposition is therefore **`observe_only`**. Host redirection,
installed extension identity, global storage, and effective reload remain
explicit status boundaries. The original blocked mutation evidence is retained
above; no Zoo file writer, editor/cache writer, or effective probe is implied.

## Implementation notes

- Execution capability: direct-read boundary worker; no nested agent or peer was
  used, matching caller ownership and egress constraints.
- Review weight: not applicable — child story checkpoint.
- Files changed: this story only.
- Tests added/removed: none; `candidate_zoo_boundary.rs` is forbidden until all
  native roots and processes are safely isolated.
- Simplification: stopped before speculative fixtures or a production adapter
  could turn source inference into runtime authority.
- Discrepancies from design: the host and installed Zoo extension are absent;
  current docs/source also leave a precedence contradiction and no stable
  platform-independent global MCP path.
- Adjacent issues parked: none.
