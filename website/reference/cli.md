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
| `--yes` | boolean | `false` | Auto-select all skills, auto-accept clean installs, skip agent symlink prompt. Security warnings still prompt. |
| `--strict` | boolean | from config | Abort on any security warning (exit 1) |
| `--no-strict` | boolean | `false` | Override `on_warn = "fail"` in config for this invocation |
| `--semantic` | boolean | from config | Force Layer 2 semantic scan (runs automatically, no prompt) |
| `--skip-scan` | boolean | `false` | Skip security scanning. Blocked if `require_scan = true` in config. |

### Prompt Behavior

Prompts appear in this order: scope → agents → (clone) → skill selection → scan → install confirm.

| Flags | Scope | Agents (--also) | Skill selection | Static warnings | Semantic offer | Install confirm |
|-------|-------|-----------------|-----------------|-----------------|----------------|-----------------|
| (none) | Prompt | Prompt | Prompt if multiple | Prompt | Offered if warnings | **Prompt (Y/n)** |
| `--project` | Project | Prompt | Prompt if multiple | Prompt | Offered if warnings | **Prompt (Y/n)** |
| `--global` | Global | Prompt | Prompt if multiple | Prompt | Offered if warnings | **Prompt (Y/n)** |
| `--also <agent>` | Prompt | Skipped | Prompt if multiple | Prompt | Offered if warnings | **Prompt (Y/n)** |
| `--yes` | **Still prompts** | Config default | Auto-select all | **Still prompts** | Offered if warnings | Auto-accept if clean |
| `--yes --global` | Global | Config default | Auto-select all | **Still prompts** | Offered if warnings | Auto-accept if clean |
| `--semantic` | Prompt | Prompt | Prompt if multiple | Prompt | **Always auto-runs** | **Prompt (Y/n)** |
| `--strict` | Prompt | Prompt | Prompt if multiple | **Abort (exit 1)** | -- | -- |
| `--strict --yes --global` | Global | Config default | Auto-select all | **Abort (exit 1)** | -- | -- |
| `--skip-scan --yes --global` | Global | Config default | Auto-select all | Skipped | Skipped | Auto-accept |

Notes:
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
| `--npm` | boolean | `false` | Search npm registry instead of taps. Uses the positional `query` arg as the search term. Cannot be combined with `-i`. Blocked when `registry.allow_npm = false` in config. |

### Examples

```bash
# Search for skills matching "review"
skilltap find review

# List all skills from all taps
skilltap find

# Interactive fuzzy finder (type to filter, arrow keys, Enter to install)
skilltap find -i

# Search npm registry for skills matching "commit"
skilltap find commit --npm

# Search all npm packages with agent-skill keyword
skilltap find --npm

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

Add a tap. Supports git repos and HTTP registry endpoints.

```
skilltap tap add <name> <url>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Local name for this tap |
| `url` | Yes | Git URL of the tap repo, or HTTP registry base URL |

### Behavior

Auto-detects the tap type by probing the URL. If the URL returns a valid HTTP registry response, it's registered as an HTTP tap (no local clone). Otherwise, it clones the repo to `~/.config/skilltap/taps/{name}/`, validates `tap.json`, and records the tap in `config.toml`.

If the tap name already exists: exit 1 with `Tap 'name' already exists. Remove it first with 'skilltap tap remove name'.`

### Examples

```bash
# Git tap
skilltap tap add home https://gitea.example.com/nathan/my-skills-tap
skilltap tap add community https://github.com/someone/awesome-skills-tap

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

Writes to the standard location for each shell and prints activation instructions:

| Shell | Writes to |
|-------|-----------|
| bash | `~/.local/share/bash-completion/completions/skilltap` |
| zsh | `~/.zfunc/_skilltap` |
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
