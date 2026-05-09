---
description: Configure skilltap via TOML file or interactive wizard. Set default scope, agent symlinks, scan policy, and non-interactive automation flags.
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

Security settings live in a single flat `[security]` block. Per-source exemptions go in `[[security.overrides]]` entries that map a tap or source type to a preset. Use the dedicated security wizard:

```bash
skilltap config security                       # interactive wizard
skilltap config set security.on_warn fail      # set a single key
skilltap config set security.scan semantic     # require semantic scan
```

See the [Security guide](/guide/security#configuring-security-behavior) for full details.

## Non-interactive automation

skilltap detects non-interactive contexts automatically — there is no separate "agent mode" to enable. Use these knobs when invoking skilltap from AI agents, CI scripts, cron jobs, or shell pipelines:

### TTY detection (automatic)

When stdout is not a TTY (e.g. piped, redirected, or invoked from a child process), skilltap drops spinners and clack prompts and emits plain text suitable for parsing. No flag required.

### `--yes` (auto-confirm)

```bash
skilltap install skill foo --yes
skilltap update --yes
```

Auto-accepts every confirmation prompt. Combine with non-TTY contexts to get a fully unattended run. Note that security warnings still respect `[security] on_warn`: set `on_warn = "fail"` for hard-fail behavior in CI.

### `--json` (machine-readable output)

```bash
skilltap install skill foo --json
skilltap list --json
```

Emits structured JSON instead of human-formatted text. Pair with `--yes` for scripted use.

::: tip Persistent agent mode was retired in v2.0
v1 had a `--agent` flag, a `SKILLTAP_AGENT` env var, and an `[agent-mode]` config block. All three were removed. TTY detection plus `--yes`/`--json` covers every non-interactive case without a separate runtime mode.
:::

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

Only preference keys are settable via `config set`. Security policy keys and telemetry are blocked -- use the interactive wizard or dedicated subcommands for those. See the [CLI reference](/reference/cli#skilltap-config-set) for the full list of settable keys.

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

A single flat block controls scanning and warning behavior. Per-source trust is configured via `[[security.overrides]]` entries that apply a preset to a named tap or source type.

| Key             | Type     | Default    | Description                                                   |
| --------------- | -------- | ---------- | ------------------------------------------------------------- |
| `scan`          | string   | `"static"` | Scan mode: `"off"`, `"static"`, or `"semantic"`               |
| `on_warn`       | string   | `"prompt"` | Warning behavior: `"prompt"`, `"fail"`, or `"allow"`          |
| `require_scan`  | boolean  | `false`    | When `true`, `--skip-scan` is blocked                         |
| `agent_cli`     | string   | `""`       | Agent CLI for semantic scan (e.g. `"claude"`)                 |
| `threshold`     | number   | `5`        | Semantic score threshold (0-10, chunks >= this flagged)       |
| `max_size`      | number   | `51200`    | Max total skill size in bytes before warning (default 50 KB)  |
| `ollama_model`  | string   | `""`       | Ollama model name for semantic scanning                       |
| `overrides`     | table[]  | `[]`       | Per-tap or per-source preset overrides (see below)            |

Each `[[security.overrides]]` entry has three fields:

| Field    | Values                                            | Description                                          |
| -------- | ------------------------------------------------- | ---------------------------------------------------- |
| `match`  | string                                            | Tap name or source type (`tap`, `git`, `npm`, `local`) |
| `kind`   | `"tap"` \| `"source"`                             | What `match` refers to                               |
| `preset` | `"none"` \| `"relaxed"` \| `"standard"` \| `"strict"` | Preset applied to matching installs               |

Presets resolve to concrete scan/on_warn/require_scan values:

| Preset     | `scan`     | `on_warn` | `require_scan` |
| ---------- | ---------- | --------- | -------------- |
| `none`     | `off`      | `allow`   | `false`        |
| `relaxed`  | `static`   | `allow`   | `false`        |
| `standard` | `static`   | `prompt`  | `false`        |
| `strict`   | `semantic` | `fail`    | `true`         |

Named tap overrides take priority over source-type overrides; first match wins. For non-interactive runs (CI, agents) set `on_warn = "fail"` so warnings hard-fail instead of prompting.

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
- `--skip-scan` bypasses scanning unless `require_scan = true` (or the resolved preset for the source sets it)
- `[[security.overrides]]` entries with `preset = "none"` are the persistent way to opt a tap or source type out of scanning entirely

Flags always override config for non-security settings like `scope` and `yes`.

## Example config

```toml
[defaults]
scope = "project"
also = ["cursor"]
yes = false

[security]
scan = "static"
on_warn = "prompt"
agent_cli = "claude"
threshold = 5

# Skip scanning for skills installed from a tap you control.
[[security.overrides]]
match = "my-corp"
kind = "tap"
preset = "none"

[registry]
enabled = ["skills.sh"]
```

For the full list of options and their allowed values, see the [Config Options Reference](/reference/config-options).
