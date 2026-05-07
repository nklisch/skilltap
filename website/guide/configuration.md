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

Agent mode runs skilltap non-interactively: auto-yes, hard-fail on security warnings, and plain-text output (no spinners or prompts). It's how AI agents, CI scripts, and cron jobs invoke skilltap. There are three ways to enable it, ordered by precedence:

### Per-invocation flag (preferred)

```bash
skilltap install foo --agent
skilltap update --agent
```

The `--agent` flag forces agent mode for that one command, regardless of any persistent config. This is the recommended way for one-off automation runs because nothing leaks into your shell environment or config file.

### Environment variable

```bash
SKILLTAP_AGENT=1 skilltap install foo
```

Set `SKILLTAP_AGENT=1` to make every skilltap invocation in that shell run in agent mode. Useful when wrapping skilltap in a script or harness.

### Persistent config

```bash
skilltap config agent-mode
```

Interactive wizard that sets `[agent-mode] enabled = true` in your config. After running it, every skilltap invocation defaults to agent mode (you can still override with `--agent=false` per command). Requires a TTY — must be run by a human, not by an agent.

Precedence (highest to lowest): `--agent` flag > `SKILLTAP_AGENT` env var > `[agent-mode] enabled` in config.

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

### Open the config in `$EDITOR`

For sweeping edits, open the file directly:

```bash
skilltap config edit
```

This opens `~/.config/skilltap/config.toml` in your `$EDITOR` (falls back to `nano`). On exit, skilltap re-validates the file against the schema and reports parse/schema errors before considering the edit complete. Useful when you want to make several related changes at once instead of running `config set` repeatedly.

### Telemetry

`skilltap` collects no telemetry by default. To opt in (or check status):

```bash
skilltap config telemetry status     # show current state + reasoning
skilltap config telemetry enable     # opt in (anonymous, see below)
skilltap config telemetry disable    # opt out (default)
```

If enabled, an anonymous one-way `anonymous_id` (UUID written into `config.toml`) accompanies aggregated event counts. The `DO_NOT_TRACK` environment variable always wins regardless of config; setting it to any value disables telemetry for that invocation. See the [Security guide](/guide/security#telemetry) for what's collected.

### Self-update

To upgrade the skilltap binary itself (when installed as a prebuilt binary):

```bash
skilltap self-update           # check + interactively confirm
skilltap self-update --yes     # apply without confirmation
```

The auto-update behavior on startup is configured via the `[updates]` block (see [config-options.md](/reference/config-options#updates)). `auto_update = "off"` (default) means startup checks are notify-only.

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
