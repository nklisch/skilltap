---
description: conceptual skilltap CLI index and executable-help guidance.
---

# CLI Reference

skilltap is a clear, non-interactive CLI that works well from a terminal or an
agent. Running it without a subcommand prints concise help and exits with an
input error.

## Executable help is authoritative

This page is a conceptual index, not a second command grammar. The compiled
binary owns names, arguments, validators, meaningful flags, and exit guidance.
Ask the executable whenever an agent needs exact syntax:

```console
$ skilltap --help
$ skilltap <command-family> --help
$ skilltap <command-family> <operation> --help
```

The public command families are:

| Family | Purpose | Start with |
| --- | --- | --- |
| Harness | Enable, disable, and inspect registered target policy and support tiers | `skilltap harness --help` |
| Adopt | Import native resources into desired state | `skilltap adopt --help` |
| Status | Inspect desired, observed, and managed health | `skilltap status --help` |
| Plan | Preview operations without mutating native state | `skilltap plan --help` |
| Sync | Apply the selected desired-state operations | `skilltap sync --help` |
| Marketplaces | Register, update, remove, and list native marketplace registrations | `skilltap marketplace --help` |
| Plugins | Install, update, remove, and list plugins | `skilltap plugin --help` |
| Skills | Install, update, remove, and list complete skill directories | `skilltap skill --help` |
| Instructions | Set up and repair canonical `AGENTS.md` bridges | `skilltap instructions --help` |
| Daemon | Opt into, inspect, run, or disable background updates | `skilltap daemon --help` |

The standalone `adopt`, `status`, `plan`, and `sync` commands are the state
control-plane operations. Their exact scope, target, selection, and
acknowledgment flags are shown by their leaf help pages.

There is no `init` command. The first mutating command creates skilltap's
configuration directory when necessary.

## Common selectors

| Flag | Meaning |
| --- | --- |
| `--target <registered-id\|all>` | Select harnesses independently of scope; use `harness list` for current ids. |
| no scope flag | Operate globally. |
| `--project` | Use the project containing the current directory. |
| `--project <path>` | Use the project containing the supplied path. |
| `--all-scopes` | Use global scope and every managed project. |
| `--json` | Render the same result as a stable JSON envelope. |
| `--yes` | Acknowledge every eligible partial, lossy, or effective-unverified declaration consequence; blocked work remains blocked. |
| `--include <selector>` | Include matching resources or components; repeatable. |
| `--exclude <selector>` | Exclude matches; repeatable and wins over inclusion. |

Only commands for which a flag is meaningful accept it. `--project` and
`--all-scopes` are mutually exclusive; the executable rejects misplaced flags.

Every leaf help page also states the process exit classes: `0` completed, `1`
invalid or pre-mutation failure, `2` attention or user decision required, and
`3` partial mutation requiring recovery.

Resource and component selectors begin as logical input. skilltap resolves
them only within the command's selected scopes. Every planned operation then
uses the exact resource key—the logical ID plus concrete global or project
scope—and rejects any selector whose scope disagrees with the operation.

When `config.toml` is missing, no harness is enabled. Read-only `status` remains
available and creates nothing; file-only observe-only targets require no guessed
binary. A mutating command creates owned configuration only when that command
requires it.

## Target support tiers

`harness list`, `status`, and `plan` derive target behavior from the typed
registry and exact observed profile:

- **Verified** capabilities may execute normally.
- **Declaration-managed** components require foreground `--yes`; skilltap
  verifies its owned bytes while effective state remains unverified.
- **Observe-only** targets expose safe documented reads and no mutation ports.
- **Unsupported** components or scopes remain blocked without affecting safe
  siblings.

Native commands always require verified `Supported` authority. Unknown versions
never mutate, and the daemon never supplies declaration acknowledgment.

## Results and exit codes

Plain output uses the same stable result labels across commands rather than
exposing raw implementation details. Human-readable output ends with one of:

```text
Result: completed
Result: attention required
Result: invalid
Result: partial apply; recovery required
```

The process exit code is authoritative for automation; `--json` carries the
same result class and typed next actions for agents.

`--json` emits exactly one schema-1 JSON document. `schema`, `command`,
`result`, `summary`, `resources`, `operations`, `warnings`, `errors`, and
`next_actions` are always present. `scope` is present for scoped command
results. Harness selections and exit codes are not separate top-level fields;
target details belong in the applicable summary, resource, or operation data,
and the process reports its exit code to the caller.

```json
{
  "schema": 1,
  "command": "status",
  "result": "attention_required",
  "scope": {
    "kind": "all"
  },
  "summary": {},
  "resources": [],
  "operations": [],
  "warnings": [],
  "errors": [],
  "next_actions": []
}
```

| Code | Meaning |
| --- | --- |
| `0` | Operation completed and desired state is satisfied. |
| `1` | Invalid input, invalid configuration, or operational failure before mutation. |
| `2` | Drift, planned changes, or a user decision requires attention. |
| `3` | Mutation partially completed and recovery is required. |

For `plan`, a non-empty plan exits `2`. For `status`, unhealthy state or
required changes exits `2`. For `sync`, blocked operations exit `2` when no
mutation failed; a failed or partial mutation exits `3`. Consult the leaf help
and structured `next_actions` for the recovery command an agent should run.
