# Architecture

## Overview

skilltap is a CLI tool that installs agent skills and plugins from any git host. It clones repos, scans for SKILL.md files and plugin manifests, runs security checks, and places skills in the universal `.agents/skills/` directory. For plugins, it also injects MCP server configs into agent platform config files and places agent definitions.

This document describes how skilltap is built internally — module boundaries, data flow, key abstractions, and technology decisions.

## Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Runtime | Bun | Fast, compiles to single binary (`bun build --compile`), native TypeScript |
| Language | TypeScript | Type safety, matches ecosystem (npm security libs) |
| CLI framework | citty (UnJS) | Declarative, TypeScript-first, tiny, good subcommand support |
| Terminal UI (top-down) | @clack/prompts | Modern prompts, spinners, select menus. Used for one-shot flows. |
| Multi-screen TUI | Ink (React for terminals) | Persistent state across screens for the dashboard, find, toggle, and adopt screens. |
| Config format | TOML (smol-toml) | Human-friendly, clear sections. smol-toml is small and spec-compliant. |
| Validation | Zod 4 | Runtime schema validation at every data boundary (config, state, manifests, frontmatter, agent responses). |
| Git | Shell out to `git` CLI | User's auth (SSH, credential helpers) just works. Zero git library deps. |
| Testing | Bun test runner | Built-in, fast, no extra deps. PTY smoke tests via `node-pty`. |
| Platform | Linux + macOS | Symlinks, XDG paths. Windows later if demand. |

### Distribution

1. `bunx skilltap` — for Bun users
2. `npx skilltap` — for Node users
3. Standalone binary via `bun build --compile` — no runtime dependency
4. Homebrew: `brew install nklisch/skilltap/skilltap`
5. Install script: `curl -fsSL https://raw.githubusercontent.com/nklisch/skilltap/main/install.sh | sh`

GitHub Actions release workflow (`.github/workflows/release.yml`) builds 4 platform binaries (linux-x64, linux-arm64, darwin-x64, darwin-arm64) on `v*` tag push, attests each binary with `actions/attest-build-provenance`, generates `checksums.txt`, and publishes `skilltap` and `@skilltap/core` to npm with `--provenance`. A `repository_dispatch` event then triggers the Homebrew formula update in `homebrew-skilltap/`. The release workflow also runs the full CLI test suite against each platform's host-arch artifact (linux-x64 and darwin-arm64) before signing/upload.

## Monorepo Structure

Bun workspaces with three packages:

