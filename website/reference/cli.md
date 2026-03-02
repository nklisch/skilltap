# CLI Reference

Complete reference for all skilltap commands, arguments, flags, and behavior.

## Global Behavior

- **Exit codes:** `0` success, `1` error, `2` user cancelled
- **Errors** are written to stderr with an `error:` prefix and optional `hint:`
- **Config** is stored at `~/.config/skilltap/config.toml`
- **State** is tracked in `~/.config/skilltap/installed.json`
- **Agent mode** (when enabled via config) changes all commands to non-interactive, strict-security, plain-text output

---

## skilltap install

Install a skill from a URL, tap name, or local path.

```
skilltap install <source> [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `source` | Yes | Git URL, `github:owner/repo`, `owner/repo`, tap skill name, `name@ref`, or local path |

### Source Formats

| Format | Example |
|--------|---------|
| Git URL (any host) | `https://gitea.example.com/user/repo` |
| SSH | `git@github.com:user/repo.git` |
| GitHub shorthand | `user/repo` |
| GitHub explicit | `github:user/repo` |
| Tap name | `commit-helper` |
| Tap name + ref | `commit-helper@v1.2.0` |
| Local path | `./my-skill` |

Source resolution order:

1. `https://`, `http://`, `git@`, `ssh://` -- git adapter
2. `github:` prefix -- GitHub adapter
3. `./`, `/`, `~/` -- local adapter
4. Contains `/` with no protocol -- treated as `github:source`
5. Contains `@` -- split into name + ref, resolve from taps
6. Otherwise -- search taps for matching skill name

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--project` | boolean | `false` | Install to `.agents/skills/` in current project |
| `--global` | boolean | `false` | Install to `~/.agents/skills/` |
| `--also <agent>` | string | from config | Also create symlink in agent-specific directory. Repeatable. Values: `claude-code`, `cursor`, `codex`, `gemini`, `windsurf` |
| `--ref <ref>` | string | default branch | Branch or tag to install |
| `--yes` | boolean | `false` | Auto-select all skills, auto-accept clean installs. Security warnings still prompt. |
| `--strict` | boolean | from config | Abort on any security warning (exit 1) |
| `--no-strict` | boolean | `false` | Override `on_warn = "fail"` in config for this invocation |
| `--semantic` | boolean | from config | Force Layer 2 semantic scan |
| `--skip-scan` | boolean | `false` | Skip security scanning. Blocked if `require_scan = true` in config. |

### Prompt Behavior

| Flags | Skill selection | Scope | Security warnings | Clean install |
|-------|----------------|-------|-------------------|---------------|
| (none) | Prompt if multiple | Prompt | Prompt | Prompt |
| `--project` | Prompt if multiple | Project | Prompt | Prompt |
| `--global` | Prompt if multiple | Global | Prompt | Prompt |
| `--yes` | Auto-select all | **Still prompts** | **Still prompts** | Auto-accept |
| `--yes --global` | Auto-select all | Global | **Still prompts** | Auto-accept |
| `--yes --project` | Auto-select all | Project | **Still prompts** | Auto-accept |
| `--strict` | Prompt if multiple | Prompt | **Abort (exit 1)** | Prompt |
| `--strict --yes --global` | Auto-select all | Global | **Abort (exit 1)** | Auto-accept |
| `--skip-scan --yes --global` | Auto-select all | Global | Skipped | Auto-accept |

::: info Scope always prompts
`--yes` does **not** skip the scope prompt. Use `--yes --global` or `--yes --project` for fully non-interactive installs.
:::

::: warning Security is a hard gate
`--yes` does **not** bypass security warnings. `--strict` goes further: any warning is a hard failure. The only way to skip scanning entirely is `--skip-scan`, which is blocked when `require_scan = true`.
:::

### Examples

```bash
# Install from any git URL
skilltap install https://gitea.example.com/user/commit-helper

# GitHub shorthand
skilltap install user/repo

# Install a specific version from a tap
skilltap install commit-helper@v1.2.0

# Fully non-interactive (clean skills only)
skilltap install commit-helper --yes --global

# Install to project scope with agent symlinks
skilltap install my-skill --project --also claude-code --also cursor

