---
description: Complete reference for all skilltap CLI commands — install, update, remove, toggle, adopt, sync, doctor, migrate, find, try, and more.
---

# CLI Reference

Complete reference for all skilltap commands, arguments, flags, and behavior. Every flag table on this page is generated from the citty `defineCommand` args definition for that command — the source of truth.

> For full prompt flows and worked combinations, see [docs/UX.md](https://github.com/nklisch/skilltap/blob/main/docs/UX.md) in the repository.

## Global Behavior

- **Exit codes:** `0` success, `1` error, `2` user cancelled, `130` Ctrl+C
- **Errors** are written to stderr with an `error:` prefix and optional `hint:`
- **Config** is stored at `~/.config/skilltap/config.toml`
- **State** is tracked in `~/.config/skilltap/state.json` (global) or `<project>/.agents/state.json` (project)
- **Output mode** is decided by TTY detection: TTY → colors + spinners; non-TTY → plain text; `--json` → newline-delimited JSON events

### Smart-scope default

When `--scope` is not passed and `defaults.scope` is empty in config, scope is inferred from the cwd: inside a git repo → `project`; outside → `global`. No prompt — the inferred scope is reported in output as `→ scope: project (inferred from cwd)`.

### Universal flags

Most commands accept:

| Flag | Description |
|------|-------------|
| `--json` | Machine-readable output (also auto-selected when stdout is not a TTY) |
| `--yes` / `-y` | Auto-accept "do it" prompts |
| `--quiet` | Suppress non-essential output |
| `--scope project\|global` | Explicit scope. Default: smart-scope (project inside a git repo, global otherwise). |

`info` and `status` deliberately retain the legacy boolean pair (`--global` / `--project`) — they are read-only and the boolean shape matches their per-scope filtering UX.

---

## skilltap install

Install a skill, plugin, or MCP server. **Type is required.**

```
skilltap install skill   <source> [flags]
skilltap install plugin  <source> [flags]
skilltap install mcp     <source> [flags]
```

Each invocation takes one type. To install a skill and an MCP from the same repo, run `install skill` and `install mcp` separately (or use a plugin install if the repo defines one).

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
| Multi-plugin selector | `acme/tools:auth` (one) or `acme/tools:*` (all) |

The same source forms work for all three types. The type subcommand decides what skilltap looks for in the resolved source.

### `install skill` flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<source>` (positional, required) | string | — | One or more sources (separate with spaces to install multiple in sequence) |
| `--scope project\|global` | string | smart-scope | Installation scope |
| `--also <agent>` | string | from config | Also create symlink in agent-specific directory. **Repeatable** (`--also a --also b`). Values: `claude-code`, `cursor`, `codex`, `gemini`, `windsurf` |
| `--ref <ref>` | string | default branch | Branch or tag to install |
| `--skip-scan` | boolean | `false` | Skip security scanning |
| `--yes` / `-y` | boolean | `false` | Auto-select all skills, auto-accept clean installs. Security warnings still prompt. |
| `--strict` | boolean | from config | Abort on any security warning (exit 1) |
| `--quiet` | boolean | `false` | Suppress install step details (overrides `verbose = true` in config) |
| `--semantic` | boolean | `false` | Force Layer 2 semantic scan |
| `--json` | boolean | `false` | Output as JSON |

### `install plugin` flags

Same as `install skill`, plus capture controls (mutually exclusive — passing both is an error):

| Flag | Type | Description |
|------|------|-------------|
| `--force-capture` | boolean | Capture an existing same-named plugin into the new source non-interactively. |
| `--no-capture` | boolean | Refuse capture even when a name match exists; install side-by-side under a derived name. |

### `install mcp` flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<source>` (positional, required) | string | — | One or more sources (space-separated) |
| `--scope project\|global` | string | smart-scope | Installation scope |
| `--also <agent>` | string | from config | Agent dirs to inject into. **Repeatable.** |
| `--yes` / `-y` | boolean | `false` | Auto-accept prompts |
| `--quiet` | boolean | `false` | Suppress output details |
| `--json` | boolean | `false` | Output as JSON |

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
# Skill from tap name (smart-scope picks project or global)
skilltap install skill commit-helper

# Skill from git URL, explicit project scope
skilltap install skill https://gitea.example.com/u/repo --scope project

# Skill from GitHub shorthand, global, with two agent symlinks
skilltap install skill user/my-skill --scope global --also claude-code --also cursor

# Fully non-interactive (no TTY, --yes)
skilltap install skill commit-helper --scope global --yes

# Plugin
skilltap install plugin corp/dev-toolkit --scope global --also claude-code

# Specific plugin in a multi-plugin repo
skilltap install plugin corp/dev-toolkit:frontend

# Every plugin defined in a multi-plugin repo
skilltap install plugin corp/dev-toolkit:*

# Capture an existing plugin into a new source
skilltap install plugin corp/dev-toolkit --force-capture

# MCP server from npm
skilltap install mcp npm:@modelcontextprotocol/server-postgres
```

---

## skilltap remove

```
skilltap remove skill   [name...] [flags]
skilltap remove plugin  [name...] [flags]
skilltap remove mcp     [name...]  [flags]
```

`remove plugin <name>` removes the plugin record **and all its components** (skills, MCP injections, agent definitions). Calling `remove skill <name>` on a plugin component errors with a hint.

Omit the name to open an interactive picker (TTY only).

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<name>` (positional, optional) | string[] | — | Names to remove (space-separated for multiple) |
| `--scope project\|global` | string | smart-scope | Scope to remove from |
| `--yes` / `-y` | boolean | `false` | Skip confirmation |
| `--json` | boolean | `false` | Output as JSON |

---

## skilltap update

```
skilltap update                  # update everything
skilltap update skill            # update all skills
skilltap update plugin           # update all plugins
skilltap update mcp              # update all standalone MCPs
skilltap update skill <name>     # update one skill
skilltap update plugin <name>
skilltap update mcp <name>
```

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<type>` (positional, optional) | `skill\|plugin\|mcp` | — | Restrict to one type (omit to update everything) |
| `<name>` (positional, optional) | string | — | Restrict to one specific item |
| `--scope project\|global` | string | smart-scope | Scope to update from |
| `--yes` / `-y` | boolean | `false` | Auto-accept clean updates |
| `--strict` | boolean | from config | Skip items with security warnings in diff |
| `--semantic` | boolean | `false` | Run Layer 2 semantic scan on diff |
| `--check` / `-c` | boolean | `false` | Check for updates without applying. Refreshes the update cache. |
| `--force` / `-f` | boolean | `false` | Force update even if already at latest (re-applies and re-scans) |
| `--skip-scan` | boolean | `false` | Skip security scanning |
| `--quiet` | boolean | `false` | Suppress output details |
| `--json` | boolean | `false` | Output as JSON |

---

## skilltap toggle

```
skilltap toggle                              # TUI: pick type → name → component(s)
skilltap toggle skill <name>                 # toggle whole skill active/inactive
skilltap toggle plugin <name>                # TUI scoped to plugin's components
skilltap toggle plugin <name>:<component>    # toggle one component directly
skilltap toggle mcp <name>                   # toggle whole MCP active/inactive
```

Only `plugin` accepts the `:<component>` suffix. Omitting both type and target opens a picker (TTY only); missing one of the two errors with a hint.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<type>` (positional, optional) | `skill\|plugin\|mcp` | — | Type to toggle |
| `<target>` (positional, optional) | string | — | Name (or `name:component` for plugins) |
| `--json` | boolean | `false` | Output as JSON |

---

## skilltap try

Preview a source (clone, parse, scan) without installing. Nothing is written to disk or state.

```
skilltap try <type> <source> [flags]
```

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<type>` (positional, required) | `skill\|plugin\|mcp` | — | What to look for in the source |
| `<source>` (positional, required) | string | — | URL, owner/repo shorthand, npm: prefix, or local path |
| `--skip-scan` | boolean | `false` | Skip the static security scan |
| `--json` | boolean | `false` | Output as JSON |

`try` reads `default_git_host` from `config.toml` so `owner/repo` shorthand resolves the same way as `install`.

---

## skilltap find

```
skilltap find [query] [flags]
```

Multi-word queries work without quoting — `skilltap find git hooks` and `skilltap find "git hooks"` are equivalent (the rest of `args._` is folded into the query).

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<query>` (positional, optional) | string | — | Search term (matched against name, description, tags) |
| `--interactive` / `-i` | boolean | `false` | Interactive search mode with type-ahead filtering |
| `--local` / `-l` | boolean | `false` | Search local taps only (skip registries) |
| `--json` | boolean | `false` | Output as JSON |

---

## skilltap adopt

Bring an external skill or agent-managed plugin into skilltap state.

```
skilltap adopt [path-or-name] [flags]
```

**Modes:**
- No args → TUI picker over all unmanaged skills + Claude Code plugins (TTY required)
- With path → adopt an external directory in place
- `--source claude-code` → picker filtered to one agent's plugins

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<target>` (positional, optional) | string | — | External path, or name of an unmanaged skill or agent-managed plugin |
| `--source <name>` | string | — | Filter picker to one source (e.g., `claude-code`) |
| `--scope project\|global` | string | smart-scope | Scope for adoption |
| `--also <agent>` | string | — | Agent dirs to symlink into. **Repeatable.** |
| `--move` | boolean | `false` | When adopting a path: physically move the directory (default: track-in-place symlink) |
| `--skip-scan` | boolean | `false` | Skip security scan |
| `--yes` / `-y` | boolean | `false` | Auto-accept all prompts |
| `--json` | boolean | `false` | Output as JSON |

---

## skilltap sync

Show drift between `skilltap.toml`, `skilltap.lock`, and the project's `state.json`. Reconciles all three — including `[[mcps]]` entries.

```
skilltap sync [flags]
```

Requires a project root (a directory containing `skilltap.toml` or `.git`). Without one, `sync` errors with a hint.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--apply` | boolean | `false` | Execute the plan via install/remove |
| `--strict` | boolean | `false` | Stop at first failure during apply |
| `--json` | boolean | `false` | Output the plan as JSON |

---

## skilltap doctor

```
skilltap doctor [flags]                 # environment + state health check
skilltap doctor skill <path> [flags]    # validate a skill
skilltap doctor plugin <path> [flags]   # validate a plugin
```

Per-artifact validation replaces the removed `verify` command.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--fix` | boolean | `false` | Auto-repair safe issues (broken symlinks, orphan records, corrupt manifests) |
| `--json` | boolean | `false` | Machine-readable output |

`--fix` exits 0 when all blocking failures are fixed (warnings alone never trigger non-zero). The `--json` payload includes per-check `info` (extra structured context), per-issue `fixDescription` (what `--fix` would do for that issue), and a top-level `detail` summary.

---

## skilltap migrate

```
skilltap migrate [--json]
```

One-shot upgrade from any prior config/state version. Run once after upgrading. The migration:

- Translates `[security.human]` / `[security.agent]` → flat `[security]` (with `scan` / `on_warn` / `trust`)
- Splits operational keys (`agent_cli`, `ollama_model`, `threshold`, `max_size`) into the sibling `[scanner]` block
- Drops `[agent-mode]`, `[agent]`, `[[security.overrides]]`, `preset = …`, `require_scan` (with notes printed for each removed key)
- Consolidates `installed.json` / `plugins.json` → `state.json`; renames originals to `*.v1.bak`
- Converts legacy HTTP taps into errors with a list for manual handling

After migration, `loadConfig` hard-fails on any remaining legacy markers — there is no silent translation at runtime, the migration is the explicit upgrade path.

---

## skilltap status

```
skilltap status [flags]
```

Headless dashboard — skills, plugins, MCPs, taps, drift. Safe to pipe. Equivalent to bare `skilltap` (the TUI) but works without a TTY.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--json` | boolean | `false` | Output as JSON |
| `--unmanaged` | boolean | `false` | Show unmanaged skills (on disk but not in state) |
| `--disabled` | boolean | `false` | Show only disabled items |
| `--active` | boolean | `false` | Show only active items |
| `--global` | boolean | `false` | Show only global scope |
| `--project` | boolean | `false` | Show only project scope |

Note: `status` retains the legacy boolean `--global` / `--project` pair instead of the canonical `--scope` (deliberate carve-out — this is a read-only filter, not an install scope).

---

## Bare `skilltap`

Opens the Ink-based TUI dashboard (TTY only). Tabs: Installed, Taps, Updates, Drift.

Key bindings: arrow keys navigate; `i` install; `r` remove; `t` toggle; `u` update; `f` find; `a` adopt; `q`/`Esc` exit.

Without a TTY: error with hint to use `skilltap status`.

---

## skilltap info

```
skilltap info <name> [flags]
```

Show details about an installed skill, plugin, or MCP server.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<name>` (positional, required) | string | — | Name of the skill, plugin, or MCP server |
| `--global` | boolean | `false` | Restrict lookup to global scope |
| `--project` | boolean | `false` | Restrict lookup to project scope |
| `--json` | boolean | `false` | Output as JSON |

Note: `info` retains the legacy boolean `--global` / `--project` pair (deliberate carve-out — read-only filter).

---

## skilltap move

```
skilltap move <name> --scope project|global [--also <agent>]
```

Move a managed skill between global and project scope. Relocates files, updates `state.json`, recreates symlinks.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<name>` (positional, required) | string | — | Skill name to move |
| `--scope project\|global` | string | required | Target scope (no smart-scope here — must be explicit) |
| `--also <agent>` | string | — | Agent dirs to symlink into. **Repeatable.** |

---

## skilltap create

```
skilltap create [name] [flags]
```

Scaffold a new skill from a template. Opens an interactive wizard in TTY mode; non-interactive when both `name` and `--template` are provided.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<name>` (positional, optional) | string | — | Skill name (kebab-case) |
| `--template` / `-t` | string | (prompt) | Template: `basic`, `npm`, or `multi` |
| `--dir` | string | `./{name}` | Output directory |

---

## skilltap tap

```
skilltap tap add <name> <url>   # Add a git tap
skilltap tap remove <name>      # Remove a tap
skilltap tap list               # List taps
skilltap tap info <name>        # Inspect a tap
skilltap tap init <name>        # Initialize a new tap directory
```

Browsing and installing from taps go through `skilltap find` and `skilltap install skill <name>` — there is no `tap install` subcommand.

---

## skilltap config

```
skilltap config                    # interactive setup wizard
skilltap config get [key]          # get a config value (--json for full dump)
skilltap config set <key> <value>  # set a config value
skilltap config edit               # open config in $EDITOR
skilltap config security           # interactive security wizard
skilltap config telemetry          # telemetry status / enable / disable
```

`config set` is restricted to settable keys (the V2 surface). Internal fields (`telemetry.notice_shown`, `telemetry.anonymous_id`) and tap entries are blocked — use `tap add` / `tap remove` and `config telemetry` for those.

See [config-options.md](/reference/config-options) for the full settable-key list and value rules.

---

## skilltap completions

```
skilltap completions <shell> [--install]
```

Generate a shell tab-completion script for `bash`, `zsh`, or `fish`. Without `--install`, the script is printed to stdout (so you can `eval "$(skilltap completions bash)"`). With `--install`, it's written to the standard location for that shell.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<shell>` (positional, required) | `bash\|zsh\|fish` | — | Shell type |
| `--install` | boolean | `false` | Write to the shell-standard completion location |

---

## skilltap self-update

```
skilltap self-update [--force]
```

Update the running binary in place from GitHub Releases.

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--force` | boolean | `false` | Bypass cache and re-install even if already on the latest version |

---

## Removed in v2.2

The following commands were removed in the v2.2 redesign and emit a hint pointing at the canonical replacement:

| Removed | Replacement |
|---------|-------------|
| `verify <path>` | `doctor skill <path>` (or `doctor plugin <path>`) |
| `link <path>` | `adopt <path>` |
| `unlink <name>` | `remove <type> <name>` |
| `enable <name>` | `toggle <type> <name>[:<component>]` |
| `disable <name>` | `toggle <type> <name>[:<component>]` |
| `skills [...]` | Top-level `list` / `install` / `remove` / `update` / `toggle` |
| `plugin info\|toggle\|remove` | Top-level `info`, `toggle`, `remove` |
| `tap install` | `install skill <name>` (tap-resolved names work directly) |
| `config agent-mode` | No replacement — agent mode (flag, env var, config block) was removed entirely |
| `install <source>` (no type) | `install skill\|plugin\|mcp <source>` |
| `remove <name>` (no type) | `remove skill\|plugin\|mcp <name>` |
| `--no-strict` | Removed; `--strict` remains for one-shot hard-fail. |
| `--project` / `--global` boolean pair (install/remove/update/etc.) | `--scope project\|global` (`info` and `status` keep the legacy pair) |

Old paths return clear errors with hints pointing at the canonical command.
