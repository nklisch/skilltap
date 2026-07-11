---
description: skilltap command and flag reference.
---

# CLI Reference

skilltap is deterministic and non-interactive. Running it without a subcommand
prints concise help and exits with an input error.

## Commands

```text
skilltap harness list
skilltap harness enable <codex|claude>
skilltap harness disable <codex|claude>

skilltap adopt
skilltap status
skilltap plan
skilltap sync

skilltap marketplace add <source>
skilltap marketplace remove <name>
skilltap marketplace update [name]
skilltap marketplace list

skilltap plugin install <plugin>@<marketplace>
skilltap plugin remove <plugin>
skilltap plugin update [plugin]
skilltap plugin list

skilltap skill install <source>
skilltap skill remove <skill>
skilltap skill update [skill]
skilltap skill list

skilltap instructions setup
skilltap instructions status
skilltap instructions repair

skilltap daemon enable
skilltap daemon disable
skilltap daemon status
skilltap daemon run
```

There is no `init` command. The first mutating command creates skilltap's
configuration directory when necessary.

## Common selectors

| Flag | Meaning |
| --- | --- |
| `--target <codex\|claude\|all>` | Select harnesses independently of scope. |
| no scope flag | Operate globally. |
| `--project` | Use the project containing the current directory. |
| `--project <path>` | Use the project containing the supplied path. |
| `--all-scopes` | Use global scope and every managed project. |
| `--json` | Render the same result as a stable JSON envelope. |
| `--yes` | Acknowledge a reported partial foreground result. |
| `--include <selector>` | Include matching resources or components; repeatable. |
| `--exclude <selector>` | Exclude matches; repeatable and wins over inclusion. |

Only commands for which a flag is meaningful accept it. `--project` and
`--all-scopes` are mutually exclusive.

Resource and component selectors begin as logical input. skilltap resolves
them only within the command's selected scopes. Every planned operation then
uses the exact resource key—the logical ID plus concrete global or project
scope—and rejects any selector whose scope disagrees with the operation.

When `config.toml` is missing, no harness is enabled. Read-only `status` remains
available and creates nothing; a mutating command creates owned configuration
only when that command requires it.

## Results and exit codes

Plain output uses command-specific human result labels rather than exposing the
JSON result class verbatim. Human-readable `status` output ends with one of:

```text
Result: healthy
Result: changes needed
Result: user decision required
Result: unhealthy
```

Other commands use the same human vocabulary where it applies to their
outcome; for example, successful marketplace and daemon operations end with
`Result: healthy`, while a blocked partial transfer ends with
`Result: user decision required`.

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
required changes exit `2`. For `sync`, blocked operations exit `2` when no
mutation failed; a failed or partial mutation exits `3`.
