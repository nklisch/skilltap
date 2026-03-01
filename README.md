# skilltap

**Install agent skills from any git host.** Homebrew taps for AI agent skills — agent-agnostic, multi-source, secure.

## Install

**Standalone binary** (recommended):
```bash
# Download the latest release binary for your platform
chmod +x skilltap && mv skilltap /usr/local/bin/
```

**Via bunx** (requires [Bun](https://bun.sh)):
```bash
bunx skilltap --help
```

**Via npx** (requires Bun on PATH — the package uses `#!/usr/bin/env bun`):
```bash
npx skilltap --help
```

## Quickstart

```bash
# 1. Add a tap (registry of skills)
skilltap tap add my-tap https://github.com/example/my-tap

# 2. Browse available skills
skilltap find

# 3. Install a skill
skilltap install my-skill --global

# 4. List installed skills
skilltap list

# 5. Update all skills
skilltap update
```

## Commands

| Command | Description |
|---|---|
| `install <source>` | Install a skill from a URL, GitHub shorthand, or tap name |
| `remove <name>` | Remove an installed skill |
| `update [name]` | Update one or all installed skills |
| `list` | List installed skills |
| `info <name>` | Show details about a skill (installed or available in taps) |
| `find [query]` | Search skills across configured taps |
| `link <path>` | Link a local skill directory |
| `unlink <name>` | Remove a linked skill |
| `tap add <name> <url>` | Add a tap |
| `tap remove <name>` | Remove a tap |
| `tap update [name]` | Update one or all taps |
| `tap list` | List configured taps |
| `tap init <name>` | Initialize a new tap directory |
| `config` | Interactive configuration wizard |
| `config agent-mode` | Enable/disable agent mode |

Most commands accept `--global` / `--project` for scope and `--yes` to skip prompts.

## How it works

Skills are directories containing a `SKILL.md` file. skilltap installs them to `~/.agents/skills/<name>/` (global) or `.agents/skills/<name>/` (project), then creates symlinks at each agent's expected location (`.claude/skills/`, `.cursor/rules/`, etc.) so every agent picks them up automatically.

## Security

Every install runs a static scan checking for invisible Unicode, hidden HTML/CSS, markdown injection, obfuscated code, suspicious URLs, dangerous shell patterns, and tag injection. An optional semantic scan (powered by your local AI agent) provides deeper analysis of skill intent.

Use `--skip-scan` to bypass scanning, `--strict` to block installation on any warning, or `--semantic` to enable the semantic scan.

## Agent mode

Enable agent mode so skilltap works headlessly from within AI agents:

```bash
skilltap config agent-mode
```

In agent mode, all prompts are suppressed, `--yes` is implied, security issues block installation with a machine-readable message, and output is plain text (no ANSI codes).

## Configuration

Config is stored at `~/.config/skilltap/config.toml`. Run the interactive wizard:

```bash
skilltap config
```

Key settings: default scope (`global`/`project`), additional agent symlinks (`--also`), security scan mode (`static`/`semantic`/`off`), and `on_warn` behavior (`block`/`warn`/`allow`).

## Creating skills

```bash
mkdir my-skill
cat > my-skill/SKILL.md << 'EOF'
---
name: my-skill
description: What this skill does
---

# My Skill

Instructions for the AI agent...
EOF

# Test locally
skilltap link ./my-skill --global

# Push and share
git init my-skill && cd my-skill && git add . && git commit -m "Initial skill"
git remote add origin https://github.com/you/my-skill
git push -u origin main
```

Others can install with: `skilltap install you/my-skill`

## License

MIT
