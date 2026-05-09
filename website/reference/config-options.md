---
description: Complete reference for ~/.config/skilltap/config.toml. All options, defaults, and policy composition rules for install, update, and security.
---

# Configuration Reference

Complete reference for `~/.config/skilltap/config.toml` — every settable key, its allowed values, and the policy composition rules that combine config with CLI flags.

## File Location

```
~/.config/skilltap/config.toml
```

Created with defaults on first run. Edit manually, use `skilltap config` for the interactive wizard, or use `skilltap config get`/`skilltap config set` for scripted access.

State is tracked separately in `~/.config/skilltap/state.json` (machine-managed, do not edit). The project-scoped equivalent is `<project>/.agents/state.json`.

---

## `[defaults]`

Default settings for install and update commands.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scope` | `"global"` \| `"project"` \| `""` | `""` | Default install scope. Empty string means smart-scope (project inside a git repo, global otherwise). |
| `also` | array of strings | `[]` | Agent-specific directories to auto-symlink on every install. Values: `"claude-code"`, `"cursor"`, `"codex"`, `"gemini"`, `"windsurf"` |
| `yes` | boolean | `false` | Auto-accept prompts. Auto-selects all skills and auto-accepts clean installs. Security warnings still prompt. |

### Example

```toml
[defaults]
scope = "global"
also = ["claude-code", "cursor"]
yes = false
```

---

## `[security]`

The flat policy block. Three keys: `scan`, `on_warn`, `trust`. Operational settings (which agent CLI to invoke, size limits, scoring threshold) live in the sibling `[scanner]` block.

Configure interactively via `skilltap config security`, or scripted via `skilltap config set security.<key> <value>`.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scan` | `"semantic"` \| `"static"` \| `"none"` | `"static"` | Scan mode. `"static"` runs the built-in pattern detectors; `"semantic"` adds an LLM review pass; `"none"` disables scanning. |
| `on_warn` | `"prompt"` \| `"fail"` \| `"install"` | `"install"` | What to do when warnings are found. `"prompt"` asks; `"fail"` aborts the install; `"install"` logs and proceeds. |
| `trust` | array of strings | `[]` | Glob patterns matched against the resolved source URL. Sources matching any pattern bypass scanning entirely. |

### `trust` glob examples

```toml
[security]
trust = [
  # Anything in your team's GitHub org
  "github.com/my-corp/*",
  # Self-hosted Gitea instance
  "https://gitea.acme.com/eng/*",
  # Specific npm scope
  "npm:@my-corp/*",
]
```

A trust match short-circuits both static and semantic scans for that install. Use sparingly — it disables an integrity check entirely for matching sources.

### Example

```toml
[security]
scan = "static"
on_warn = "prompt"
trust = []
```

---

## `[scanner]`

Operational settings for the scanner. Tells skilltap which agent CLI to invoke for the semantic scan, which Ollama model to use (when applicable), the warning threshold, and the size limit at which the static scan flags an oversized skill.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `agent_cli` | string | `""` | Agent CLI for semantic scan. Values: `"claude"`, `"gemini"`, `"codex"`, `"opencode"`, `"ollama"`, or an absolute path. Empty = prompt on first use. |
| `ollama_model` | string | `""` | Model name when `agent_cli = "ollama"` |
| `threshold` | integer 0–10 | `5` | Risk threshold for semantic scan. Chunks scoring `>= threshold` are flagged. |
| `max_size` | integer (bytes) | `51200` | Max total skill directory size before warning. Default is 50 KB. |

### Supported agents

| `agent_cli` value | Binary | Invocation |
|-------------------|--------|------------|
| `"claude"` | `claude` | `claude --print -p '<prompt>' --tools "" --output-format json` |
| `"gemini"` | `gemini` | `echo '<prompt>' \| gemini --non-interactive` |
| `"codex"` | `codex` | `codex --prompt '<prompt>' --no-tools` |
| `"opencode"` | `opencode` | `opencode --prompt '<prompt>'` |
| `"ollama"` | `ollama` | `ollama run <model> '<prompt>'` (requires `ollama_model`) |
| Absolute path | any | `echo '<prompt>' \| /path/to/binary` |

All agents are invoked without tool access — they can only produce text output, never execute commands or read files during the scan.

### Example

```toml
[scanner]
agent_cli = "claude"
ollama_model = ""
threshold = 5
max_size = 51200
```

---

## Non-interactive use

skilltap detects non-interactive contexts automatically — there is no separate "agent mode" config, env var, or flag.

