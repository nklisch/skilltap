---
provenance: agent-synthesis
status: current
updated: 2026-07-12
temporal_contract: re-engage-on-trigger
scope: agent harnesses with whole-directory Agent Skills and global/project MCP configuration
supersedes: .research/analysis/briefs/harness-adapter-candidates-2026-07-12.md
refresh_verification: source-direct attestations refreshed under a materially revised admission gate; citation lint required
---

# Harness adapter targets under the skills-plus-MCP contract

## Decision

This brief **reverses** the prior candidate decision. The earlier report made a
native marketplace and plugin lifecycle a hard admission gate. The revised
product contract permits skilltap to own acquisition, updates, materialization,
and removal, so a target now needs only:

1. documented complete skill directories with `SKILL.md` and sibling resources;
2. documented MCP configuration at both global and project scope;
3. supported configuration surfaces that skilltap can inspect and reconcile
   without writing an opaque cache.

Hooks, instructions, agents, commands, and other plugin components affect
compatibility classification, but their absence does not disqualify the target.

{inferred: admission decision} Eleven harnesses clear this gate with documented
write surfaces now: **Factory Droid, Qwen Code, GitHub Copilot CLI, Gemini CLI,
Junie, Kimi Code CLI, OpenCode, Kilo Code, Mistral Vibe, Kiro CLI, and Amp**.

{inferred: boundary-spike decision} **Cursor, Zoo Code, and ZCode** expose the
required behavior, but each retains a documentation gap at a write boundary.
Treat them as candidates only after a small isolated adapter spike identifies
and verifies the exact current global/project files.

{inferred: conditional decision} **Pi** qualifies only when skilltap also owns
installation and health of the `pi-mcp-adapter` package. Pi core explicitly has
no MCP client. **Goose, Windsurf/Devin Desktop Cascade, and Cline** remain below
the gate because their current official contracts do not establish an ambient,
project-scoped MCP configuration surface. **Roo Code** is retired and should not
receive a new adapter.

## Revised gate and adapter meaning

A qualifying adapter may use native lifecycle commands when they exist, but it
does not depend on them. Skilltap may instead:

- acquire a source into its own managed store;
- project each complete skill directory into the documented harness load path;
- translate supported MCP declarations into the target's documented config;
- preserve unrelated/unknown native configuration;
- observe files plus any documented list/status surface;
- update or remove only artifacts whose ownership and last-applied fingerprint
  are recorded by skilltap.

This is component-level portability, not an assertion that every source plugin
is portable. A bundle with skills and MCP can be faithful on those components
while hooks, agents, commands, LSP, apps, or harness-specific code remain
partial or blocked.

## Capability matrix

