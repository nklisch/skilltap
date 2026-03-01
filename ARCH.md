# Architecture

## Overview

skilltap is a CLI tool that installs agent skills from any git host. It clones repos, scans for SKILL.md files, runs security checks, and places skills in the universal `.agents/skills/` directory.

This document describes how skilltap is built internally — module boundaries, data flow, key abstractions, and technology decisions.

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
4. Homebrew formula (future)

## Monorepo Structure

Bun workspaces with three packages:

```
skilltap/
├── packages/
│   ├── core/                   # Library — all business logic
│   │   ├── src/
│   │   │   ├── git.ts          # Git operations (clone, pull, fetch, diff)
│   │   │   ├── scanner.ts      # Skill discovery (find SKILL.md in repos)
│   │   │   ├── security/
│   │   │   │   ├── static.ts   # Layer 1 — pattern matching
│   │   │   │   ├── semantic.ts # Layer 2 — agent-based evaluation
│   │   │   │   └── patterns.ts # Detection patterns (Unicode, URLs, etc.)
│   │   │   ├── config.ts       # Config read/write (TOML)
│   │   │   ├── install.ts      # Install/remove/update logic
│   │   │   ├── taps.ts         # Tap management
│   │   │   ├── symlink.ts      # Agent-specific symlink creation
│   │   │   ├── schemas/
│   │   │   │   ├── config.ts   # config.toml Zod schema
│   │   │   │   ├── installed.ts # installed.json Zod schema
│   │   │   │   ├── tap.ts      # tap.json Zod schema
│   │   │   │   ├── skill.ts    # SKILL.md frontmatter Zod schema
│   │   │   │   └── agent.ts    # Agent response Zod schema
│   │   │   ├── adapters/
│   │   │   │   ├── types.ts    # SourceAdapter interface
│   │   │   │   ├── git.ts      # Git URL adapter
│   │   │   │   ├── github.ts   # GitHub shorthand adapter
│   │   │   │   ├── npm.ts      # npm tarball adapter (future)
│   │   │   │   └── local.ts    # Local path adapter
│   │   │   ├── agents/
│   │   │   │   ├── types.ts    # AgentAdapter interface
│   │   │   │   ├── detect.ts   # Auto-detect installed agents
│   │   │   │   ├── claude.ts   # Claude Code adapter
│   │   │   │   ├── gemini.ts   # Gemini CLI adapter
│   │   │   │   ├── codex.ts    # Codex CLI adapter
│   │   │   │   ├── opencode.ts # OpenCode adapter
│   │   │   │   └── ollama.ts   # Ollama adapter (local models)
│   │   │   └── types.ts        # Shared types
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── cli/                    # CLI entry point — commands and UI
│   │   ├── src/
│   │   │   ├── index.ts        # Entry point (runMain)
│   │   │   ├── commands/
│   │   │   │   ├── install.ts
│   │   │   │   ├── remove.ts
│   │   │   │   ├── list.ts
│   │   │   │   ├── update.ts
│   │   │   │   ├── find.ts
│   │   │   │   ├── link.ts
│   │   │   │   ├── unlink.ts
│   │   │   │   ├── info.ts
│   │   │   │   ├── config/
│   │   │   │   │   ├── index.ts      # skilltap config wizard
│   │   │   │   │   └── agent-mode.ts # skilltap config agent-mode wizard
│   │   │   │   └── tap/
│   │   │   │       ├── add.ts
│   │   │   │       ├── remove.ts
│   │   │   │       ├── list.ts
│   │   │   │       ├── update.ts
│   │   │   │       └── init.ts
│   │   │   └── ui/
│   │   │       ├── format.ts   # Output formatting (tables, colors)
│   │   │       ├── agent-out.ts # Agent mode plain text output
│   │   │       ├── prompts.ts  # @clack/prompts wrappers
│   │   │       └── scan.ts     # Security scan result display
│   │   ├── package.json        # Published as "skilltap" on npm
│   │   └── tsconfig.json
│   └── test-utils/             # Shared test fixtures and helpers
│       ├── src/
│       │   ├── fixtures.ts     # Create mock repos, skills, taps
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
└── SPEC.md
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
- `clone(url, dest, opts)` — `git clone --depth 1` (shallow by default)
- `pull(dir)` — `git pull`
- `fetch(dir)` — `git fetch`
- `diff(dir, from, to)` — `git diff from..to`
- `revParse(dir)` — current HEAD SHA
- `log(dir, n)` — last n commits

**scanner.ts** — Finds SKILL.md files in a directory tree. Returns structured results with name, description (from frontmatter), and path. See [SPEC.md — Skill Discovery](./SPEC.md#skill-discovery) for the scanning algorithm.

**security/static.ts** — Layer 1 pattern matching. Takes file contents, returns warnings with line numbers, category, and raw/visible text. Uses `anti-trojan-source` and `out-of-character` for Unicode detection, regex for everything else. See [SPEC.md — Layer 1](./SPEC.md#layer-1-static-analysis) for detection categories.

**security/semantic.ts** — Layer 2 agent-based evaluation. Chunks content, invokes agent adapter, aggregates scores. See [SPEC.md — Layer 2](./SPEC.md#layer-2-semantic-scan) for the chunking algorithm and security prompt.

**config.ts** — Reads/writes `~/.config/skilltap/config.toml` and `~/.config/skilltap/installed.json`. Ensures directories exist on first use.

**install.ts** — Orchestrates the install/remove/update flows. Coordinates git, scanner, security, config, and symlink modules.

**taps.ts** — Manages tap repos. Clone, pull, parse `tap.json`, search across taps.

**symlink.ts** — Creates and removes symlinks for agent-specific directories. Knows the path conventions for each supported agent.

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
- [tap.json](./SPEC.md#tapjson) — `TapSchema`, `TapSkillSchema`
- [SKILL.md frontmatter](./SPEC.md#skillmd-parsing) — `SkillFrontmatterSchema`
- [Agent response](./SPEC.md#json-extraction) — `AgentResponseSchema`

Zod validates at every data boundary: parsing TOML config, reading installed.json, parsing tap.json, extracting SKILL.md frontmatter, and parsing agent CLI output. Adapter return values are validated before entering core logic.

### Adapter Interfaces

Adapters use standard TypeScript interfaces (not Zod) since they define behavior, not data:

```typescript
// Source adapter — resolves a user-provided source to a cloneable URL
interface SourceAdapter {
  name: string;
  canHandle(source: string): boolean;
  resolve(source: string): Promise<ResolvedSource>;  // validated by Zod
}

