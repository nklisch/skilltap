# skilltap

A simple CLI for installing agent skills from any git host. Git-native, agent-agnostic, multi-source.

## Problem

The SKILL.md format is standardized across 40+ agents (Claude Code, Cursor, Codex CLI, Gemini CLI, etc.), but distribution is fragmented. skills.sh only indexes GitHub. Other tools are centralized hosted services. There's no way to point at your own Gitea/GitLab instance and install from there.

Individual agents are starting to build their own distribution — Claude Code has a full [plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces) system with `marketplace.json`, `/plugin install`, and support for git, npm, and pip sources. But these are agent-specific. There's no agent-agnostic tool for distributing skills to whatever agent(s) you use.

## Core Idea

**Skills are git repos. Git is the transport. The CLI just clones and links.**

Think: **Homebrew taps for agent skills.**

skilltap installs to the universal `.agents/skills/` directory defined by the [Agent Skills spec](https://agentskills.io/specification). This is the agent-agnostic path that works across all conforming agents. If you also want skills in an agent-specific directory (`.claude/skills/`, `.cursor/skills/`), you can opt in to symlinking.

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

### Repo scanning

When you point skilltap at a repo, it scans for all SKILL.md files and lets you choose which to install. Single-skill repos install directly; multi-skill repos prompt for selection.

See [SPEC.md — Skill Discovery](./SPEC.md#skill-discovery) for the full scanning algorithm and SKILL.md frontmatter validation.

### A tap = a git repo listing other skills

A tap is a curated index — a git repo containing a `tap.json` that lists skill names, descriptions, repo URLs, and tags. Taps are how you share a curated collection. Your friends add your tap, they see your skills.

See [SPEC.md — tap.json](./SPEC.md#tapjson) for the format specification.

## CLI

skilltap provides commands for installing, removing, updating, linking, and searching for skills, plus tap management.

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

## HTTP Registry (optional, for non-git sources)

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
- **Not an agent plugin system.** Claude Code has its own [plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces) that handles plugins (skills + commands + hooks + MCP servers + agents). skilltap is simpler — it only distributes SKILL.md files, and it works across all agents, not just one.

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

- **v0.1** — Core install/remove/update/link + taps + security scanning (static + semantic)
- **v0.2** — npm adapter, HTTP registry, standalone binary, shell completions
- **v0.3** — Community trust signals, `skilltap publish`, skill templates