| Mechanism | Effect |
|-----------|--------|
| Pipe stdout (no TTY) | Auto-detects non-interactive use; output switches to plain text (no spinners, colors, or Unicode decorations). |
| `--yes` | Auto-confirms prompts. Combine with `[defaults] scope` to skip the scope prompt as well. |
| `--json` | Emits machine-readable JSON instead of human-readable text. |
| `[security] on_warn = "fail"` | Hard-fail on any security warning instead of prompting — the right setting for CI. |

For automation pipelines, the recommended baseline is `[defaults] yes = true`, `[defaults] scope = "global"` (or `"project"`), and `[security] on_warn = "fail"`.

---

## `[updates]`

Controls how skilltap checks for and applies CLI updates, and how often it checks installed skills for updates in the background.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `auto_update` | `"off"` \| `"patch"` \| `"minor"` | `"off"` | Automatically install updates on startup. `"patch"` applies patch releases silently; `"minor"` applies patch and minor releases. Major releases are always notify-only. Only applies to compiled binaries. |
| `interval_hours` | integer | `24` | How often (in hours) to check GitHub for a new skilltap release. The check is non-blocking — it fires in the background and updates a local cache for the next run. Set to `0` to check on every invocation. |
| `skill_check_interval_hours` | integer | `24` | How often (in hours) to check installed skills for updates in the background. When updates are available, a dim notice is printed to stderr. Use `skilltap update --check` to force an immediate check. |
| `show_diff` | `"full"` \| `"stat"` \| `"none"` | `"full"` | Verbosity of the diff displayed when an update changes a skill's content. |

### Example

```toml
[updates]
auto_update = "patch"
interval_hours = 12
skill_check_interval_hours = 6
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

Anonymous usage telemetry. Managed by `skilltap config telemetry enable` / `skilltap config telemetry disable`. Internal fields (`anonymous_id`, `notice_shown`) are blocked from `config set` — use the subcommands.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `false` | Whether telemetry is active. |
| `anonymous_id` | string | `""` | A random UUID assigned on enable. Never tied to a user or machine identity. *(Internal — blocked from `config set`.)* |
| `notice_shown` | boolean | `false` | Internal flag; set to `true` once the startup opt-in banner has been displayed. *(Internal — blocked from `config set`.)* |

**Environment overrides:** `DO_NOT_TRACK=1` or `SKILLTAP_TELEMETRY_DISABLED=1` disable telemetry regardless of these config values.

### Example

```toml
[telemetry]
enabled = true
```

---

## `[registry]`

Controls which skill registries are searched when running `skilltap find <query>`.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | array of strings | `["skills.sh"]` | Which registries to search, in order. Built-in: `"skills.sh"`. Set to `[]` to disable all registry search. |
| `sources` | array of tables | `[]` | Custom registry definitions. Each entry needs `name` and `url`. |

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
enabled = ["skills.sh", "acme"]

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

Top-level boolean. Controls whether install step details (fetched, scan clean) are logged during `skilltap install`.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `verbose` | boolean | `true` | Show step-by-step install progress. Set to `false` to suppress. Override per invocation with `--quiet`. |

---

## `builtin_tap`

Top-level boolean. Controls whether the built-in `skilltap-skills` tap (a curated catalog maintained by the project) is enabled.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `builtin_tap` | boolean | `true` | When `true`, the built-in tap is auto-cloned and searchable via `skilltap find`. Set to `false` to opt out — useful for fully air-gapped or corp-only setups where you only want your own taps to appear. |

---

## `default_git_host`

Top-level string. The git host used to resolve `owner/repo` shorthand into a full URL.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `default_git_host` | string | `"https://github.com"` | Base URL for `owner/repo` shorthand. With `default_git_host = "https://gitea.example.com"`, `skilltap install skill nathan/commit-helper` resolves to `https://gitea.example.com/nathan/commit-helper`. |

---

## `[[taps]]`

Tap definitions. Managed by `skilltap tap add` and `skilltap tap remove` — the `taps` key is blocked from `config set`. Each entry is a TOML array table.

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Local name for the tap |
| `url` | string | Git URL of the tap repo |

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

## Project manifest schema (`skilltap.toml`)

When a project pins its skill / plugin / MCP dependencies, the manifest lives at `<project>/skilltap.toml` and the lockfile at `<project>/skilltap.lock`. Three tables are recognised:

