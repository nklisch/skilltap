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
2. `github:` prefix — GitHub adapter
3. `npm:` prefix — npm registry adapter
4. `./`, `/`, `~/` — local adapter
5. Contains `/` with no protocol — treated as `github:source`
6. Contains `@` — split into name + ref, resolve from taps
7. Otherwise — search taps for matching skill name

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
| `--agent <name>` | string | from config | Agent CLI for semantic scan (e.g. `"claude-code"`). See [config reference](/reference/config-options#security) for all supported values. |
| `--skip-scan` | boolean | `false` | Skip security scanning. Blocked if `require_scan = true` in config. |

### Prompt Behavior

| Flags | Skill selection | Scope | Agents | Static warnings | Semantic offer | Install confirm |
|-------|----------------|-------|--------|-----------------|----------------|-----------------|
| (none) | Prompt if multiple | Prompt | Prompt | Prompt | Offered if warnings | Prompt |
| `--project` | Prompt if multiple | Project | Prompt | Prompt | Offered if warnings | Prompt |
| `--global` | Prompt if multiple | Global | Prompt | Prompt | Offered if warnings | Prompt |
| `--also <agent>` | Prompt if multiple | Prompt | Skipped | Prompt | Offered if warnings | Prompt |
| `--yes` | Auto-select all | **Still prompts** | Config default | **Still prompts** | Offered if warnings | Auto-accept if clean |
| `--yes --global` | Auto-select all | Global | Config default | **Still prompts** | Offered if warnings | Auto-accept if clean |
| `--semantic` | Prompt if multiple | Prompt | Prompt | Prompt | **Always runs** | Prompt |
| `--strict` | Prompt if multiple | Prompt | Prompt | **Abort (exit 1)** | -- | -- |
| `--strict --yes --global` | Auto-select all | Global | Config default | **Abort (exit 1)** | -- | -- |
| `--skip-scan --yes --global` | Auto-select all | Global | Config default | Skipped | Skipped | Auto-accept |

When the semantic scan is offered and accepted, skilltap prompts for which agent CLI to use if one hasn't been configured yet. The choice is saved to `config.toml` for future use.

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
Global:
  commit-helper    v1.2.0  home   ✓ provenance  Conventional commit messages
  code-review      v2.0.0  home   ◆ curated     Thorough code review

Project (/home/nathan/dev/termtube):
  termtube-dev     main    local  ○ unverified  Development workflow
```

Columns: name, ref, source (tap name or `local`/`url`), trust tier, description (truncated to terminal width).

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
| `--npm <query>` | string | — | Search the npm registry for packages tagged with `agent-skill`. Cannot be combined with `-i`. |

### Examples

```bash
# Search for skills matching "review"
skilltap find review

# List all skills from all taps
skilltap find

# Interactive fuzzy finder (type to filter, arrow keys, Enter to install)
skilltap find -i

# Search npm registry for skills
skilltap find --npm commit

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
  Trust:  ✓ Provenance verified
    npm:  github.com/nathan/commit-helper
    Built by: .github/workflows/release.yml
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
| `--template` | `basic` \| `npm` \| `multi` | prompted | Template to use |
| `--dir` | string | `./<name>` | Directory to create the skill in |
| `--description` | string | prompted | Short description for the skill |
| `--author` | string | from git config | Author name |

When both `name` and `--template` are provided, runs non-interactively. Otherwise prompts for missing values.

### Templates

| Template | Generated files |
|----------|-----------------|
| `basic` | `SKILL.md`, `README.md`, `.gitignore` |
| `npm` | `SKILL.md`, `README.md`, `.gitignore`, `package.json`, `.github/workflows/publish.yml` |
| `multi` | `.agents/skills/<skill-a>/SKILL.md`, `.agents/skills/<skill-b>/SKILL.md`, `README.md`, `.gitignore` |

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
  "valid": true,
  "issues": [],
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