```
skilltap/
├── packages/
│   ├── core/                           # Library — all business logic. Zero CLI deps.
│   │   ├── src/
│   │   │   ├── types.ts                # Result<T,E>, ok(), err(), error hierarchy
│   │   │   ├── fs.ts                   # Global base path helpers, temp dir management
│   │   │   ├── paths.ts                # scopeBase(), skillInstallDir, findProjectRoot
│   │   │   ├── git.ts                  # Git operations (clone, pull, fetch, diff)
│   │   │   ├── scanner.ts              # Skill discovery (find SKILL.md in repos)
│   │   │   ├── frontmatter.ts          # parseSkillFrontmatter() — YAML-style parser
│   │   │   ├── config.ts               # Config + state.json read/write; loadSkillState/saveSkillState
│   │   │   ├── config-keys.ts          # Config get/set helpers (dot-path resolve, coerce)
│   │   │   ├── install.ts              # Install orchestration
│   │   │   ├── remove.ts               # Remove skill logic
│   │   │   ├── update.ts               # Update skill logic (fetch, diff, pull)
│   │   │   ├── discover.ts             # Scan + correlate with state.json
│   │   │   ├── adopt.ts                # Adopt unmanaged skills (move/track-in-place + Claude Code plugins)
│   │   │   ├── move.ts                 # Move skills between scopes
│   │   │   ├── disable.ts              # Disable/enable individual skills
│   │   │   ├── taps.ts                 # Tap management (add, remove, update, search)
│   │   │   ├── marketplace.ts          # adaptMarketplaceToTap() — marketplace.json → Tap
│   │   │   ├── symlink.ts              # Agent-specific symlink creation; AGENT_PATHS
│   │   │   ├── try.ts                  # Readonly preview (clone to temp, parse, scan)
│   │   │   ├── mcp-install.ts          # Standalone MCP install (install mcp <source>)
│   │   │   ├── self-update.ts          # skilltap self-update
│   │   │   ├── skill-check.ts          # Background skill update check
│   │   │   ├── skills-registry.ts      # External skill registry search (skills.sh, custom)
│   │   │   ├── npm-registry.ts         # npm registry API client
│   │   │   ├── orphan.ts               # Orphan record cleanup
│   │   │   ├── shell.ts                # Shell completion endpoint helpers
│   │   │   ├── debug.ts                # Diagnostics for debug command
│   │   │   ├── json-state.ts           # Generic JSON file I/O (loadJsonState/saveJsonState)
│   │   │   ├── policy/                 # Policy composition (single source for security decisions)
│   │   │   │   ├── compose.ts          # composePolicy(config, flags) → EffectivePolicy
│   │   │   │   ├── trust-glob.ts       # Glob matcher for security.trust against tap name / source URL
│   │   │   │   ├── types.ts            # EffectivePolicy, CliFlags
│   │   │   │   └── index.ts
│   │   │   ├── output/                 # Output mode abstraction
│   │   │   │   └── types.ts            # Output interface, OutputMode, JsonShapes
│   │   │   ├── manifest/               # skilltap.toml + skilltap.lock
│   │   │   │   ├── schemas.ts          # ProjectManifestSchema, LockfileSchema (skills + plugins + mcps)
│   │   │   │   ├── load.ts             # loadManifest(projectRoot)
│   │   │   │   ├── save.ts             # saveManifest, saveLockfile (atomic + ensureDirs)
│   │   │   │   ├── update.ts           # addSkill/addPlugin/addMcp + remove counterparts
│   │   │   │   ├── publish.ts          # discoverPublishablePlugins(repoRoot)
│   │   │   │   ├── range.ts            # Range parsing/matching (^, ~, *, exact)
│   │   │   │   ├── recover.ts          # Lockfile recovery from state when missing
│   │   │   │   └── paths.ts            # Manifest path resolution
│   │   │   ├── sync/                   # Cargo-style reconcile engine
│   │   │   │   ├── drift.ts            # detectDrift(state, manifest, lockfile) → DriftReport (skills/plugins/mcps)
│   │   │   │   ├── plan.ts             # planSync() → SyncPlan
│   │   │   │   ├── apply.ts            # applySync(plan) — runs install/remove/update; capture callbacks
│   │   │   │   ├── types.ts
│   │   │   │   └── index.ts
│   │   │   ├── state/                  # Unified state.json schema
│   │   │   │   └── schema.ts           # StateSchema { version, skills, plugins, mcpServers }
│   │   │   ├── status/                 # Status dashboard data assembly
│   │   │   ├── migrate/                # Migration
│   │   │   │   ├── run.ts              # Top-level migrate orchestrator
│   │   │   │   ├── config.ts           # Translates legacy [security.*]/[agent-mode] keys
│   │   │   │   ├── state.ts            # Translates legacy installed.json + plugins.json → state.json
│   │   │   │   └── manifest.ts         # skilltap.toml shape verification
│   │   │   ├── doctor/                 # Diagnostic checks (per-area files + index)
│   │   │   │   ├── checks/             # Each check is a function returning DoctorCheck
│   │   │   │   ├── fix/                # Auto-repair functions (--fix)
│   │   │   │   └── index.ts            # runDoctor({ fix?, onCheck? }) → DoctorResult
│   │   │   ├── schemas/
│   │   │   │   ├── config.ts           # ConfigSchema (flat [security], [scanner], etc.)
│   │   │   │   ├── tap.ts              # TapSchema, TapSkillSchema, TapPluginSchema
│   │   │   │   ├── marketplace.ts      # marketplace.json (Claude Code format)
│   │   │   │   ├── plugin.ts           # PluginManifestSchema + PLUGIN_FORMATS
│   │   │   │   ├── skill.ts            # SKILL.md frontmatter
│   │   │   │   ├── agent.ts            # Agent response + ResolvedSource
│   │   │   │   └── index.ts
│   │   │   ├── adapters/
│   │   │   │   ├── types.ts            # SourceAdapter interface
│   │   │   │   ├── git.ts              # https / git@ / ssh
│   │   │   │   ├── github.ts           # github:owner/repo, owner/repo
│   │   │   │   ├── npm.ts              # npm:@scope/name[@version]
│   │   │   │   ├── local.ts            # Filesystem paths
│   │   │   │   ├── resolve.ts          # resolveSource() orchestrator
│   │   │   │   └── index.ts
│   │   │   ├── agents/
│   │   │   │   ├── types.ts            # AgentAdapter interface
│   │   │   │   ├── detect.ts           # Auto-detect installed agents, resolveAgent()
│   │   │   │   ├── adapters.ts         # Built-in adapters (claude, gemini, codex, opencode)
│   │   │   │   ├── factory.ts          # createCliAdapter() shared factory
│   │   │   │   ├── ollama.ts           # Ollama adapter
│   │   │   │   ├── custom.ts           # Custom binary adapter
│   │   │   │   ├── extract.ts          # extractAgentResponse() JSON pipeline
│   │   │   │   └── index.ts
│   │   │   ├── agent-plugins/          # Pluggable scanner for external plugin systems
│   │   │   │   ├── types.ts            # AgentPluginScanner interface
│   │   │   │   ├── claude-code.ts      # Reads ~/.claude/plugins/installed_plugins.json
│   │   │   │   ├── codex.ts            # Stub (no marketplace today)
│   │   │   │   └── index.ts            # registerScanner(), scanAll()
│   │   │   ├── security/
│   │   │   │   ├── patterns.ts         # 7 detection functions
│   │   │   │   ├── static.ts           # Layer 1 — scanStatic(), scanDiff()
│   │   │   │   ├── semantic.ts         # Layer 2 — scanSemantic(), chunking
│   │   │   │   └── index.ts
│   │   │   ├── trust/
│   │   │   │   ├── verify-npm.ts       # Sigstore/SLSA attestation verification
│   │   │   │   ├── verify-github.ts    # GitHub attestation via `gh attestation verify`
│   │   │   │   ├── resolve.ts          # resolveTrust() — compute tier from signals
│   │   │   │   └── index.ts
│   │   │   ├── plugin/                 # Plugin detection, install, lifecycle, MCP injection
│   │   │   │   ├── detect.ts           # detectPlugin(dir) — priority: .skilltap/ → .claude-plugin/ → .codex-plugin/
│   │   │   │   ├── parse-claude.ts
│   │   │   │   ├── parse-codex.ts
│   │   │   │   ├── parse-common.ts     # discoverSkills() shared helper
│   │   │   │   ├── mcp.ts              # MCP config normalization
│   │   │   │   ├── mcp-inject.ts       # MCP_AGENT_CONFIGS registry + inject/remove/list
│   │   │   │   ├── agents.ts           # Agent definition (.md) reader
│   │   │   │   ├── install.ts          # installPlugin() orchestration
│   │   │   │   ├── lifecycle.ts        # removeInstalledPlugin, toggleInstalledComponent
│   │   │   │   ├── capture.ts          # canonicalizeSourceUrl, detectCaptureMatches, applyCapture
│   │   │   │   ├── state.ts            # Plugin slice of state.json
│   │   │   │   └── index.ts
│   │   │   ├── plugin-v2/              # Native skilltap plugin format reader
│   │   │   │   ├── parse-toml.ts       # Parse .skilltap/<name>.toml
│   │   │   │   ├── discover.ts         # Find all .skilltap/*.toml in a repo
│   │   │   │   ├── normalize.ts        # SkilltapPluginManifest → PluginManifest
│   │   │   │   └── index.ts
│   │   │   ├── templates/
│   │   │   │   ├── basic.ts
│   │   │   │   ├── npm.ts
│   │   │   │   ├── multi.ts
│   │   │   │   └── index.ts
│   │   │   └── index.ts                # Package barrel export
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── cli/                            # CLI entry point — commands, prompts, TUI
│   │   ├── src/
│   │   │   ├── index.ts                # citty runMain entry, subCommand router
│   │   │   ├── commands/
│   │   │   │   ├── install/            # install <type> <source> — typed router
│   │   │   │   │   ├── index.ts        # Router dispatching to skill/plugin/mcp
│   │   │   │   │   ├── skill.ts
│   │   │   │   │   ├── plugin.ts       # Multi-plugin syntax + capture flags
│   │   │   │   │   ├── mcp.ts          # Standalone MCP install
│   │   │   │   │   └── shared.ts
│   │   │   │   ├── remove/             # remove <type> <name>
│   │   │   │   │   ├── index.ts
│   │   │   │   │   ├── skill.ts
│   │   │   │   │   ├── plugin.ts
│   │   │   │   │   ├── mcp.ts
│   │   │   │   │   └── shared.ts
│   │   │   │   ├── update.ts           # update [type] [name]
│   │   │   │   ├── toggle.ts           # toggle [type] [name[:component]]
│   │   │   │   ├── try.ts              # try <type> <source>
│   │   │   │   ├── adopt.ts            # adopt [path] (replaces link/unlink)
│   │   │   │   ├── sync.ts
│   │   │   │   ├── status.ts
│   │   │   │   ├── doctor.ts           # doctor [skill|plugin <path>]
│   │   │   │   ├── migrate.ts
│   │   │   │   ├── move.ts
│   │   │   │   ├── find.ts
│   │   │   │   ├── info.ts
│   │   │   │   ├── create.ts
│   │   │   │   ├── self-update.ts
│   │   │   │   ├── completions.ts
│   │   │   │   ├── config.ts           # Routes to config/*
│   │   │   │   ├── config/
│   │   │   │   │   ├── get.ts
│   │   │   │   │   ├── set.ts
│   │   │   │   │   ├── security.ts     # --scan / --on-warn / --trust-add / --trust-remove / --trust-list
│   │   │   │   │   ├── telemetry.ts
│   │   │   │   │   └── edit.ts
│   │   │   │   └── tap/
│   │   │   │       ├── add.ts
│   │   │   │       ├── remove.ts
│   │   │   │       ├── list.ts
│   │   │   │       ├── info.ts
│   │   │   │       └── init.ts
│   │   │   ├── tui/                    # Ink-based multi-screen TUI
│   │   │   │   ├── App.tsx             # Root component, screen router
│   │   │   │   ├── context.ts          # AppContext factory wiring TUI to core dispatchers
│   │   │   │   ├── index.ts            # mountTui(initialScreen)
│   │   │   │   ├── keys.ts             # Key-binding registry
│   │   │   │   ├── state/              # Pure reducers per screen (testable with bun:test)
│   │   │   │   └── screens/            # dashboard/, find/, toggle/, adopt/, install/, plugin-manager/
│   │   │   ├── completions/
│   │   │   │   └── generate.ts         # Bash/zsh/fish scripts
│   │   │   └── ui/
│   │   │       ├── format.ts           # Output formatting (tables, colors, ansi)
│   │   │       ├── prompts.ts          # @clack/prompts wrappers
│   │   │       ├── scan.ts             # Security scan result display
│   │   │       ├── trust.ts            # Trust tier display helpers
│   │   │       ├── policy.ts           # loadPolicyOrExit() — CLI adapter for composePolicy
│   │   │       ├── plugin-format.ts    # componentSummary() display helpers
│   │   │       ├── capture.ts          # printCaptureConflict, printCaptureSummary
│   │   │       ├── output/             # tty/plain/json Output implementations
│   │   │       └── resolve.ts          # resolveScope, parseAlsoFlag, resolveAgent helpers
│   │   ├── package.json                # Published as "skilltap" on npm
│   │   └── tsconfig.json
│   └── test-utils/                     # Shared test fixtures and helpers
│       ├── src/
│       │   ├── fixtures.ts             # createTapWithPlugins, mock skill repos
│       │   ├── env.ts                  # createTestEnv() + pathExists()
│       │   ├── git.ts                  # Test git helpers (init, commit)
│       │   ├── tmp.ts                  # Temp directory management
│       │   ├── runSkilltap.ts          # Subprocess runner; honors SKILLTAP_TEST_BIN
│       │   ├── runInteractive.ts       # PTY runner for clack/Ink rendering
│       │   └── cliCmd.ts               # Returns [binary] if SKILLTAP_TEST_BIN, else bun args
│       ├── fixtures/                   # standalone-skill, multi-skill-repo, malicious-skill, sample-tap
│       ├── package.json                # Private, not published
│       └── tsconfig.json
├── package.json                        # Workspace root
├── bunfig.toml
├── tsconfig.json                       # Base TypeScript config
└── docs/                               # VISION, ARCH, SPEC, UX, SECURITY, ROADMAP
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
- `flipUrlProtocol(url)` — converts between HTTPS and SSH git URL forms. Returns `null` for unrecognized patterns.
- `pull(dir)`, `fetch(dir)`, `diff(dir, from, to)`, `revParse(dir)`, `log(dir, n)`.

**scanner.ts** — Finds SKILL.md files in a directory tree. Returns structured results with name, description (from frontmatter), and path. See [SPEC.md — Skill Discovery](./SPEC.md#skill-discovery) for the scanning algorithm.

**frontmatter.ts** — `parseSkillFrontmatter(content)` parses YAML-style `---` frontmatter blocks into a plain object. Shared by scanner.ts and the `doctor skill` validator.

**security/static.ts** — Layer 1 pattern matching. Takes file contents, returns warnings with line numbers, category, and raw/visible text. Uses `anti-trojan-source` and `out-of-character` for Unicode detection, regex for everything else.

**security/semantic.ts** — Layer 2 agent-based evaluation. Chunks content, invokes agent adapter, aggregates scores. See [SPEC.md — Layer 2](./SPEC.md#layer-2-semantic-scan) for the chunking algorithm and security prompt.

**config.ts** — Reads/writes `~/.config/skilltap/config.toml` and the canonical `state.json` (per scope). Production code reads and writes `state.json`. `loadConfig` validates against the current schema and rejects unknown shapes. Exports `loadSkillState`/`saveSkillState` for the skills slice.

**config-keys.ts** — Pure helpers for `config get`/`config set`: dot-path resolution, value coercion, settable key allowlist (`SETTABLE_KEYS`), immutable deep-set, plain-text formatting.

**install.ts** — Orchestrates the install flow. Coordinates git, scanner, security, config, and symlink modules. Updates `state.json` (skill slice) and the project manifest+lockfile when a project root is detected.

**discover.ts** — `discoverSkills(options?)` scans all skill directories at both global and project scope. Cross-references with `state.json` to classify skills as managed or unmanaged. Detects symlinks, reads SKILL.md frontmatter for descriptions, detects git remotes on unmanaged skills.

**adopt.ts** — `adoptSkill(skill, options?)` brings an unmanaged skill under skilltap management. Two modes: `move` relocates the skill dir to `.agents/skills/` and creates symlinks from original locations; `track-in-place` (default) creates a "linked" record without moving. Also accepts a path argument for adopting external skills (replacing the old `link` semantics). Scans through registered `AgentPluginScanner`s when no path is given.

**move.ts** — Moves a managed skill between scopes (global ↔ project). Handles symlink cleanup and recreation, state.json record transfer, and linked→managed conversion. Writes manifest+lockfile when a project root is involved.

**disable.ts** — Toggle a single skill on/off via `.disabled/` directory mechanism. Writes manifest+lockfile when project-scoped.

**try.ts** — Readonly preview. Clone to temp, parse manifests, run static scan, render summary, cleanup. No state.json or filesystem writes outside the temp dir. Loads config and threads `default_git_host` through the source resolver.

**mcp-install.ts** — Standalone MCP install (`install mcp <source>`). Extracts MCP server entries from a source, injects into agent configs via `mcp-inject.ts`, tracks in `state.mcpServers`. Honors smart-scope when run outside a git repo.

**skill-check.ts** — Background skill update check. `checkForSkillUpdates(intervalHours, projectRoot)` reads cache and fires a background refresh if stale. `fetchSkillUpdateStatus()` does the actual network check.

**taps.ts** — Manages tap repos. Clone, pull, parse tap index (`tap.json` or `.claude-plugin/marketplace.json`), search across taps. Git-only — taps are cloned, never fetched via HTTP. `loadTaps()` returns entries for both `skills` and `plugins` arrays from tap.json.

**marketplace.ts** — Adapts Claude Code `marketplace.json` to skilltap's internal `Tap` type. For relative-path sources in a local tap directory, auto-detects `.claude-plugin/plugin.json` via `detectPlugin()` and produces `TapPlugin` entries (full skills/MCP/agents) when found. Plugin-only features (LSP, hooks, commands) are silently ignored.

**symlink.ts** — Creates and removes symlinks for agent-specific directories. Knows `AGENT_PATHS` for each supported agent. Idempotent — replaces stale symlinks and leftover real directories instead of failing on EEXIST. Single source for `AGENT_PATHS` / `AGENT_LABELS` / `VALID_AGENT_IDS`.

**npm-registry.ts** — npm registry API client. `parseNpmSource()`, `fetchPackageMetadata()`, `resolveVersion()`, `downloadAndExtract()`. Private registry support via `NPM_CONFIG_REGISTRY`, `.npmrc`, or `~/.npmrc`.

**skills-registry.ts** — External skill registry search system. `SkillRegistry` interface with `{ name, search(query, limit) }`. Built-in: `skillsShRegistry`. `createCustomRegistry(name, url)` factory for any URL implementing the search API. `resolveRegistries(config)` reads `[registry].enabled` + `[[registry.sources]]`.

**doctor/** — Diagnostic checks. `runDoctor({ fix?, onCheck? })` → `DoctorResult` runs the check functions serially, streaming results via `onCheck`. `--fix` triggers auto-repair functions for safely-fixable issues. `DoctorCheck` carries `info`/`fixDescription`/`detail`/`fixed?` fields. `doctor --json` includes those fields. `doctor skill <path>` and `doctor plugin <path>` validate a single artifact (replacing the old `verify` command).

**trust/** — Trust tier resolution pipeline. `resolveTrust()` computes tier from npm attestation (Sigstore), GitHub attestation (`gh attestation verify`), and tap metadata. Injectable verify functions for testing. Injected into install/update flows as an optional post-download step.

**templates/** — TypeScript functions generating `Record<string, string>` (relPath → content). Embedded in the compiled binary (no runtime file reads). Three templates: `basicTemplate()`, `npmTemplate()`, `multiTemplate()`.

### Policy Module

**policy/compose.ts** — `composePolicy(config, flags) → Result<EffectivePolicy, UserError>`. Single canonical resolver for security decisions. Reads `[security]` (3 keys: `scan`, `on_warn`, `trust`) and `[scanner]` (4 keys: `agent_cli`, `ollama_model`, `threshold`, `max_size`). Resolves CLI flag overrides (`--strict`, `--deep`, `--skip-scan`, `--scope`). No agent-mode branch, no per-mode security selection, no preset resolution, no override array.

**policy/trust-glob.ts** — `composePolicyForSource(config, flags, source)` checks the requested install source against `security.trust` glob patterns (matched against tap name OR full source URL). Sources matching any glob skip Layer 1 and Layer 2 entirely. Returns `EffectivePolicy` with `scan = "none"` for trusted sources.

### Output Module

**core/src/output/types.ts** — `Output` interface (`info`, `warn`, `error`, `success`, `json`, `progress`) and `OutputMode` (`tty | plain | json`). Core functions never write to stdout/stderr. The CLI layer's `setupOutput(args)` (`packages/cli/src/ui/output/`) constructs the right implementation per mode. `pickMode(opts)` resolves from `--json` flag, TTY detection, and explicit override.

### Manifest + Sync Modules

**manifest/schemas.ts** — `ProjectManifestSchema` and `LockfileSchema`. Both have three top-level arrays: `[[skills]]`, `[[plugins]]`, `[[mcps]]`. Lockfile entries record exact resolved refs and SHAs.

**manifest/load.ts**, **save.ts** — Load/save `skilltap.toml` + `skilltap.lock` with `findProjectRoot()` integration. Atomic writes, ensureDirs.

**manifest/update.ts** — `addSkill(...)`, `addPlugin(...)`, `addMcp(...)`, and their `remove*` counterparts. All `install`/`remove`/`update`/`move`/`adopt`/`disable`/`enable` lifecycle commands write through these helpers when a project root is present.

**manifest/range.ts** — Parses and matches version ranges (`^`, `~`, `*`, exact tag, branch ref).

**manifest/publish.ts** — `discoverPublishablePlugins(repoRoot)` returns all `.skilltap/<name>.toml` with `publish = true`.

**manifest/recover.ts** — Reconstructs missing lockfile entries from state when the lockfile drifts.

**sync/drift.ts** — `detectDrift(state, manifest, lockfile)` → `DriftReport` covering all three state types (skills, plugins, mcps). Six drift cases per type: declared-not-installed, installed-not-declared, ref-mismatch, sha-mismatch, lockfile-only, lockfile-orphan.

**sync/plan.ts** — `planSync(manifest, lockfile, state)` → `SyncPlan` with action list and rationale per item.

**sync/apply.ts** — Executes the plan via existing install/remove/update machinery. Updates lockfile after each step. Plumbs through `onCaptureConfirm`/`onCaptureConflict` callbacks for plugin capture during sync.

### State Module

**state/schema.ts** — `StateSchema { version, skills: [], plugins: [], mcpServers: [] }`. Single store per scope at `~/.config/skilltap/state.json` (global) or `<project>/.agents/state.json` (project). Written by every lifecycle command.

### Migrate Module

**migrate/run.ts** — Top-level `migrate` orchestrator. Detects legacy markers (`installed.json`, `plugins.json`, `[security.human]`/`[security.agent]`, `[[security.overrides]]`, `[agent-mode]`, `[agent]`). If none, exits clean. Otherwise translates and writes current-format files; renames originals to `*.v1.bak` / `*.v2.bak`. Runs doctor post-migrate.

**migrate/config.ts** — Translates legacy configs:

| Source | Destination |
|---|---|
| `[security].scan` (top-level legacy) | `[security].scan` |
| `[security].on_warn` (top-level legacy) | `[security].on_warn` |
| `[security.<mode>]` (per-mode) | `[security]` (stricter mode wins; warn on mismatch) |
| `[security.<mode>].agent_cli` etc. | `[scanner].agent_cli` etc. |
| `[security].threshold` / `max_size` (top-level) | `[scanner].threshold` / `max_size` |
| `[[security.overrides]] preset = "none"` | `security.trust` glob entry |
| `[[security.overrides]] preset = relaxed/standard/strict` | dropped with warning |
| `[agent-mode]` | dropped with warning |
| `[agent]` | dropped with warning |
| `[registry].allow_npm` | dropped |
| `scan = "off"` | `scan = "none"` |
| `on_warn = "allow"` | `on_warn = "install"` |

**migrate/state.ts** — Reads legacy `installed.json` + `plugins.json`, writes unified `state.json`. Preserves any existing `state.mcpServers` (does not overwrite with `[]`).

**migrate/manifest.ts** — Verifies `skilltap.toml` shape parses against current schema.

### Agent-Plugins Module

**agent-plugins/types.ts** — `AgentPluginScanner` interface (`name`, `detect()`, `scan()`). Pluggable framework for scanning external plugin systems during `adopt`.

**agent-plugins/claude-code.ts** — Reads `~/.claude/plugins/installed_plugins.json` and `known_marketplaces.json`. Tolerant Zod parser with `passthrough()` for forward-compat. Doctor check warns when overlapping components exist between Claude Code's plugin store and skilltap state.

**agent-plugins/codex.ts** — Stub.

### Schemas (Zod 4)

All data boundaries are validated with Zod 4 schemas. Types are inferred from schemas — no separate interface definitions.

```typescript
import { z } from "zod/v4"

