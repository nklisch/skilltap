# Command-Line Experience

skilltap is a non-interactive CLI for people, agents, scripts, and background automation.

The CLI favors explicit commands, stable output, and actionable failure over prompts, menus, and implicit decisions.

## Interaction Principles

1. **No prompts.** Commands either have enough information to run or explain what is missing.
2. **Global by default.** A scoped command without a scope flag operates globally.
3. **Explicit projects.** `--project` selects the current project; `--project <path>` selects another.
4. **Explicit sources.** The caller names the marketplace, plugin, skill source, or path.
5. **Plans explain mutations.** Every change has a visible reason.
6. **Safe progress is allowed.** Unrelated safe operations may proceed while one resource remains blocked.
7. **Loss requires acknowledgment.** `--yes` permits a reported partial result.
8. **Piecewise control is optional.** `--include` and `--exclude` narrow an operation when necessary.
9. **Agents receive next actions.** Blocked output says what the user must decide.
10. **JSON is a representation, not a separate mode.** Command semantics do not change under `--json`.

## Command Tree

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
skilltap daemon run [--json]
```

There is no bare interactive experience and no `init` command. Running `skilltap` without a subcommand prints concise help and exits with an input error.

## Target and Scope

Target and scope are independent.

`--target` selects harnesses:

```text
--target codex
--target claude
--target all
```

When omitted, the target is every enabled harness.

Scope selects where resources apply:

```text
no scope flag       global scope
--project           project containing the current directory
--project <path>    project containing the supplied path
--all-scopes        global and every managed project scope
```

The containing Git root is the project root when one exists. Outside Git, the canonical directory itself is the project root.

Examples:

```console
$ skilltap plugin install formatter@example
# Global, all enabled harnesses

$ skilltap plugin install formatter@example --project --target claude
# Current project, Claude only

$ skilltap sync --project ~/src/my-app
# The project containing ~/src/my-app

$ skilltap status --all-scopes
# Complete managed computer
```

No skilltap metadata is written into a project. A plan lists any native project files a harness operation changes.

## Common Flags

```text
--target <codex|claude|all>
--project [<path>]
--all-scopes
--json
--yes
--include <selector>
--exclude <selector>
```

Only commands for which a flag is meaningful accept it. `--project` and `--all-scopes` are mutually exclusive.

`--yes` acknowledges a reported partial or lossy foreground operation. It does not bypass invalid configuration, missing dependencies, unsupported required components, local drift, or native harness policy.

`--include` and `--exclude` are repeatable. Exclusion wins when both match the same resource or component.

Input selectors are resolved separately inside every concrete selected scope.
Planned operations carry the resulting exact scope-bearing resource or
component selector, so equal logical IDs in global and project scopes never
authorize one another.

## First Use

`status` works before any skilltap files exist.

```console
$ skilltap status

Global agent environment: unmanaged

Harnesses
  codex   installed  0.x.x  not enabled
  claude  installed  2.x.x  not enabled

Native resources
  codex   2 marketplaces, 4 plugins, 7 skills
  claude  1 marketplace, 3 plugins, 5 skills

Instructions
  global AGENTS.md   present
  global CLAUDE.md   separate content

Next actions
  skilltap harness enable codex
  skilltap harness enable claude
  skilltap adopt
  skilltap instructions status

Result: changes needed
```

Status discovers the global environment but does not create configuration. Use `status --project` for the current project or `status --all-scopes` for the complete managed computer.

When configuration is missing, installation detection does not imply
enablement: both known harnesses are reported as not enabled until an explicit
`harness enable` command records policy.

## Enabling Harnesses

```console
$ skilltap harness enable codex
Enabled codex
Binary: /usr/local/bin/codex
Version: 0.x.x
Native configuration was not changed.

Next action: skilltap adopt --from codex
```

Enabling a harness creates skilltap configuration if required. It does not adopt or synchronize native resources.

## Adoption

Bare adoption imports global resources:

```console
$ skilltap adopt --from codex

Observed codex global scope
  adopted     2 marketplaces
  adopted     4 plugins
  adopted     7 standalone skills
  conflict    global instructions