| Harness | Whole skill directory | Global skill | Project skill | Global MCP | Project MCP | Observable/reload behavior | Tier |
|---|---|---|---|---|---|---|---|
| Factory Droid | `SKILL.md` + siblings | `~/.factory/skills` | `.factory/skills` | `~/.factory/mcp.json` | `.factory/mcp.json` | Auto reload; CLI/UI state | A |
| Qwen Code | `SKILL.md` + siblings | `~/.qwen/skills` | `.qwen/skills` | `~/.qwen/settings.json` | `.qwen/settings.json` | `/mcp`; restart after change | A |
| GitHub Copilot CLI | `SKILL.md` + siblings | `~/.copilot/skills`, `~/.agents/skills` | `.github/skills`, `.agents/skills` | `~/.copilot/mcp-config.json` | `.mcp.json`, `.github/mcp.json` | `mcp list/get --json` | A |
| Gemini CLI | `SKILL.md` + siblings | `~/.gemini/skills`, `~/.agents/skills` | `.gemini/skills`, `.agents/skills` | user settings | project settings | CLI list; `/mcp reload` | A |
| Junie | `SKILL.md` + siblings | `~/.junie/skills` | `.junie/skills` | `~/.junie/mcp/mcp.json` | `.junie/mcp/mcp.json` | `/mcp` scoped state | A |
| Kimi Code CLI | directory form + siblings | Kimi and `.agents` roots | Kimi and `.agents` roots | user `mcp.json` | project `mcp.json` | `/mcp`; new-session reload | A |
| OpenCode | `SKILL.md` + siblings | global OpenCode/agent roots | project OpenCode/agent roots | global `opencode.json` | project `opencode.json` | CLI list/debug; config merge | A |
| Kilo Code | `SKILL.md` + siblings | global Kilo/agent roots | project Kilo/agent roots | global `kilo.jsonc` | project `kilo.jsonc` | config state and runtime errors | A |
| Mistral Vibe | `SKILL.md` + siblings | `~/.vibe/skills`, `.agents` | `.vibe/skills`, `.agents` | user `config.toml` | project `config.toml` | `/mcp`; trusted-project gate | A |
| Kiro CLI | `SKILL.md` + siblings | `~/.kiro/skills` | `.kiro/skills` | global MCP JSON | workspace MCP JSON | hot reload; `/mcp` | A |
| Amp | `SKILL.md` + siblings | user agent/Amp roots | `.agents/skills` | user settings | `.amp/settings.json` | `mcp doctor`; workspace trust | A |
| Cursor | `SKILL.md` + resources | supported, exact path not attested | supported, exact path not attested | `~/.cursor/mcp.json` | `.cursor/mcp.json` | CLI MCP list/tools | B |
| Zoo Code | `SKILL.md` + linked files | global `.roo`/`.agents` | project `.roo`/`.agents` | UI-opened `mcp_settings.json` | `.roo/mcp.json` | UI state | B |
| ZCode | directory `SKILL.md` | `~/.zcode/skills` | import target documented | `.zcode` config | workspace `.zcode` config | UI list/enable state | B |
| Pi + adapter | `SKILL.md` + siblings | `.pi`/`.agents` roots | `.pi`/`.agents` roots | adapter global files | adapter project files | package + `/mcp` state | Conditional |
| Goose | complete `.agents/skills` | yes | yes | global extension YAML | recipe-only, explicit run | no ambient project MCP | Exclude |
| Windsurf Cascade | `SKILL.md` + siblings | yes | yes | global MCP JSON | not documented | UI state | Exclude |
| Cline | `SKILL.md` + siblings | yes | yes | global MCP JSON | not documented | JSON CLI observation | Exclude |
| Roo Code | historical support | historical | historical | historical | historical | extension shut down | Exclude |

## Tier A adapter findings

### Factory Droid

Factory already documents complete personal/workspace skills. Its MCP files are
supported at both scopes, project removal is explicitly a file edit, and Droid
automatically reloads changes. [factory-skills]{2} [factory-mcp]{27}

**Adapter posture:** use `.factory/skills` and `mcp.json` as supported write
surfaces for materialized components; use native plugin lifecycle when the same
plugin exists natively; never edit Factory's plugin cache.

### Qwen Code

Qwen has complete user/project skill roots and user/project MCP settings with a
scope-aware CLI. A session restart may be required after changing MCP state.
[qwen-skills]{5} [qwen-mcp]{28}

**Adapter posture:** prefer native Qwen extension conversion where it is
available, otherwise materialize skills and merge `mcpServers` into the scoped
settings file. Report Qwen conversion as a native conversion, not proof that
hooks or agents remain equivalent.

### GitHub Copilot CLI

Copilot exposes complete personal/project skills. [copilot-skills]{9} It merges
user, workspace, repository, and plugin MCP definitions, and its MCP CLI
provides JSON list/get observation. [copilot-mcp]{29}

**Adapter posture:** keep native marketplace/plugin resources native; use skill
roots and repository MCP files only for components lacking a native package.
Enterprise allowlists and repository trust can narrow effective state.

### Gemini CLI

