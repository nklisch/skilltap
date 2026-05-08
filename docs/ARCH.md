# Architecture

## Overview

skilltap is a CLI tool that installs agent skills and plugins from any git host. It clones repos, scans for SKILL.md files and plugin manifests, runs security checks, and places skills in the universal `.agents/skills/` directory. For plugins, it also injects MCP server configs into agent platform config files and places agent definitions.

This document describes how skilltap is built internally — module boundaries, data flow, key abstractions, and technology decisions.

> **State-store note (v2.1 cutover, Phase 31c-c-2d-1):** The references below to `installed.json` / `plugins.json` describe the v0.x file layout. As of v2.1, the canonical store is a single `state.json` per scope — `~/.config/skilltap/state.json` (global) or `<project>/.agents/state.json` (project). `loadInstalled()` and `loadPlugins()` keep the same internal API but now read from `state.json` first; legacy `installed.json` / `plugins.json` are only read once as a fallback for unmigrated v0.x users. `saveInstalled()` and `savePlugins()` write only to `state.json`. The v0.x Zod schemas remain in `core/src/schemas/{installed,plugins}.ts` because their inner record types (`InstalledSkill`, `PluginRecord`) are reused unchanged in `state.json`'s arrays. The legacy file layout described elsewhere in this doc still accurately describes the on-disk fallback shape and the migrate command's input.

## Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Runtime | Bun | Fast, compiles to single binary (`bun build --compile`), native TypeScript |
| Language | TypeScript | Type safety, matches ecosystem (npm security libs) |
| CLI framework | citty (UnJS) | Declarative, TypeScript-first, tiny, good subcommand support |
| Terminal UI | @clack/prompts | Modern prompts, spinners, select menus. Clean output. |
| Config format | TOML (smol-toml) | Human-friendly, VISION.md spec. smol-toml is small and spec-compliant |
| Validation | Zod 4 | Runtime schema validation for config, tap.json, installed.json, frontmatter, agent responses |
| Git | Shell out to `git` CLI | User's auth (SSH, credential helpers) just works. Zero git library deps |
| Testing | Bun test runner | Built-in, fast, no extra deps |
| Platform | Linux + macOS | Symlinks, XDG paths. Windows later if demand |

### Distribution

1. `bunx skilltap` — for Bun users
2. `npx skilltap` — for Node users (Bun packages work on npm)
3. Standalone binary via `bun build --compile` — no runtime dependency
4. Homebrew: `brew install nklisch/skilltap/skilltap`
5. Install script: `curl -fsSL https://raw.githubusercontent.com/nklisch/skilltap/main/install.sh | sh`

GitHub Actions release workflow (`.github/workflows/release.yml`) builds 4 platform binaries (linux-x64, linux-arm64, darwin-x64, darwin-arm64) on `v*` tag push, attests each binary with `actions/attest-build-provenance`, generates `checksums.txt`, and publishes `skilltap` and `@skilltap/core` to npm with `--provenance`. A `repository_dispatch` event then triggers the Homebrew formula update in `homebrew-skilltap/`.

## Monorepo Structure

Bun workspaces with three packages:

```
skilltap/
├── packages/
│   ├── core/                   # Library — all business logic
│   │   ├── src/
│   │   │   ├── types.ts        # Result<T,E>, ok(), err(), error hierarchy
│   │   │   ├── fs.ts           # Global base path helpers, temp dir management
│   │   │   ├── paths.ts        # skillInstallDir, skillCacheDir, findProjectRoot
│   │   │   ├── git.ts          # Git operations (clone, pull, fetch, diff, diffStat)
│   │   │   ├── scanner.ts      # Skill discovery (find SKILL.md in repos)
│   │   │   ├── frontmatter.ts  # parseSkillFrontmatter() — shared YAML-style frontmatter parser
│   │   │   ├── config.ts       # Config read/write (TOML)
│   │   │   ├── config-keys.ts  # Config get/set helpers (dot-path resolve, coerce, validate)
│   │   │   ├── install.ts      # Install orchestration
│   │   │   ├── remove.ts       # Remove skill logic + removeAnySkill (managed + unmanaged)
│   │   │   ├── update.ts       # Update skill logic (fetch, diff, pull)
│   │   │   ├── discover.ts     # Scan all skill dirs, correlate with installed.json
│   │   │   ├── adopt.ts        # Adopt unmanaged skills (move + symlink or track-in-place)
│   │   │   ├── move.ts         # Move skills between global/project scopes
│   │   │   ├── link.ts         # Link/symlink local skill
│   │   │   ├── taps.ts         # Tap management (add, remove, update, search)
│   │   │   ├── marketplace.ts  # adaptMarketplaceToTap() — marketplace.json → Tap adapter
│   │   │   ├── symlink.ts      # Agent-specific symlink creation
│   │   │   ├── policy.ts       # composePolicy() — config + CLI flag composition
│   │   │   ├── schemas/
│   │   │   │   ├── config.ts   # config.toml Zod schema
│   │   │   │   ├── installed.ts # installed.json Zod schema
│   │   │   │   ├── tap.ts      # tap.json Zod schema (TapSchema, TapSkillSchema, TapPluginSchema)
│   │   │   │   ├── marketplace.ts # marketplace.json Zod schema (Claude Code format)
│   │   │   │   ├── plugin.ts   # PluginManifestSchema + PLUGIN_FORMATS constant
│   │   │   │   ├── plugins.ts  # PluginsJsonSchema, PluginRecordSchema, PluginComponentSchema
│   │   │   │   ├── skill.ts    # SKILL.md frontmatter Zod schema
│   │   │   │   ├── agent.ts    # Agent response + ResolvedSource schemas
│   │   │   │   └── index.ts    # Barrel export
│   │   │   ├── adapters/
│   │   │   │   ├── types.ts    # SourceAdapter interface
│   │   │   │   ├── git.ts      # Git URL adapter
│   │   │   │   ├── github.ts   # GitHub shorthand adapter
│   │   │   │   ├── local.ts    # Local path adapter
│   │   │   │   ├── resolve.ts  # resolveSource() orchestrator
│   │   │   │   └── index.ts    # Barrel export
│   │   │   ├── agents/
│   │   │   │   ├── types.ts    # AgentAdapter interface
│   │   │   │   ├── detect.ts   # Auto-detect installed agents, resolveAgent()
│   │   │   │   ├── adapters.ts # All CLI adapters (claude, gemini, codex, opencode)
│   │   │   │   ├── factory.ts  # createCliAdapter() shared factory
│   │   │   │   ├── ollama.ts   # Ollama adapter (local models)
│   │   │   │   ├── custom.ts   # Custom binary adapter
│   │   │   │   ├── extract.ts  # extractAgentResponse() JSON pipeline
│   │   │   │   └── index.ts    # Barrel export
│   │   │   ├── security/
│   │   │   │   ├── patterns.ts # 7 detection functions (Unicode, URLs, etc.)
│   │   │   │   ├── static.ts   # Layer 1 — scanStatic(), scanDiff()
│   │   │   │   ├── semantic.ts # Layer 2 — scanSemantic(), chunking
│   │   │   │   └── index.ts    # Barrel export
│   │   │   ├── npm-registry.ts # npm registry API client (fetch metadata, tarball, search)
│   │   │   ├── validate.ts     # validateSkill() — SKILL.md validation for create/verify
│   │   │   ├── doctor.ts       # runDoctor() — environment diagnostics, --fix support
│   │   │   ├── trust/
│   │   │   │   ├── types.ts    # TrustInfo schema (tier, npm, github, publisher, tap)
│   │   │   │   ├── verify-npm.ts  # Sigstore/SLSA attestation verification
│   │   │   │   ├── verify-github.ts # GitHub attestation via `gh attestation verify`
│   │   │   │   ├── resolve.ts  # resolveTrust() — compute tier from available signals
│   │   │   │   └── index.ts
│   │   │   │                          # (registry/ — HTTP registry client — removed in Phase 31b)
│   │   │   ├── json-state.ts          # loadJsonState()/saveJsonState() — generic JSON file I/O
│   │   │   ├── plugin/                # Plugin detection, parsing, and MCP injection
│   │   │   │   ├── detect.ts          # detectPlugin(dir) — find and parse plugin manifest
│   │   │   │   ├── parse-claude.ts    # Claude Code .claude-plugin/plugin.json parser
│   │   │   │   ├── parse-codex.ts     # Codex .codex-plugin/plugin.json parser
│   │   │   │   ├── parse-common.ts    # discoverSkills() — shared skill discovery for both parsers
│   │   │   │   ├── mcp.ts             # MCP config normalization from .mcp.json
│   │   │   │   ├── mcp-inject.ts      # MCP_AGENT_CONFIGS registry + inject/remove/list functions
│   │   │   │   ├── agents.ts          # Agent definition (.md) reader
│   │   │   │   ├── install.ts         # Plugin install orchestration (installPlugin)
│   │   │   │   ├── lifecycle.ts       # removeInstalledPlugin(), toggleInstalledComponent()
│   │   │   │   ├── state.ts           # plugins.json load/save/modify + mcpServerToStored()
│   │   │   │   └── index.ts
│   │   │   ├── templates/
│   │   │   │   ├── basic.ts    # basicTemplate() — standalone git repo
│   │   │   │   ├── npm.ts      # npmTemplate() — npm package with provenance
│   │   │   │   ├── multi.ts    # multiTemplate() — multiple skills in one repo
│   │   │   │   └── index.ts
│   │   │   └── index.ts        # Package barrel export
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── cli/                    # CLI entry point — commands and UI
│   │   ├── src/
│   │   │   ├── index.ts        # Entry point (runMain)
│   │   │   ├── commands/
│   │   │   │   ├── install.ts
│   │   │   │   ├── update.ts
│   │   │   │   ├── find.ts
│   │   │   │   ├── create.ts         # skilltap create — scaffold new skills
│   │   │   │   ├── verify.ts         # skilltap verify — validate skills before sharing
│   │   │   │   ├── doctor.ts         # skilltap doctor — environment diagnostics
│   │   │   │   ├── completions.ts    # skilltap completions — shell tab-completion
│   │   │   │   ├── config.ts         # Routes to config/index.ts
│   │   │   │   ├── config/
│   │   │   │   │   ├── index.ts      # skilltap config wizard (was config.ts)
│   │   │   │   │   ├── agent-mode.ts # skilltap config agent-mode wizard
│   │   │   │   │   ├── get.ts        # skilltap config get — read config values
│   │   │   │   │   └── set.ts        # skilltap config set — write config values
│   │   │   │   ├── skills/
│   │   │   │   │   ├── index.ts      # skilltap skills — unified skill view
│   │   │   │   │   ├── adopt.ts      # skilltap skills adopt — adopt unmanaged skills
│   │   │   │   │   ├── move.ts       # skilltap skills move — move between scopes
│   │   │   │   │   ├── remove.ts     # skilltap skills remove — remove any skill
│   │   │   │   │   ├── info.ts       # skilltap skills info — show skill details
│   │   │   │   │   ├── link.ts       # skilltap skills link — symlink local skill
│   │   │   │   │   └── unlink.ts     # skilltap skills unlink — remove linked skill
│   │   │   │   ├── plugin/
│   │   │   │   │   ├── index.ts      # skilltap plugin — list installed plugins
│   │   │   │   │   ├── info.ts       # skilltap plugin info — plugin details + components
│   │   │   │   │   ├── toggle.ts     # skilltap plugin toggle — enable/disable components
│   │   │   │   │   └── remove.ts     # skilltap plugin remove — remove plugin + all components
│   │   │   │   └── tap/
│   │   │   │       ├── add.ts
│   │   │   │       ├── remove.ts
│   │   │   │       ├── list.ts
│   │   │   │       ├── update.ts
│   │   │   │       └── init.ts
│   │   │   ├── completions/
│   │   │   │   └── generate.ts       # generateCompletions(shell) — bash/zsh/fish scripts
│   │   │   └── ui/
│   │   │       ├── format.ts        # Output formatting (tables, colors, ansi)
│   │   │       ├── agent-out.ts     # Agent mode plain text output
│   │   │       ├── prompts.ts       # @clack/prompts wrappers
│   │   │       ├── scan.ts          # Security scan result display
│   │   │       ├── trust.ts         # Trust tier display helpers
│   │   │       ├── policy.ts        # loadPolicyOrExit() — CLI adapter for composePolicy
│   │   │       ├── plugin-format.ts # componentSummary() — plugin component display helpers
│   │   │       └── resolve.ts       # resolveScope, parseAlsoFlag, resolveAgent helpers
│   │   ├── package.json        # Published as "skilltap" on npm
│   │   └── tsconfig.json
│   └── test-utils/             # Shared test fixtures and helpers
│       ├── src/
│       │   ├── fixtures.ts     # Create mock repos, skills, taps, plugins (createTapWithPlugins)
│       │   ├── env.ts          # createTestEnv() + pathExists() — isolated test environment setup
│       │   ├── git.ts          # Test git helpers (init, commit)
│       │   └── tmp.ts          # Temp directory management
│       ├── fixtures/
│       │   ├── standalone-skill/
│       │   │   └── SKILL.md
│       │   ├── multi-skill-repo/
│       │   │   └── .agents/skills/
│       │   │       ├── skill-a/SKILL.md
│       │   │       └── skill-b/SKILL.md
│       │   ├── malicious-skill/
│       │   │   └── SKILL.md    # Contains known-bad patterns
│       │   └── sample-tap/
│       │       └── tap.json
│       ├── package.json        # Private, not published
│       └── tsconfig.json
├── package.json                # Workspace root
├── bunfig.toml
├── tsconfig.json               # Base TypeScript config
├── VISION.md
├── ARCH.md
├── SPEC.md
└── UX.md
```

### Package Dependencies

```
cli → core
cli → test-utils (dev)
core → test-utils (dev)
```

`core` has zero runtime dependency on `cli`. This makes `@skilltap/core` embeddable in other tools (editors, other CLIs, CI systems).

### Package Names

| Package | npm name | Published |
|---------|----------|-----------|
| core | `@skilltap/core` | Yes |
| cli | `skilltap` | Yes (main entry) |
| test-utils | `@skilltap/test-utils` | No (private) |

## Module Architecture

### Core Modules

**git.ts** — Thin wrapper around the `git` CLI. All git operations go through here.
- `clone(url, dest, opts)` → `Result<CloneResult, GitError>` — shallow clone with automatic HTTPS↔SSH protocol fallback on auth failure. Returns `{ effectiveUrl }` so callers can persist the URL that actually worked.
- `flipUrlProtocol(url)` — converts between HTTPS and SSH git URL forms (e.g. `https://github.com/o/r.git` ↔ `git@github.com:o/r.git`). Returns `null` for unrecognized patterns.
- `pull(dir)` — `git pull`
- `fetch(dir)` — `git fetch`
- `diff(dir, from, to)` — `git diff from..to`
- `revParse(dir)` — current HEAD SHA
- `log(dir, n)` — last n commits