// [security] — policy. 3 keys.
export const SecurityConfigSchema = z.object({
  scan: z.enum(["semantic", "static", "none"]).default("static"),
  on_warn: z.enum(["prompt", "fail", "install"]).default("install"),
  trust: z.array(z.string()).default([]),
}).prefault({})

// [scanner] — operational. 4 keys.
export const ScannerConfigSchema = z.object({
  agent_cli: z.string().default(""),
  ollama_model: z.string().default(""),
  threshold: z.number().int().min(0).max(10).default(5),
  max_size: z.number().int().default(51200),
}).prefault({})

// Top-level config
export const ConfigSchema = z.object({
  defaults: ConfigDefaultsSchema,
  security: SecurityConfigSchema,
  scanner: ScannerConfigSchema,
  registry: RegistryConfigSchema,
  taps: z.array(TapEntrySchema).default([]),
  updates: UpdatesConfigSchema,
  telemetry: TelemetryConfigSchema,
  builtin_tap: z.boolean().default(true),
  verbose: z.boolean().default(true),
  default_git_host: z.string().default("https://github.com"),
})
```

Other canonical schemas:

- **state.json** — `StateSchema { version, skills: InstalledSkillSchema[], plugins: PluginRecordSchema[], mcpServers: [] }`.
- **skilltap.toml** — `ProjectManifestSchema` with `[[skills]]`, `[[plugins]]`, `[[mcps]]` arrays + `[targets]`.
- **skilltap.lock** — `LockfileSchema` mirroring the manifest with resolved refs/SHAs.
- **.skilltap/<name>.toml** — `SkilltapPluginManifestSchema` (native plugin format).
- **tap.json** — `TapSchema`, `TapSkillSchema`, `TapPluginSchema`.
- **marketplace.json** — `MarketplaceSchema` (Claude Code format, adapted to `Tap`).
- **plugin.json** (Claude Code / Codex) — `PluginManifestSchema` (unified internal representation).
- **SKILL.md frontmatter** — `SkillFrontmatterSchema`.
- **Agent response** — `AgentResponseSchema`.

Zod validates at every data boundary: parsing TOML config, reading `state.json`, parsing tap.json, parsing marketplace.json, parsing manifests, extracting SKILL.md frontmatter, and parsing agent CLI output. Adapter return values are validated before entering core logic.

### Adapter Interfaces

Adapters use standard TypeScript interfaces (not Zod) since they define behavior, not data:

```typescript
// Source adapter — resolves a user-provided source to a cloneable URL
interface SourceAdapter {
  name: string
  canHandle(source: string): boolean
  resolve(source: string): Promise<Result<ResolvedSource, UserError>>
}

