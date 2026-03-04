---
description: Complete reference for ~/.config/skilltap/config.toml. All options, defaults, and policy composition rules for install, update, security, and agent mode.
---

# Configuration Reference

Complete reference for `~/.config/skilltap/config.toml` -- all options, defaults, and policy composition rules.

## File Location

```
~/.config/skilltap/config.toml
```

Created with defaults on first run. Edit manually, use `skilltap config` for the interactive wizard, or use `skilltap config get`/`skilltap config set` for scripted access.

State is tracked separately in `~/.config/skilltap/installed.json` (machine-managed, do not edit).

---

## `[defaults]`

Default settings for install and link commands.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scope` | `"global"` \| `"project"` \| `""` | `""` | Default install scope. Empty string means prompt every time. |
| `also` | array of strings | `[]` | Agent-specific directories to auto-symlink on every install. Values: `"claude-code"`, `"cursor"`, `"codex"`, `"gemini"`, `"windsurf"` |
| `yes` | boolean | `false` | Auto-accept prompts. Auto-selects all skills and auto-accepts clean installs. Security warnings still require confirmation. Scope still prompts unless `scope` is also set. |

### Example

```toml
[defaults]
scope = "global"
also = ["claude-code", "cursor"]
yes = false
```

### Interaction Between `scope` and `yes`

| `yes` | `scope` | Behavior |
|-------|---------|----------|
| `false` | `""` | Prompt: skill selection, scope, install confirm |
| `false` | `"global"` | Prompt: skill selection, install confirm (scope set) |
| `true` | `""` | Auto-select skills, **still prompt: scope**, auto-install if clean |
| `true` | `"global"` | Auto-select, scope=global, auto-install if clean |
| `true` | `"project"` | Auto-select, scope=project, auto-install if clean |

CLI flags always override config: `--project` overrides `scope`, `--yes` overrides `yes`.

---

## `[security]`

Security scanning settings for install and update commands.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scan` | `"static"` \| `"semantic"` \| `"off"` | `"static"` | Scan mode. `"static"` = Layer 1 pattern matching. `"semantic"` = Layer 1 + Layer 2 AI-powered analysis. `"off"` = no scanning. |
| `on_warn` | `"prompt"` \| `"fail"` | `"prompt"` | What to do when security warnings are found. `"prompt"` = show warnings and ask. `"fail"` = abort immediately (same as `--strict`). |
| `require_scan` | boolean | `false` | When `true`, blocks `--skip-scan` from being used. Useful for org or machine-level security policy. |
| `agent` | string | `""` | Agent CLI for semantic scanning. Values: `"claude"`, `"gemini"`, `"codex"`, `"opencode"`, `"ollama"`, or an absolute path to a custom binary. Empty string = prompt on first use, then save selection. |
| `threshold` | integer 0-10 | `5` | Risk threshold for semantic scan. Chunks scoring at or above this value are flagged. |
| `max_size` | integer (bytes) | `51200` | Max total skill directory size before warning. Default is 50 KB. |
| `ollama_model` | string | `""` | Model name when using the Ollama adapter. Required when `agent = "ollama"`. |

### Example

```toml
[security]
scan = "semantic"
on_warn = "fail"
require_scan = true
agent = "claude"
threshold = 5
max_size = 51200
ollama_model = ""
```

### Supported Agents

| Value | Binary | Invocation |
|-------|--------|------------|
| `"claude"` | `claude` | `claude --print -p '<prompt>' --tools "" --output-format json` |
| `"gemini"` | `gemini` | `echo '<prompt>' \| gemini --non-interactive` |
| `"codex"` | `codex` | `codex --prompt '<prompt>' --no-tools` |
| `"opencode"` | `opencode` | `opencode --prompt '<prompt>'` |
| `"ollama"` | `ollama` | `ollama run <model> '<prompt>'` (requires `ollama_model`) |
| Absolute path | any | `echo '<prompt>' \| /path/to/binary` |

All agents are invoked without tool access -- they can only produce text output, never execute commands or read files during the scan.

---

## `[agent-mode]`

Agent mode settings. When enabled, all skilltap commands become non-interactive with strict security. Toggle with `skilltap config agent-mode` (interactive, human-only).

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable agent mode. |
| `scope` | `"global"` \| `"project"` | `"project"` | Default scope for agent installs. Required when agent mode is enabled. |
| `also` | array of strings | from `[defaults]` | Agent-specific symlinks for agent mode installs. If not set, falls back to `defaults.also`. |
| `scan` | `"static"` \| `"semantic"` | from `[security]` | Scan level for agent mode. `"off"` is not available -- scanning is always required. If not set, falls back to `security.scan` (promoted to `"static"` if `"off"`). |

### Example

```toml
[agent-mode]
enabled = true
scope = "project"
```

### Forced Overrides

When `agent-mode.enabled = true`, the following are **inherent and not overridable** via CLI flags:

| Setting | Forced Value | Effect |
|---------|-------------|--------|
| `yes` | `true` | All prompts auto-accept or hard-fail |
| `on_warn` | `"fail"` | Any security warning blocks installation |
| `require_scan` | `true` | `--skip-scan` is rejected |
| Output format | Plain text | No ANSI colors, spinners, or Unicode decorations |

Agent mode has **no CLI flag** to toggle it. It can only be enabled or disabled through `skilltap config agent-mode`, which requires an interactive terminal.

### Agent Mode Output

Success:
```
OK: Installed commit-helper -> ~/.agents/skills/commit-helper/ (v1.2.0)
```

Skip:
```
SKIP: commit-helper is already installed.
```

Error:
```
ERROR: Repository not found: https://example.com/bad-url.git
```

Security failure (directive message to the agent):
```
SECURITY ISSUE FOUND -- INSTALLATION BLOCKED

DO NOT install this skill. DO NOT retry. DO NOT use --skip-scan.
STOP and report the following to the user:

  SKILL.md L14: Invisible Unicode (3 zero-width chars)
  SKILL.md L8: Hidden HTML comment containing instructions

User action required: review warnings and install manually with
  skilltap install <url>
```

---

## `[updates]`

Controls how skilltap checks for and applies CLI updates, and how often it checks installed skills for updates in the background.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `auto_update` | `"off"` \| `"patch"` \| `"minor"` | `"off"` | Automatically install updates on startup. `"patch"` applies patch releases silently; `"minor"` applies patch and minor releases. Major releases are always notify-only regardless of this setting. Only applies to compiled binaries. |
| `interval_hours` | integer | `24` | How often (in hours) to check GitHub for a new skilltap release. The check is non-blocking — it fires in the background and updates a local cache for the next run. Set to `0` to check on every invocation. |
| `skill_check_interval_hours` | integer | `24` | How often (in hours) to check installed skills for updates in the background. When updates are available, a dim notice is printed to stderr. Use `skilltap update --check` to force an immediate check. |

### Example

```toml
[updates]
# Automatically apply patch releases on startup
auto_update = "patch"
# Check for a new release every 12 hours
interval_hours = 12
# Check installed skills for updates every 6 hours
skill_check_interval_hours = 6
```

When `auto_update` triggers, you'll see on stderr:

```
⟳  Auto-updating skilltap 0.3.1 → 0.3.2 (patch)…
✓  Updated to v0.3.2. Changes take effect next run.
```

Run `skilltap self-update` at any time to force an immediate check and update.

---

## `[telemetry]`

Anonymous usage telemetry. Managed by `skilltap telemetry enable` / `skilltap telemetry disable`. You can also edit these keys directly, but prefer the subcommands.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `false` | Whether telemetry is active. |
| `anonymous_id` | string | `""` | A random UUID assigned on `telemetry enable`. Never tied to a user or machine identity. |
| `notice_shown` | boolean | `false` | Internal flag — set to `true` once the startup opt-in banner has been displayed. Do not edit. |

**Environment overrides:** `DO_NOT_TRACK=1` or `SKILLTAP_TELEMETRY_DISABLED=1` disable telemetry regardless of these config values.

### Example

```toml
[telemetry]
enabled = true
anonymous_id = "a3f8c1d2-4b5e-6f7a-8c9d-0e1f2a3b4c5d"
```

---

## `[registry]`

Controls which skill registries are searched when running `skilltap find <query>`.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | array of strings | `["skills.sh"]` | Which registries to search, in order. Built-in: `"skills.sh"`. Set to `[]` to disable all registry search. |
| `sources` | array of tables | `[]` | Custom registry definitions. Each entry needs `name` and `url`. |
| `allow_npm` | boolean | `true` | *(Deprecated)* Use `enabled = []` to disable registries instead. |

### Custom Registries

Any URL implementing the skills.sh search API can be added as a custom registry:

```
GET {url}/api/search?q={query}&limit={n}
→ { "skills": [{ "id": string, "name": string, "description": string, "source": string, "installs": number }] }
```

The `source` field in each result must be a valid skilltap install ref (e.g. `owner/repo` for GitHub sources, a full git URL, or `npm:package`).

### Example

```toml
[registry]
# Search skills.sh and a company registry
enabled = ["skills.sh", "acme"]

# Custom registry definition
[[registry.sources]]
name = "acme"
url = "https://skills.acme.com"
```

To disable all registry search (taps only):

```toml
[registry]
enabled = []
```

---

## `[[taps]]`

Tap definitions. Managed by `skilltap tap add` and `skilltap tap remove`. Each entry is a TOML array table.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | — | Local name for the tap |
| `url` | string | — | URL of the tap (git repo or HTTP registry endpoint) |
| `type` | `"git"` \| `"http"` | auto-detected | Tap type. `"git"` clones the repo locally; `"http"` queries a live API. Auto-detected on `tap add`. |
| `auth_token` | string | — | Static bearer token for HTTP tap authentication. Prefer `auth_env` over this. |
| `auth_env` | string | — | Name of an environment variable containing the bearer token for HTTP tap authentication. |