No Codex configuration was changed.

Result: user decision required
```

Project adoption is explicit:

```console
$ skilltap adopt --from claude --project
Observed Claude configuration for /home/user/src/project
Adopted 2 plugins and 3 skills.
```

Adoption imports all non-conflicting resources. One conflict does not discard unrelated adoptable state. Adoption does not automatically push resources into another harness.

Claude project-shared declarations remain visible in project observations and
health output, but they are not adoption candidates for personal project scope.
The current CLI has no shared-scope adoption selector.

## Status

A healthy global environment is terse:

```console
$ skilltap status

Scope         global
Harnesses     codex, claude
Resources     3 marketplaces, 6 plugins, 9 skills
Instructions  healthy
Updates       current
Daemon        enabled, last run 18 minutes ago
Drift         none

Projects      4 managed; run skilltap status --all-scopes for details
Result: healthy
```

Attention is grouped by action:

```console
$ skilltap status --all-scopes

Updates
  available   skill:commit-helper
              8a21c4d -> c17f092

Blocked
  plugin:review-tools@team
    Claude component `lsp:typescript` has no faithful Codex equivalent.

Drift
  /home/user/src/app
    skill:release-helper differs from its managed Git revision.

Next actions
  skilltap skill update commit-helper
  skilltap plan --include plugin:review-tools@team
  skilltap status --project /home/user/src/app

Result: user decision required
```

Health findings use registered codes and authored summaries with typed scalar
context. Plain and JSON output never include raw native argv, stdout/stderr,
settings objects, unknown JSON, or dynamic parser messages.

## Planning and Synchronization

Bare planning and synchronization operate globally. Project or whole-computer operation is explicit.

```console
$ skilltap plan --project

Scope: /home/user/src/project

Plan
  codex
    install       plugin:commit-tools@personal        native
    link          skill:release-helper                faithful

  claude
    install       plugin:commit-tools@personal        native
    materialize   plugin:review-tools@personal        partial, blocked
      include     skill:review
      include     mcp:review-api
      omit        hook:codex-session-start

Summary
  safe operations       3
  blocked operations    1
  user decisions        1
```

`sync` applies safe operations and reports blocked resources. A non-empty plan exits `2`.

Native mutation is available only through a verified compiled capability
profile for the detected executable version and concrete scope. Runtime probes
may narrow that profile. Unknown versions remain observable but mutation is
reported as blocked.

```console
$ skilltap sync --project

Applied
  codex    installed plugin:commit-tools@personal
  codex    linked skill:release-helper
  claude   installed plugin:commit-tools@personal

Blocked
  claude   plugin:review-tools@personal
    `hook:codex-session-start` has no faithful Claude equivalent.

To install the compatible subset:
  skilltap sync --project --include plugin:review-tools@personal --yes

Explain the omitted hook to the user before confirming.
Result: user decision required
```

## Marketplace Management

The caller provides the marketplace source directly:

```console
$ skilltap marketplace add anthropics/claude-plugins --target claude

Registered anthropics-claude-plugins with Claude Code globally.
Result: healthy
```

Project registration uses the same command with `--project`. A repository containing native catalogs for both harnesses may target both. A source supporting only one target reports the other target without inventing a catalog.

`marketplace list` shows registered marketplaces only. It never displays or searches their available plugins.

## Plugin Management

Plugin installation requires an exact selector:

```console
$ skilltap plugin install formatter@team-tools --target all
```

If both harnesses expose the plugin natively, skilltap invokes both native lifecycles. If only one does, skilltap evaluates whether the source plugin can be materialized for the other target.

```console
$ skilltap plugin install deploy@claude-tools --target all

Installed
  claude   deploy@claude-tools   native

Blocked
  codex
    Materialization can preserve 2 of 3 components.
    supported     skill:deploy
    supported     mcp:deploy-api
    unsupported   agent:release-manager

To accept this partial Codex plugin:
  skilltap plugin install deploy@claude-tools --target codex --yes

Explain the missing `release-manager` agent to the user before confirming.
Result: user decision required
```

`plugin list` shows installed and desired plugins. It does not show the available contents of marketplaces.

## Standalone Skills

A skill is the complete directory containing a top-level `SKILL.md`.

```console
$ skilltap skill install ./commit-helper --target all