```toml
[targets]
also = ["claude-code"]   # default --also for installs from this manifest
scope = ""               # "" | "global" | "project"

[skills]
"github:acme/commit-helper" = "^1.0"
"github:acme/code-review" = { ref = "main" }   # inline-table form

[plugins]
"github:acme/dev-toolkit" = "^2.1"
"github:acme/dev-toolkit" = { ref = "v2.1.0", components = { "test-runner" = false } }

# [[mcps]] — standalone MCP entries (one [[mcps]] block per MCP)
[[mcps]]
name = "db-tools"
source = "github:acme/db-tools"
ref = "main"             # exact pin (no separate range field)
also = ["claude-code"]
```

`install` and `remove` automatically update both `skilltap.toml` and `skilltap.lock`. `skilltap sync` reconciles manifest ↔ lockfile ↔ `state.json` for all three tables.

---

## Full Example

```toml
[defaults]
scope = "global"
also = ["claude-code", "cursor"]
yes = false

[security]
scan = "static"
on_warn = "prompt"
trust = [
  "github.com/my-corp/*",
]

[scanner]
agent_cli = "claude"
threshold = 5
max_size = 51200
ollama_model = ""

[updates]
auto_update = "patch"
interval_hours = 24

[registry]
enabled = ["skills.sh"]

[telemetry]
enabled = false

verbose = true
builtin_tap = true
default_git_host = "https://github.com"

[[taps]]
name = "home"
url = "https://gitea.example.com/nathan/my-skills-tap"
```

---

## Settable keys

`skilltap config set` only accepts the following keys. All others (internal telemetry fields, tap entries) error with a hint.

| Key | Type | Notes |
|-----|------|-------|
| `defaults.scope` | enum | `""`, `"global"`, `"project"` |
| `defaults.also` | string[] | Repeat values to set multiple |
| `defaults.yes` | boolean | |
| `security.scan` | enum | `"semantic"`, `"static"`, `"none"` |
| `security.on_warn` | enum | `"prompt"`, `"fail"`, `"install"` |
| `security.trust` | string[] | Glob list |
| `scanner.agent_cli` | string | |
| `scanner.ollama_model` | string | |
| `scanner.threshold` | number | 0–10 |
| `scanner.max_size` | number | bytes |
| `registry.enabled` | string[] | |
| `telemetry.enabled` | boolean | |
| `updates.auto_update` | enum | `"off"`, `"patch"`, `"minor"` |
| `updates.interval_hours` | number | |
| `updates.show_diff` | enum | `"full"`, `"stat"`, `"none"` |
| `builtin_tap` | boolean | |
| `verbose` | boolean | |
| `default_git_host` | string | |

---

## Policy Composition Rules

Config options and CLI flags compose into an effective policy per invocation. CLI flags override config; the most restrictive option wins for security.

### Flag overrides

| Config | CLI Flag | Result |
|--------|----------|--------|
| `on_warn = "prompt"` | `--strict` | strict (flag wins) |
| `on_warn = "fail"` | (none) | strict (config wins) |
| `scan = "semantic"` | (none) | Layer 1 + Layer 2 |
| `scan = "static"` | `--semantic` | Layer 1 + Layer 2 (flag adds) |
| `scan = "none"` | `--semantic` | Layer 2 only |
| `yes = false` | `--yes` | yes (flag wins) |
| `scope = "global"` | `--scope project` | project (flag overrides) |

### Trust glob short-circuit

A `[security] trust` glob match against the resolved source URL bypasses the static and semantic scans entirely for that install. The trust check happens before scan policy is applied — a matching source never runs the scanner regardless of `scan` / `on_warn` / `--strict`.

### Worked example

```toml
[defaults]
also = ["claude-code", "cursor"]
yes = true
scope = "global"

[security]
scan = "semantic"
on_warn = "fail"
trust = []

[scanner]
agent_cli = "claude"
threshold = 3
max_size = 102400
```

With this config:

```bash
skilltap install skill <url>
# → auto-select all skills (yes = true)
# → scope = global (no prompt)
# → symlinks to claude-code + cursor
# → Layer 1 + Layer 2 scan (security.scan = semantic)
# → abort on any warning (security.on_warn = fail)
# → claude used for semantic scan (scanner.agent_cli)
# → flag chunks scoring >= 3 (scanner.threshold)

skilltap install skill <url> --skip-scan
# → no scan runs (--skip-scan wins for this invocation)

skilltap install skill <url> --scope project
# → --scope flag overrides defaults.scope for this invocation
```
