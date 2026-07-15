---
id: epic-expanded-harness-support-candidate-admission-zcode-boundary
kind: story
stage: done
tags: [testing]
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-gate]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/zcode-skills.md
  - .research/attestation/zcode-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Validate ZCode Boundaries

## Checkpoint

Identify ZCode's exact native files and deterministic effective observation in
an isolated installation before adding production constants or ports. Preserve
the documented global `~/.zcode/skills` evidence, but do not infer the missing
project skill path from the `.zcode` family name or import UI.

Validation must establish exact detection/version identity; project skill root;
user/workspace MCP files; direct-edit support; copy-versus-symlink behavior;
complete sibling/executable access; global/project precedence; per-skill and
per-server enablement; reload/effective state; unknown-field preservation;
owned update/removal; and cache/credential boundaries on macOS and Linux.

Use `crates/harnesses/tests/candidate_zcode_boundary.rs` only if an official
redirectable host/CLI boundary exists. Record exact sources, commands, bytes,
paths, and results in this story body and conclude `admitted`, `observe_only`,
or `blocked`.

## Original mutation disposition

**`blocked`** for the current official ZCode 3.3.5 boundary.

The current public docs establish the global skill root and exact native
user/workspace MCP files, but do not establish an exact project skill root,
deterministic non-UI installation/version observation, a redirectable isolated
profile, or a headless effective-state/reload surface. These gaps remain
mutation blockers and no file writer is authorized.

## Relaxed registry disposition — 2026-07-15

The relaxed gate admits reliable product identity plus safe documented reads.
ZCode therefore registers a typed file-only observe-only adapter for the exact
global `~/.zcode/skills`, global `~/.zcode/cli/config.json`, and project
`.zcode/config.json` MCP declarations. Project skill observation remains
unsupported rather than inferred. Installed identity, effective reload, and
cache-independent proof remain explicit unresolved boundaries.

The registry disposition is **`observe_only`**; the original blocked mutation
evidence above is retained.

## Source-direct evidence

Evidence was refreshed on 2026-07-15 through direct unauthenticated HTTPS only.
No browser, login, OAuth flow, application UI, interactive import, or host
mutation was used.

| Official source | HTTP result | Retrieved bytes / SHA-256 | Boundary established |
|---|---|---|---|
| `https://zcode.z.ai/en/docs/install` | `200`, same final URL | `117630` / `0e9693ea710ab5c864eb3a2579c00f6077700bd2bb0939221556e7b23e38b23a` | Desktop release `3.3.5`; direct macOS and Windows downloads; Linux only through a beta group. |
| `https://zcode.z.ai/en/changelog` | `200`, same final URL | `97243` / `74184fefb537572f5e9695b37a4afb77b391a77e1d36ab256ad6a025190e8c53` | Release `3.3.5`, dated 2026-07-13. |
| `https://zcode.z.ai/en/docs/skill` | `200`, same final URL | `69989` / `ffe7a8ad97690e18eac9d800b16d777ed92907b4773577b65bca963837d49ad6` | A skill is a directory containing `SKILL.md`; exact user path is `~/.zcode/skills/<skill-name>/SKILL.md`; settings refresh and per-skill enablement are UI surfaces. |
| `https://zcode.z.ai/cn/docs/skill` | `200`, same final URL | `69865` / `12c28d5095b44332e99dc07a393aad5f4642224367373d5ca5a9df1a90748cf9` | Corroborates the global path and UI import targets; symlink points at the external skill directory, while copy creates an independent copy. Neither localization names the current-project destination. |
| `https://zcode.z.ai/en/docs/mcp-services` | `200`, same final URL | `75007` / `7e858c48cc855d5ae8f2b6fb9c49f27ae0a524f4178860428098fd898fbdee1c` | User/workspace scope, stdio/HTTP/SSE, JSON import, and UI enable/edit/delete; it names only the `.zcode` configuration family. |
| `https://zcode.z.ai/cn/docs/mcp-services` | `200`, same final URL | `100069` / `ec19beeafed6ad48a0a64a2be304e8a19a375effe58e735fd4ad1e78ca6f26c5` | Exact native files, schemas, direct editing, source precedence, and enablement described below. |

The current Chinese MCP contract is materially more complete than the English
localization:

- Native user file: `~/.zcode/cli/config.json`, key `mcp.servers`.
- Native workspace file: `<project-root>/.zcode/config.json`, key
  `mcp.servers`.
- Compatible fallback files: `~/.agents/mcp.json` and
  `<project-root>/.agents/mcp.json`, key `mcpServers`.
- Direct manual editing of these fixed paths is explicitly supported.
- When a workspace is open, workspace scope is read before user scope and both
  scopes appear in the list. The docs do not settle same-name collision
  behavior between those scopes.
- Within one scope, any MCP service in the native `.zcode` file causes the whole
  `.agents/mcp.json` file to be skipped; the files are not merged.
- `"enable": false` disables a server; omission means enabled. Settings writes
  always target the native `.zcode` file and leave `.agents` unchanged.

A source-direct sweep of all 21 English and 21 Chinese documentation pages
linked by the official docs navigation found no second project skill path, no
`--version` contract, no profile/data-root redirection, no headless mode, and no
documented CLI command for skill/MCP list, status, or reload. This is bounded to
those current official pages; missing contracts are recorded as unknown rather
than replaced with guessed Electron or `.zcode` paths.

## Safe actions and exact results