// Agent adapter — invokes an LLM for semantic scanning
interface AgentAdapter {
  name: string
  cliName: string
  detect(): Promise<boolean>
  invoke(prompt: string): Promise<Result<AgentResponse, ScanError>>
}

// Agent-plugin scanner — discovers plugins managed by external agent systems
interface AgentPluginScanner {
  name: string
  detect(): Promise<boolean>
  scan(): Promise<DiscoveredAgentPlugin[]>
}
```

### Source Adapters

| Adapter | Handles | Resolution |
|---------|---------|------------|
| git | `https://`, `git@`, `ssh://` URLs | Pass-through (already a git URL) |
| npm | `npm:@scope/name[@version]` | Fetch tarball from npm registry, verify SHA-512 integrity |
| github | `github:owner/repo`, `owner/repo` shorthand | Resolve to `${default_git_host}/owner/repo.git` |
| local | Filesystem paths (`./`, `/`, `~/`) | Validate path exists, has SKILL.md or plugin manifest |

### Agent Adapters

| Agent | Binary | Invocation |
|-------|--------|------------|
| Claude Code | `claude` | `claude --print -p '<prompt>' --tools "" --output-format json` |
| Gemini CLI | `gemini` | `echo '<prompt>' \| gemini --non-interactive` |
| Codex CLI | `codex` | `codex --prompt '<prompt>' --no-tools` |
| OpenCode | `opencode` | `opencode --prompt '<prompt>'` |
| Ollama | `ollama` | `ollama run <model> '<prompt>'` |
| Custom | any path | Reads prompt from stdin, writes JSON to stdout |

