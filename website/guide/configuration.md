---
description: Configure skilltap via TOML file or interactive wizard. Set default scope, agent symlinks, scan policy, and agent mode for non-interactive CI use.
---

# Configuration

## Config file location

skilltap stores its configuration at:

```
~/.config/skilltap/config.toml
```

You can edit this file directly or use the interactive wizard.

## Interactive wizard

Run the configuration wizard:

```bash
skilltap config
```

This walks you through each setting with prompts and sensible defaults. Existing values are preserved -- the wizard only updates what you change.

## Agent mode setup

Agent mode configures skilltap for use by AI agents (non-interactive, strict security, machine-readable output). Set it up with:

```bash
skilltap config agent-mode
```

This command requires a TTY -- it must be run by a human, not by an agent. It prompts for agent-mode-specific settings like default scope, scan level, and which agent to use.

## Config reference

### `[defaults]`

General defaults for install and update commands.

| Key     | Type     | Default | Description                                                     |
| ------- | -------- | ------- | --------------------------------------------------------------- |
| `scope` | string   | `""`    | Default install scope: `"global"` or `"project"`                |
| `also`  | string[] | `[]`    | Additional agent symlinks to create (e.g. `["cursor", "codex"]`) |
| `yes`   | boolean  | `false` | Skip confirmation prompts                                       |

### `[security]`

Controls scanning and warning behavior.

| Key             | Type    | Default    | Description                                          |
| --------------- | ------- | ---------- | ---------------------------------------------------- |
| `scan`          | string  | `"static"` | Scan mode: `"off"`, `"static"`, or `"semantic"`      |
| `on_warn`       | string  | `"prompt"` | Warning behavior: `"prompt"` (ask) or `"fail"` (block) |
| `require_scan`  | boolean | `false`    | Block `--skip-scan` flag                             |
| `agent`         | string  | `""`       | Agent for semantic scan (e.g. `"claude"`)            |
| `threshold`     | number  | `5`        | Semantic score threshold (0-10, chunks >= this flagged) |
| `max_size`      | number  | `51200`    | Max total skill size in bytes before warning (default 50 KB) |

### `[agent-mode]`

Settings applied when an AI agent runs skilltap.

| Key       | Type    | Default     | Description                                           |
| --------- | ------- | ----------- | ----------------------------------------------------- |
| `enabled` | boolean | `false`     | Enable agent mode                                     |
| `scope`   | string  | `"project"` | Default scope for agent installs: `"global"` or `"project"` |

Agent mode inherits `defaults.also` and `security.scan` from the corresponding sections — there are no separate agent-mode overrides for those fields.

## How config and flags compose

When a CLI flag and a config value conflict, the **most restrictive** option wins:

- `--strict` overrides `on_warn` to `"fail"`
- `--no-strict` overrides `on_warn` to `"prompt"`
- `require_scan = true` blocks `--skip-scan` (returns an error)
- Agent mode forces `on_warn = "fail"` and blocks `--skip-scan`
- Agent mode promotes `scan = "off"` to `scan = "static"`

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
require_scan = false
agent = "claude"
threshold = 5

[agent-mode]
enabled = false
scope = "project"
```

For the full list of options and their allowed values, see the [Config Options Reference](/reference/config-options).