Gemini's skills use complete directories at user/workspace scope. MCP has the
same two scopes, deterministic shell commands, status, and reload. Project
configuration is ignored until the workspace is trusted. [gemini-skills]{15}
[gemini-mcp]{31}

**Adapter posture:** this is a direct materialization target. Treat workspace
trust as health evidence and never claim a configured project MCP is active
until the harness reports or the trusted-session probe observes it.

### Junie

Junie now documents complete user/project skill folders and manually editable
user/project MCP files with scoped runtime state. This reverses the earlier
exclusion, which was based on its interactive extension manager rather than its
component load paths. [junie-skills]{32} [junie-mcp]{33}

**Adapter posture:** manage standalone skills and MCP files directly; retain
native Junie extensions as native when installed. OAuth/secret state remains
outside skilltap inventory.

### Kimi Code CLI

Kimi's plugin installs remain user-only, but its standalone skill and MCP
contracts both support user and project scope. That distinction removes the
prior scope blocker under the revised component-level gate. [kimi-skills]{34}
[kimi-mcp]{35}

**Adapter posture:** materialize skills and MCP independently. Starting a new
session is part of verification. Hooks and plugin session-start behavior are
optional components requiring partial classification.

### OpenCode

OpenCode's complete skills and layered global/project configuration make it a
supported file-managed target even though it lacks the marketplace lifecycle
required by the prior gate. Its MCP state is regular config rather than the Bun
plugin cache. [opencode-skills]{20} [opencode-mcp]{36}

**Adapter posture:** merge the `mcp` object while preserving unknown config;
materialize skills to `.agents/skills` where possible; keep OAuth tokens and
startup dependency caches out of skilltap state.

### Kilo Code

Kilo has complete global/project skills and direct global/project MCP JSONC
files with documented precedence. Marketplace UI is no longer relevant to
admission. [kilo-skills]{22} [kilo-mcp]{39}

**Adapter posture:** write through a comment-preserving JSONC editor or a
documented native command; project trust/auth failures remain observed health,
not configuration drift.

### Mistral Vibe

Vibe's skill roots and its user/project `config.toml` satisfy the component
gate. Its MCP client lacks OAuth, which is a target capability limitation rather
than a reason to exclude all MCP. [mistral-skills]{24} [mistral-mcp]{40}

**Adapter posture:** translate only stdio/static-auth HTTP MCP definitions;
classify OAuth-required MCP as blocked for Vibe and require the normal partial
acknowledgment when the surrounding resource remains useful.

### Kiro CLI

The earlier Kiro exclusion is obsolete for the CLI: current Kiro documentation
defines complete `SKILL.md` directories and user/workspace MCP files with hot
reload and status. [kiro-skills]{41} [kiro-mcp]{42}

**Adapter posture:** target CLI skills and MCP directly; do not translate the
separate IDE Power lifecycle unless a source resource actually requires it.

### Amp

Amp documents whole-directory skills, `.agents/skills`, user/workspace MCP
settings, workspace trust, and `mcp doctor`. Skills may also bundle `mcp.json`,
which is useful when one portable resource owns both instruction and tool
configuration. [amp-manual]{43}

**Adapter posture:** prefer separate managed skills plus scoped settings when
cross-harness portability matters; preserve skill-local MCP only when its
relative paths and lazy-loading semantics are required for faithfulness.

## Tier B: candidate after boundary verification

### Cursor

Cursor officially supports Agent Skills in both editor and CLI and supports
global/project MCP files. The fetched skills source confirms bundled scripts
and resources but does not attest the current filesystem paths. [cursor-skills]{44}
[cursor-mcp]{45}

**Required spike:** in an isolated home/project, create candidate skills in the
currently documented Cursor roots, run Cursor CLI, observe discovery, edit a
sibling file, and verify project/global precedence before granting mutation.

### Zoo Code

Zoo Code, the community successor to Roo, documents complete global/project
skills and global/project MCP. Its global MCP file is opened through the
extension UI but the public page does not give a stable platform-independent
path. [zoocode-skills]{51} [zoocode-mcp]{52}