Installed skill:commit-helper globally
Source: /home/user/src/commit-helper
Files: 8
Targets: codex, claude
Representation: complete directory

Result: healthy
```

A skill inside a repository requires an explicit path:

```console
$ skilltap skill install https://github.com/example/agent-tools \
    --path skills/commit-helper \
    --project \
    --target all

Installed skill:commit-helper
Scope: /home/user/src/project
Source: https://github.com/example/agent-tools
Path: skills/commit-helper
Revision: 8a21c4d
Targets: codex, claude

Result: healthy
```

A repository with multiple skills is not searched. The source root must contain `SKILL.md`, or the caller must provide `--path`.

The installed artifact includes every file under the selected skill root.

## Skill Updates

```console
$ skilltap skill update commit-helper

Updated skill:commit-helper globally
Revision: 8a21c4d -> c17f092
Files: 8 -> 10
Compatibility: compatible
Targets: codex, claude

Result: healthy
```

A pinned commit does not update. Local edits block replacement, and `--yes` does not silently overwrite unidentified local skill changes.

## Instructions

Bare instruction commands operate globally:

```console
$ skilltap instructions setup

Canonical  /home/user/AGENTS.md
Codex      ${CODEX_HOME:-/home/user/.codex}/AGENTS.md -> /home/user/AGENTS.md
Claude     /home/user/.claude/CLAUDE.md -> ../AGENTS.md

Result: healthy
```

Project setup is explicit:

```console
$ skilltap instructions setup --project

Canonical  /home/user/src/project/AGENTS.md
Claude     /home/user/src/project/CLAUDE.md -> AGENTS.md

Result: healthy
```

Existing canonical instructions are preserved. If `AGENTS.md` and `CLAUDE.md` contain different user-authored content, setup blocks without changing either file. `repair` operates only on bridges already owned by skilltap.

## Updates

Plugin and skill updates use the same planning rules as synchronization. Safe updates may proceed while an unrelated update remains blocked.

```console
$ skilltap plugin update --all-scopes

Updated
  global                      formatter@team-tools   1.4.0 -> 1.5.0
  /home/user/src/application  formatter@team-tools   1.4.0 -> 1.5.0

Blocked
  /home/user/src/service      review-tools           2.1.0 -> 3.0.0
    The new version adds an unsupported required LSP component.

Result: user decision required
```

## Daemon

The daemon always checks every managed scope.

```console
$ skilltap daemon enable --interval 6h

Enabled skilltap update daemon
Service: systemd --user
Policy: apply-safe
Interval: 6h
Scopes: global and 4 managed projects

Result: healthy
```

On macOS, the service is reported as `launchd`.

The daemon never confirms partial updates, overwrites local drift, or resolves conflicts. `daemon run` runs the same update process in the foreground for diagnostics.

## JSON Output

`--json` emits exactly one JSON document.

```json
{
  "schema": 1,
  "command": "status",
  "result": "attention_required",
  "scope": {
    "kind": "all"
  },
  "summary": {
    "harnesses": 2,
    "plugins": 6,
    "skills": 9,
    "blocked": 1
  },
  "resources": [],
  "operations": [],
  "warnings": [],
  "errors": [],
  "next_actions": []
}
```

JSON schemas use stable field names. New optional fields may be added without changing existing meanings.

## Errors

Errors lead with the failed outcome, then give evidence and a next action.

```console
Error: Codex rejected plugin installation.

Plugin: formatter@team-tools
Scope: global
Command: codex plugin add formatter@team-tools
Exit status: 1
Reason: marketplace `team-tools` is not registered

Next action:
  skilltap marketplace add <source> --target codex
```

Sensitive environment values and native authentication material are never printed.

## Exit Codes

```text
0  completed; desired state is satisfied
1  invalid input, configuration, or pre-mutation operational failure
2  drift, planned changes, or user decision requires attention
3  mutation partially completed; recovery is required
```

Agents should use both the exit code and structured result. Exit code `2` is an attention state, not an execution crash.
