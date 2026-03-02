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
skilltap install user/commit-helper --global
```

Here's what happens:

```
Cloning user/commit-helper...
Scanning commit-helper...
✓ No warnings

Install? (Y/n): y
✓ Installed commit-helper → ~/.agents/skills/commit-helper/
```

1. skilltap clones the repo to a temp directory
2. Finds the `SKILL.md` file and reads its metadata
3. Runs a static security scan on all files
4. Shows the result and asks you to confirm
5. Moves the skill to `~/.agents/skills/commit-helper/`

The skill is now available to any agent that reads from `~/.agents/skills/`.

### Make it visible to a specific agent

If you also want the skill in your agent's directory, use `--also`:

```bash
skilltap install user/commit-helper --global --also claude-code
```

This creates a symlink at `~/.claude/skills/commit-helper/` pointing to the canonical install location. Supported agents: `claude-code`, `cursor`, `codex`, `gemini`, `windsurf`.

## Add a tap

A tap is a curated index of skills -- a git repo containing a `tap.json` that lists skill names, descriptions, and URLs. Adding a tap lets you install skills by name instead of URL.

```bash
skilltap tap add community https://github.com/example/awesome-skills-tap
```

```
Cloning tap...
✓ Added tap 'community' (12 skills)
```

You can add as many taps as you want -- your own, a friend's, a team's.

## Search and install from a tap

Browse available skills:

```bash
skilltap find review
```

```
  code-review        Thorough code review with security focus   [community]
  pr-reviewer        Pull request review checklist              [community]
```

Install by name:

```bash
skilltap install code-review --global
```

skilltap resolves the name from your configured taps, finds the repo URL, and runs the normal clone-scan-install flow.

You can also pin a version:

```bash
skilltap install code-review@v2.0.0 --global
```

## List installed skills

```bash
skilltap list
```

```
Global:
  commit-helper      v1.2.0   community   Conventional commit messages
  code-review        v2.0.0   community   Thorough code review
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
Checking commit-helper... abc123 → def456 (2 files changed)
  M SKILL.md (+5 -2)
  A scripts/helper.sh (new, 180 bytes)

Scanning changes... ✓ No warnings
Apply update? (y/N): y
✓ Updated commit-helper (v1.2.0 → v1.3.0)

Checking code-review... Already up to date.
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
