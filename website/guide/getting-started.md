---
description: Install skilltap, install your first skill, set up a tap, and configure defaults. Get from zero to working AI agent skills in under five minutes.
---

# Getting Started

This guide walks you through installing skilltap, installing your first skill, setting up a tap, and configuring defaults.

## Install skilltap

Download the standalone binary:

```bash
curl -fsSL https://skilltap.dev/install.sh | sh
```

This puts the `skilltap` binary on your PATH. No runtime dependencies required.

Alternatively, if you have [Bun](https://bun.sh) installed:

```bash
bunx skilltap
```

Verify the install:

```bash
skilltap --version
```

## Install your first skill

`install` requires a type subcommand — `skill`, `plugin`, or `mcp`. No auto-detect.

```bash
skilltap install skill user/commit-helper
```

skilltap walks you through the process interactively:

```
◇ Cloning user/commit-helper...
◇ Scanning commit-helper...
│ ✓ No warnings

◇ Install commit-helper?
│ › Yes

✓ Installed commit-helper → .agents/skills/commit-helper/
  (scope: project, inferred from git repo)
```

Here's what happened:

1. **Scope was inferred automatically** (smart-scope-default): inside a git repo → project scope (`.agents/skills/`); outside → global (`~/.agents/skills/`). No prompt. Pass `--scope global` or `--scope project` to override.
2. skilltap cloned the repo to a temp directory
3. A static security scan checked all files for suspicious content
4. No warnings found — you confirmed the install
5. The skill was placed at `.agents/skills/commit-helper/`

If the scan found warnings, skilltap would show them and offer to run a deeper [semantic scan](./security) before asking you to confirm.

### Override the scope

```bash
# Always install globally
skilltap install skill user/commit-helper --scope global

# Always install to the current project
skilltap install skill user/commit-helper --scope project
```

### Non-interactive use (CI, scripts, AI agents)

```bash
# --yes auto-accepts clean installs; piped stdout automatically gives plain output
skilltap install skill user/commit-helper --scope global --yes | cat

# JSON output for scripting
skilltap install skill user/commit-helper --scope global --yes --json
```

There is no `--agent` flag. TTY detection + `--yes` + `--json` covers all automation use cases.

::: tip Preview before installing
Use `skilltap try skill <source>` to clone, scan, and inspect a source without writing anything. Safe to run on unfamiliar sources.
:::

::: info Coming from v2.1 or earlier?
Run `skilltap migrate` once to translate your config and state files. Migrates `[security.human]`/`[security.agent]` blocks, removes `[agent-mode]`, and consolidates `installed.json` / `plugins.json` → `state.json`.
:::

### Agent symlinks

Pass `--also` to symlink into agent-specific directories:

```bash
skilltap install skill user/commit-helper --scope global --also claude-code
```

This creates:
- `~/.agents/skills/commit-helper/` (the actual files)
- `~/.claude/skills/commit-helper/` (symlink)

Supported agents: `claude-code`, `cursor`, `codex`, `gemini`, `windsurf`.

Set a permanent default in config so you don't need to repeat it:

```bash
skilltap config set defaults.also claude-code
```

## Add a tap

A tap is a curated index of skills — a git repo containing a `tap.json` that lists skill names, descriptions, and URLs. Adding a tap lets you install skills by name instead of URL.

```bash
skilltap tap add skilltap https://github.com/nklisch/skilltap-skills
```

```
Cloning tap...
✓ Added tap 'skilltap' (2 skills)
```

You can add as many taps as you want — your own, a friend's, a team's.

## Search and install from a tap

Search for skills:

```bash
skilltap find
```

This opens an interactive TUI browser. Type a query, browse results, and press Enter to install. You can also search directly:

```bash
skilltap find review
```

Install by name from a tap:

```bash
skilltap install skill commit-helper --scope global
```

skilltap resolves the name from your configured taps, finds the repo URL, and runs the normal clone-scan-install flow.

Pin a version:

```bash
skilltap install skill commit-helper@v1.0.0 --scope global
```

## Install a plugin

A plugin bundles skills + MCP servers + agent definitions as a single installable unit.

```bash
skilltap install plugin corp/dev-toolkit --scope global --also claude-code
```

```
Detected plugin: dev-toolkit
  3 skills, 2 MCP servers, 1 agent definition

✓ Installed plugin dev-toolkit
  Skills: code-review, commit-helper, test-generator
  MCPs: database, file-search → claude-code
  Agent: code-review.md
```

Manage plugin components:

```bash
skilltap toggle plugin dev-toolkit:test-generator   # disable one component
skilltap info dev-toolkit                            # view component details
```

## View installed skills and plugins

For a full dashboard (skills, plugins, MCPs, taps, drift):

```bash
# Opens TUI dashboard in a terminal
skilltap

# Headless (safe to pipe, JSON-friendly)
skilltap status
skilltap status --json
```

## Adopt existing skills

If you've placed skills manually or want to track a local dev skill, use `adopt`:

```bash
# Open TUI picker for all unmanaged skills
skilltap adopt

# Adopt a specific external path (replaces the old `link` command)
skilltap adopt ~/dev/my-skill --also claude-code

# Adopt all Claude Code plugins into skilltap management
skilltap adopt --source claude-code
```

## Move between scopes

Move a skill from project scope to global (or vice versa):

```bash
skilltap move commit-helper --scope global
```

## Update skills

Update all installed skills, plugins, and MCP servers:

```bash
skilltap update
```

```
Checking commit-helper... abc123 → def456 (2 files changed)
  M SKILL.md (+5 -2)
  A scripts/helper.sh (+18)

Scanning changes... ✓ No warnings
Apply update? (y/N): y
✓ Updated commit-helper

Checking code-review... Already up to date.

Updated: 1   Skipped: 0   Up to date: 1
```

Update a specific item:

```bash
skilltap update skill commit-helper
skilltap update plugin dev-toolkit
```

## Configure defaults

Run the interactive setup wizard:

```bash
skilltap config
```

Or set values directly:

```bash
skilltap config set defaults.scope project
skilltap config set defaults.also claude-code
```

Your settings are saved to `~/.config/skilltap/config.toml`. See the [Configuration](/guide/configuration) guide for the full reference.

## Configure security

skilltap scans every skill for suspicious content before installing. Fine-tune security:

```bash
skilltap config security
```

The `[security]` block in `config.toml` controls scan mode and warning behavior:

```toml
[security]
scan = "static"      # "semantic" | "static" | "off"
on_warn = "prompt"   # "prompt" | "fail" | "allow"
```

For non-interactive use, hard-fail on any warning:

```bash
skilltap config set security.on_warn fail
```

To skip scanning for a tap you control, add an override entry to `config.toml`:

```toml
[[security.overrides]]
match = "my-corp"
kind = "tap"
preset = "none"
```

See the [Security](/guide/security) guide for full details.
