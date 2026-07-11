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

## Results and exit codes

Human output ends with `healthy`, `changes needed`, `user decision required`,
or `unhealthy`. JSON output includes schema version, command, scope, targets,
summary, resources, operations, warnings, errors, next actions, result, and exit
code.

| Code | Meaning |
| --- | --- |
| `0` | Healthy or requested operation completed. |
| `2` | Invalid command or arguments. |
| `3` | User decision or acknowledgment required. |
| `4` | Configuration or validation error. |
| `5` | Native harness or external command failure. |
| `6` | Drift, conflict, or unsafe overwrite prevented mutation. |
| `7` | Partial application; some independent operations succeeded. |