// Agent adapter — invokes an LLM for semantic scanning
interface AgentAdapter {
  name: string;
  cliName: string;   // binary name on PATH
  detect(): Promise<boolean>;
  invoke(prompt: string): Promise<AgentResponse>;    // validated by Zod
}
```

### Source Adapters (v0.1)

| Adapter | Handles | Resolution |
|---------|---------|------------|
| git | `https://`, `git@`, `ssh://` URLs | Pass-through (already a git URL) |
| github | `github:owner/repo`, `owner/repo` shorthand | Resolve to `https://github.com/owner/repo.git` |
| local | Filesystem paths (`./`, `/`, `~/`) | Validate path exists, has SKILL.md |

### Agent Adapters (v0.1)

| Agent | Binary | Invocation |
|-------|--------|------------|
| Claude Code | `claude` | `claude --print -p '<prompt>' --no-tools --output-format json` |
| Gemini CLI | `gemini` | `gemini --prompt '<prompt>'` |
| Codex CLI | `codex` | `codex --prompt '<prompt>'` |
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
5. Skill selection (single → auto, multiple → prompt or auto with --yes)
6. Scope resolution (--project, --global, or prompt)
7. Security scan (static.ts, optionally semantic.ts)
8. Move skill directory to install path
   - Standalone repo → move entire temp clone
   - Multi-skill repo → copy skill dir, cache repo clone
9. Update installed.json (config.ts)
10. Create agent symlinks if --also (symlink.ts)
11. Clean up temp dir
```

### Install from Tap Name

```
1. Load all taps, parse tap.json (taps.ts)
2. Search for name across all taps
3. Resolve to repo URL (single match → use, multiple → prompt)
4. → Continue from step 2 of "Install from URL"
```

### Update

```
1. Look up in installed.json → get repo URL, current SHA
2. git fetch (git.ts)
3. Compare HEAD SHA to remote SHA
4. If different: show diff summary → scan diff (static.ts) → apply if accepted
5. Update installed.json with new SHA
```

## Storage Layout

```
~/.config/skilltap/
├── config.toml                  # User configuration
├── installed.json               # Installation state (machine-managed)
├── taps/
│   ├── home/                    # Cloned tap repo
│   │   ├── tap.json
│   │   └── .git/
│   └── community/
│       ├── tap.json
│       └── .git/
└── cache/
    └── {repo-url-hash}/        # Cached full clones for multi-skill repos
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

.agents/skills/                  # Project-scoped (same structure)
└── project-skill/
    └── SKILL.md
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

**CLI tests** — Snapshot tests for command output formatting. Mock core functions, verify terminal output.

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
| Platform | Linux + macOS | Cross-platform | Ship fast, add Windows when demand exists |