**Required spike:** resolve the global file through supported extension state,
verify it survives restart and can be observed without VS Code internal-cache
mutation, then record platform paths in a verified profile.

### ZCode

ZCode documents directory skills, global/project import targets, scoped MCP,
copy/symlink import, and per-server state. The English public contract refers
to the chosen-scope `.zcode` configuration without naming the exact native
filename. [zcode-skills]{53} [zcode-mcp]{54}

**Required spike:** identify the native user/workspace files from a clean
installation, verify direct edits are supported and reloaded, and confirm that
symlinked complete skills preserve sibling access.

## Conditional target: Pi

Pi core deliberately ships without MCP. [pi-no-mcp]{30} Its package system and
skill model are otherwise suitable, and the cataloged `pi-mcp-adapter` provides
documented global/project MCP files and observation. [pi-packages]{11}
[pi-skills]{12} [pi-mcp-adapter]{55}

{inferred: conditional admission} Pi can be supported only as a compound
adapter contract: `pi` plus a skilltap-managed, versioned MCP adapter package.
Status must distinguish “Pi reachable” from “Pi MCP capability installed and
healthy.” Removal of the last managed MCP resource may remove the dependency
only when it was installed and remains owned by skilltap.

## Exclusions under the revised gate

### Goose

Goose discovers project/global skills. [goose-skills]{21} Persistent extension
configuration is documented only in the user config. [goose-config]{37}
Project recipes can carry MCP servers, but those servers apply to explicitly
run recipe sessions rather than ordinary sessions opened in that project.
[goose-recipes]{38}

### Windsurf/Devin Desktop Cascade

Cascade's skills satisfy both scopes, but the MCP contract fetched for this
engagement names only the global `mcp_config.json`. A project skill cannot
faithfully stand in for a project MCP server. [windsurf-skills]{46}
[windsurf-mcp]{47}

### Cline

Cline has complete project/global skills and strong user-level MCP observation,
but its current MCP guide names only `~/.cline/mcp.json`. Project-scoped plugin
installation does not establish project-scoped MCP configuration. [cline-skills]{48}
[cline-mcp]{49}

### Roo Code

Roo historically satisfied the component gate, but its official documentation
states that the extension was shut down on May 15, 2026. New support should
target an active successor instead. [roo-shutdown]{50}

## Portability and warning model

{inferred: compatibility model} For every admitted target, skilltap should
classify source components independently:

| Source component | Minimum faithful target | Otherwise |
|---|---|---|
| Skill | Complete directory, entrypoint/frontmatter accepted, siblings reachable, executable permissions preserved | partial or blocked |
| MCP stdio | Command/args/env/cwd and tool filters preserved | partial or blocked |
| MCP HTTP | URL, transport, headers/environment references, auth requirements preserved | partial or blocked |
| Hook | Equivalent lifecycle event and blocking semantics | warn/partial |
| Agent/command | Equivalent invocation, model/tool constraints, and context behavior | warn/partial |
| LSP/app/connector/theme | A target-native semantic equivalent | warn/partial |

Unsupported optional components may proceed only through the product's explicit
partial-acknowledgment contract. Unsupported required components remain blocked.
Secrets are references or native credentials, never copied into skilltap state.

## Implementation order

{inferred: sequencing by unresolved boundary count} A practical delivery order
is:

1. **Gemini CLI, OpenCode, Kiro CLI** — direct scoped files plus documented
   CLI/runtime observation.
2. **Factory Droid, Qwen Code, GitHub Copilot CLI** — direct component paths
   plus native lifecycle coexistence and richer precedence.
3. **Kimi Code CLI, Mistral Vibe, Kilo Code** — direct files with new-session,
   trust, JSONC, or transport limitations to encode.
4. **Junie and Amp** — clear contracts with interactive/trust state that needs
   isolated native validation.