# Strict security + semantic scan
skilltap install some-skill --strict --semantic
```

---

## skilltap remove

Remove an installed skill.

```
skilltap remove <name> [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Name of installed skill |

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--project` | boolean | `false` | Remove from project scope instead of global |
| `--yes` | boolean | `false` | Skip confirmation prompt |

### Behavior

Removes the skill directory, any agent-specific symlinks (from the `also` list), and the cache entry if this was the last skill from a multi-skill repo. Updates `installed.json`.

### Examples

```bash
# Remove with confirmation prompt
skilltap remove commit-helper

# Skip confirmation
skilltap remove commit-helper --yes

# Remove from project scope
skilltap remove termtube-dev --project
```

---

## skilltap list

List installed skills.

```
skilltap list [flags]
```

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--global` | boolean | `false` | Show only global skills |
| `--project` | boolean | `false` | Show only project skills |
| `--json` | boolean | `false` | Output as JSON |

### Output Format

```
Global:
  commit-helper      v1.2.0   home    Conventional commit messages
  code-review        v2.0.0   home    Thorough code review

Project (/home/nathan/dev/termtube):
  termtube-dev       main     local   Development workflow
```

Columns: name, ref, source (tap name or `local`/`url`), description (truncated to terminal width).

If no skills are installed: `No skills installed. Run 'skilltap install <url>' to get started.`

### Examples

```bash
# List all installed skills
skilltap list

# List only global skills
skilltap list --global

# Machine-readable output
skilltap list --json
```

---

## skilltap update

Update installed skills.

```
skilltap update [name] [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | No | Specific skill to update. If omitted, updates all. |

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--yes` | boolean | `false` | Auto-accept clean updates. Security warnings still prompt. |
| `--strict` | boolean | from config | Skip skills with security warnings in the diff (does not abort; continues to next skill). |
| `--semantic` | boolean | from config | Force Layer 2 semantic scan on diff content |

### Behavior

For each skill:

1. `git fetch` in installed dir (standalone) or cache dir (multi-skill)
2. Compare local HEAD SHA to remote
3. If identical: `Already up to date.`
4. If different: show diff summary, run static scan on changed lines only
5. If `--strict` and warnings found: skip this skill, continue to next
6. If warnings found (not strict): prompt to apply
7. Apply: `git pull` (standalone) or pull cache + re-copy (multi-skill)
8. Update `installed.json` with new SHA and `updatedAt`
9. Re-create agent symlinks if target dirs are missing

Linked skills (from `skilltap link`) are always skipped.

### Examples

```bash
# Update all skills interactively
skilltap update

# Auto-accept clean updates
skilltap update --yes

# Update one skill
skilltap update commit-helper

# CI: fail on any new warnings
skilltap update --strict
```

---

## skilltap find

Search for skills across all configured taps.

```
skilltap find [query] [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `query` | No | Search term (fuzzy matched against name, description, tags). If omitted, lists all skills. |

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-i` | boolean | `false` | Interactive fuzzy finder mode |
| `--json` | boolean | `false` | Output as JSON |

### Examples

```bash
# Search for skills matching "review"
skilltap find review

# List all skills from all taps
skilltap find

# Interactive fuzzy finder (type to filter, arrow keys, Enter to install)
skilltap find -i

# Machine-readable output
skilltap find --json
```

If no taps are configured: `No taps configured. Run 'skilltap tap add <name> <url>' to add one.`

---

## skilltap link

Symlink a local skill directory into the install path. For development workflows.

```
skilltap link <path> [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `path` | Yes | Path to local skill directory (must contain SKILL.md) |

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--project` | boolean | `false` | Link to project scope instead of global |
| `--global` | boolean | `false` | Link to global scope (explicit, for scripts) |
| `--also <agent>` | string | from config | Also symlink to agent-specific directory. Repeatable. |

### Behavior

Resolves the path to absolute, validates SKILL.md exists, parses frontmatter for the skill name, and creates a symlink at the install path. Records in `installed.json` with `scope: "linked"`.

Does **not** clone or copy -- the symlink points to the original directory.

### Examples

```bash
# Link current directory
skilltap link .

# Link to project scope with agent symlink
skilltap link . --project --also claude-code

# Link a specific path
skilltap link ~/dev/my-new-skill
```

---

## skilltap unlink

Remove a linked skill.

```
skilltap unlink <name>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Name of linked skill |

### Behavior

Verifies the skill was linked (not installed via clone), removes the symlink from the install path, removes any agent-specific symlinks, and updates `installed.json`.

Does **not** delete the original skill directory.

### Examples

```bash
skilltap unlink my-new-skill
```

---

## skilltap info

Show details about an installed or available skill.

```
skilltap info <name>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Skill name |

### Output

For an installed skill:

```
commit-helper (installed, global)
  Generates conventional commit messages
  Source: https://gitea.example.com/nathan/commit-helper
  Ref:    v1.2.0 (abc123de)
  Tap:    home
  Also:   claude-code
  Size:   12.3 KB (3 files)
  Installed: 2026-02-28
  Updated:   2026-02-28
```

For a linked skill:

```
my-local-skill (linked, global)
  My development skill
  Path:   /home/nathan/dev/my-local-skill
  Also:   ---
  Linked: 2026-02-28
```

For a skill available in a tap (not installed):

```
unknown-skill (available)
  Some useful skill
  Repo: https://github.com/someone/unknown-skill
  Tap:  community
  Tags: productivity, workflow

  Run 'skilltap install unknown-skill' to install.
```

If not found anywhere: exit 1 with `Skill 'name' not found. Try 'skilltap find name'.`

### Examples

```bash
skilltap info commit-helper
skilltap info termtube-dev
```

---

## skilltap config

Interactive setup wizard for generating `config.toml`.

```
skilltap config [flags]
```

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--reset` | boolean | `false` | Overwrite existing config (prompts for confirmation first) |

**Always interactive.** This command requires a TTY. It cannot be run non-interactively or by an agent.

### Wizard Flow

The wizard prompts for:

1. Default install scope (ask each time / always global / always project)
2. Auto-symlink agent selection (Claude Code, Cursor, Codex, Gemini, Windsurf)
3. Security scan level (static / static + semantic / off)
4. Agent CLI for scanning (if semantic selected)
5. Behavior when security warnings are found (ask / always block)

Writes the result to `~/.config/skilltap/config.toml`.

### Examples

```bash
# Run the setup wizard
skilltap config

# Reset and reconfigure
skilltap config --reset
```

---

## skilltap config agent-mode

Interactive wizard for enabling or disabling agent mode.

```
skilltap config agent-mode
```

**Always interactive.** Requires a TTY. This is the **only** way to toggle agent mode -- there are no CLI flags or environment variables. An agent cannot enable or disable its own safety constraints.

### Wizard Flow (enabling)

The wizard prompts for:

1. Enable or disable agent mode
2. Default scope for agent installs (project recommended / global)
3. Auto-symlink agent selection
4. Security scan level (static / static + semantic; "off" is not offered)
5. Agent CLI for scanning (if semantic selected)

### What Agent Mode Does

When enabled, all skilltap commands behave differently:

- All prompts auto-accept or hard-fail (no interactive input)
- Security warnings always block installation (`on_warn = "fail"`)
- Security scanning cannot be skipped (`require_scan = true`)
- Output is plain text (no colors, spinners, or Unicode)
- Security failures emit a directive message telling the agent to stop

These constraints are **not overridable** via CLI flags.

### Non-TTY Error

```
error: 'skilltap config agent-mode' must be run interactively.
Agent mode can only be enabled or disabled by a human.
```

### Examples

```bash
# Enable agent mode
skilltap config agent-mode

# Disable agent mode (select "No" in the wizard)
skilltap config agent-mode
```

---

## skilltap tap add

Add a tap (a git repo containing `tap.json`).

```
skilltap tap add <name> <url>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Local name for this tap |
| `url` | Yes | Git URL of the tap repo |

### Behavior

Clones the tap repo to `~/.config/skilltap/taps/{name}/`, validates `tap.json` exists and parses correctly, and records the tap in `config.toml`.

If the tap name already exists: exit 1 with `Tap 'name' already exists. Remove it first with 'skilltap tap remove name'.`

### Examples

```bash
skilltap tap add home https://gitea.example.com/nathan/my-skills-tap
skilltap tap add community https://github.com/someone/awesome-skills-tap
```

---

## skilltap tap remove

Remove a configured tap.

```
skilltap tap remove <name>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Name of tap to remove |

### Behavior

Removes the tap directory from `~/.config/skilltap/taps/{name}/` and the tap entry from `config.toml`.

Does **not** uninstall skills that were installed from this tap. Those skills remain independent.

### Examples

```bash
skilltap tap remove community
```

---

## skilltap tap list

List configured taps.

```
skilltap tap list
```

### Output

```
  home       https://gitea.example.com/nathan/my-skills-tap     3 skills
  community  https://github.com/someone/awesome-skills-tap      12 skills
```

If no taps configured: `No taps configured. Run 'skilltap tap add <name> <url>' to add one.`

### Examples

```bash
skilltap tap list
```

---

## skilltap tap update

Update tap repos (git pull).

```
skilltap tap update [name]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | No | Specific tap to update. If omitted, updates all. |

### Examples

```bash
# Update all taps
skilltap tap update

# Update a specific tap
skilltap tap update home
```

---

## skilltap tap init

Initialize a new tap repository.

```
skilltap tap init <name>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Directory name for the new tap |

### Behavior

Creates the directory, initializes a git repo, and generates a `tap.json` with an empty skills array.

### Output

```
Created my-tap/
  tap.json
  .git/

Edit tap.json to add skills, then push:
  cd my-tap && git remote add origin <url> && git push
```

### Examples

```bash
skilltap tap init my-tap
```
