---
description: Complete reference for ~/.config/skilltap/config.toml. All options, defaults, and policy composition rules for install, update, and security.
---

# Configuration Reference

Complete reference for `~/.config/skilltap/config.toml` -- all options, defaults, and policy composition rules.

## File Location

```
~/.config/skilltap/config.toml
```

Created with defaults on first run. Edit manually, use `skilltap config` for the interactive wizard, or use `skilltap config get`/`skilltap config set` for scripted access.

State is tracked separately in `~/.config/skilltap/state.json` (machine-managed, do not edit). Pre-v2.1 used `installed.json` + `plugins.json`; those are now read-fallback only for unmigrated users.

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

Single flat security block. Configure via `skilltap config security` (interactive wizard or flags).

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scan` | `"static"` \| `"semantic"` \| `"off"` | `"static"` | Scan mode. `"static"` runs the built-in pattern detectors; `"semantic"` adds an LLM review pass; `"off"` disables scanning. |
| `on_warn` | `"prompt"` \| `"fail"` \| `"allow"` | `"prompt"` | What to do when warnings are found. `"prompt"` asks; `"fail"` aborts the install; `"allow"` logs and proceeds. |
| `require_scan` | boolean | `false` | When `true`, blocks `--skip-scan`. |
| `agent_cli` | string | `""` | Agent CLI for semantic scanning. Values: `"claude"`, `"gemini"`, `"codex"`, `"opencode"`, `"ollama"`, or an absolute path. Empty = prompt on first use. |
| `threshold` | integer 0-10 | `5` | Risk threshold for semantic scan. Chunks scoring at or above this value are flagged. |
| `max_size` | integer (bytes) | `51200` | Max total skill directory size before warning. Default is 50 KB. |
| `ollama_model` | string | `""` | Model name when using the Ollama adapter. Required when `agent_cli = "ollama"`. |
| `overrides` | array of tables | `[]` | Per-source trust overrides (see below). |

::: info v2.0 redesign
The pre-v2.0 `[security.human]` / `[security.agent]` per-mode split was removed. There is now one set of security settings; the same rules apply to interactive use, scripted use (`--yes`), and machine output (`--json`). Run `skilltap migrate` to translate any legacy per-mode config.
:::

### Trust Tier Overrides: `[[security.overrides]]`

Each entry overrides security for one source. Named tap overrides take priority over source-type overrides; first match wins.

| Option | Type | Description |
|--------|------|-------------|
| `match` | string | Tap name or source type (`tap`, `git`, `npm`, `local`) |
| `kind` | `"tap"` \| `"source"` | What `match` refers to |
| `preset` | `"none"` \| `"relaxed"` \| `"standard"` \| `"strict"` | Security preset to apply for this tier |

### Presets

| Preset | Scan | On Warn | Require Scan |
|--------|------|---------|--------------|
| `none` | off | allow | false |
| `relaxed` | static | allow | false |
| `standard` | static | prompt | false |
| `strict` | semantic | fail | true |

### Example

```toml
[security]
scan = "static"
on_warn = "prompt"
require_scan = false
agent_cli = "claude"
threshold = 5
max_size = 51200

[[security.overrides]]
match = "my-company-tap"
kind = "tap"
preset = "none"
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

## Non-interactive use (replaces v0.x agent mode)

The `--agent` flag, `SKILLTAP_AGENT=1` env var, `[agent-mode]` config block, and `skilltap config agent-mode` subcommand were all removed in the v2.0 redesign. There is no persistent "agent mode" config.

For scripts, CI, and AI-agent invocations, use per-command flags:

| Mechanism | Effect |
|-----------|--------|
| Pipe stdout (no TTY) | Auto-detects non-interactive use; output switches to plain text (no spinners, colors, or Unicode decorations). |
| `--yes` | Auto-confirms prompts. Combine with `[defaults] scope` to skip the scope prompt as well. |
| `--json` | Emits machine-readable JSON instead of human-readable text. |

If you previously relied on `--agent` for hard-failing on warnings, set `[security] on_warn = "fail"` and `require_scan = true` instead.

---

## `[updates]`

Controls how skilltap checks for and applies CLI updates, and how often it checks installed skills for updates in the background.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `auto_update` | `"off"` \| `"patch"` \| `"minor"` | `"off"` | Automatically install updates on startup. `"patch"` applies patch releases silently; `"minor"` applies patch and minor releases. Major releases are always notify-only regardless of this setting. Only applies to compiled binaries. |
| `interval_hours` | integer | `24` | How often (in hours) to check GitHub for a new skilltap release. The check is non-blocking — it fires in the background and updates a local cache for the next run. Set to `0` to check on every invocation. |
| `skill_check_interval_hours` | integer | `24` | How often (in hours) to check installed skills for updates in the background. When updates are available, a dim notice is printed to stderr. Use `skilltap update --check` to force an immediate check. |
| `show_diff` | `"full"` \| `"stat"` \| `"none"` | `"full"` | Verbosity of the diff displayed when an update changes a skill's content. `"full"` shows the full unified diff; `"stat"` shows only file-level summary stats; `"none"` suppresses the diff entirely. |