5. **Pi compound adapter**, after dependency ownership is modeled.
6. **Cursor, Zoo Code, and ZCode**, only after their boundary spikes close the
   documented path gaps.

This order is not an effort estimate. It prioritizes adapters whose supported
write and observation boundaries are already explicit in source.

## Adapter acceptance matrix

Every adapter, including conditional adapters, should pass these isolated tests:

| Area | Required acceptance behavior |
|---|---|
| Detection | Known harness version/profile is mutable; unknown version is observe-only |
| Global skill | Install a directory with `SKILL.md`, nested reference, script, and executable bit; harness discovers it |
| Project skill | Same test at project scope; project/global name collision follows documented precedence |
| Whole-directory update | Source revision change updates every owned sibling and preserves modes; repeat is a no-op |
| Skill drift | User edit after apply is reported; foreground sync does not overwrite without the normal drift decision |
| Global MCP | Add stdio and HTTP definitions; unrelated keys and servers survive |
| Project MCP | Add a same-named override; effective precedence matches the harness contract |
| MCP secrets | Environment/secret references survive translation; secret values never enter inventory/state |
| MCP observation | File state and documented list/status evidence agree, or status reports the disagreement |
| Reload | Execute documented hot reload/restart/new-session step and verify tools are visible |
| Trust/policy | Untrusted project or enterprise allowlist produces attention-required health, not false drift |
| Partial plugin | Skill+MCP materialize; unsupported optional hook/agent is itemized; required unsupported component blocks |
| Removal | Remove only owned files/config entries; preserve user/native resources and unknown fields |
| Idempotency | Repeat every mutation immediately and produce no operations |
| Conditional dependency | For Pi, adapter package health is separately observed and ownership-safe |

## Contradictions and qualifications

- **Prior verdict vs revised verdict — reversal.** The earlier exclusions for
  Gemini, Junie, Kimi, OpenCode, Kilo, Vibe, and Kiro were correct under a
  marketplace/lifecycle gate. They do not survive the newly authorized
  skilltap-owned lifecycle model.
- **Pi core vs Pi package — qualifies.** Pi says it has no built-in MCP, while a
  package in the Pi catalog supplies MCP. Both stand: Pi is conditional, not a
  native MCP target. [pi-no-mcp]{30} [pi-mcp-adapter]{55}
- **Kimi plugin scope vs component scope — qualifies.** Plugins are user-only,
  while standalone skills and MCP have project paths. The component adapter is
  eligible; native project plugins are not. [kimi-plugins]{17}
  [kimi-skills]{34} [kimi-mcp]{35}
- **Goose project recipe vs project configuration — qualifies.** Recipes can
  package MCP but only for explicitly launched recipe sessions; that does not
  establish ambient project reconciliation. [goose-recipes]{38}
- **Roo capabilities vs product lifecycle — contradicts as a target decision.**
  Historical capability cannot outweigh the current shutdown notice.
  [roo-shutdown]{50}

## Disconfirming analysis

Before promoting each prior exclusion, the research looked specifically for a
missing project scope, cache-only storage, interactive-only state, retired
product status, or absence of core MCP.

- The search rejected Goose, Windsurf, and Cline because no source-direct
  project MCP write surface was established.
- It downgraded Cursor, Zoo Code, and ZCode because a behavior claim exists but
  an exact current write-path boundary remains insufficiently attested.
- It made Pi conditional because its own product page explicitly disclaims MCP.
- It removed Roo despite historical feature parity because the current product
  is shut down.
- It retained trust, OAuth, enterprise policy, and restart requirements as
  observable health constraints instead of smoothing them into “supported.”

No fetched source disconfirmed the complete skill or two-scope MCP contracts
for the eleven Tier A candidates. Native-version validation is still required
before any adapter gains mutation authority.

## Revisions

- **2026-07-12 — reversal:** supersedes the marketplace/lifecycle-gated report
  with the operator-approved skills-plus-MCP gate. The prior artifact remains
  as the historical decision record.
