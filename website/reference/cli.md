---
description: Complete reference for all skilltap CLI commands — install, update, remove, link, find, tap, create, verify, config, doctor, and completions.
---

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
| npm package | `npm:vibe-rules` |
| npm scoped + version | `npm:@scope/skill@1.2.0` |
| Tap name | `commit-helper` |
| Tap name + ref | `commit-helper@v1.2.0` |
| Local path | `./my-skill` |

Source resolution order:

1. `https://`, `http://`, `git@`, `ssh://` — git adapter
2. `npm:` prefix — npm registry adapter
3. `url:` prefix — HTTP tarball (used internally by HTTP registry taps)
4. `./`, `/`, `~/` — local adapter
5. `github:` prefix, or contains `/` with no protocol — GitHub adapter
6. Contains `@` — split into name + ref, resolve from taps
7. Otherwise — search taps for matching skill name

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--project` | boolean | `false` | Install to `.agents/skills/` in current project |
| `--global` | boolean | `false` | Install to `~/.agents/skills/` |
| `--also <agent>` | string | from config | Also create symlink in agent-specific directory. Repeatable. Values: `claude-code`, `cursor`, `codex`, `gemini`, `windsurf` |
| `--ref <ref>` | string | default branch | Branch or tag to install |
| `--yes` | boolean | `false` | Auto-select all skills, auto-accept clean installs, auto-update already-installed skills, skip agent symlink prompt. Security warnings still prompt. |
| `--strict` | boolean | from config | Abort on any security warning (exit 1) |
| `--no-strict` | boolean | `false` | Override `on_warn = "fail"` in config for this invocation |
| `--semantic` | boolean | from config | Force Layer 2 semantic scan (runs automatically, no prompt) |
| `--skip-scan` | boolean | `false` | Skip security scanning. Blocked if `require_scan = true` in config. |

### Prompt Behavior

Prompts appear in this order: scope → agents → (clone) → skill selection → conflict check → scan → install confirm.

| Flags | Scope | Agents (--also) | Skill selection | Already-installed conflict | Static warnings | Install confirm |
|-------|-------|-----------------|-----------------|---------------------------|-----------------|-----------------|
| (none) | Prompt | Prompt | Prompt if multiple | Prompt to update | Prompt | **Prompt (Y/n)** |
| `--project` | Project | Prompt | Prompt if multiple | Prompt to update | Prompt | **Prompt (Y/n)** |
| `--global` | Global | Prompt | Prompt if multiple | Prompt to update | Prompt | **Prompt (Y/n)** |
| `--also <agent>` | Prompt | Skipped | Prompt if multiple | Prompt to update | Prompt | **Prompt (Y/n)** |
| `--yes` | **Still prompts** | Config default | Auto-select all | **Auto-update** | **Still prompts** | Auto-accept if clean |
| `--yes --global` | Global | Config default | Auto-select all | **Auto-update** | **Still prompts** | Auto-accept if clean |
| `--semantic` | Prompt | Prompt | Prompt if multiple | Prompt to update | Prompt | **Prompt (Y/n)** |
| `--strict` | Prompt | Prompt | Prompt if multiple | Prompt to update | **Abort (exit 1)** | -- |
| `--strict --yes --global` | Global | Config default | Auto-select all | **Auto-update** | **Abort (exit 1)** | -- |
| `--skip-scan --yes --global` | Global | Config default | Auto-select all | **Auto-update** | Skipped | Auto-accept |

Notes:
- **Already-installed conflict**: if a selected skill is already installed, skilltap prompts `"{name}" is already installed. Update it instead?` before running the security scan. With `--yes`, the update runs automatically. Non-conflicting skills install normally.
- **Agent symlink prompt** (`--also`): skipped when `--also` is passed, `--yes` is set, **or** `config.defaults.also` is non-empty (saved default from `skilltap config`).
- `--semantic` causes the semantic scan to run automatically without a "Run semantic scan?" prompt.
- The semantic offer prompt only appears when static warnings are found and `--semantic` was not passed.
- When the semantic scan runs for the first time and no agent is configured, skilltap prompts to pick an agent CLI. The choice is saved to `config.toml`.

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

# Install from npm
skilltap install npm:vibe-rules
skilltap install npm:@scope/my-skills@2.0.0

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

Remove one or more installed skills.

```
skilltap remove [name...] [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | No | Name(s) of installed skills; omit to select interactively |

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--project` | boolean | `false` | Remove from project scope instead of global |
| `--yes` | boolean | `false` | Skip confirmation prompt |

### Behavior

When no name is given, shows an interactive multiselect of all installed skills (no separate confirmation step). When names are supplied, validates each exists and exits on the first unknown name; duplicate names are ignored. For each skill, removes the skill directory, any agent-specific symlinks (from the `also` list), and the cache entry if this was the last skill from that repo. Updates `installed.json` after each removal.

### Examples

```bash
# Remove with confirmation prompt
skilltap remove commit-helper

# Skip confirmation
skilltap remove commit-helper --yes

# Remove from project scope
skilltap remove termtube-dev --project

# Remove multiple skills at once
skilltap remove skill-a skill-b --yes

# Interactive multiselect — pick from all installed skills
skilltap remove
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
Global (2 skills)
  Name           Ref     Source                                   Trust          Description
  ────────────────────────────────────────────────────────────────────────────────────────────────
  commit-helper  v1.2.0  https://github.com/user/commit-helper…  ✓ provenance   Conventional commit messages
  code-review    v2.0.0  https://github.com/user/code-review…    ◆ curated      Thorough code review

Project (2 skills)
  Name            Ref   Source                                  Trust          Description
  ──────────────────────────────────────────────────────────────────────────────────────────
  termtube-dev    main  https://github.com/user/termtube…       ○ unverified   Development workflow
```

Columns: Name, Ref, Source (raw repo URL, truncated), Trust, Description. Linked skills appear in a separate **Linked** section.

The Source column shows the repo URL (or `local` for linked skills, `npm:...` for npm sources).

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
| `--agent <name>` | string | from config | Agent CLI for semantic scan (e.g. `"claude-code"`). See [config reference](/reference/config-options#security) for all supported values. |

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

Search for skills across configured taps and the [skills.sh](https://skills.sh) public registry.

```
skilltap find [query...] [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `query` | No | Search term. Multiple words can be given without quoting. |

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-i` | boolean | `false` | Force interactive search mode. Prompts for query if not given, then shows autocomplete picker. |
| `-l, --local` | boolean | `false` | Search local taps only (skip registries) |
| `--json` | boolean | `false` | Output as JSON |

### Behavior

- **No query, TTY**: enters interactive search — prompts for a search term, searches taps + registries, then shows an autocomplete picker. Enter on a result installs it.
- **No query, non-TTY**: lists all skills from configured taps as a table.
- **With query** (≥ 2 chars): searches taps locally, then appends results from the skills.sh registry (up to 20 results, sorted by install count descending).
- **`-i` with query**: skips the search prompt, goes straight to the autocomplete picker with results.
- **With `--local`**: skips all registry searches, only shows tap results.
- Install counts are shown for skills.sh results (e.g., `184.5K installs`).
- For skills.sh multi-skill repos, the specific skill is auto-selected during install — no extra prompt.
- Picker hints adapt to terminal width — descriptions are truncated to fit.

### Examples

```bash
# Search taps + skills.sh registry
skilltap find react

# Multi-word query — no quoting needed
skilltap find git hooks

# Search taps only, skip registries
skilltap find --local react

# Interactive search — prompts for query, then autocomplete picker
skilltap find

# Force interactive with a pre-filled query
skilltap find react -i

# Machine-readable output
skilltap find --json
```

If no taps are configured and no query is given (non-TTY): `No taps configured. Run 'skilltap tap add <name> <url>' to add one.`

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

For an installed skill (key-value rows, keys left-padded to 13 chars):

```
name:          commit-helper
description:   Generates conventional commit messages
scope:         global
source:        https://gitea.example.com/nathan/commit-helper
ref:           v1.2.0
sha:           abc123d
trust:         ✓ Provenance verified
  source:      github.com/nathan/commit-helper
  build:       .github/workflows/release.yml
path:          /home/nathan/.agents/skills/commit-helper
agents:        claude-code
installed:     2026-02-28T12:00:00.000Z
updated:       2026-02-28T12:00:00.000Z
```

`agents:` shows which agent-specific symlinks currently exist on disk.

For a skill available in a tap (not installed):

```
name:          commit-helper
description:   Some useful skill
status:        (available)
tap:           community
source:        https://github.com/someone/commit-helper
tags:          productivity, workflow

Run 'skilltap install commit-helper' to install.
```

If not found anywhere: exit 1 with `error: Skill 'name' is not installed` and hint `Run 'skilltap find name' to search`.

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
6. Search public registries (skills.sh) when using `skilltap find` (yes / no)
7. Anonymous usage telemetry (yes / no)

Writes the result to `~/.config/skilltap/config.toml`.

### Examples

```bash
# Run the setup wizard
skilltap config

# Reset and reconfigure
skilltap config --reset
```

---

## skilltap config get

Read config values. Non-interactive — safe for agents and scripts.

```
skilltap config get [key] [--json]
```

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--json` | boolean | `false` | Output as JSON |

### Behavior

- `skilltap config get <key>` — prints the value for a dot-notation key (e.g. `defaults.scope`)
- `skilltap config get --json` — prints the full config as JSON
- `skilltap config get <key> --json` — prints the single value as JSON
- Arrays are printed space-separated in plain text mode
- No key without `--json` prints all values as `section.field = value` lines
- Unknown keys exit 1 with an error message

### Examples

```bash
skilltap config get defaults.scope
# → global

skilltap config get defaults.also
# → claude-code cursor

skilltap config get --json
# → { "defaults": { ... }, "security": { ... }, ... }

skilltap config get security.scan --json
# → "static"
```

---

## skilltap config set

Set config values. Non-interactive — safe for agents and scripts.

```
skilltap config set <key> <value...>
```

Only preference keys are settable. Security policy keys (`security.scan`, `security.on_warn`, `security.require_scan`, `security.max_size`, `security.threshold`), agent mode keys, and telemetry keys are blocked with hints pointing to the appropriate command.

### Settable Keys

| Key | Type | Accepted values |
|-----|------|-----------------|
| `defaults.scope` | enum | `""`, `"global"`, `"project"` |
| `defaults.also` | string[] | Agent names (variadic; omit values to clear) |
| `defaults.yes` | boolean | `true`/`false`/`yes`/`no`/`1`/`0` |
| `security.agent` | string | Agent CLI name or absolute path |
| `security.ollama_model` | string | Model name |
| `updates.auto_update` | enum | `"off"`, `"patch"`, `"minor"` |
| `updates.interval_hours` | number | Positive integer |

### Behavior

- Silent on success (exit 0, no stdout). Agent-friendly.
- Invalid key, blocked key, or invalid value: error on stderr, exit 1.
- For `string[]` type with zero values, sets to empty array (clears the field).

### Examples

```bash
skilltap config set defaults.scope global
skilltap config set defaults.also claude-code cursor
skilltap config set defaults.also          # clears to []
skilltap config set defaults.yes true
skilltap config set updates.auto_update patch

# Blocked keys show hints:
skilltap config set agent-mode.enabled true
# error: 'agent-mode.enabled' cannot be set via 'config set'
# hint: Use 'skilltap config agent-mode'
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

Add a tap. Supports git repos, HTTP registry endpoints, and GitHub shorthand.

```
skilltap tap add <name> <url>
skilltap tap add <owner/repo>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Local name for this tap, or GitHub shorthand (`owner/repo`) |
| `url` | No | Git URL or HTTP registry URL (required unless GitHub shorthand is used) |

### Behavior

When two positional args are given, the first is the tap name and the second is the URL.

When one positional arg is given and it matches `owner/repo` or `github:owner/repo`, the URL is expanded to `https://github.com/owner/repo.git` and the tap name is derived from the repo portion (e.g. `user/my-tap` → name `my-tap`).

Auto-detects the tap type by probing the URL. If the URL returns a valid HTTP registry response, it's registered as an HTTP tap (no local clone). Otherwise, it clones the repo to `~/.config/skilltap/taps/{name}/`, validates `tap.json`, and records the tap in `config.toml`.

If the tap name already exists: exit 1 with `Tap 'name' already exists. Remove it first with 'skilltap tap remove name'.`

### Examples

```bash
# GitHub shorthand — name derived from repo
skilltap tap add someone/awesome-skills-tap

# Explicit name + URL
skilltap tap add home https://gitea.example.com/nathan/my-skills-tap

# HTTP registry tap (auto-detected)
skilltap tap add enterprise https://skills.example.com/api/v1
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
  home        git   https://gitea.example.com/nathan/my-skills-tap     3 skills
  community   git   https://github.com/someone/awesome-skills-tap      12 skills
  enterprise  http  https://skills.example.com/api/v1                  47 skills
```

Columns: name, type (`git`/`http`), URL, skill count.

If no taps configured: `No taps configured. Run 'skilltap tap add <name> <url>' to add one.`

### Examples

```bash
skilltap tap list
```

---

## skilltap tap update

Update tap repos (git pull). HTTP taps are always live — this is a no-op for them.

```
skilltap tap update [name]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | No | Specific tap to update. If omitted, updates all git taps. |

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

---

## skilltap create

Scaffold a new skill from a template.

```
skilltap create [name] [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | No | Skill name (kebab-case). If omitted, prompted interactively. |

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--template`, `-t` | `basic` \| `npm` \| `multi` | prompted | Template to use |
| `--dir` | string | `./<name>` | Directory to create the skill in |

When both `name` and `--template` are provided, runs non-interactively (uses `"{name} skill"` as description, MIT as license). Otherwise prompts for name, description, template, and license.

### Templates

| Template | Generated files |
|----------|-----------------|
| `basic` | `SKILL.md`, `.gitignore` |
| `npm` | `SKILL.md`, `.gitignore`, `package.json`, `.github/workflows/publish.yml` |
| `multi` | `.agents/skills/<skill-a>/SKILL.md`, `.agents/skills/<skill-b>/SKILL.md`, `.gitignore` |

### Behavior

Creates the skill directory (errors if it already exists), writes template files, and prints next-step instructions for testing, verifying, and publishing.

### Examples

```bash
# Interactive mode
skilltap create

# Non-interactive
skilltap create my-skill --template basic

# npm package skill
skilltap create my-skill --template npm --description "My skill description"

# Custom output directory
skilltap create my-skill --template basic --dir ~/dev/my-skill
```

---

## skilltap verify

Validate a skill before publishing.

```
skilltap verify [path] [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `path` | No | Path to the skill directory. Defaults to `.`. |

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--json` | boolean | `false` | Output result as JSON |

### Behavior

Runs the following checks:

1. `SKILL.md` exists
2. Frontmatter is valid (required fields present, values within constraints)
3. `name` field matches the parent directory name
4. No static security issues (same checks as `skilltap install`)
5. Total directory size is within the limit

On success, also prints a `tap.json` snippet ready to paste into a tap.

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All checks passed |
| `1` | One or more errors found |

### Output

```
✓ commit-helper is valid

  SKILL.md   ✓
  name       ✓ matches directory
  security   ✓ no issues
  size       ✓ 4.2 KB (3 files)

tap.json snippet:

  {
    "name": "commit-helper",
    "description": "Generates conventional commit messages",
    "repo": "https://github.com/user/commit-helper",
    "tags": []
  }
```

With `--json`:

```json
{
  "name": "commit-helper",
  "valid": true,
  "issues": [],
  "frontmatter": { "name": "commit-helper", "description": "Generates conventional commit messages" },
  "fileCount": 3,
  "totalBytes": 4301
}
```

### Examples

```bash
# Verify current directory
skilltap verify

# Verify a specific skill
skilltap verify ./path/to/my-skill

# CI-friendly output
skilltap verify --json

# Pre-push git hook
echo 'skilltap verify' > .git/hooks/pre-push && chmod +x .git/hooks/pre-push
```

---

## skilltap status

Show agent mode status and current configuration summary. Agents should run this first to verify they are operating in agent mode.

```
skilltap status [--json]
```

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--json` | boolean | `false` | Output as JSON |

### Output

**Plain text** (one `key: value` line per field):

```
agent-mode: enabled
scope: project
scan: static
agent: (none)
also: claude-code
taps: 2
```

**JSON:**

```json
{
  "agentMode": true,
  "scope": "project",
  "scan": "static",
  "agent": null,
  "also": ["claude-code"],
  "taps": 2
}
```

| Field | Description |
|-------|-------------|
| `agentMode` | Whether agent mode is enabled |
| `scope` | Default install scope (`project`, `global`, or `null` if unconfigured) |
| `scan` | Security scan level (`static`, `semantic`, `off`) |
| `agent` | Configured LLM for semantic scan, or `null` |
| `also` | Agent directories to symlink into on each install |
| `taps` | Number of configured taps |

Does not trigger the update check or telemetry notice.

---

## skilltap doctor

Check your environment, configuration, and installed state for problems.

```
skilltap doctor [flags]
```

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--fix` | boolean | `false` | Auto-repair issues where safe (recreate symlinks, remove orphan records, create missing dirs, re-clone missing taps) |
| `--json` | boolean | `false` | Output as JSON (for CI/scripting) |

### Checks

Doctor runs 9 independent checks and streams each result as it completes:

| # | Check | What it verifies |
|---|-------|-----------------|
| 1 | git | git is on PATH, version ≥ 2.25 |
| 2 | config | `config.toml` exists, valid TOML, passes schema |
| 3 | dirs | All required directories exist |
| 4 | installed.json | State file parses and passes schema validation |
| 5 | skills | Every `installed.json` entry has a directory on disk |
| 6 | symlinks | Agent symlinks for `also` entries are valid |
| 7 | taps | Each configured tap has a valid local clone |
| 8 | agents | Detected agent CLIs; configured agent is available |
| 9 | npm | npm is on PATH and registry reachable (only if npm skills are installed) |

A failure in one check does not skip subsequent checks.

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All checks pass, or only warnings |
| `1` | One or more failures (corrupt files, missing git) |

Warnings alone (missing optional features) produce exit code 0.

### Examples

```bash
# Check everything
skilltap doctor

# Auto-repair where possible
skilltap doctor --fix

# CI health check (machine-readable)
skilltap doctor --json
```

See the [Doctor guide](/guide/doctor) for detailed output examples and what each check covers.

---

## skilltap completions

Generate a shell completion script for bash, zsh, or fish.

```
skilltap completions <shell> [flags]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `shell` | Yes | `bash`, `zsh`, or `fish` |

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--install` | boolean | `false` | Write the completion script to the shell's standard location |

### Without `--install`

Prints the completion script to stdout. Useful for piping to a file or evaluating inline:

```bash
# Evaluate inline (add to ~/.bashrc)
eval "$(skilltap completions bash)"

# Write to a file manually
skilltap completions zsh > ~/.zfunc/_skilltap
```

### With `--install`

Writes to the standard location for each shell and prints activation instructions. If `$SHELL` doesn't match the specified shell, a hint is printed to stderr.

| Shell | Writes to |
|-------|-----------|
| bash | `~/.local/share/bash-completion/completions/skilltap` |
| zsh | `~/.zfunc/_skilltap` (also patches `~/.zshrc` with `fpath` setup if missing) |
| fish | `~/.config/fish/completions/skilltap.fish` |

### Dynamic Completions

The completion scripts call `skilltap --get-completions <type>` to provide live values:

- `installed-skills` — for `remove`, `update`, `info`
- `linked-skills` — for `unlink`
- `tap-skills` — for `install`
- `tap-names` — for `tap remove`, `tap update`

### Examples

```bash
# Install for your shell
skilltap completions bash --install
skilltap completions zsh --install
skilltap completions fish --install

# Print script (manual setup)
skilltap completions bash
```

See the [Shell Completions guide](/guide/shell-completions) for setup details and troubleshooting.

---

## skilltap config telemetry

Manage anonymous usage telemetry. Subcommand of `skilltap config`.

```
skilltap config telemetry <subcommand>
```

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `status` | Show current telemetry state and what is collected |
| `enable` | Opt in to anonymous telemetry |
| `disable` | Opt out of telemetry |

### Behavior

Telemetry preference is stored in `config.toml` under `[telemetry]`. Two environment variables always override the config:

| Variable | Effect |
|----------|--------|
| `DO_NOT_TRACK=1` | Disables telemetry regardless of config |
| `SKILLTAP_TELEMETRY_DISABLED=1` | Disables telemetry regardless of config |

**What is collected:** OS, architecture, CLI version, command name, success/failure, error type, installed skill count, command duration. No skill names, paths, repo URLs, or personal information are ever collected.

**`config telemetry status` output (example):**
```
Telemetry: enabled
Anonymous ID: a3f8c1d2-...

What's collected: OS, arch, CLI version, command success/failure,
error type, skill count, duration. No skill names, paths, or personal info.
Set DO_NOT_TRACK=1 or SKILLTAP_TELEMETRY_DISABLED=1 to always opt out.
```

**First-run consent prompt:** On the first interactive run (before a preference is recorded), skilltap asks via a yes/no prompt whether you want to share anonymous usage data. Answering yes enables telemetry and fires a one-time `skilltap_installed` event recording that you installed the tool. In non-interactive environments (piped input, CI), a one-time informational banner is printed to stderr instead, and telemetry remains disabled.

Running `skilltap config` also includes a telemetry opt-in/out question.

Set `DO_NOT_TRACK=1` to skip the prompt entirely without opting in. The `config telemetry` and `status` commands never trigger the prompt.

### Examples

```bash
# Check current state
skilltap config telemetry status

# Opt in
skilltap config telemetry enable

# Opt out
skilltap config telemetry disable
```

---

## skilltap self-update

Update the skilltap binary to the latest GitHub release.

```
skilltap self-update [flags]
```

### Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--force` | boolean | `false` | Bypass cache and re-install even if already on the latest version |

### Behavior

Fetches the latest release from GitHub and replaces the running binary in-place. Only works when running as a compiled binary (installed via the install script or downloaded directly). If you installed via npm or are running from source, it prints the appropriate update command instead.

**Platform support:** Linux x64/arm64 and macOS x64/arm64.

### Startup Notifications

skilltap checks for updates in the background on every command (except `self-update`, `status`, `--version`, `--help`, and agent mode). It reads a local cache so the check never blocks startup. When an update is found, a notice is printed to stderr:

| Type | Message |
|------|---------|
| patch | `↑  skilltap 0.3.1 → 0.3.2 available. Run: skilltap self-update` (dim) |
| minor | `↑  Update available: v0.3.1 → v0.4.0 (minor) Run: skilltap self-update` (bold) |
| major | `⚠  Major update available: v0.3.1 → v1.0.0  Breaking changes may apply.` (yellow) |

Configure automatic updates via the `[updates]` config section — see [Configuration Options](/reference/config-options#updates).

### Examples

```bash
# Check and update to the latest release
skilltap self-update

# Force a fresh fetch from GitHub and re-install (bypasses cache)
skilltap self-update --force
```
