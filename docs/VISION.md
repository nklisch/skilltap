# skilltap

A CLI for installing agent skills and plugins from any git host. Git-native, agent-agnostic, multi-source. Installs skills individually or as plugin bundles (skills + MCP servers + agent definitions) across multiple agent platforms.

## Problem

The SKILL.md format is standardized across 40+ agents (Claude Code, Cursor, Codex CLI, Gemini CLI, etc.), but distribution is fragmented. skills.sh only indexes GitHub. Other tools are centralized hosted services. There's no way to point at your own Gitea/GitLab instance and install from there.

Individual agents are starting to build their own distribution — Claude Code has a full [plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces) system with `marketplace.json`, `/plugin install`, and support for git, npm, and pip sources. But these are agent-specific. There's no agent-agnostic tool for distributing skills to whatever agent(s) you use.

## Core Idea

**Skills are git repos. Git is the transport. The CLI just clones and links.**

Think: **Homebrew taps for agent skills.**

skilltap installs to the universal `.agents/skills/` directory defined by the [Agent Skills spec](https://agentskills.io/specification). This is the agent-agnostic path that works across all conforming agents. If you also want skills in an agent-specific directory (`.claude/skills/`, `.cursor/skills/`), opt in via `--also`.

skilltap also understands **plugins** — bundles that package skills alongside MCP servers and agent definitions. When you install a plugin, skilltap extracts the portable components (skills, MCP configs, agents) and installs them into each target agent platform. You can then toggle individual components on/off within an installed plugin.

## Design Principles

1. **Git-native.** Clone, shallow-clone, pull. No custom protocol. Git already handles versioning, auth, and distribution.
2. **Agent-agnostic.** Installs to `.agents/skills/` by default — the universal path. Not tied to any single agent's ecosystem.
3. **Multi-source.** Configure multiple sources (taps) — your Gitea, a friend's GitLab, public GitHub repos. Search across all of them.
4. **Minimal.** No scoring, no benchmarking, no composition engine. Clone repos, make links. That's it.
5. **One runtime.** Every command works headless. TTY detection picks output style; `--json` forces machine output anywhere; `--yes` resolves "do it" prompts; required args resolve "what" prompts. No separate agent runtime, no per-mode security split.

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

A plugin is a repo (or directory) containing a plugin manifest that groups skills with MCP server configs and agent definitions. skilltap reads three formats:

- `.skilltap/<plugin>.toml` (native; multiple plugins per repo)
- `.claude-plugin/plugin.json` (Claude Code)
- `.codex-plugin/plugin.json` (Codex)

```
my-plugin/
  .skilltap/
    dev-toolkit.toml    # native manifest (publish = true to be installable)
  skills/
    helper/SKILL.md
    reviewer/SKILL.md
  .mcp.json             # MCP server definitions
  agents/
    code-review.md      # Agent definition (Claude Code format)
```

skilltap extracts the portable subset: **skills** (installed via the existing skill system), **MCP servers** (injected into each target agent's config with the `skilltap:` namespace), and **agents** (placed in agent-specific directories, Claude Code for now). Platform-specific components (hooks, LSP, commands, output styles) are ignored.

After installing, you can toggle individual components — disable a specific MCP server or skill within the plugin without removing the whole thing.

### Plugin capture

Installing a plugin that bundles a skill you already have as a standalone is idempotent. skilltap detects the collision, transfers ownership of the standalone to the plugin (atomic, with rollback on failure), and proceeds with the plugin install. Cross-source collisions (a different repo bundles the same skill name) prompt for confirmation in TTY mode and error out non-interactively unless `--force-capture` or `--no-capture` is set.

### Repo scanning

When you point skilltap at a repo, it scans for all SKILL.md files and lets you choose which to install. Single-skill repos install directly; multi-skill repos prompt for selection.

See [SPEC.md — Skill Discovery](./SPEC.md#skill-discovery) for the full scanning algorithm and SKILL.md frontmatter validation.

### A tap = a git repo listing other skills

A tap is a curated index — a git repo containing a `tap.json` (or Claude Code `marketplace.json`) that lists skill names, descriptions, repo URLs, and tags. Taps are how you share a curated collection. Your friends add your tap, they see your skills.

See [SPEC.md — tap.json](./SPEC.md#tapjson) for the format specification.

### Project manifest

Every project gets a `skilltap.toml` declaring the skills, plugins, MCP servers, and taps it depends on, plus its default agent targets. A companion `skilltap.lock` records exact resolved refs.

- `skilltap install <type> <source>` adds to the manifest and lockfile.
- `skilltap sync` installs from the lockfile, prompts on any drift between declared / locked / installed.
- `skilltap update` refreshes the lockfile to the latest matching range.
- A teammate clones the repo, runs `skilltap sync`, and gets the exact same setup.

The manifest tracks all three state types (`[[skills]]`, `[[plugins]]`, `[[mcps]]`); sync reconciles all three.

## CLI

skilltap has one verb per action with typed args:

- `install <type> <source>` — type is `skill | plugin | mcp`. Required.
- `remove <type> <name>` — type required.
- `update [type] [name]` — bare = all; `update <type>` = all of type; `update <type> <name>` = one.
- `toggle [type] [name[:component]]` — TUI when args missing.
- `try <type> <source>` — readonly preview.
- `find [query]` — TUI when interactive, `--json` when not.
- `adopt [path]` — bring unmanaged skills under management. TUI when no path.
- `sync` — reconcile manifest ↔ lockfile ↔ state.
- `doctor [skill|plugin <path>]` — env check or per-artifact validation.
- `status [--json]` — headless dashboard.
- `migrate` — one-shot upgrade for pre-V2 setups.

Bare `skilltap` opens a multi-screen TUI dashboard (TTY only). See [UX.md](./UX.md) for the full command tree, flag combinations, and interactive prompt flows. See [SPEC.md](./SPEC.md#cli-commands) for the precise behavioral specification of each command.

**Quick examples:**

```bash
# Install from any git URL (typed)
skilltap install skill https://gitea.example.com/user/commit-helper

# Install from a tap by name
skilltap install skill commit-helper

# GitHub shorthand
skilltap install skill user/repo

# Project scope + agent symlinks (smart-scope picks project automatically inside a git repo)
skilltap install skill commit-helper --scope project --also claude-code

# Search across all taps
skilltap find review

# Adopt a local skill for development
skilltap adopt . --also claude-code

# Install a plugin (one of many in a multi-plugin repo)
skilltap install plugin user/dev-toolkit:test-runner --also claude-code --also cursor

# Install all publishable plugins from a multi-plugin repo
skilltap install plugin user/dev-toolkit:*

# Install just an MCP server
skilltap install mcp user/some-mcp

# Toggle plugin components
skilltap toggle plugin dev-toolkit
skilltap toggle plugin dev-toolkit:test-generator   # direct component address
```

## Security Scanning

Every install runs a multi-layer scan before writing anything to disk. Suspicious content surfaces with context; the user (or `on_warn` policy) decides.

### Layer 1 — Static analysis (instant, no LLM)

Fast pattern matching on every install. Detects invisible Unicode, hidden HTML/CSS, markdown hiding tricks, obfuscation (base64, hex, variable expansion), suspicious URLs (known exfiltration services), dangerous shell patterns, tag injection attempts, and suspicious file types/sizes.

Warnings show the raw escaped content inline (so you can see what's hiding) and the file path + line number. The source is cloned to a temp dir before anything is installed.

See [SPEC.md — Layer 1: Static Analysis](./SPEC.md#layer-1-static-analysis) for the complete detection pattern reference.

### Layer 2 — Semantic scan (opt-in, uses the user's own agent)

When `scan = "semantic"` or `--deep` is passed, skilltap uses the locally installed agent CLI to evaluate the skill's intent.

**How it works:**

1. **Chunk** the skill content into small blocks (~200–500 tokens). Pre-scan for tag injection attempts and auto-flag if found.
2. **Send each chunk** to the user's agent in an isolated context — fresh session, no tools, no file access, randomized security wrapper tags.
3. **Aggregate scores** across all chunks. Flag anything above threshold (default: 5).

See [SPEC.md — Layer 2: Semantic Scan](./SPEC.md#layer-2-semantic-scan) for the full chunking algorithm, security prompt template, and agent invocation details.

**Why chunking matters:**
- A full skill can be thousands of tokens — attackers hide malicious instructions in the middle of legitimate content hoping they get lost in context.
- Small chunks force focused evaluation on each section.
- Each chunk is evaluated independently — no cross-contamination between sections.
- Parallelizable — send up to 4 chunks concurrently for speed.

**Why the user's own agent:**
- Zero infrastructure — no API keys, no external service, no skilltap account.
- Works offline if the agent supports it.
- The user already trusts and pays for their agent.
- No data leaves the user's machine beyond what their agent already does.

### One policy

The same `[security]` block applies to every caller — human, AI agent, CI, cron. What changes between callers is **resolution behavior** (TTY → prompt vs error, `--yes` auto-confirms, `--json` forces machine output), not security policy.

See [SECURITY.md](./SECURITY.md) for the full configuration reference, scan modes, `on_warn` behavior, and trust list.

### Additional hardening

- **Scan the entire skill directory**, not just SKILL.md — 91% of real attacks hide payloads in auxiliary files.
- **Flag non-plaintext files** — binaries, compiled code, minified JS.
- **Size limits** — flag skills over `scanner.max_size` (default 50 KB).
- **Diff on update** — `skilltap update` shows what changed and re-scans the diff.

### Future — community trust signals

Taps could optionally carry trust metadata (`verified`, `reviewedBy`). Social trust signal, not a security guarantee. Already implemented via npm provenance (Sigstore/SLSA) and GitHub attestations; per-skill trust tier displays in `list` / `info` / `find` output.

## Auth

Follows git's auth model — no custom auth layer:

1. Git credential helpers (already configured for your hosts).
2. SSH keys (for `git@` URLs).
3. Token-in-URL (for HTTPS, e.g. Gitea access tokens).
4. `GH_TOKEN` / `GITLAB_TOKEN` env vars (for API-based search).

## TUI

Bare `skilltap` opens a multi-screen Ink-based TUI dashboard with tabs for installed skills, plugins, taps, and updates. `find`, `toggle`, and `adopt` open TUIs when invoked without disambiguating arguments. Each TUI screen has a flat command-line equivalent — every TUI action is reproducible from a script.

The TUI is humans-only. Headless callers use `skilltap status` for the dashboard view, or invoke commands with their typed args directly.

## What This Is NOT

- **Not a package manager.** No dependencies, no build step, no install scripts.
- **Not a marketplace.** No centralized index. Taps are just git repos anyone can create.
- **Not a runtime.** Skills are static files. No execution engine.
- **Not a full plugin runtime.** Claude Code and Codex have their own plugin systems with hooks, channels, LSP, and other platform-specific features. skilltap reads their plugin formats but only installs the portable components (skills, MCP servers, agents). For the full platform-specific experience, use each agent's native plugin system.

## Prior Art

| Project | Relationship |
|---------|-------------|
| [Agent Skills spec](https://agentskills.io/specification) | The SKILL.md format we distribute. The standard across 40+ agents. |
| [Claude Code plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces) | Claude Code's built-in system for distributing plugins (which include skills). Agent-specific. skilltap reads its `plugin.json` and `marketplace.json` formats. |
| [MCP Registry API](https://registry.modelcontextprotocol.io/) | Most formally specified registry API for MCP servers. |
| [Homebrew taps](https://docs.brew.sh/Taps) | Direct inspiration for the git-repo-as-index tap model. |
| [skills.sh](https://skills.sh/) | GitHub-only CLI. No self-hosting, no registry API. Passive telemetry leaderboard. |
| [Skillshub](https://github.com/EYH0602/skillshub) | Rust CLI with tap support. Similar direction, less mature. |
| [ClawHub](https://github.com/openclaw/clawhub) | Largest index (13k+ skills). Convex backend, no open API spec. Had security incident (4.4% malicious). |
| [OpenAI Skills API](https://developers.openai.com/api/docs/guides/tools-skills/) | Proprietary REST API. Cloud-only. |

## Landscape

**Skill format**: Settled. The [Agent Skills spec](https://agentskills.io/specification) (SKILL.md) is adopted by Anthropic, OpenAI, Google, GitHub, Cursor, and 30+ others.

**Agent-specific distribution**: Emerging. Claude Code has a full plugin marketplace. OpenAI has a REST API. But these only work within their own agent.

**Agent-agnostic distribution**: No standard. skills.sh is GitHub-only. Skillshub is Rust/early. ClawHub is centralized. skilltap fills the gap with a simple, self-hostable, git-native tool that installs to the universal `.agents/skills/` path.

## Read formats — the ecosystem stays compatible

skilltap reads (and where applicable, writes) these formats:

- `SKILL.md` (Agent Skills spec)
- `tap.json`
- `.claude-plugin/marketplace.json`
- `.claude-plugin/plugin.json`
- `.codex-plugin/plugin.json`
- `.skilltap/<name>.toml` (native plugin manifest)
- `skilltap.toml` + `skilltap.lock` (project manifest + lockfile)
- `state.json` (per-scope canonical state)

Skills and plugins published in any of these formats keep working.

---

## Considered and removed

These were proposed at various points and are not part of skilltap:

- **HTTP registry adapter** — taps are git-only. The HTTP adapter and `[[registries]]` HTTP endpoint design shipped briefly in v0.2 and was removed before V2; auth/error paths were a maintenance burden, real usage was minimal, and the manifest+lockfile model assumes git-based reproducibility.
- **`link` / `unlink` commands** — folded into `adopt [path]` (track-in-place vs `--move`).
- **`verify` command** — folded into `doctor skill <path>` and `doctor plugin <path>`.
- **`enable` / `disable` commands** — accessed via `toggle` or the TUI plugin manager.
- **`skills` and `plugin` subcommand groups** — collapsed to top-level operations and the TUI.
- **`mcp:` URL prefix** — type is explicit via `install mcp <source>`.
- **Per-mode security (`[security.human]` / `[security.agent]`)** — replaced by single `[security]`.
- **Security presets (`none`/`relaxed`/`standard`/`strict`)** — dropped; configure `scan` and `on_warn` directly.
- **`[[security.overrides]]`** — replaced by the simpler `security.trust = []` glob list.
- **`require_scan` config key** — set `scan = "static"` (or `"semantic"`) to require scanning; `none` is the only way to opt out.
- **`[agent-mode]` / `[agent]` config blocks** — there is no separate agent runtime.
- **`--agent` flag and `SKILLTAP_AGENT` env var** — TTY detection + `--yes` + `--json` cover unattended use.
- **Agent symlinks across all agents by default** — `--also` is opt-in per agent.