**scanner.ts** — Finds SKILL.md files in a directory tree. Returns structured results with name, description (from frontmatter), and path. See [SPEC.md — Skill Discovery](./SPEC.md#skill-discovery) for the scanning algorithm.

**frontmatter.ts** — `parseSkillFrontmatter(content)` parses YAML-style `---` frontmatter blocks into a plain object. Shared by scanner.ts and validate.ts.

**security/static.ts** — Layer 1 pattern matching. Takes file contents, returns warnings with line numbers, category, and raw/visible text. Uses `anti-trojan-source` and `out-of-character` for Unicode detection, regex for everything else. See [SPEC.md — Layer 1](./SPEC.md#layer-1-static-analysis) for detection categories.

**security/semantic.ts** — Layer 2 agent-based evaluation. Chunks content, invokes agent adapter, aggregates scores. See [SPEC.md — Layer 2](./SPEC.md#layer-2-semantic-scan) for the chunking algorithm and security prompt.

**config.ts** — Reads/writes `~/.config/skilltap/config.toml` and the canonical state at `~/.config/skilltap/state.json` (since v2.1; `installed.json` and `plugins.json` are read transparently as a one-time fallback for unmigrated v0.x users and never written). Ensures directories exist on first use.

**config-keys.ts** — Pure helpers for `config get`/`config set`: dot-path resolution, value coercion (string→typed), settable key allowlist/blocklist, immutable deep-set, plain-text formatting.

**install.ts** — Orchestrates the install flow. Coordinates git, scanner, security, config, and symlink modules. **remove.ts**, **update.ts**, and **link.ts** handle their respective flows.

**discover.ts** — `discoverSkills(options?)` scans all skill directories (`.agents/skills/` and every agent-specific dir from `AGENT_PATHS`) at both global and project scope. Detects symlinks, cross-references with the loaded skill records (from `state.json`, with v0.x fallback) to classify skills as managed or unmanaged, reads SKILL.md frontmatter for descriptions, and detects git remotes on unmanaged skills. Returns `DiscoverResult` with a unified skill inventory.

**adopt.ts** — `adoptSkill(skill, options?)` brings an unmanaged `DiscoveredSkill` under skilltap management. Two modes: `move` (default) moves the skill dir to `.agents/skills/` and creates symlinks from original locations, `track-in-place` creates a "linked" record without moving. Runs static security scan, detects git remote/ref/sha, writes to `state.json`.

**move.ts** — `moveSkill(name, options)` moves a managed skill between scopes (global ↔ project). Handles symlink cleanup and recreation, `state.json` record transfer across scopes, and linked→managed conversion.

**skill-check.ts** — Background skill update check. `checkForSkillUpdates(intervalHours, projectRoot)` reads the cache and fires a background refresh if stale. `fetchSkillUpdateStatus(projectRoot)` does the actual network check: groups git skills by cache dir (one `git fetch` per unique repo), compares `HEAD` vs `FETCH_HEAD`; fetches npm metadata for npm skills and compares versions. `writeSkillUpdateCache(updates, projectRoot)` persists results to `~/.config/skilltap/skills-update-check.json`.

**taps.ts** — Manages tap repos. Clone, pull, parse tap index (`tap.json` or `.claude-plugin/marketplace.json`), search across taps. Supports git-cloned taps and Claude Code marketplace repos (marketplace.json adapted to Tap via `marketplace.ts`). HTTP registry taps were retired in Phase 31b — v0.x configs with `type = "http"` are silently filtered with a one-time stderr warning. `loadTaps()` returns entries for both `skills` and `plugins` arrays from tap.json. `tapPluginToManifest(plugin, tapDir)` converts a `TapPlugin` entry to a `PluginManifest` for use with `installPlugin()`.

**marketplace.ts** — Adapts Claude Code `marketplace.json` to skilltap's internal `Tap` type. `adaptMarketplaceToTap(marketplace, tapUrl, tapDir?)` is async: for relative-path sources in a local tap directory, it auto-detects `.claude-plugin/plugin.json` via `detectPlugin()` and produces `TapPlugin` entries (with full skill/MCP/agent components) when a plugin manifest is found. Otherwise produces `TapSkill` entries with `plugin: true` flag. Non-relative sources (github, npm, url, git-subdir) always produce `TapSkill` entries. Plugin-only features (LSP, hooks, commands) are silently ignored.

**symlink.ts** — Creates and removes symlinks for agent-specific directories. Knows the path conventions for each supported agent. Idempotent — gracefully replaces stale symlinks and leftover real directories instead of failing on EEXIST.

**npm-registry.ts** — npm registry API client. `parseNpmSource()`, `fetchPackageMetadata()`, `resolveVersion()`, `downloadAndExtract()`. Private registry support via `NPM_CONFIG_REGISTRY` env, `.npmrc`, or `~/.npmrc`.

**skills-registry.ts** — Extensible skill registry system. `SkillRegistry` interface with `{ name, search(query, limit) }`. Built-in: `skillsShRegistry` (skills.sh). `createCustomRegistry(name, url)` factory for any URL implementing the search API. `resolveRegistries(config)` reads `[registry].enabled` + `[[registry.sources]]` and returns active registries. `searchRegistries(query, registries, limit?)` queries all in parallel, tagging results with `registryName`.

**validate.ts** — `validateSkill(dir)` → `Result<ValidationResult, UserError>`. Checks SKILL.md exists, frontmatter valid, name matches directory, static security scan, and size limit. Used by `skilltap verify` and as a post-scaffold check in `skilltap create`.

**doctor.ts** — `runDoctor({ fix?, onCheck? })` → `DoctorResult`. Runs 9 check functions serially, streaming results via the `onCheck` callback. Supports `--fix` for safe auto-repairs (missing dirs, broken symlinks, orphan records, missing taps).

**trust/** — Trust tier resolution pipeline. `resolveTrust()` computes tier from npm attestation (`verify-npm.ts` via `sigstore`), GitHub attestation (`verify-github.ts` via `gh` CLI), and tap metadata. Injectable verify functions for testing. Injected into install/update flows as an optional post-download step.

**templates/** — TypeScript functions generating `Record<string, string>` (relPath → content). Embedded in the compiled binary (no runtime file reads). Three templates: `basicTemplate()`, `npmTemplate()`, `multiTemplate()`.

> **registry/** module (v0.2 HTTP registry client) was removed in Phase 31b — the directory and its `fetchRegistryList()` / `fetchRegistryDetail()` helpers no longer exist. See the v2.0 changes section below for context.

### Plugin Modules

**plugin/detect.ts** — `detectPlugin(dir)` → `Result<PluginManifest | null, ...>`. Checks for `.claude-plugin/plugin.json` first, then `.codex-plugin/plugin.json`. Returns a normalized manifest with component list, or `null` if not a plugin.

**plugin/parse-claude.ts** — Parses Claude Code `.claude-plugin/plugin.json`. Extracts skill paths (from `skills` field or default `skills/` directory), MCP server configs (from `mcpServers` field or `.mcp.json`), and agent definitions (from `agents` field or `agents/` directory). Handles both path override and auto-discovery modes.

**plugin/parse-codex.ts** — Parses Codex `.codex-plugin/plugin.json`. Extracts skill paths and MCP server configs. Codex plugins don't have agent definitions.

**plugin/parse-common.ts** — `discoverSkills(dir)` shared skill discovery helper used by both Claude Code and Codex parsers.

**plugin/mcp.ts** — `parseMcpConfig(path)` → `McpServerConfig[]`. Reads `.mcp.json` files and normalizes server entries into `{ name, command, args, env }`. Handles both Claude Code and Codex MCP formats (they're compatible).

**plugin/mcp-inject.ts** — Data-driven MCP injection. `MCP_AGENT_CONFIGS` registry maps agent names to config file paths (5 agents: claude-code, cursor, codex, gemini, windsurf). `injectMcpServers()`, `removeMcpServers()`, `listMcpServers()`. Server names namespaced via `SKILLTAP_MCP_PREFIX` (`skilltap:`). All writes create a `.skilltap.bak` backup before first modification.

**plugin/agents.ts** — `parseAgentDefinitions(dir)` → `AgentDefinition[]`. Reads `agents/*.md` files, parses frontmatter (model, effort, maxTurns, tools, isolation) and body content. Claude Code-only for now.

**plugin/install.ts** — `installPlugin()` — plugin install orchestration. Coordinates skill extraction (delegates to existing `install.ts`), MCP injection (via `mcp-inject.ts`), and agent placement. Produces a `PluginInstallResult` with the full component inventory and `PluginRecord`.

**plugin/lifecycle.ts** — `removeInstalledPlugin()` and `toggleInstalledComponent()` — post-install plugin lifecycle. Remove cleans up all skills, MCP entries, and agent definitions. Toggle enables/disables individual components by type (skill → `.disabled/`, MCP → agent config, agent → `.disabled/`).

**plugin/state.ts** — Plugin state management. `loadPlugins(scope)`, `savePlugins(scope, data)`, `addPlugin(record)`, `removePlugin(name)`, `toggleComponent(pluginName, componentName)`, `mcpServerToStored()`. Reads/writes the `plugins[]` slice of `state.json` (with v0.x `plugins.json` fallback at load time only).

**json-state.ts** — Generic JSON file I/O. `loadJsonState(path, schema)` and `saveJsonState(path, data)`. Shared by `config.ts`, `plugin/state.ts`, and any other module that needs validated JSON read/write.

**paths.ts** additions — `scopeBase(scope, projectRoot?)` replaces inline ternaries; `agentDefPath(scope, platform, name, projectRoot?)` and `agentDefDisabledPath()` compute agent definition placement paths using `AGENT_DEF_PATHS` from `symlink.ts`.

### Schemas (Zod 4)

All data boundaries are validated with Zod 4 schemas. Types are inferred from schemas — no separate interface definitions. Schema files live in `packages/core/src/schemas/`.

```typescript
import { z } from 'zod/v4'

// --- Data schemas (parsed from files/responses) ---

export const ResolvedSourceSchema = z.object({
  url: z.string(),
  ref: z.string().optional(),
  adapter: z.string(),
})

export const SecurityConfigSchema = z.object({
  scan: z.enum(['static', 'semantic', 'off']).default('static'),
  on_warn: z.enum(['prompt', 'fail']).default('prompt'),
  require_scan: z.boolean().default(false),
  agent: z.string().default(''),
  threshold: z.number().int().min(0).max(10).default(5),
  max_size: z.number().int().default(51200),
  ollama_model: z.string().default(''),
})

export const AgentModeSchema = z.object({
  enabled: z.boolean().default(false),
  scope: z.enum(['global', 'project']).default('project'),
})

export const ConfigSchema = z.object({
  defaults: z.object({
    also: z.array(z.string()).default([]),
    yes: z.boolean().default(false),
    scope: z.enum(['global', 'project', '']).default(''),
  // .prefault({}) passes {} through the schema (applying nested defaults).
  // Zod 4's .default({}) short-circuits without parsing, so nested defaults won't apply.
  }).prefault({}),
  security: SecurityConfigSchema.prefault({}),
  'agent-mode': AgentModeSchema.prefault({}),
  taps: z.array(z.object({
    name: z.string(),
    url: z.string(),
  })).default([]),
})

// Types inferred from schemas
export type ResolvedSource = z.infer<typeof ResolvedSourceSchema>
export type Config = z.infer<typeof ConfigSchema>
// ... etc
```

Additional schemas defined in SPEC.md:
- [installed.json](./SPEC.md#installedjson) — `InstalledJsonSchema`, `InstalledSkillSchema`
- [tap.json](./SPEC.md#tapjson) — `TapSchema`, `TapSkillSchema`, `TapPluginSchema` (with inline skills, mcpServers, agents)
- [marketplace.json](./SPEC.md#marketplacejson) — `MarketplaceSchema` (Claude Code format, adapted to `Tap`)
- [plugins.json](./SPEC.md#pluginsjson) — `PluginsJsonSchema`, `PluginRecordSchema`, `PluginComponentSchema`
- [Plugin manifest](./SPEC.md#plugin-manifest) — `PluginManifestSchema` (unified internal representation); `PLUGIN_FORMATS = ["claude-code", "codex", "skilltap"]`
- [MCP config](./SPEC.md#mcp-config) — `McpServerConfigSchema` (normalized MCP server entry)
- [SKILL.md frontmatter](./SPEC.md#skillmd-parsing) — `SkillFrontmatterSchema`
- [Agent response](./SPEC.md#json-extraction) — `AgentResponseSchema`

Zod validates at every data boundary: parsing TOML config, reading installed.json, parsing tap.json, parsing marketplace.json (Claude Code format), extracting SKILL.md frontmatter, and parsing agent CLI output. Adapter return values are validated before entering core logic.

### Adapter Interfaces

Adapters use standard TypeScript interfaces (not Zod) since they define behavior, not data:

```typescript
// Source adapter — resolves a user-provided source to a cloneable URL
interface SourceAdapter {
  name: string;
  canHandle(source: string): boolean;
  resolve(source: string): Promise<Result<ResolvedSource, UserError>>;
}

// Agent adapter — invokes an LLM for semantic scanning
interface AgentAdapter {
  name: string;
  cliName: string;   // binary name on PATH
  detect(): Promise<boolean>;
  invoke(prompt: string): Promise<Result<AgentResponse, ScanError>>;
}
```

### Source Adapters

| Adapter | Handles | Resolution |
|---------|---------|------------|
| git | `https://`, `git@`, `ssh://` URLs | Pass-through (already a git URL) |
| npm | `npm:@scope/name[@version]` | Fetch tarball from npm registry, verify SHA-512 integrity |
| github | `github:owner/repo`, `owner/repo` shorthand | Resolve to `https://github.com/owner/repo.git` |
| local | Filesystem paths (`./`, `/`, `~/`) | Validate path exists, has SKILL.md |

### Agent Adapters (v0.1)

| Agent | Binary | Invocation |
|-------|--------|------------|
| Claude Code | `claude` | `claude --print -p '<prompt>' --no-tools --output-format json` |
| Gemini CLI | `gemini` | `echo '<prompt>' \| gemini --non-interactive` |
| Codex CLI | `codex` | `codex --prompt '<prompt>' --no-tools` |
| OpenCode | `opencode` | `opencode --prompt '<prompt>'` |
| Ollama | `ollama` | `ollama run <model> '<prompt>'` |

See [SPEC.md — Agent Adapters](./SPEC.md#agent-adapters) for detection logic, first-use selection flow, JSON extraction, and custom binary support.

## Key Flows

These flows show how modules coordinate. See [SPEC.md](./SPEC.md#cli-commands) for the precise behavioral spec (flags, prompts, exit codes).

### Install from URL

```
1. Parse source → select SourceAdapter (git)
2. Resolve → { url, ref }
3. Clone to temp dir (/tmp/skilltap-{random}/)
4. Scan for SKILL.md files (scanner)
   - Deep scan: prompt user if non-standard paths found (onDeepScan callback)
5. Skill selection (single → auto, multiple → onSelectSkills callback)
6. Security scan (static.ts, optionally semantic.ts)
   - onWarnings / onSemanticWarnings callbacks for per-skill UI decisions
7. Clean-install confirmation (onConfirmInstall callback, skipped with --yes)
8. Resolve trust tier (trust/)
9. Move skill directory to install path
   - Standalone repo → move entire temp clone
   - Multi-skill repo → copy skill dir, cache repo clone
10. Update installed.json (config.ts)
11. Create agent symlinks if --also (symlink.ts)
12. Clean up temp dir
```

### Install from Tap Name

```
1. Load all taps, parse tap index — tap.json or marketplace.json (taps.ts, marketplace.ts)
2. Search for name across all taps
3. Resolve to repo URL (single match → use, multiple → prompt)
4. → Continue from step 2 of "Install from URL"
```

### Install Plugin (from URL/git)

```
1. Parse source → select SourceAdapter → resolve → clone to temp dir
2. Run plugin detection: check for .claude-plugin/plugin.json, then .codex-plugin/plugin.json
3. If plugin detected: parse manifest, extract component list (skills, MCP servers, agents)
4. If not a plugin: fall back to standard skill install flow
5. onPluginDetected callback: prompt "Install as plugin? (Y/n)" (auto-accept with --yes)
6. Scope resolution (same as skill install: --project/--global/prompt)
7. Security scan all plugin content (skills + agent .md files + MCP commands)
8. For each skill: install via existing skill machinery (place in .agents/skills/, symlink)
9. For each MCP server: inject into target agent configs (mcp-inject.ts)
   - Namespace: skilltap:<plugin-name>:<server-name>
   - Backup agent config before first write (.skilltap.bak)
10. For each agent definition: place .md in .claude/agents/ (Claude Code only)
    - agentDefPath() from paths.ts determines target path
11. Record plugin in plugins.json with all components (active: true)
12. Clean up temp dir
```

### Install Tap Plugin (tap-name/plugin-name)

```
1. parseTapPluginRef() detects "tap-name/plugin-name" pattern
2. loadTaps() → find entry where tapName + tapPlugin.name match
3. tapPluginToManifest(tapPlugin, tapDir) → PluginManifest
4. onPluginDetected callback (same as above)
5. installPlugin() with tapDir as source (no git clone needed — already on disk)
6. Record in plugins.json with tap reference
```

### Plugin Toggle

```
1. Load plugins.json, find plugin by name
2. Show interactive component picker (checkboxes, grouped by type)
3. For toggled skills: move to/from .disabled/ (existing mechanism)
4. For toggled MCP servers: add/remove entries from agent config files
5. For toggled agents: move .md to/from .disabled/ subdirectory
6. Update component active state in plugins.json
```

### Update

```
1. Look up in installed.json → get repo URL, current SHA (or npm version)
2. git fetch (git.ts) or npm registry check (npm-registry.ts)
3. Compare HEAD SHA to FETCH_HEAD (git) or installed version to latest (npm)
4. If different: show diff summary (onDiff callback)
5. Scan diff (static.ts) → onShowWarnings callback
6. Confirm update (onConfirm callback) or skip on --strict
7. git pull (or tarball replace for npm)
8. Optionally run semantic scan on updated directory (semantic.ts)
9. Re-create agent symlinks
10. Re-resolve trust tier (trust/)
11. Update installed.json with new SHA / version / updatedAt
```

## Storage Layout

```
~/.config/skilltap/
├── config.toml                  # User configuration
├── installed.json               # Installation state — skills (machine-managed)
├── plugins.json                 # Installation state — plugins (machine-managed)
├── taps/
│   ├── home/                    # Cloned tap repo (tap.json format)
│   │   ├── tap.json
│   │   └── .git/
│   ├── community/               # Another tap repo (tap.json format)
│   │   ├── tap.json
│   │   └── .git/
│   └── anthropic-skills/        # Marketplace repo (marketplace.json format)
│       ├── .claude-plugin/
│       │   └── marketplace.json
│       └── .git/
└── cache/
    └── {repo-url-hash}/        # Cached full clones for multi-skill repos / plugins
        ├── .git/
        ├── .agents/skills/
        │   ├── skill-a/
        │   └── skill-b/
        └── ...

~/.agents/skills/                # Global install directory (canonical)
├── commit-helper/               # Standalone — this IS the git clone
│   ├── SKILL.md
│   ├── .git/
│   └── scripts/
├── termtube-dev/                # Copied from multi-skill repo
│   └── SKILL.md
└── termtube-review/
    └── SKILL.md

~/.claude/skills/                # Agent-specific (symlinks)
├── commit-helper -> ~/.agents/skills/commit-helper/
└── termtube-dev -> ~/.agents/skills/termtube-dev/

~/.claude/agents/                # Agent definitions (plugin-installed, Claude Code only)
└── code-review.md               # From a plugin's agents/ directory

~/.claude/settings.json          # Agent config (MCP entries injected by skilltap)
  # "mcpServers": { "skilltap:my-plugin:db": { "command": "...", "args": [...] } }

.agents/skills/                  # Project-scoped (same structure)
└── project-skill/
    └── SKILL.md

.agents/plugins.json             # Project-scoped plugin state
```

See [SPEC.md — Installation Paths](./SPEC.md#installation-paths) for the full path table and symlink agent identifiers.

### Standalone vs Multi-Skill Repos

**Standalone repos** (repo root has SKILL.md): The cloned repo IS the installed skill. Git history preserved. `skilltap update` runs `git pull` directly.

**Multi-skill repos** (repo contains skills in subdirectories): The full repo is cloned to the cache dir (`~/.config/skilltap/cache/{hash}/`). Selected skill directories are **copied** to the install path. Updates pull the cache repo and re-copy.

Why copy instead of symlink for multi-skill? Symlinks would break if the cache is cleaned. The cache is a performance optimization (avoids re-cloning on update), not a dependency. If the cache is missing, skilltap re-clones.

## Error Handling

Core functions return typed results, not thrown exceptions:

```typescript
type Result<T, E = Error> =
  | { ok: true; value: T }
  | { ok: false; error: E }
```

Error categories:
- **UserError** — Bad input, skill not found, invalid config. Show message, exit 1.
- **GitError** — Clone/pull failed, auth error, repo not found. Show git stderr, exit 1.
- **ScanError** — Security scan couldn't complete (agent not found, parse failure). Show details, offer to skip.
- **NetworkError** — Can't reach host. Show URL, suggest checking connection.

The CLI layer (`packages/cli`) catches results and formats them for terminal display. The core library never writes to stdout/stderr directly.

See [SPEC.md — Error Handling](./SPEC.md#error-handling) for exit codes, error message format, and the full error condition table.

## Testing Strategy

**Unit tests** — Pure functions: scanner, security patterns, config parsing, TOML schema validation. Fast, no I/O.

**Integration tests** — Git operations with real repos (test fixtures initialized via `test-utils`). Tap resolution, multi-skill scanning, symlink creation.

**CLI tests** — Full subprocess tests via `Bun.spawn` with `SKILLTAP_HOME`/`XDG_CONFIG_HOME` env vars. Tests run the actual CLI binary end-to-end.

**Security scanner tests** — Known-malicious patterns from the SkillJect research and ClawHavoc incident. Regression suite to ensure detection of:
- Invisible Unicode in SKILL.md
- Hidden HTML comments with instructions
- Base64-encoded shell commands
- Tag injection attempts
- Suspicious URLs (exfiltration services)

All tests run with `bun test`. CI runs on Linux and macOS.

## Decision Log

| Decision | Choice | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Runtime | Bun | Node.js, Deno | Single binary compilation, fast, native TS |
| CLI framework | citty | commander.js, cac, clipanion | TypeScript-first, declarative, UnJS ecosystem |
| Terminal UI | @clack/prompts | inquirer, prompts, hand-rolled | Modern, beautiful output, maintained |
| Git interaction | Shell out | isomorphic-git | Auth inherited, simpler, no library edge cases |
| Config format | TOML | JSON, YAML | Human-friendly editing, clear sections |
| TOML parser | smol-toml | @iarna/toml, toml-eslint-parser | Small, spec-compliant, works with Bun |
| Validation | Zod 4 | io-ts, arktype, manual validation | Industry standard, infer types from schemas, great errors |
| Project structure | Monorepo | Single package | Core embeddable separately, clean test isolation |
| Security Unicode | anti-trojan-source + out-of-character | Custom regex | Battle-tested, maintained, cover edge cases |
| Semantic scan | Shell out to agent CLI | Direct API calls | Zero API key requirement, works with user's existing setup |
| Agent detection | Auto-detect on PATH | Manual config only | Zero-config experience, user can override |
| Multi-skill install | Copy to install dir + cache repo | Symlink from cache | Cache is optimization not dependency; copy survives cache clean |
| npm provenance | sigstore-js with bun patches | Direct Sigstore API | Reuse existing Sigstore ecosystem; two `bun patch` fixes for BoringSSL compat |
| Trust tier storage | Optional field in installed.json | Separate trust file | Simplest structure; trust is per-install, not per-skill globally |
| Template format | TypeScript functions returning Record<string,string> | File system templates | Binary embeddable; no runtime file reads; type-safe; easily testable |
| Doctor checks | 9 sequential checks with onCheck callback | Parallel checks | Streaming output UX; failures in one check don't block others |
| Platform | Linux + macOS | Cross-platform | Ship fast, add Windows when demand exists |
| Plugin state | Separate plugins.json | Extend installed.json with type field | Clean separation, no migration, independent schemas |
| Plugin scope | Portable subset (skills + MCP + agents) | Full plugin support (hooks, LSP, etc.) | Portable components work across agents; platform-specific features are low value for cross-agent tool |
| MCP injection | Direct config write with backup | Generate snippets for user to copy | Best UX; backup + doctor checks provide safety net |
| MCP namespacing | `skilltap:<plugin>:<server>` prefix | No prefix | Prevents collisions with user-configured MCP servers |
| Agent definitions | Claude Code only (for now) | All agents | Only Claude Code has a documented agent definition format; extensible later |
| Plugin detection | Auto-detect in install flow | Separate `plugin install` command | One command for everything; plugin vs. skill is a property of the source, not the user's intent |

---

## v2.0 Architecture Additions

This section documents the architecture changes introduced by the v2.0 redesign. The v0.1–v1.0 architecture above remains the foundation; the additions below describe new modules, modified data flow, and removed components. See [VISION.md — v2.0](./VISION.md#v20-direction-simplification-unification-project-manifest) and [SPEC.md — v2.0](./SPEC.md#v20--tooling-surface-redesign) for behavior.

### New Core Modules

```
packages/core/src/
├── manifest/                     # NEW — project manifest + lockfile
│   ├── schemas.ts                # ProjectManifestSchema, PluginManifestV2Schema, LockfileSchema
│   ├── load.ts                   # loadManifest(projectRoot), loadLockfile(projectRoot)
│   ├── save.ts                   # saveManifest, saveLockfile (atomic write + ensureDirs)
│   ├── resolve.ts                # resolveDeps(manifest, lockfile) → ResolvedDeps[]
│   ├── range.ts                  # Range parsing/matching (^, ~, *, exact ref)
│   └── publish.ts                # discoverPublishablePlugins(repoRoot) → PluginManifestV2[]
├── sync/                         # NEW — Cargo-style reconcile engine
│   ├── plan.ts                   # planSync(manifest, lockfile, state) → SyncPlan with adds/removes/updates
│   ├── apply.ts                  # applySync(plan, options, callbacks) — runs install/remove/update
│   └── drift.ts                  # detectDrift(state, manifest, lockfile) → DriftReport
├── state/                        # NEW — replaces installed.ts/plugins/state.ts file split
│   ├── schema.ts                 # StateSchema { version: 2, skills: [], plugins: [], mcpServers: [] }
│   ├── load.ts                   # loadState(scope, projectRoot?) → State
│   ├── save.ts                   # saveState(state, scope, projectRoot?)
│   └── migrate-v1.ts             # detect + read installed.json + plugins.json, write state.json
├── agent-flag/                   # NEW — replaces agent-mode logic across the codebase
│   ├── resolve.ts                # resolveAgentFlag({ flag, env, config }) → AgentEffective
│   └── enforce.ts                # enforceBlock(config, requested) → Result<void, UserError>
├── policy/                       # SIMPLIFIED — collapses old per-mode policy
│   └── compose.ts                # composePolicy(config, flags, source) → EffectivePolicy
│                                 # No more human/agent split. trust-list short-circuits scanning.
├── plugin-v2/                    # NEW — native skilltap plugin format reader/writer
│   ├── parse-toml.ts             # parse .skilltap/<name>.toml
│   ├── discover.ts               # find all .skilltap/*.toml in a repo
│   └── normalize.ts              # PluginManifestV2 → existing PluginManifest internal type
└── try.ts                        # NEW — readonly preview (clone to temp, parse, scan, display, cleanup)
```

### Modified Modules

- **`config.ts`** — schema collapsed (see SPEC.md `[v2.0 Configuration]`). Old keys read by `migrate.ts` only; v2.0 reader rejects them with a hint to run `migrate`.
- **`security/policy.ts`** — single policy. No `[security.human]` / `[security.agent]` branching. `trust = []` checked first; matching sources skip scanning entirely.
- **`install.ts`** — extended to update `skilltap.toml` and `skilltap.lock` when running inside a project root. Smart scope default: `findProjectRoot()` is checked first; if found and no scope flag, default to project. Otherwise global.
- **`taps.ts`** — HTTP tap adapter removed. `loadTaps()` only loads git-cloned and built-in taps.
- **`plugin/install.ts`** — multi-plugin repo support. After `discoverPublishablePlugins()`, prompt or pick by `:plugin-name` suffix.
- **`mcp-inject.ts`** — adds `claude-desktop` to `MCP_AGENT_CONFIGS` registry, mapping to platform-specific paths (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS, etc.).
- **`doctor.ts`** — adds v2.0 checks: manifest drift, lockfile drift, plugin manifest validity, MCP injection consistency. Existing 9 checks preserved.

### Removed Modules

- **`registry/`** (HTTP registry client) — removed entirely.
- **`security/overrides.ts`** if it exists — replaced with simple glob match in `policy/compose.ts`.

### CLI Module Additions

```
packages/cli/src/commands/
├── sync.ts                       # NEW
├── status.ts                     # NEW (also wired as the bare-command default)
├── try.ts                        # NEW
├── migrate.ts                    # NEW
├── enable.ts                     # NEW (top-level shortcut, also under plugin/)
├── disable.ts                    # NEW
├── toggle.ts                     # CHANGED — now top-level, accepts plugin:component syntax
├── install.ts                    # CHANGED — supports `mcp:` prefix, manifest write
├── update.ts                     # CHANGED — refreshes lockfile, semantically distinct from sync
└── config/
    ├── agent-mode.ts             # REMOVED — replaced by `config set agent.default <bool>`
    └── set.ts                    # CHANGED — accepts agent.default, agent.block; rejects v1.0 keys
```

### Data Flow: `skilltap install` (v2.0)

```
1. Parse source, detect mcp:/<owner/repo:name>/etc syntax variants
2. resolveAgentFlag() → effectivePolicy.agent
3. resolveSource() → cloneable URL (no HTTP registry)
4. Find project root (smart scope default)
5. Clone to temp dir
6. detectPlugin(tempDir) — try .skilltap/, .claude-plugin/, .codex-plugin/ in priority order
7. If multiple publishable .skilltap/*.toml files: prompt or :name suffix selects
8. Run security scan (single [security] policy, trust-list shortcuts)
9. composePolicy → effective decision (install / prompt / fail)
10. If running in project, update skilltap.toml + skilltap.lock atomically
11. Place skill / inject MCP / place agent definitions per plugin manifest
12. Update state.json (single file per scope)
13. Print summary; non-interactive variant uses plain text
```

### Data Flow: `skilltap sync`

```
1. loadManifest(projectRoot) → manifest
2. loadLockfile(projectRoot) → lockfile (or null)
3. loadState(project) + loadState(global) → states
4. planSync(manifest, lockfile, state) → SyncPlan
   - adds: declared but not in state
   - updates: declared at different ref than locked / installed
   - removes: in state but not declared (only with --prune)
   - lockfile-only: in lockfile but no state record (treat as add)
5. If interactive and plan non-empty: show diff, prompt to confirm
6. If --strict and plan non-empty: error out
7. applySync(plan, callbacks) — runs install/remove/update for each entry
8. Update lockfile if any range resolved to a new ref
9. Update state.json
10. Print summary
```

### Data Flow: `skilltap migrate`

```
1. Detect v1.0 markers: installed.json, plugins.json, [security.human], [agent-mode], [[security.overrides]]
2. If no markers: exit 0 with "Already on v2.0".
3. Read all v1.0 files. Parse with v1.0 schemas (kept as legacy schemas in core/src/schemas/v1/).
4. Translate:
   - installed.json + plugins.json → state.json (consolidate)
   - [security.human] / [security.agent] → [security] (warn if mismatch; pick stricter or prompt)
   - [[security.overrides]] → [security] trust = [...] (warn — trust list is less expressive than overrides)
   - [agent-mode] → [agent] (default: enabled→default, scope: ignored, deprecated)
   - [registry] → [[registries]] (renamed; functionality preserved)
   - HTTP taps in [[taps]] → error, list affected taps for manual handling
5. Write v2.0 files. Rename originals to *.v1.bak.
6. Run `skilltap doctor` to verify.
7. Print migration summary with diff.
```

### v2.0 Schemas (Zod 4)

```typescript
// manifest/schemas.ts

export const TargetsSchema = z.object({
  also: z.array(z.string()).default([]),
  scope: z.enum(["", "global", "project"]).default(""),
}).prefault({})

export const ManifestEntrySchema = z.union([
  z.string(),                            // "^1.0", "*", "v1.2.3"
  z.object({
    ref: z.string().optional(),
    components: z.record(z.string(), z.boolean()).optional(),
  }),
])

export const ProjectManifestSchema = z.object({
  targets: TargetsSchema,
  skills: z.record(z.string(), ManifestEntrySchema).default({}),
  plugins: z.record(z.string(), ManifestEntrySchema).default({}),
  taps: z.record(z.string(), z.string()).default({}),
})

export const LockEntrySchema = z.object({
  source: z.string(),
  ref: z.string(),
  sha: z.string().optional(),
  range: z.string(),
})

export const LockfileSchema = z.object({
  version: z.literal(1),
  skill: z.array(LockEntrySchema).default([]),
  plugin: z.array(LockEntrySchema).default([]),
})

// Native skilltap plugin manifest (.skilltap/<name>.toml)
export const PluginManifestV2Schema = z.object({
  name: z.string(),
  version: z.string(),
  description: z.string().optional(),
  publish: z.boolean().default(false),
  skills: z.array(z.object({
    name: z.string(),
    path: z.string(),
  })).default([]),
  servers: z.array(z.object({
    name: z.string(),
    type: z.enum(["stdio", "http"]),
    command: z.string().optional(),
    args: z.array(z.string()).default([]),
    env: z.record(z.string(), z.string()).default({}),
    url: z.string().optional(),
    headers: z.record(z.string(), z.string()).default({}),
  })).default([]),
  agents: z.array(z.object({
    name: z.string(),
    path: z.string(),
  })).default([]),
})

// Simplified config (replaces v1.0 ConfigSchema)
export const SecurityConfigV2Schema = z.object({
  scan: z.enum(["semantic", "static", "none"]).default("static"),
  on_warn: z.enum(["prompt", "fail", "install"]).default("install"),
  trust: z.array(z.string()).default([]),
}).prefault({})

export const AgentConfigSchema = z.object({
  default: z.boolean().default(false),
  block: z.boolean().default(false),
}).prefault({})

export const ConfigV2Schema = z.object({
  defaults: z.object({
    also: z.array(z.string()).default([]),
    scope: z.enum(["", "global", "project"]).default(""),
  }).prefault({}),
  agent: AgentConfigSchema,
  security: SecurityConfigV2Schema,
  taps: z.array(z.object({ name: z.string(), url: z.string() })).default([]),
  updates: UpdatesConfigSchema.prefault({}),
  telemetry: TelemetryConfigSchema.prefault({}),
  builtin_tap: z.boolean().default(true),
  verbose: z.boolean().default(true),
  default_git_host: z.string().default("https://github.com"),
})

// Unified state file (replaces installed.json + plugins.json)
export const StateSchema = z.object({
  version: z.literal(2),
  skills: z.array(InstalledSkillSchema).default([]),
  plugins: z.array(PluginRecordSchema).default([]),
  mcpServers: z.array(z.object({
    name: z.string(),
    source: z.string(),
    config: z.any(),  // McpServerConfigSchema
    targets: z.array(z.string()),
    installedAt: z.string(),
  })).default([]),
})
```

### Decision Log Additions (v2.0)

| Decision | Choice | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Manifest format | TOML at project root | JSON, package.json-style sub-key | Matches existing config.toml conventions; human-friendly; matches Cargo's design |
| Native plugin format | TOML in `.skilltap/<name>.toml` | JSON to match Claude Code | Skilltap's own files use TOML for consistency; Claude/Codex JSON formats remain readable inputs |
| Lockfile | Yes, Cargo-style | No lockfile, manifest only, single combined file | Reproducibility is the headline value of `sync`; users expect it from package-manager-shaped tools |
| Sync drift | Prompt by default | Strict-by-default, additive-only | Prompt avoids destructive surprises while preserving the deterministic value when `--yes` |
| Scope detection | Smart default (git → project) | Always prompt, always global | Most installs in a git repo are project-scoped; prompt fatigue is a known v1.0 issue |
| State file | Single state.json per scope | Keep installed.json + plugins.json | One file per scope = easier backup, less file-proliferation, simpler doctor checks |
| Security model | One [security] block, no per-mode | Keep human/agent split | One rule for everyone; agent flag becomes about UX (prompts, output), not policy |
| Agent flag | --agent + env + config | Just config, just flag | Layered control: agents always set the flag; sticky config for CI; block flag for shared machines |
| HTTP registry | Removed | Keep, mark deprecated | Real-world usage was minimal; removing the adapter cuts schemas, adapters, tests, and docs |
| Multi-plugin repos | `.skilltap/<name>.toml` per plugin | Single skilltap.toml with [[publish.plugins]] | Mirrors `.claude-plugin/plugin.json` and `.codex-plugin/plugin.json` shape; one file per plugin |
| Migration | Explicit `migrate` command | Auto-migrate on first run | Migration touches multiple files; users should be intentional, especially when HTTP taps exist |
| Telemetry | Unchanged from v1.0 | Drop entirely | User signal valuable; behavior already privacy-preserving; no churn needed |

---

## v2.0 Redesign Architecture

> Second-pass redesign per [VISION.md — v2.0 Redesign](./VISION.md#v20-redesign-current-direction). Supersedes the architecture deltas above where they conflict. The v0.1–v2.0 foundation remains; this section describes deletions, additions, and reshapes for the redesign.

### Deletions

Modules removed entirely:

- `packages/cli/src/ui/agent-out.ts` — `agentSuccess`/`agentError`/`agentSecurityBlock` and `exitWithError(agentMode, ...)` all gone. Output goes through the new `Output` interface (Phase 40).
- `packages/core/src/agent-env.ts` — `isAgentEnv()` deleted. The `SKILLTAP_AGENT` env var contract is retired.
- `packages/cli/src/commands/config/agent-mode.ts` — wizard deleted. No persistent agent-mode toggle anywhere.
- `packages/cli/src/commands/skills/` — entire subcommand group folded into top-level. `link.ts` / `unlink.ts` deleted (replaced by extended `adopt`). `info.ts` / `remove.ts` move to top-level commands. `move.ts` becomes a flag on `adopt`.
- `packages/cli/src/commands/plugin/` — `info`/`toggle`/`remove` accessible as top-level commands or via TUI screens. Subcommand group deleted.
- `packages/cli/src/commands/tap/install.ts` — duplicate of `install skill <name>` when `<name>` resolves through a tap. Deleted.

State-store fallbacks deleted:

- `loadInstalled()` no longer falls back to `installed.json`.
- `loadPlugins()` no longer falls back to `plugins.json`.
- `state.json` is the only canonical source. Migration is the explicit responsibility of `skilltap migrate`.

Schema fields removed:

- `AgentModeSchema` (`[agent-mode]` block).
- `[security.agent]` and `[security.human]` blocks → single `[security]`.
- Security presets (`none`/`relaxed`/`standard`/`strict`).
- `[[security.overrides]]` (kind/match/preset).

### Reshapes

**`packages/core/src/policy.ts`** — `composePolicy(config, flags)` returns a single `EffectivePolicy`. No agent-mode branch. No per-mode security selection. The function shrinks substantially.

**`packages/cli/src/commands/install.ts`** — collapses `runAgentMode` + `runInteractiveMode` into one `runInstall(args, output)`. Output mode (TTY/plain/JSON) is decided once at command entry; the orchestration body is identical regardless. Callbacks reduce from ~15 to ~6 — the surplus existed only to express agent-mode auto-fail semantics, which now live in the output layer.

**`packages/cli/src/commands/update.ts`** — same collapse as install. Now also accepts `[type]` and `[name]` positional args (`update skill <name>` / `update plugin <name>` / `update mcp <name>`). Bare `update` updates everything.

**`packages/cli/src/commands/toggle.ts`** — accepts `[type] [name[:component]]`. Opens TUI when args missing.

**`packages/cli/src/index.ts`** — subCommands shrinks from 35+ entries to ~16. Silent aliases (`list`, bare `remove`/`info`/`link`/`unlink`) removed. Old paths surface as clear errors with hints.

### Additions

```
packages/cli/src/
├── tui/                              # NEW — Ink-based TUI layer
│   ├── index.ts                      # mountTui(initialScreen) — root component, screen router
│   ├── screens/
│   │   ├── dashboard/                # Bare `skilltap` — tabs: Installed, Taps, Updates, Drift
│   │   ├── find/                     # Type-ahead search across taps + registries
│   │   ├── toggle/                   # Pick type → name → components
│   │   ├── adopt/                    # Unmanaged skills + Claude Code plugins
│   │   ├── install/                  # Type picker → source picker → confirm
│   │   └── plugin-manager/           # Plugin detail + component toggles
│   ├── state/                        # Pure reducers per screen (testable with bun:test)
│   ├── components/                   # Shared Ink components (List, Detail, Tabs, KeyHints)
│   └── keys.ts                       # Key-binding registry
└── output/                           # NEW — output mode abstraction
    ├── tty.ts                        # Rich output for TTY (colors, spinners, clack-style)
    ├── plain.ts                      # Plain text for non-TTY
    └── json.ts                       # Structured JSON output (per-command schemas)

packages/core/src/
├── agent-plugins/                    # NEW — generic agent-plugin discovery framework
│   ├── types.ts                      # AgentPluginScanner interface
│   ├── claude-code.ts                # Reads ~/.claude/plugins/installed_plugins.json
│   ├── codex.ts                      # Stub (no marketplace today; pluggable for future)
│   └── index.ts                      # registerScanner(), scanAll()
├── plugin/capture.ts                 # NEW — plugin capture (per design/plugin-capture.md)
│   # canonicalizeSourceUrl(), detectCaptureMatches(), applyCapture()
└── output/                           # NEW — Output interface used by both cli/ and tui/
    └── types.ts                      # Output interface, OutputMode, JsonShapes
```

### Architecture risks

- **Ink stability under Bun** — Ink targets Node.js. Bun is mostly compatible but has had quirks with raw-mode terminal handling and signal cleanup. Mitigation: PTY-based smoke tests on every supported platform; fall back to clack-style top-down prompts if Ink misbehaves on a specific screen.
- **Claude Code plugin format drift** — `~/.claude/plugins/installed_plugins.json` is observable but undocumented. Mitigation: Zod parser uses `passthrough()` for unknown fields; doctor warns on unrecognized schema; graceful no-op when format diverges (don't crash, surface in doctor instead).
- **TUI testability** — multi-screen UI is harder to test than top-down prompts. Mitigation: state machine per screen lives in pure reducers (testable with bun:test); Ink components only render. PTY snapshot tests catch regressions in render output.
- **Output mode discipline** — risk of regressions where a developer writes directly to `process.stdout` instead of through `Output`. Mitigation: lint rule (or simple grep in CI) blocking direct stdout/stderr writes outside `cli/output/`.

### Decision Log Additions (v2.0 Redesign)

| Decision | Choice | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Agent-mode removal | Single runtime, TTY/JSON drives output | Keep `--agent` flag, keep config block | Agent-mode added two parallel runtimes for one concern (output style). Collapsing it removes ~330 lines of duplicated orchestration in install/update alone. |
| Install disambiguation | Required subcommand (`install skill\|plugin\|mcp <source>`) | Auto-detect, hybrid, `--as` flag | User-driven decision. Explicit type means no auto-detect heuristics, no `mcp:` URL prefix, no prompt-on-ambiguity. Symmetric with `remove`/`update`/`toggle`. |
| TUI library | Ink (React-for-terminals) | Custom @clack orchestrator, full TUI à la lazygit | Multi-screen flows (dashboard, find, toggle, adopt) need persistent state; clack is top-down only. lazygit-style overshoots for a CLI used a few times per project. |
| Language migration | Stay TS+Bun | Migrate to Go/Rust | Skilltap pain is CLI ergonomics, not core. Core has substantial logic (sigstore, npm registry, security scanners, Zod schemas) that costs months to port for marginal runtime gain. |
| `verify` retirement | Fold into `doctor` | Keep separate, fold into `create` | Both verify and doctor are check-things commands. Single verb (`doctor`) with arg-based scope: env check (no args) vs per-artifact (`doctor skill <path>`). Reduces top-level surface. |
| `link`/`unlink` retirement | Fold into `adopt` | Keep separate | `link <path>` and `adopt --track-in-place` did the same thing for different inputs. Unifying to `adopt [path]` removes the duplicate verb. |
| State-store fallbacks | Removed | Keep one more cycle | Pre-v2.0-redesign "no users" assumption. `migrate` command is the explicit upgrade path; no transparent fallback. |
| Output abstraction | New `Output` interface | Keep mixed `successLine`/`agentSuccess` | Output mode (TTY/plain/JSON) is one decision; routing it through an interface makes mode-specific behavior testable and prevents per-command output drift. |
| Adoption framework | Pluggable `AgentPluginScanner` | Hardcoded Claude Code path | Codex may ship a marketplace eventually; future agents will have plugin systems. Pluggable scanner avoids retrofit. |
| Bare `skilltap` | TUI dashboard (TTY only) | Print status text, error always | Matches lazygit/k9s conventions. Headless callers use `skilltap status` explicitly. Bare command is humans-only. |
