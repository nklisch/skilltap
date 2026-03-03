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

Alternatively, if you use Bun or Node:

```bash
bunx skilltap    # Bun
npx skilltap     # Node
```

Verify the install:

```bash
skilltap --version
```

## Install your first skill

Install a skill from a git URL:

```bash
skilltap install user/commit-helper
```

skilltap walks you through the process interactively:

```
◆ skilltap

◆ Install to:
│ ● Global (~/.agents/skills/)
│ ○ Project (.agents/skills/)

◆ Which agents should this skill be available to?
│ ◼ Claude Code
│ ◻ Cursor
│ ◻ Codex
│ ◻ Gemini
│ ◻ Windsurf

◆ Save agent selection as default?
│ Yes

◇ Cloning user/commit-helper...
◇ Scanning commit-helper...
│ ✓ No warnings

◇ Install commit-helper?
│ › Yes

✓ Installed commit-helper → ~/.agents/skills/commit-helper/
```

Here's what happened:

1. You chose where to install (global or project scope)
2. You selected which agents should see the skill (symlinks are created automatically)
3. skilltap cloned the repo
4. A static security scan checked all files for suspicious content
5. No warnings found — you confirmed the install
6. The skill was placed at `~/.agents/skills/commit-helper/` with a symlink in `~/.claude/skills/commit-helper/`

If the scan had found warnings, skilltap would show them and offer to run a deeper [semantic scan](./security) using your local AI agent before asking you to confirm.

Skip all prompts with `--yes --global` for clean CI-style installs:

```bash
skilltap install user/commit-helper --global --yes
```

You can skip the scope prompt with `--global` or `--project`:

```bash
skilltap install user/commit-helper --global
```

### Agent symlinks

During install, skilltap asks which agents the skill should be visible to. Your selection creates symlinks into agent-specific directories (e.g. `~/.claude/skills/`). You can save your choice as the default for future installs.

The prompt is skipped automatically once you've saved a default (via `skilltap config` or "Save as default?" during a previous install). You can also skip it explicitly:

```bash
skilltap install user/commit-helper --global --also claude-code
```

This creates:
- `~/.agents/skills/commit-helper/` (the actual files)
- `~/.claude/skills/commit-helper/` (symlink)

Supported agents: `claude-code`, `cursor`, `codex`, `gemini`, `windsurf`.

## Add a tap

A tap is a curated index of skills -- a git repo containing a `tap.json` that lists skill names, descriptions, and URLs. Adding a tap lets you install skills by name instead of URL.

```bash
skilltap tap add skilltap https://github.com/nklisch/skilltap-skills
```

```
Cloning tap...
✓ Added tap 'skilltap' (2 skills)
```

You can add as many taps as you want -- your own, a friend's, a team's.

## Search and install from a tap

Browse available skills:

```bash
skilltap find
```

```
  skilltap        Manage agent skills with the skilltap CLI   [skilltap]
  skilltap-find   Discover and search for agent skills        [skilltap]
```

Install by name:

```bash
skilltap install skilltap --global
```

skilltap resolves the name from your configured taps, finds the repo URL, and runs the normal clone-scan-install flow.

You can also pin a version:

```bash
skilltap install skilltap@v1.0.0 --global
```

## List installed skills

```bash
skilltap list
```

```
Global (1 skill)
  Name       Ref     Source                                  Trust          Description
  ──────────────────────────────────────────────────────────────────────────────────────────────────
  skilltap   main    https://github.com/nklisch/skilltap-…   ○ unverified   Manage agent skills
```

Filter by scope:

```bash
skilltap list --global     # only global skills
skilltap list --project    # only project-scoped skills
```

## Update skills

Update all installed skills:

```bash
skilltap update
```

```
◆ Checking commit-helper...
│ abc123 → def456 (2 files changed)
  M SKILL.md (+5 -2)
  A scripts/helper.sh (+18)

◇ Apply update to commit-helper?
│ › Yes
✓ Updated commit-helper

◆ Checking code-review...
│ Already up to date.

Updated: 1   Skipped: 0   Up to date: 1
```

Updates fetch the latest changes, show you a diff summary, scan the changed files for security issues, and ask before applying.

Update a specific skill:

```bash
skilltap update commit-helper
```

## Configure defaults

Run the interactive setup wizard:

```bash
skilltap config
```

This walks you through:

- Default install scope (global, project, or ask each time)
- Which agents to auto-symlink to on every install
- Security scan level (static only, static + semantic, or off)
- What to do when security warnings are found

Your settings are saved to `~/.config/skilltap/config.toml`. See the [Configuration](/guide/configuration) guide for the full reference.

## Next steps

- [Installing Skills](./installing-skills) -- all source formats, flags, scopes, and multi-skill repos
- [Creating Skills](./creating-skills) -- write and publish your own skills
- [Taps](./taps) -- create and manage skill indexes
- [Security](./security) -- how scanning works and how to configure it
- [Configuration](./configuration) -- full config file reference