- `command -v zcode` -> exit `1`, no path.
- `command -v zcode-cli` -> exit `1`, no path.
- `command -v zai` -> exit `1`, no path.
- Validation host: `Linux 7.0.9-200.nobara.fc43.x86_64 x86_64`.
- Direct `HEAD` requests confirmed the four official 3.3.5 macOS/Windows
  artifacts return `200`; no artifacts were downloaded or executed:
  - `https://cdn-zcode.z.ai/zcode/electron/releases/3.3.5/ZCode-3.3.5-mac-arm64.dmg`,
    `161593509` bytes.
  - `https://cdn-zcode.z.ai/zcode/electron/releases/3.3.5/ZCode-3.3.5-mac-x64.dmg`,
    `169853442` bytes.
  - `https://cdn-zcode.z.ai/zcode/electron/releases/3.3.5/ZCode-3.3.5-win-x64.exe`,
    `138197432` bytes.
  - `https://cdn-zcode.z.ai/zcode/electron/releases/3.3.5/ZCode-3.3.5-win-arm64.exe`,
    `141007240` bytes.
- The Linux documentation redirects acquisition to an interactive Feishu beta
  group and supplies no deterministic package URL. It says the app can be
  launched from the command line but names no executable or arguments.
- The macOS/Windows artifacts are desktop installers, not a documented
  redirectable validation host: the official docs provide no isolated profile
  root, headless effective-state observation, or reload command.
- Because no official redirectable deterministic host/CLI exists for this
  validation boundary, no isolated skill/MCP writes, app launches, reloads,
  imports, update/removal exercises, or repeat mutations were attempted.

## Admission matrix

| Check | Result | Exact evidence or blocker |
|---|---|---|
| `ExactInstallationIdentity` | missing | Official current release is 3.3.5, but no installed binary or documented version command exists; an artifact URL is not observed installation identity. |
| `DocumentedGlobalSkillRoot` | proven | `~/.zcode/skills/<skill-name>/SKILL.md`. |
| `DocumentedProjectSkillRoot` | missing | Official docs say only current `Project`; no path is named. |
| `CompleteSkillSiblings` | missing | Directory symlink import is documented, but sibling and executable access were neither specified nor isolatedly observed. |
| `SkillPrecedenceAndReload` | missing | UI refresh and enablement are documented; same-name scope precedence and deterministic reload observation are not. |
| `DocumentedGlobalMcpFile` | proven | `~/.zcode/cli/config.json`; `.agents` fallback also documented. |
| `DocumentedProjectMcpFile` | proven | `<project-root>/.zcode/config.json`; `.agents` fallback also documented. |
| `McpSchemaAndPrecedence` | missing | Native/fallback schema and same-scope source precedence are documented, but same-name workspace/user collision behavior was not established or reproduced. |
| `EffectiveReloadObservation` | missing | Only the settings list is documented; UI operation is forbidden and no CLI/host observation contract exists. |
| `UnknownFieldAndSiblingPreservation` | missing | No supported isolated writer/reload boundary was available. |
| `OwnershipSafeUpdateAndRemoval` | missing | No supported isolated mutation boundary was available. |
| `CacheIndependentBoundary` | missing | Declared files are documented, but effective state is observable only through the UI in the current public contract. |
| `SharedAdapterAcceptance` | missing | No eligible host meant no candidate test could be created or run. |
| `ImmediateRepeatNoChange` | missing | No mutation was permitted or available to repeat. |

The gate was not invoked with documentary labels: its contract requires a
production-aware runner to perform the checks before returning them. The three
proven path facts are insufficient for read-only admission.

## Acceptance evidence

- [ ] Exact project skill and both MCP files are source-direct, not inferred.
      Both MCP files are now exact; the project skill destination remains
      unnamed.
- [ ] Direct writes are proven effective and supported rather than merely
      importable or editable through UI state. MCP direct editing is officially
      supported, but effective reload could not be reproduced without a safe
      host; skill direct-write behavior is incomplete.
- [ ] Symlink mode preserves the complete skill tree and works with the shared
      canonical project-skill contract; copy mode is not silently substituted.
      The modes are distinguished in docs but could not be exercised.
- [ ] Same-name scope precedence and enablement survive update/removal without
      changing unowned state. MCP source precedence and enablement encoding are
      documented, but collision, update, and removal behavior remain untested.
- [ ] Every mutation and reload repeats to no change in isolated roots. No safe
      mutation/reload boundary exists.
- [x] Missing filenames, direct-edit authority, or deterministic effective
      observation prevents `admitted` and is retained as the explicit blocker.

## Implementation notes

- Execution capability: direct-read only; source refresh and non-mutating host
  detection were sufficient to identify the blocker, and nested agents/peers
  were explicitly forbidden.
- Review weight: not applicable; this is a child-story checkpoint and child
  stories advance directly to `done` after verification.
- Files changed: this story and the essential current-source correction to
  `.research/attestation/zcode-mcp.md`.
- Tests added/removed: none; no official redirectable host/CLI exists, so a
  documentary-label candidate test would violate the admission gate.
- Simplification: no code or production surface was introduced.
- Discrepancies from design: the exact MCP filenames are now source-direct from
  the official Chinese docs; the project skill path and effective observation
  boundary remain missing.
- Adjacent issues parked: none.

## Verification

- Re-read every changed line against the six direct official responses and the
  shared fourteen-check candidate matrix.
- Confirmed only this story and the essential MCP attestation correction are in
  the owned diff.
- Confirmed no adapter, registry, fixture, profile, test, operator state, or
  `.work/bin/work-view` content was changed by this story.

## Ordering

Ran after the completed shared gate and before ZCode's admission checkpoint. It
creates no production adapter or registry entry.