See [SPEC.md — Agent Adapters](./SPEC.md#agent-adapters) for detection logic, first-use selection flow, JSON extraction, and custom binary support.

## Key Flows

These flows show how modules coordinate. See [SPEC.md](./SPEC.md#cli-commands) for the precise behavioral spec (flags, prompts, exit codes).

### Install Skill from URL

```
1. setupOutput(args) → Output (tty/plain/json)
2. composePolicy(config, flags) → EffectivePolicy
3. resolveSource(source, default_git_host) → { url, ref, adapter }
4. composePolicyForSource(config, flags, source) — trust-glob short-circuit
5. Find project root (smart scope default: project inside git, global outside)
6. Clone to temp dir (/tmp/skilltap-{random}/)
7. Scan for SKILL.md files (scanner.ts)
8. Skill selection (single → auto, multiple → onSelectSkills callback)
9. Security scan (static.ts; semantic.ts if scan = "semantic" or --deep)
10. Trust resolution (trust/) — npm provenance, GitHub attestation
11. Move skill directory to install path
    - Standalone repo → move entire temp clone
    - Multi-skill repo → copy skill dir, cache repo clone
12. Update state.json (skill slice) via saveSkillState
13. Update skilltap.toml + skilltap.lock if a project manifest exists
14. Create agent symlinks if --also (symlink.ts)
15. Clean up temp dir
```

### Install Plugin

```
1-6. Same as skill install through clone + smart scope resolution
7. detectPlugin(tempDir) — priority: .skilltap/<name>.toml → .claude-plugin/plugin.json → .codex-plugin/plugin.json
8. Multi-plugin selection: user/repo:plugin-name picks one; user/repo:* picks all publishable
9. Security scan all plugin content (skills + agent .md files + MCP commands)
10. Plugin capture detection (capture.ts):
    - Same-source collisions: atomic ownership transfer (auto-confirm)
    - Cross-source collisions: prompt in TTY, error non-interactive (--force-capture / --no-capture override)
11. For each skill: install via existing skill machinery
12. For each MCP server: inject into target agent configs (mcp-inject.ts)
    - Namespace: skilltap:<plugin-name>:<server-name>
    - Backup agent config before first write (.skilltap.bak)
13. For each agent definition: place .md in .claude/agents/
14. Update state.json (plugin slice)
15. Update skilltap.toml + skilltap.lock [[plugins]]
16. Clean up temp dir
```

### Install MCP

```
1. Resolve source through SourceAdapter
2. Clone, parse plugin manifest or .mcp.json for [[servers]]
3. Inject into target agent configs (mcp-inject.ts), namespace skilltap:<source>:<server>
4. Update state.mcpServers in state.json
5. Update skilltap.toml + skilltap.lock [[mcps]]
6. Smart-scope inferred: project inside git, global outside
```

### Sync

```
1. loadManifest(projectRoot) → manifest
2. loadLockfile(projectRoot) → lockfile (or recover from state if missing)
3. loadState(project) + loadState(global) → states
4. detectDrift(state, manifest, lockfile) → DriftReport (skills + plugins + mcps)
5. planSync(...) → SyncPlan
   - adds: declared but not in state
   - updates: declared at different ref than locked / installed
   - removes: in state but not declared (only with --prune)
   - lockfile-only: in lockfile but no state record (treat as add)
6. If interactive and plan non-empty: show diff, prompt to confirm
7. If --strict and plan non-empty: error out
8. applySync(plan, callbacks) — runs install/remove/update for each entry per type
9. Update lockfile if any range resolved to a new ref
10. Update state.json
11. Print summary
```

### Migrate

```
1. Detect legacy markers: [agent-mode], [agent], [security.human]/[security.agent], [[security.overrides]],
   v0.x installed.json, v0.x plugins.json, scan = "off", on_warn = "allow"
2. If no markers: exit 0 (nothing to migrate).
3. Read all legacy files (parse with legacy schemas in migrate/legacy-schemas.ts)
4. Translate config (migrate/config.ts) per the table above
5. Translate state (migrate/state.ts) — installed.json + plugins.json → state.json; preserve mcpServers
6. Verify manifest (migrate/manifest.ts) — skilltap.toml shape unchanged
7. Write translated files; rename originals to *.v1.bak / *.v2.bak
8. Run doctor to verify
9. Print migration summary with diff
```

### Update

```
1. Look up in state.json (skill slice) → repo URL, current SHA / npm version
2. git fetch (git.ts) or npm registry check (npm-registry.ts)
3. Compare HEAD SHA to FETCH_HEAD or installed version to latest
4. If different: show diff summary
5. Scan diff (static.ts)
6. Confirm update (or skip on --strict)
7. git pull (or tarball replace for npm)
8. Optionally run semantic scan on updated directory
9. Re-create agent symlinks
10. Re-resolve trust tier
11. Update state.json with new SHA / version / updatedAt
12. Update skilltap.toml + skilltap.lock if project manifest exists
```

## Storage Layout

```
~/.config/skilltap/
├── config.toml                  # User configuration
├── state.json                   # Canonical state — skills + plugins + mcpServers
├── taps/
│   ├── home/                    # Cloned tap repo (tap.json format)
│   ├── community/               # Another tap repo (tap.json format)
│   └── anthropic-skills/        # Marketplace repo (marketplace.json format)
└── cache/
    └── {repo-url-hash}/         # Cached full clones for multi-skill repos / plugins

~/.agents/skills/                # Global install directory (canonical)
├── commit-helper/               # Standalone — this IS the git clone
└── termtube-dev/                # Copied from multi-skill repo

~/.claude/skills/                # Agent-specific (symlinks)
├── commit-helper -> ~/.agents/skills/commit-helper/
└── termtube-dev -> ~/.agents/skills/termtube-dev/

~/.claude/agents/                # Agent definitions (plugin-installed, Claude Code only)
└── code-review.md

~/.claude/settings.json          # Agent config (MCP entries injected by skilltap, namespaced skilltap:*)

# Project scope
.agents/skills/                  # Same structure as global
.agents/state.json               # Project-scoped state
skilltap.toml                    # Project manifest (skills + plugins + mcps)
skilltap.lock                    # Lockfile mirroring manifest
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

The CLI layer catches results and formats them via `Output`. Core never writes to stdout/stderr directly.

See [SPEC.md — Error Handling](./SPEC.md#error-handling) for exit codes, error message format, and the full error condition table.

## Removed-Command Errors

Six retired command names exit non-zero with an explicit replacement hint:

| Removed | Replacement |
|---|---|
| `verify <path>` | `doctor skill <path>` (or `doctor plugin <path>`) |
| `link <path>` | `adopt <path>` (default track-in-place; `--move` to relocate) |
| `unlink <name>` | `remove skill <name>` |
| `enable <name>` | `toggle skill <name>` |
| `disable <name>` | `toggle skill <name>` |
| `skills <subcommand>` | top-level `list` / `info` / `remove skill` / `move` |

Type is explicit via `install mcp <source>`.

## Testing Strategy

**Unit tests** — Pure functions: scanner, security patterns, config parsing, TOML schema validation, manifest range matching, drift detection. Fast, no I/O.

**Integration tests** — Git operations with real repos (test fixtures via `test-utils`). Tap resolution, multi-skill scanning, symlink creation, manifest+lockfile round-trip, plugin capture flows.

**CLI tests** — Full subprocess tests via `runSkilltap` (pipe mode) and `runInteractive` (PTY mode for clack/Ink rendering). Both honor `SKILLTAP_TEST_BIN` so the same suite can run against the compiled binary via `bun run verify:binary:tests`.

**TUI tests** — `tui.smoke.test.ts` exercises the TUI through a PTY against the compiled binary. State machine reducers are testable directly with `bun:test` (no Ink rendering needed).

**Security scanner tests** — Known-malicious patterns from the SkillJect research and ClawHavoc incident. Regression suite for invisible Unicode, hidden HTML comments, base64-encoded shell, tag injection, suspicious URLs.

All tests run with `bun test`. CI runs source-mode and compiled-binary suites on Linux and macOS.

## Decision Log

| Decision | Choice | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Runtime | Bun | Node.js, Deno | Single binary compilation, fast, native TS |
| CLI framework | citty | commander.js, cac, clipanion | TypeScript-first, declarative, UnJS ecosystem |
| Top-down prompts | @clack/prompts | inquirer, prompts | Modern, beautiful output, maintained |
| Multi-screen TUI | Ink | Custom @clack orchestrator, lazygit-style | Persistent state across screens; clack is top-down only. lazygit overshoots. |
| Git interaction | Shell out | isomorphic-git | Auth inherited, simpler, no library edge cases |
| Config format | TOML | JSON, YAML | Human-friendly editing, clear sections |
| Validation | Zod 4 | io-ts, arktype, manual | Industry standard, infer types, great errors |
| Project structure | Monorepo | Single package | Core embeddable separately, clean test isolation |
| Security Unicode | anti-trojan-source + out-of-character | Custom regex | Battle-tested, maintained, cover edge cases |
| Semantic scan | Shell out to agent CLI | Direct API calls | Zero API key requirement, works with user's existing setup |
| Multi-skill install | Copy to install dir + cache repo | Symlink from cache | Cache is optimization not dependency; copy survives cache clean |
| npm provenance | sigstore-js | Direct Sigstore API | Reuse existing Sigstore ecosystem |
| Trust tier storage | Optional field in state.json (skill slice) | Separate trust file | Simplest structure; trust is per-install |
| Template format | TS functions returning Record<string,string> | Filesystem templates | Binary embeddable; no runtime file reads |
| Doctor checks | Streamed via onCheck callback | Parallel checks | Streaming UX; failures don't block other checks |
| Plugin scope | Portable subset (skills + MCP + agents) | Full plugin support | Portable components work cross-agent; platform-specific features are low value here |
| MCP namespacing | `skilltap:<plugin>:<server>` prefix | No prefix | Prevents collisions with user-configured MCP servers |
| State store | Single state.json per scope | Separate v0.x installed.json + plugins.json | One file per scope = easier backup, simpler doctor checks |
| Manifest format | TOML at project root with [[skills]]/[[plugins]]/[[mcps]] | JSON, single combined object | Matches existing config.toml conventions; human-friendly |
| Native plugin format | TOML in `.skilltap/<name>.toml` | JSON to match Claude Code | Skilltap's own files use TOML for consistency; Claude/Codex JSON formats remain readable inputs |
| Lockfile | Yes, Cargo-style | No lockfile, manifest only | Reproducibility is the headline value of `sync` |
| Sync drift | Prompt by default | Strict-by-default, additive-only | Prompt avoids destructive surprises while preserving deterministic value with `--yes` |
| Scope detection | Smart default (git → project) | Always prompt, always global | Most installs in a git repo are project-scoped; prompt fatigue is real |
| Security model | One [security] block + [scanner] | Per-mode split, presets, overrides | One rule for everyone; output style is a separate concern from policy |
| Single runtime | TTY/JSON drives output | Keep --agent flag, keep config block | A single runtime cuts duplicated orchestration; output mode is one decision |
| Install disambiguation | Required subcommand (skill/plugin/mcp) | Auto-detect, hybrid, --as flag | Explicit type means no auto-detect heuristics, no `mcp:` URL prefix, symmetric with remove/update/toggle |
| `verify` retirement | Fold into `doctor` | Keep separate | Single verb (`doctor`) with arg-based scope: env (no args) vs per-artifact (`doctor skill <path>`) |
| `link`/`unlink` retirement | Fold into `adopt` | Keep separate | `link <path>` and `adopt --track-in-place` did the same thing |
| Migration | Explicit `migrate` command, hard-fail on legacy | Auto-migrate on first run | Migration touches multiple files; users should be intentional |
| Adoption framework | Pluggable `AgentPluginScanner` | Hardcoded Claude Code path | Future agents will have plugin systems; pluggable scanner avoids retrofit |
| Bare `skilltap` | TUI dashboard (TTY only) | Print status text, error always | Matches lazygit/k9s conventions; headless callers use `skilltap status` |
| Output abstraction | `Output` interface | Mixed `successLine`/`agentSuccess` | Mode-specific behavior testable; prevents per-command output drift |
| Language migration | Stay TS+Bun | Migrate to Go/Rust | Skilltap pain is CLI ergonomics, not core. Core has substantial logic (sigstore, npm registry, security scanners, Zod schemas) that costs months to port for marginal runtime gain. |

## Architecture Risks

- **Ink stability under Bun** — Ink targets Node.js. Bun is mostly compatible but has had quirks with raw-mode terminal handling and signal cleanup. Mitigation: PTY-based smoke tests on every supported platform; fall back to clack-style top-down prompts if Ink misbehaves on a specific screen.
- **Claude Code plugin format drift** — `~/.claude/plugins/installed_plugins.json` is observable but undocumented. Mitigation: Zod parser uses `passthrough()` for unknown fields; doctor warns on unrecognized schema; graceful no-op when format diverges (don't crash, surface in doctor instead).
- **TUI testability** — multi-screen UI is harder to test than top-down prompts. Mitigation: state machine per screen lives in pure reducers (testable with `bun:test`); Ink components only render. PTY snapshot tests catch regressions in render output.
- **Output mode discipline** — risk of regressions where a developer writes directly to `process.stdout` instead of through `Output`. Mitigation: lint rule (or simple grep in CI) blocking direct stdout/stderr writes outside `cli/ui/output/`.