### Example

```toml
[[taps]]
name = "home"
url = "https://gitea.example.com/nathan/my-skills-tap"

[[taps]]
name = "community"
url = "https://github.com/someone/awesome-skills-tap"

[[taps]]
name = "enterprise"
url = "https://skills.example.com/api/v1"
type = "http"
auth_env = "SKILLS_REGISTRY_TOKEN"
```

Git taps are cloned to `~/.config/skilltap/taps/{name}/`. HTTP taps have no local clone — they are queried live.

---

## Full Example

A complete `config.toml` with all options:

```toml
# Default settings for install commands
[defaults]
# Default install scope. "" = prompt, "global" or "project" = skip prompt.
scope = "global"
# Auto-symlink to these agent directories on every install.
also = ["claude-code", "cursor"]
# Auto-accept clean installs (security warnings still prompt).
yes = false

# Security scanning settings
[security]
# Scan mode: "static" (fast), "semantic" (thorough), "off" (not recommended)
scan = "static"
# On warnings: "prompt" (ask user) or "fail" (abort, same as --strict)
on_warn = "prompt"
# Block --skip-scan entirely. For org-level security policy.
require_scan = false
# Agent CLI for semantic scan. "" = prompt on first use.
agent = ""
# Flag semantic chunks scoring >= this value (0-10)
threshold = 5
# Warn if skill directory exceeds this size in bytes (default 50KB)
max_size = 51200
# Required when agent = "ollama"
ollama_model = ""

# Agent mode -- for when skilltap is invoked by AI agents.
# Toggle with: skilltap config agent-mode
[agent-mode]
enabled = false
scope = "project"

# CLI update settings
[updates]
auto_update = "patch"
interval_hours = 24

# Skill registry search settings
[registry]
# Which registries to query with 'skilltap find'. Set to [] to disable.
enabled = ["skills.sh"]

# Telemetry (managed via `skilltap telemetry enable/disable`)
[telemetry]
enabled = false

# Tap definitions
[[taps]]
name = "home"
url = "https://gitea.example.com/nathan/my-skills-tap"
```

---

## Policy Composition Rules

Config options and CLI flags compose together. The general rule: **most restrictive wins**.

### Flag Overrides

| Config | CLI Flag | Result |
|--------|----------|--------|
| `on_warn = "prompt"` | `--strict` | strict (flag wins) |
| `on_warn = "fail"` | (none) | strict (config wins) |
| `on_warn = "fail"` | `--no-strict` | prompt (flag overrides) |
| `require_scan = true` | `--skip-scan` | **ERROR** (config blocks) |
| `scan = "semantic"` | (none) | Layer 1 + Layer 2 |
| `scan = "static"` | `--semantic` | Layer 1 + Layer 2 (flag adds) |
| `scan = "off"` | `--semantic` | Layer 2 only |
| `yes = false` | `--yes` | yes (flag wins) |
| `scope = "global"` | `--project` | project (flag overrides) |

### Agent Mode Overrides

When `agent-mode.enabled = true`, the following are forced regardless of other config or flags:

- `yes` = `true`
- `on_warn` = `"fail"`
- `require_scan` = `true`
- `scan = "off"` is promoted to `"static"`
- `--skip-scan` is rejected with an error

These cannot be overridden by CLI flags. Agent mode can only be toggled interactively via `skilltap config agent-mode`.

### Worked Example: Power User

```toml
[defaults]
also = ["claude-code", "cursor"]
yes = true
scope = "global"

[security]
scan = "semantic"
on_warn = "fail"
require_scan = true
agent = "claude"
threshold = 3
max_size = 102400
```

With this config:

```bash
skilltap install <url>
# -> auto-select all skills (yes = true)
# -> scope = global (no prompt)
# -> symlinks to claude-code + cursor
# -> Layer 1 + Layer 2 scan
# -> abort on any warning (on_warn = fail)
# -> --skip-scan blocked (require_scan = true)
# -> claude used for semantic scan
# -> flag chunks scoring >= 3

skilltap install <url> --no-strict
# -> same as above but warnings prompt instead of abort

skilltap install <url> --skip-scan
# -> ERROR: Security scanning is required by config

skilltap install <url> --project
# -> --project overrides scope for this invocation
```

### Worked Example: Agent Mode

```toml
[defaults]
also = ["claude-code"]

[security]
scan = "static"
agent = "claude"

[agent-mode]
enabled = true
scope = "project"
```

With this config:

```bash
skilltap install <url>
# -> auto-select all (forced)
# -> scope = project (from agent-mode.scope)
# -> symlinks to claude-code (from defaults.also)
# -> Layer 1 scan
# -> any warning = SECURITY ISSUE FOUND directive + exit 1
# -> --skip-scan blocked (forced)
# -> plain text output, no colors
```