### Example

```toml
[updates]
# Automatically apply patch releases on startup
auto_update = "patch"
# Check for a new release every 12 hours
interval_hours = 12
# Check installed skills for updates every 6 hours
skill_check_interval_hours = 6
# Show only file-level stats when a skill update changes content
show_diff = "stat"
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

## `verbose`

Top-level boolean. Controls whether install step details (fetched, scan clean) are logged during `skilltap install` and `skilltap tap install`.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `verbose` | boolean | `true` | Show step-by-step install progress. Set to `false` to suppress. Can also be overridden per-invocation with `--quiet`. |

### Example

```toml
# Suppress step details during install (show only success/error lines)
verbose = false
```

---

## `builtin_tap`

Top-level boolean. Controls whether the built-in `skilltap-skills` tap (a curated catalog maintained by the project) is enabled.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `builtin_tap` | boolean | `true` | When `true`, the built-in tap is auto-cloned and searchable via `skilltap find`. Set to `false` to opt out — useful for fully air-gapped or corp-only setups where you only want your own taps to appear. |

### Example

```toml
# Disable the built-in skilltap-skills tap
builtin_tap = false
```

---

## `default_git_host`

Top-level string. The git host used to resolve `owner/repo` shorthand into a full URL.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `default_git_host` | string | `"https://github.com"` | Base URL for `owner/repo` shorthand. For example, with `default_git_host = "https://gitea.example.com"`, `skilltap install skill nathan/commit-helper` resolves to `https://gitea.example.com/nathan/commit-helper`. |

### Example

```toml
# Resolve owner/repo shorthand against an internal Gitea
default_git_host = "https://gitea.corp.example.com"
```

---

## `[[taps]]`

Tap definitions. Managed by `skilltap tap add` and `skilltap tap remove`. Each entry is a TOML array table.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | — | Local name for the tap |
| `url` | string | — | Git URL of the tap repo |
| `type` | `"git"` | `"git"` | Tap type. v2.0 supports git only — pre-v2.0 also accepted `"http"` for live API registries; those entries are now silently filtered with a warning, and `skilltap migrate` flags them for manual conversion. |

::: warning v2.0 — HTTP tap removal
`type = "http"`, `auth_token`, and `auth_env` were removed in v2.0. Self-host a private git repo (Gitea, GitLab, bare HTTP repo) for non-public distribution; git authentication via SSH keys or credential helpers covers the auth use case.
:::

### Example

```toml
[[taps]]
name = "home"
url = "https://gitea.example.com/nathan/my-skills-tap"

[[taps]]
name = "community"
url = "https://github.com/someone/awesome-skills-tap"
```

Git taps are cloned to `~/.config/skilltap/taps/{name}/`.

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

# Security settings — single flat block with optional trust overrides.
# Configure via: skilltap config security
[security]
scan = "static"
on_warn = "prompt"
require_scan = false
agent_cli = ""
threshold = 5
max_size = 51200
ollama_model = ""

# Trust overrides — per-tap or per-source-type presets
# [[security.overrides]]
# match = "my-company-tap"
# kind = "tap"
# preset = "none"

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

# Show step details during install (fetched, scan clean). Set false to silence.
verbose = true

# Tap definitions
[[taps]]
name = "home"
url = "https://gitea.example.com/nathan/my-skills-tap"
```

---

## Policy Composition Rules

Config options and CLI flags compose into an effective policy per invocation. Trust tier overrides replace the flat-block defaults when a matching tap or source type is configured.

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

### Trust Tier Override Resolution

Override priority: named tap match > source type match > flat-block default. CLI flags still override on top of trust tier settings.

### Worked Example

```toml
[defaults]
also = ["claude-code", "cursor"]
yes = true
scope = "global"

[security]
scan = "semantic"
on_warn = "fail"
require_scan = true
agent_cli = "claude"
threshold = 3
max_size = 102400
```

With this config:

```bash
skilltap install skill <url>
# -> auto-select all skills (yes = true)
# -> scope = global (no prompt)
# -> symlinks to claude-code + cursor
# -> Layer 1 + Layer 2 scan (security.scan = semantic)
# -> abort on any warning (security.on_warn = fail)
# -> --skip-scan blocked (security.require_scan = true)
# -> claude used for semantic scan (security.agent_cli)
# -> flag chunks scoring >= 3 (security.threshold)

skilltap install skill <url> --no-strict
# -> same as above but warnings prompt instead of abort

skilltap install skill <url> --skip-scan
# -> ERROR: Security scanning is required by config

skilltap install skill <url> --project
# -> --project overrides scope for this invocation
```
