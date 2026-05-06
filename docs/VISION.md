# skilltap

A CLI for installing agent skills and plugins from any git host. Git-native, agent-agnostic, multi-source. Installs skills individually or as plugin bundles (skills + MCP servers + agents) across multiple agent platforms.

## Problem

The SKILL.md format is standardized across 40+ agents (Claude Code, Cursor, Codex CLI, Gemini CLI, etc.), but distribution is fragmented. skills.sh only indexes GitHub. Other tools are centralized hosted services. There's no way to point at your own Gitea/GitLab instance and install from there.

Individual agents are starting to build their own distribution — Claude Code has a full [plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces) system with `marketplace.json`, `/plugin install`, and support for git, npm, and pip sources. But these are agent-specific. There's no agent-agnostic tool for distributing skills to whatever agent(s) you use.

## Core Idea

**Skills are git repos. Git is the transport. The CLI just clones and links.**

Think: **Homebrew taps for agent skills.**

skilltap installs to the universal `.agents/skills/` directory defined by the [Agent Skills spec](https://agentskills.io/specification). This is the agent-agnostic path that works across all conforming agents. If you also want skills in an agent-specific directory (`.claude/skills/`, `.cursor/skills/`), you can opt in to symlinking.

skilltap also understands **plugins** — bundles that package skills alongside MCP servers and agent definitions. When you install a plugin, skilltap extracts the portable components (skills, MCP configs, agents) and installs them into each target agent platform. You can then toggle individual components on/off within an installed plugin.

## Design Principles

1. **Git-native.** Clone, shallow-clone, pull. No custom protocol. Git already handles versioning, auth, and distribution.
2. **Agent-agnostic.** Installs to `.agents/skills/` by default — the universal path. Not tied to any single agent's ecosystem.
3. **Multi-source.** Configure multiple sources (taps) — your Gitea, a friend's GitLab, public GitHub repos. Search across all of them.
4. **Minimal.** No scoring, no benchmarking, no composition engine. Clone repos, make links. That's it.

## How It Works

```
┌─────────────┐  ┌─────────────┐  ┌──────────────┐
│  Gitea       │  │  GitHub      │  │  GitLab      │
│  (private)   │  │  (public)    │  │  (team)      │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │
       └─────────┬───────┴─────────────────┘
                 │  git clone
           ┌─────▼──────┐
           │  skilltap   │
           └─────┬───────┘
                 │
                 ▼
          ~/.agents/skills/       ← universal (default)
                 │
                 │  optional symlinks
      ┌──────────┼──────────┐
      ▼          ▼          ▼
 ~/.claude/  ~/.cursor/  ~/.codex/
  skills/    skills/     skills/
```

### A skill = a directory with SKILL.md

A skill can live anywhere — as a standalone repo, or inside a larger project:

```
# Standalone skill repo
commit-helper/
  SKILL.md              # required
  scripts/              # optional
  templates/            # optional
  REFERENCE.md          # optional

# Skills inside a code repo (co-located with the project they're for)
termtube/
  src/
  tests/
  .agents/skills/
    termtube-dev/
      SKILL.md
    termtube-review/
      SKILL.md
```

No build step. No manifest file. If it has a SKILL.md, it's a skill.

### A plugin = a bundle of skills + MCP servers + agents

A plugin is a repo (or directory) containing a `plugin.json` manifest that groups skills with MCP server configs and agent definitions. skilltap reads both Claude Code (`.claude-plugin/plugin.json`) and Codex (`.codex-plugin/plugin.json`) formats.

```
# Claude Code plugin
my-plugin/
  .claude-plugin/
    plugin.json         # manifest (name, components, user config)
  skills/
    helper/SKILL.md
    reviewer/SKILL.md
  .mcp.json             # MCP server definitions
  agents/
    code-review.md      # Agent definition (Claude Code format)

# Codex plugin
my-plugin/
  .codex-plugin/
    plugin.json
  skills/
    helper/SKILL.md
  .mcp.json
```

skilltap extracts the portable subset: **skills** (installed via the existing skill system), **MCP servers** (injected into each target agent's config), and **agents** (placed in agent-specific directories, Claude Code for now). Platform-specific components (hooks, LSP, commands, output styles) are ignored.

After installing, you can toggle individual components — disable a specific MCP server or skill within the plugin without removing the whole thing.

### Repo scanning

When you point skilltap at a repo, it scans for all SKILL.md files and lets you choose which to install. Single-skill repos install directly; multi-skill repos prompt for selection.

See [SPEC.md — Skill Discovery](./SPEC.md#skill-discovery) for the full scanning algorithm and SKILL.md frontmatter validation.

### A tap = a git repo listing other skills

A tap is a curated index — a git repo containing a `tap.json` that lists skill names, descriptions, repo URLs, and tags. Taps are how you share a curated collection. Your friends add your tap, they see your skills.

See [SPEC.md — tap.json](./SPEC.md#tapjson) for the format specification.

## CLI

skilltap provides commands for installing, removing, updating, linking, and searching for skills and plugins, plus tap management.

See [UX.md](./UX.md) for the full command tree, flag combinations, and interactive prompt flows. See [SPEC.md](./SPEC.md#cli-commands) for the precise behavioral specification of each command.

**Quick examples:**

```bash
# Install from any git URL
skilltap install https://gitea.example.com/user/commit-helper

# Install from a tap by name
skilltap install commit-helper

# GitHub shorthand
skilltap install user/repo

# Project-scoped + agent symlinks
skilltap install commit-helper --project --also claude-code

# Search across all taps
skilltap find review

# Link a local skill for development
skilltap link . --also claude-code

# Install a plugin (auto-detected from plugin.json)
skilltap install user/dev-toolkit --also claude-code --also cursor

# Toggle plugin components
skilltap plugin toggle dev-toolkit

# List installed plugins
skilltap plugin
```

## Security Scanning

Every install runs a multi-layer scan before writing anything to disk. Nothing is blocked outright — the user always decides — but suspicious content is surfaced with context.

### Layer 1 — Static analysis (instant, no LLM)

Fast pattern matching that runs on every install. Detects invisible Unicode, hidden HTML/CSS, markdown hiding tricks, obfuscation (base64, hex, variable expansion), suspicious URLs (known exfiltration services), dangerous shell patterns, tag injection attempts, and suspicious file types/sizes.

Warnings show the raw escaped content inline (so you can see what's hiding) and the file path + line number. The source is cloned to a temp dir before anything is installed.

See [SPEC.md — Layer 1: Static Analysis](./SPEC.md#layer-1-static-analysis) for the complete detection pattern reference.

### Layer 2 — Semantic scan (opt-in, uses the user's own agent)

When static analysis finds warnings, or when the user wants deeper assurance, skilltap can use their locally installed agent CLI to evaluate the skill's intent.

**How it works:**

1. **Chunk** the skill content into small blocks (~200–500 tokens). Pre-scan for tag injection attempts and auto-flag if found.
2. **Send each chunk** to the user's agent in an isolated context — fresh session, no tools, no file access, randomized security wrapper tags.
3. **Aggregate scores** across all chunks. Flag anything above threshold (default: 5).
4. **The user decides.** skilltap informs, never blocks.

See [SPEC.md — Layer 2: Semantic Scan](./SPEC.md#layer-2-semantic-scan) for the full chunking algorithm, security prompt template, and agent invocation details.

**Why chunking matters:**
- A full skill can be thousands of tokens — attackers hide malicious instructions in the middle of legitimate content hoping they get lost in context
- Small chunks force focused evaluation on each section
- Each chunk is evaluated independently — no cross-contamination between sections
- Parallelizable — send all chunks concurrently for speed

**Why the user's own agent:**
- Zero infrastructure — no API keys, no external service, no skilltap account
- Works offline if the agent supports it
- The user already trusts and pays for their agent
- No data leaves the user's machine beyond what their agent already does

### Layer 3 — Community signals (future)

Taps could optionally carry trust metadata (`verified`, `reviewedBy`). Social trust signal, not a security guarantee.

### Additional hardening

- **Scan the entire skill directory**, not just SKILL.md — 91% of real attacks hide payloads in auxiliary files
- **Flag non-plaintext files** — binaries, compiled code, minified JS
- **Size limits** — flag skills over configurable threshold (default 50KB)
- **Diff on update** — `skilltap update` shows what changed and re-scans the diff

See [SPEC.md — Security Scanning](./SPEC.md#security-scanning) for the full specification including detection categories, warning output format, and configuration options.

## Auth

Follows git's auth model — no custom auth layer:

1. Git credential helpers (already configured for your hosts)
2. SSH keys (for `git@` URLs)
3. Token-in-URL (for HTTPS, e.g. Gitea access tokens)
4. `GH_TOKEN` / `GITLAB_TOKEN` env vars (for API-based search)

## HTTP Registry (removed in v2.0 — historical reference only)

> **Note (v2.0):** The HTTP registry adapter was removed in v2.0. Taps are git-only. v0.x configs with `type = "http"` are silently filtered with a one-time stderr warning, and `skilltap migrate` lists them as needing manual conversion. The section below describes the original v0.x design and is retained for historical reference; nothing here is currently implemented.

Git is the primary transport, but some environments can't use git (locked-down CI, browser-based tools, corporate proxies). For these cases, skilltap also supports a simple HTTP registry — any server implementing a handful of JSON endpoints.

**No existing standard for skill registries exists.** The closest prior art is the [MCP Registry API](https://registry.modelcontextprotocol.io/) and Anthropic's [`marketplace.json`](https://code.claude.com/docs/en/plugin-marketplaces) format. The spec below borrows from both where it makes sense.

### Endpoints

A minimal HTTP registry implements three endpoints:

```
GET /skilltap/v1/skills
GET /skilltap/v1/skills/{name}
GET /skilltap/v1/skills/{name}/download
```

#### List skills

```
GET /skilltap/v1/skills?q=search&tag=git&limit=50&cursor=abc
```

```json
{
  "skills": [
    {
      "name": "commit-helper",
      "description": "Generates conventional commit messages",
      "version": "1.2.0",
      "author": "nathan",
      "tags": ["git", "productivity"],
      "source": {
        "type": "git",
        "url": "https://gitea.example.com/nathan/commit-helper",
        "ref": "v1.2.0"
      }
    }
  ],
  "total": 42,
  "cursor": "next-page-token"
}
```

#### Skill detail

```
GET /skilltap/v1/skills/commit-helper
```

```json
{
  "name": "commit-helper",
  "description": "Generates conventional commit messages",
  "author": "nathan",
  "license": "MIT",
  "tags": ["git", "productivity"],
  "versions": [
    { "version": "1.2.0", "publishedAt": "2026-02-28T12:00:00Z" },
    { "version": "1.1.0", "publishedAt": "2026-01-15T12:00:00Z" }
  ],
  "source": {
    "type": "git",
    "url": "https://gitea.example.com/nathan/commit-helper"
  }
}
```

#### Download

```
GET /skilltap/v1/skills/commit-helper/download?version=1.2.0
```

Returns a `.tar.gz` containing the skill directory.

### Source types

The `source` object in registry responses uses the same types as Anthropic's `marketplace.json`:

```json
{ "type": "git", "url": "https://...", "ref": "v1.0.0" }
{ "type": "github", "repo": "owner/repo", "ref": "main" }
{ "type": "npm", "package": "@scope/name", "version": "^1.0.0" }
{ "type": "url", "url": "https://example.com/skill.tar.gz" }
```

### Static hosting

An HTTP registry can be a directory of static files behind any web server:

```
registry/
  skilltap/v1/
    skills.json                           # → GET /skilltap/v1/skills
    skills/
      commit-helper.json                  # → GET /skilltap/v1/skills/commit-helper
      commit-helper/
        commit-helper-1.2.0.tar.gz        # → GET .../download
```

### Adding an HTTP registry

```bash
skilltap tap add company https://skills.company.com/skilltap/v1 --type http
```

The CLI auto-detects the type: if the URL points to a git repo, it's a git tap. If it returns JSON with a `skills` array, it's an HTTP registry.

## What This Is NOT

- **Not a package manager.** No dependencies, no build step, no install scripts.
- **Not a marketplace.** No centralized index. Taps are just git repos anyone can create.
- **Not a runtime.** Skills are static files. No execution engine.
- **Not a full plugin runtime.** Claude Code and Codex have their own plugin systems with hooks, channels, LSP, and other platform-specific features. skilltap reads their plugin formats but only installs the portable components (skills, MCP servers, agents). For the full platform-specific experience, use each agent's native plugin system.

## Prior Art

| Project | Relationship |
|---------|-------------|
| [Agent Skills spec](https://agentskills.io/specification) | The SKILL.md format we distribute. The standard across 40+ agents. |
| [Claude Code plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces) | Claude Code's built-in system for distributing plugins (which include skills). Agent-specific. Source types in our HTTP registry align with theirs. |
| [MCP Registry API](https://registry.modelcontextprotocol.io/) | Most formally specified registry API. Cursor-based pagination pattern borrowed for our HTTP registry. |
| [MCP skills.json proposal](https://github.com/modelcontextprotocol/registry/discussions/895) | Proposed extending MCP Registry for skills. Not adopted. |
| [Homebrew taps](https://docs.brew.sh/Taps) | Direct inspiration for the git-repo-as-index tap model. |
| [skills.sh](https://skills.sh/) | GitHub-only CLI. No self-hosting, no registry API. Passive telemetry leaderboard. |
| [Skillshub](https://github.com/EYH0602/skillshub) | Rust CLI with tap support. Similar direction, less mature. |
| [ClawHub](https://github.com/openclaw/clawhub) | Largest index (13k+ skills). Convex backend, no open API spec. Had security incident (4.4% malicious). |
| [OpenAI Skills API](https://developers.openai.com/api/docs/guides/tools-skills/) | Proprietary REST API. Cloud-only. |

## Landscape

**Skill format**: Settled. The [Agent Skills spec](https://agentskills.io/specification) (SKILL.md) is adopted by Anthropic, OpenAI, Google, GitHub, Cursor, and 30+ others.

**Agent-specific distribution**: Emerging. Claude Code has a full plugin marketplace. OpenAI has a REST API. But these only work within their own agent.

**Agent-agnostic distribution**: No standard. skills.sh is GitHub-only. Skillshub is Rust/early. ClawHub is centralized. Nobody has shipped a simple, self-hostable, git-native tool that installs to the universal `.agents/skills/` path.

That's the gap skilltap fills.

## Scope

See [SPEC.md — Version Scope](./SPEC.md#version-scope) for the detailed roadmap. In brief:

- **v0.1** — Core install/remove/update/link + taps + security scanning (static + semantic) + standalone binary
- **v0.2** — npm adapter, HTTP registry, shell completions
- **v0.3** — Community trust signals, `skilltap publish`, skill templates
- **v1.0** — Plugin support (Claude Code + Codex formats, MCP injection, agent definitions)
- **v2.0** — Tooling-surface redesign: unified package model, simplified security, drop agent-mode-as-concept, expanded MCP story (see below)

---

## v2.0 Direction: Simplification, Unification, Project Manifest

> **Status note (post-cutover):** This section describes the **original v2.0 design intent**. Phase 31 shipped much of it — the project manifest (§1), the plugin/skill management upgrades (§2), the new helper commands (§6), the doctor v2 checks (§6), the `skilltap try` and `skilltap migrate` additions (§6), MCP-only install (§5), and the project manifest workflow are all live. **However, sections §3 (Drop "agent mode"), §4 (Simpler security), and parts of §7 (What's deprecated) were deferred.** Phase 31c-c-2 took simpler paths: the `--agent` flag and `SKILLTAP_AGENT=1` env var were added on top of the existing v0.x per-mode security and `[agent-mode]` config block, rather than replacing them. See [SPEC.md — v2.0 Security](./SPEC.md#v20-security), [v2.0 Configuration](./SPEC.md#v20-configuration), and [v2.0 Removed Features](./SPEC.md#v20-removed-features) for the actual shipped behavior. The original-intent text below is retained as the design rationale.

By v1.0, skilltap had grown three parallel concepts (skill, plugin, tap), two security modes (human, agent), four security presets, two manifest formats we read (Claude Code, Codex) plus two we publish (tap.json, marketplace.json), and an "agent mode" with its own scope and security blocks. It worked, but the surface was wide enough that a new user couldn't predict what `skilltap install foo` would do without reading the spec.

v2.0 keeps skill and plugin as the two parallel user-facing concepts (no forced unification — they really are different shapes), keeps **tap** as the canonical name for a curated git index, and reorganizes everything else around a project manifest.

### 1. Project manifest (`skilltap.toml`) + Cargo-style sync

The headline addition. Every project gets a `skilltap.toml` declaring the skills, plugins, and taps the project depends on, plus its default agent targets. A companion `skilltap.lock` records exact resolved refs.

- `skilltap install <thing>` adds to the manifest and lockfile (like `cargo add`).
- `skilltap sync` installs from the lockfile, prompts on any drift between declared / locked / installed.
- `skilltap update` refreshes the lockfile to the latest matching range.
- A teammate clones the repo, runs `skilltap sync`, and gets the exact same skill setup the project was built against.

The manifest also opens a clean path for **publishing**: a repo can opt-in to being installable as a plugin by placing one or more `.skilltap/<plugin>.toml` files (TOML, native to skilltap). Multiple plugins per repo are supported; the bare `user/repo` reference prompts the user to pick when several are publishable, or `user/repo:plugin-name` selects directly. Plugins are NOT publishable by default — `publish = true` must be set explicitly.

Existing `.claude-plugin/plugin.json` and `.codex-plugin/plugin.json` formats keep working as input — skilltap reads them, normalizes internally, and treats them like any other plugin source.

### 2. Skill vs plugin: still two concepts, easier to manage them

A skill is one SKILL.md plus assets. A plugin is a bundle (skills + MCP servers + agent definitions). Both stay first-class — no forced merge. What v2.0 changes is *managing* them:

- **`skilltap` (no args) prints a status dashboard** — managed skills, plugins (with component status), MCP servers injected per agent, taps configured, updates available, scope. One screen, one read. Like `git status`.
- **Component-ref syntax**: `skilltap toggle dev-toolkit:test-generator`, `skilltap enable foo:bar`, `skilltap disable foo:bar` for direct addressing without going through a picker.
- **Smart scope default**: inside a git repo, default scope is `project`; outside, `global`. The inferred scope is always shown in output (no surprises).
- **Single state file per scope**: `installed.json` + `plugins.json` collapse to one `state.json`. Easier to back up, easier to reason about. Config stays in TOML.
- **Top-level commands for daily use**: `install`, `remove`, `list`, `info`, `sync`, `status`, `toggle`, `enable`, `disable`, `update`, `find`, `try`. The `tap/` and `config/` groups stay; `skills/` and `plugin/` retain their groups for less common operations (`adopt`, `move`, `info`, `toggle`, `remove`) but with top-level shortcuts. Old command paths remain as silent aliases.

### 3. Drop "agent mode" as a concept

Agent mode in v1.0 was a config-only switch with its own security block, its own scope, and its own behavioral contract. v2.0 replaces it with one mechanism that any caller — human, AI agent, CI, cron — can use:

- A `--agent` flag.
- A `SKILLTAP_AGENT=1` env var with the same effect.
- Config keys: `agent.default = true|false` makes the flag sticky (always on); `agent.block = true|false` causes the CLI to refuse `--agent` (useful for shared workstations where a human wants interactivity locked in).
- The flag turns off prompts, switches to plain text output, and forces non-interactive defaults (auto-pick when only one option, error when multiple).

There's no separate `[security.agent]` block, no separate scope, no special bypass rules. **Security policy is the same regardless of whether `--agent` is set** — one rule for everyone. If a security warning would prompt a human, it would prompt an agent too (and the agent gets the non-interactive equivalent: error out unless `on_warn = "install"`).

### 4. Simpler security

v1.0 layered six things into security: per-mode config, presets, trust-tier overrides, semantic scan as a first-class config option, randomized wrapper tags, parallel chunked evaluation. Almost none of it was necessary for the 95% case.

v2.0:

- One `[security]` block. Three keys.
  - `scan` ∈ {`semantic`, `static`, `none`} — default `static`.
  - `on_warn` ∈ {`prompt`, `fail`, `install`} — default `install` (scan but proceed; warnings are reported, not blocking).
  - `trust = []` — list of glob patterns matching tap names or source URLs that skip scanning entirely.
- Drop the `[security.human]` / `[security.agent]` split.
- Drop the four-preset table (none/relaxed/standard/strict).
- Drop `[[security.overrides]]` (the kind/match/preset triple) — replaced by the simpler `trust` allowlist.
- Drop `require_scan` — replaced by removing scan = none from your config if you actually want it required.

Static scan stays on by default. Semantic stays available (just set `scan = "semantic"` or pass `--deep` per call) but is no longer the recommended default — it's the heavy option, not the default knob.

### 5. MCP as a first-class story

v1.0 supported MCP injection for five agent platforms but treated MCP as a side-effect of plugin install. v2.0 promotes it:

- **Claude Desktop** added to the supported targets (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS).
- `skilltap install mcp:<source>` — standalone MCP install for users who want only the server, no skill machinery.
- MCP servers can be declared inline in a project's `.skilltap/<plugin>.toml` — your project's own dev tools become a first-class skilltap plugin without needing a separate repo.

### 6. New helper commands

- **`skilltap try <repo>`** — read-only preview. Clones to temp, displays manifest / SKILL.md / plugin contents, runs scan, never writes to install paths. Inspect before commit.
- **`skilltap sync`** — described above. Reconciles manifest ↔ lockfile ↔ installed state.
- **`skilltap status`** — same content as bare `skilltap`, but explicit and pipe-friendly. `--json` for machine output.
- **`skilltap migrate`** — one-shot v1.0 → v2.0 upgrade. v2.0 does not auto-migrate; if v1.0 state is detected, the CLI errors with a hint to run migrate (or stay on v1.x).
- **Doctor upgrades** — `skilltap doctor` adds checks for: declared-but-not-installed manifests, drifted lockfile (lockfile records SHAs that don't match what's installed), `.skilltap/` plugin dirs missing required fields, MCP injection inconsistencies (server in state but not in agent config or vice versa).

### What stays the same

- Git is still the transport. Clone and link.
- `.agents/skills/` is still the canonical install path.
- Source adapters (git, github, npm, local) are unchanged. **HTTP registry adapter is removed** — taps are git-only.
- Agent symlinks (`.claude/skills/`, `.cursor/skills/`, etc.) are unchanged.
- Existing `.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `tap.json`, `marketplace.json` formats keep working as inputs.
- Telemetry behavior is unchanged from v1.0.

### What's deprecated or removed (original design vs shipped)

> **Status:** Bullets below were the v2.0 design intent. Two shipped, six were deferred. Verified against `core/src/schemas/config.ts` and CLAUDE.md.

**Actually removed:**
- HTTP registry tap type — removed in Phase 31b. ✓
- `installed.json` and `plugins.json` separate canonical files — merged into `state.json` in Phase 31c-c-2d-1. v0.x files remain as one-time read-fallback for unmigrated users. ✓

**Originally planned but kept in v2.1:**
- The `[security.human]` / `[security.agent]` split was kept — composePolicy still routes per-mode. The single `[security]` block is deferred.
- Security presets (none/relaxed/standard/strict) were kept — applied via `skilltap config security --preset` and trust overrides.
- `[[security.overrides]]` was kept — the `trust = []` glob design (in `policy-v2/trust-glob.ts`) is reserved scaffolding, not wired.
- `[agent-mode]` config block was kept (slated for v2.2 retirement). The proposed `[agent]` block with `default` / `block` was never built.
- `skilltap config agent-mode` interactive wizard was kept — remains the persistent-default entry point. The `--agent` flag and `SKILLTAP_AGENT=1` env var were added (Phase 31c-c-2c) as **per-invocation alternatives**, not replacements.
- The "human mode vs agent mode" mental model was kept — per-mode policy is the architecture. `--agent` activates the agent-mode block.
- **v1.0** — Plugin support: read Claude Code and Codex plugin formats, install skills + MCP servers + agents as a group, component-level toggle
