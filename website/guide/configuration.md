---
description: Configure skilltap via TOML file or interactive wizard. Set default scope, agent symlinks, scan policy, and agent mode for non-interactive CI use.
---

# Configuration

## Config file location

skilltap stores its configuration at:

```
~/.config/skilltap/config.toml
```

You can edit this file directly, use the interactive wizard, or use `config get`/`config set` for scripted access.

## Interactive wizard

Run the configuration wizard:

```bash
skilltap config
```

This walks you through each setting with prompts and sensible defaults. Existing values are preserved -- the wizard only updates what you change.

## Security setup

Security settings are per-mode (human vs agent) with optional trust tier overrides. Use the dedicated security wizard:

```bash
skilltap config security                                # interactive wizard
skilltap config security --preset strict --mode agent   # non-interactive
skilltap config security --trust tap:my-corp=none       # trust override
```

See the [Security guide](/guide/security#configuring-security-behavior) for full details.

## Agent mode setup

Agent mode configures skilltap for use by AI agents (non-interactive, machine-readable output). Set it up with:

```bash
skilltap config agent-mode
```

This command requires a TTY -- it must be run by a human, not by an agent. It prompts for agent-mode-specific settings like default scope and security preset.

## Programmatic access

For scripts and agents, use `config get` and `config set` instead of the interactive wizard:

```bash
# Read any config value
skilltap config get defaults.scope          # → global
skilltap config get defaults.also           # → claude-code cursor
skilltap config get --json                  # full config as JSON

# Set preference values (silent on success)
skilltap config set defaults.scope project
skilltap config set defaults.also claude-code cursor
skilltap config set defaults.yes true
```

Only preference keys are settable via `config set`. Security policy keys, agent mode, and telemetry are blocked -- use the interactive wizard or dedicated subcommands for those. See the [CLI reference](/reference/cli#skilltap-config-set) for the full list of settable keys.

## Config reference

### `[defaults]`

General defaults for install and update commands.

| Key     | Type     | Default | Description                                                     |
| ------- | -------- | ------- | --------------------------------------------------------------- |
| `scope` | string   | `""`    | Default install scope: `"global"` or `"project"`                |
| `also`  | string[] | `[]`    | Additional agent symlinks to create (e.g. `["cursor", "codex"]`) |
| `yes`   | boolean  | `false` | Skip confirmation prompts                                       |

### `[security]`

Shared security settings.

| Key             | Type    | Default    | Description                                          |
| --------------- | ------- | ---------- | ---------------------------------------------------- |
| `agent_cli`     | string  | `""`       | Agent CLI for semantic scan (e.g. `"claude"`)        |
| `threshold`     | number  | `5`        | Semantic score threshold (0-10, chunks >= this flagged) |
| `max_size`      | number  | `51200`    | Max total skill size in bytes before warning (default 50 KB) |
| `ollama_model`  | string  | `""`       | Ollama model name for semantic scanning              |

### `[security.human]` / `[security.agent]`

Per-mode security settings. Human mode applies when you run skilltap directly. Agent mode applies when agent mode is enabled.

| Key             | Type    | Human Default | Agent Default | Description                                          |
| --------------- | ------- | ------------- | ------------- | ---------------------------------------------------- |
| `scan`          | string  | `"static"`    | `"static"`    | Scan mode: `"off"`, `"static"`, or `"semantic"`      |
| `on_warn`       | string  | `"prompt"`    | `"fail"`      | Warning behavior: `"prompt"`, `"fail"`, or `"allow"` |
| `require_scan`  | boolean | `false`       | `true`        | Block `--skip-scan` flag                             |

### `[[security.overrides]]`

Trust tier overrides — per-tap or per-source-type security presets. See the [Security guide](/guide/security#trust-tier-overrides) for usage.

| Key      | Type   | Description                                               |
| -------- | ------ | --------------------------------------------------------- |
| `match`  | string | Tap name or source type to match                          |
| `kind`   | string | `"tap"` or `"source"`                                     |
| `preset` | string | Security preset: `"none"`, `"relaxed"`, `"standard"`, `"strict"` |

### `[agent-mode]`

Settings applied when an AI agent runs skilltap.

| Key       | Type    | Default     | Description                                           |
| --------- | ------- | ----------- | ----------------------------------------------------- |
| `enabled` | boolean | `false`     | Enable agent mode                                     |
| `scope`   | string  | `"project"` | Default scope for agent installs: `"global"` or `"project"` |

Agent mode uses `[security.agent]` for its security settings. These are independently configurable via `skilltap config security --mode agent`.

### `[registry]`

Controls which skill registries are searched when running `skilltap find`.

| Key       | Type     | Default          | Description                                                          |
| --------- | -------- | ---------------- | -------------------------------------------------------------------- |
| `enabled` | string[] | `["skills.sh"]`  | Registries to search, in order. Set to `[]` to disable all.          |
| `sources` | array    | `[]`             | Custom registry definitions (see below).                             |

skilltap includes one built-in registry: [skills.sh](https://skills.sh). You can add custom registries that implement the same search API — see [Using skilltap with a Team](/guide/teams#custom-skill-registry) for details.

## How config and flags compose

When a CLI flag and a config value conflict, the **most restrictive** option wins:

- `--strict` overrides `on_warn` to `"fail"`
- `--no-strict` overrides `on_warn` to `"prompt"`
- `require_scan = true` in the active mode blocks `--skip-scan` (returns an error)
- Agent mode uses `[security.agent]` settings (fully configurable, defaults to strict)
- Trust tier overrides replace mode defaults for matching taps or source types

Flags always override config for non-security settings like `scope` and `yes`.

## Example config

```toml
[defaults]
scope = "project"
also = ["cursor"]
yes = false

[security]
agent_cli = "claude"
threshold = 5

[security.human]
scan = "static"
on_warn = "prompt"
require_scan = false

[security.agent]
scan = "static"
on_warn = "fail"
require_scan = true

[registry]
enabled = ["skills.sh"]

["agent-mode"]
enabled = false
scope = "project"
```

For the full list of options and their allowed values, see the [Config Options Reference](/reference/config-options).
