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
| `on_warn`       | string  | `"prompt"` | Warning behavior: `"prompt"`, `"fail"`, or `"allow"` |
| `require_scan`  | boolean | `false`    | Block `--skip-scan` flag                             |
| `agent`         | string  | `""`       | Agent for semantic scan (e.g. `"claude-code"`)       |
| `threshold`     | number  | `6`        | Semantic score threshold (0-10)                      |
| `max_size`      | number  | `1048576`  | Max file size in bytes before flagging               |

### `[agent-mode]`

Settings applied when an AI agent runs skilltap.

| Key       | Type    | Default    | Description                                           |
| --------- | ------- | ---------- | ----------------------------------------------------- |
| `enabled` | boolean | `false`    | Enable agent mode                                     |
| `scope`   | string  | `""`       | Default scope in agent mode                           |
| `also`    | string[] | `[]`      | Additional agent symlinks in agent mode               |
| `scan`    | string  | `"static"` | Scan mode in agent mode (`"off"` promoted to `"static"`) |

## How config and flags compose

When a CLI flag and a config value conflict, the **most restrictive** option wins:

- `--strict` overrides `on_warn` to `"fail"`
- `--no-strict` overrides `on_warn` to `"allow"`
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
agent = "claude-code"
threshold = 6

[agent-mode]
enabled = false
scope = "global"
scan = "static"
```

For the full list of options and their allowed values, see the [Config Options Reference](/reference/config-options).
