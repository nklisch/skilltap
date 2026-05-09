---
description: Complete reference for all skilltap CLI commands — install, update, remove, toggle, adopt, sync, doctor, migrate, find, try, and more.
---

# CLI Reference

Complete reference for all skilltap commands, arguments, flags, and behavior.

> For full flag combinations and prompt flows, see [docs/UX.md](https://github.com/nklisch/skilltap/blob/main/docs/UX.md) in the repository.

## Global Behavior

- **Exit codes:** `0` success, `1` error, `2` user cancelled, `130` Ctrl+C
- **Errors** are written to stderr with an `error:` prefix and optional `hint:`
- **Config** is stored at `~/.config/skilltap/config.toml`
- **State** is tracked in `~/.config/skilltap/state.json` (global) or `<project>/.agents/state.json` (project)
- **Output mode** is decided by TTY detection: TTY → colors + spinners, non-TTY → plain text, `--json` → newline-delimited JSON events

### Universal flags

Most commands accept:

| Flag | Description |
|------|-------------|
| `--json` | Machine-readable output (also auto-selected when stdout is not a TTY) |
| `--yes` / `-y` | Auto-accept "do it" prompts |
| `--quiet` | Suppress non-essential output |
| `--scope project\|global` | Explicit scope. Default: `project` inside a git repo, `global` otherwise |

---

## skilltap install

Install a skill, plugin, or MCP server. **Type is required.**

```
skilltap install skill   <source> [flags]
skilltap install plugin  <source> [flags]
skilltap install mcp     <source> [flags]
```

### Source formats

| Format | Example |
|--------|---------|
| Git URL (any host) | `https://gitea.example.com/user/repo` |
| SSH | `git@github.com:user/repo.git` |
| GitHub shorthand | `user/repo` |
| GitHub explicit | `github:user/repo` |
| npm package | `npm:vibe-rules` |
| npm scoped + version | `npm:@scope/skill@1.2.0` |
| Tap name | `commit-helper` |
| Tap name + ref | `commit-helper@v1.2.0` |
| Local path | `./my-skill` |

The same source forms work for all three types. The type subcommand decides what skilltap looks for in the resolved source.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--scope project\|global` | string | smart default | Installation scope. Default: project inside a git repo, global otherwise. |
| `--also <agent>` | string | from config | Also create symlink in agent-specific directory. Repeatable. Values: `claude-code`, `cursor`, `codex`, `gemini`, `windsurf` |
| `--ref <ref>` | string | default branch | Branch or tag to install |
| `--yes` / `-y` | boolean | `false` | Auto-select all skills, auto-accept clean installs. Security warnings still prompt. |
| `--strict` | boolean | from config | Abort on any security warning (exit 1) |
| `--semantic` | boolean | from config | Force Layer 2 semantic scan |
| `--skip-scan` | boolean | `false` | Skip security scanning. Blocked if `require_scan = true` in config. |
| `--quiet` | boolean | `false` | Suppress install step details |
| `--json` | boolean | `false` | Output as JSON |

### Smart scope default

When `--scope` is not passed and `defaults.scope` is not set in config, scope is inferred from the cwd: inside a git repo → `project`; outside → `global`. No prompt — the inferred scope is reported in output.

### Prompt behavior

| Flags | Skill selection | Security warnings | Clean install |
|-------|----------------|-------------------|---------------|
| (none) | Prompt if multiple | Prompt | Prompt |
| `--yes` | Auto-select all | **Still prompts** | Auto-accept |
| `--strict` | Prompt if multiple | **Abort (exit 1)** | Prompt |
| `--strict --yes` | Auto-select all | **Abort (exit 1)** | Auto-accept |
| `--skip-scan --yes` | Auto-select all | Skipped | Auto-accept |

### Examples

```bash
# Skill from tap name
skilltap install skill commit-helper

# Skill from git URL, scoped to project
skilltap install skill https://gitea.example.com/u/repo --scope project

# Skill from GitHub shorthand, global, with symlink
skilltap install skill user/my-skill --scope global --also claude-code

# Fully non-interactive (no TTY, --yes)
skilltap install skill commit-helper --scope global --yes

# Plugin
skilltap install plugin corp/dev-toolkit --scope global --also claude-code

# Specific plugin in a multi-plugin repo
skilltap install plugin corp/dev-toolkit:frontend

# MCP server from npm
skilltap install mcp npm:@modelcontextprotocol/server-postgres
```

---

## skilltap remove

```
skilltap remove skill   <name> [flags]
skilltap remove plugin  <name> [flags]
skilltap remove mcp     <name> [flags]
```

`remove plugin <name>` removes the plugin record **and all its components** (skills, MCP injections, agent definitions). Calling `remove skill <name>` on a plugin component errors with a hint.

### Flags

| Flag | Description |
|------|-------------|
| `--scope project\|global` | Remove from specific scope |
| `--yes` | Skip confirmation |

---

## skilltap update

```
skilltap update                  # update everything
skilltap update skill            # update all skills
skilltap update plugin           # update all plugins
skilltap update mcp              # update all standalone MCPs
skilltap update skill <name>     # update one skill
skilltap update plugin <name>
```

### Flags

| Flag | Description |
|------|-------------|
| `--yes` | Auto-accept clean updates |
| `--strict` | Skip items with security warnings in diff |
| `--semantic` | Force Layer 2 semantic scan on diff |
| `--check` / `-c` | Check for updates without applying |
| `--force` / `-f` | Force update even if already at latest |
| `--json` | Output as JSON |

---

## skilltap toggle

```
skilltap toggle                              # TUI: pick type → name → component(s)
skilltap toggle plugin <name>                # TUI scoped to plugin's components
skilltap toggle plugin <name>:<component>    # toggle one component directly
skilltap toggle skill <name>                 # toggle whole skill active/inactive
skilltap toggle mcp <name>                   # toggle whole MCP active/inactive
```

Only `plugin` accepts the `:<component>` suffix.

---

## skilltap find

```
skilltap find [query] [--json]
```

Opens a TUI browser when run interactively. Falls back to plain-text results when piped. `--json` outputs results as JSON regardless of TTY state.

---

## skilltap try

```
skilltap try <type> <source> [flags]
```

Preview a source (clone, parse, scan) without installing. Nothing is written to disk.

```
--skip-scan    Skip security scan
--json         Output as JSON
```

---

## skilltap adopt

```
skilltap adopt [path] [flags]
```

Bring an external skill or agent-managed plugin into skilltap state.

| Flag | Description |
|------|-------------|
| `--source <name>` | Filter picker to one agent source (e.g., `claude-code`) |
| `--scope project\|global` | Scope for adoption |
| `--also <agent>` | Also symlink into agent dirs |
| `--move` | Move dir into canonical location (default: track-in-place) |
| `--skip-scan` | Skip security scan |
| `--yes` | Auto-accept all prompts |

**Modes:**
- No args → TUI picker: all unmanaged skills + Claude Code plugins
- With path → adopt external dir (replaces `link`/`unlink`)
- `--source claude-code` → picker filtered to Claude Code plugins

---

## skilltap sync

```
skilltap sync [--apply] [--strict] [--json]
```

Show drift between `skilltap.toml`, `skilltap.lock`, and installed state. Requires a project root (directory containing `skilltap.toml` or `.git`).

`--apply` executes the plan via install/remove. `--strict` stops at first failure.

---

## skilltap doctor

```
skilltap doctor [skill|plugin <path>] [flags]
```

**No args** — environment + state health check: config validity, state validity, symlink integrity, manifest/lockfile drift, doctor checks 1–17.

**With artifact path** — per-artifact validation (replaces `verify`):

```bash
skilltap doctor skill ./my-skill     # validate a skill
skilltap doctor plugin ./my-plugin   # validate a plugin
```

| Flag | Description |
|------|-------------|
| `--fix` | Auto-repair safe issues (broken symlinks, orphan records, corrupt manifests) |
| `--json` | Machine-readable output |

---

## skilltap migrate

```
skilltap migrate
```

One-shot upgrade from any prior config/state version. Detection markers:

- `[agent-mode]` block → removed
- `[security.human]` / `[security.agent]` → collapsed to a single flat `[security]` block
- `[[security.overrides]]` → kept as-is (still part of the live schema)
- `installed.json` / `plugins.json` → consolidated to `state.json`
- HTTP taps → error; list affected taps for manual handling

Originals are renamed to `*.bak`. After translation, runs `doctor` to verify.

---

## skilltap status

```
skilltap status [--json]
```

Headless dashboard — skills, plugins, MCPs, taps, drift. Safe to pipe. Equivalent to bare `skilltap` but without the TUI.

---

## Bare `skilltap`

Opens the Ink-based TUI dashboard (TTY only). Tabs: Installed, Taps, Updates, Drift.

Key bindings: arrow keys navigate; `i` install; `r` remove; `t` toggle; `u` update; `f` find; `a` adopt; `q`/`Esc` exit.

Without a TTY: error with hint to use `skilltap status`.

---

## skilltap info

```
skilltap info <name> [--json]
```

Show details about an installed skill, plugin, or MCP server.

---

## skilltap move

```
skilltap move <name> --scope project|global [--also <agent>]
```

Move a managed skill between global and project scope. Relocates files, updates `state.json`, recreates symlinks.

---

## skilltap create

```
skilltap create [name]
```

Scaffold a new skill from a template. Opens an interactive wizard.

---

## skilltap tap

```
skilltap tap add <name> <url>   # Add a git tap
skilltap tap remove <name>      # Remove a tap
skilltap tap list               # List taps
skilltap tap info <name>        # Inspect a tap
skilltap tap init <name>        # Initialize a new tap directory
```

---

## skilltap config

```
skilltap config get [key]          # Get a config value
skilltap config set <key> <value>  # Set a config value
skilltap config edit               # Open config in $EDITOR
skilltap config security           # Interactive security wizard
```

---

## skilltap completions

```
skilltap completions <shell> [--install]
```

Generate shell tab-completion script for `bash`, `zsh`, or `fish`. `--install` writes to the standard location.

---

## skilltap self-update

```
skilltap self-update [--force]
```

Update the running binary in-place from GitHub Releases.

---

## Removed commands

The following commands were removed in the v2.0 redesign. Their replacements are shown:

| Removed | Replacement |
|---------|-------------|
| `verify <path>` | `doctor skill <path>` |
| `link <path>` | `adopt <path>` |
| `unlink <name>` | `remove skill <name>` (if adopted) |
| `enable <name>` | `toggle skill\|plugin\|mcp <name>` |
| `disable <name>` | `toggle skill\|plugin\|mcp <name>` |
| `skills info\|adopt\|move\|remove\|link\|unlink` | Top-level commands |
| `plugin info\|toggle\|remove` | Top-level `info`, `toggle`, `remove` |
| `tap install` | `install skill <name>` (tap-resolved names work directly) |
| `config agent-mode` | No replacement — agent mode removed entirely |
| `install <source>` (no type) | `install skill\|plugin\|mcp <source>` |
| `remove <name>` (no type) | `remove skill\|plugin\|mcp <name>` |

Old paths return clear errors with hints pointing at the canonical command.
