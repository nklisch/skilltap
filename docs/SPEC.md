# Specification

This document defines skilltap's command behavior, state model, reconciliation semantics, and filesystem rules. Harness-specific native contracts live in [HARNESS-CONTRACTS.md](./HARNESS-CONTRACTS.md).

## Product Boundary

skilltap manages one person's Codex and Claude Code environments on one computer.

It manages:

- Harness registration.
- Plugin marketplaces.
- Native and materialized plugins.
- Standalone skills from explicit sources.
- MCP servers contributed by managed plugins.
- Global and project instructions.
- Compatibility, ownership, drift, health, and updates.
- Its own optional Claude Code and Codex plugin distribution, including the
  high-level skill that teaches agents how to operate the binary.

skilltap does not search, rank, recommend, or browse available skills and plugins. The caller supplies an explicit marketplace source, plugin selector, skill source, or local path.

The self-hosted plugin is an explicit product distribution, not a discovery
catalog. It does not search or recommend other skills, plugins, or marketplaces.

## Self-Hosted Plugin Distribution

The skilltap repository is the canonical implementation and release source for
one public plugin identity with distinct native channel artifacts. The active
`../skills` repository is a maintained secondary marketplace publisher of the
same plugin; its marketplace entry points directly at this repository's
canonical plugin subdirectory rather than copying manifests or versions.

- Claude Code uses a `.claude-plugin/plugin.json` manifest and a
  `.claude-plugin/marketplace.json` catalog.
- Codex uses a `.codex-plugin/plugin.json` manifest and its documented
  `.agents/plugins/marketplace.json` catalog.
- Both channels carry the same complete `skilltap` skill directory, including
  its top-level `SKILL.md` and any supporting references. The channel manifests
  and catalogs remain native documents; one is never treated as the other's
  schema.

The skill is a high-level discovery and operations guide. It explains the
binary's purpose, command families, scope and target selection, configuration
layout, status/plan/sync workflow, update and daemon behavior, and how an agent
should turn a diagnostic into a user-facing decision. It does not duplicate
the full CLI reference, search marketplace contents, or make recommendations.
Exact syntax and behavior are learned from `skilltap --help` and the relevant
leaf command's help output.

The plugin release must provide a deterministic binary bootstrap path for the
supported macOS and Linux platforms. Where a native harness supports a
documented automatic setup hook, installation may invoke it. Otherwise the
plugin supplies one explicit, agent-invocable bootstrap command. In either
case, plugin installation and binary availability are separate observed facts:
the plugin must verify the release artifact, checksum, platform, and installed
binary before claiming setup succeeded. It must not assume that an arbitrary
post-install script executes in both harnesses, write native plugin caches, or
require elevated privileges.

Plugin metadata and binary artifacts are versioned from the same skilltap
release and retain the repository's checksum and provenance guarantees. The
legacy `nklisch/skilltap-skills` repository is a temporary migration source
only; it is not a second long-term source of truth and is archived after the
canonical plugin publication is live. The active sibling `../skills`
development repository is unrelated to this retirement and must not be
archived or modified by this feature.

Native plugin marketplaces are the primary distribution surface. The verified
one-line installer and Homebrew are direct binary alternatives when the plugin
flow cannot provide an executable. The installer downloads and verifies the
current binary, detects installed Claude Code and Codex binaries, and invokes
the same bootstrap flow to report or repair resources those harnesses can
load. Detection never implies that a harness is enabled for ordinary skilltap
reconciliation.

Help and error output are part of this distribution contract. Every public
command and leaf command exposes concise, non-interactive help with its
purpose, accepted scope/target flags, acknowledgment requirements, output
shape, and exit behavior. Errors name the failing boundary, redact secrets,
and provide an actionable next command or user decision. JSON remains one
stable document derived from the same outcome as plain output.

## Terminology

**Harness**

A supported agent runtime. The supported harness identifiers are `codex` and `claude`.

**Desired inventory**

The normalized resources skilltap intends managed harnesses to contain.

**Observed state**

The current native configuration reported by a harness adapter.

**Provenance**

The origin and ownership of a resource: native, adopted, direct, or materialized.

**Native resource**

A resource installed through the target harness's supported lifecycle.

**Adopted resource**

An existing native resource imported into skilltap's desired inventory.

**Materialized resource**

A resource created and owned by skilltap because the target harness lacks a native distribution for the source plugin.

Materialization may be the primary lifecycle for a harness that provides
documented skill and MCP load paths but no native marketplace or plugin manager.

**Faithful equivalent**

A target representation that preserves the source component's behavior and requirements.

**Partial materialization**

A representation that omits or changes one or more source components.

**Drift**

A difference between desired inventory, last-applied state, and current native state.

## Operating Model

Every command is deterministic and non-interactive.

Commands never open a picker, prompt for confirmation, or require a TTY. Missing information is an error with an actionable message.

Human-readable output is the default. Commands that inspect, plan, or mutate state accept `--json`.

`--target` accepts `codex`, `claude`, or `all`. When omitted, it resolves to every enabled harness. A command fails when no harness is enabled.

Scoped commands accept:

- No scope flag — global scope.
- `--project` — the project containing the current directory.
- `--project <path>` — the project containing the supplied path.
- `--all-scopes` — global scope and every project recorded in inventory.

`--project` and `--all-scopes` are mutually exclusive. Project resolution uses the containing Git root when one exists. Outside a Git repository, the canonical directory itself is the project root.

`--target` selects harnesses while the scope flag selects where resources apply. The two dimensions are independent.

## Configuration Directory

skilltap stores all machine-wide configuration under:

```text
${XDG_CONFIG_HOME:-$HOME/.config}/skilltap/
├── config.toml
├── inventory.toml
├── state.json
└── managed/
```

The directory is created by the first command that changes skilltap state. No explicit initialization command exists.

### `config.toml`

`config.toml` contains operating policy rather than installed resources.

```toml
schema = 1

[harnesses.codex]
enabled = true
binary = "codex"

[harnesses.claude]
enabled = true
binary = "claude"

[instructions]
claude_mode = "symlink"

[updates]
mode = "apply-safe"
interval = "6h"
```

Harness binaries default to `codex` and `claude` from `PATH`. An absolute binary path may override either default.

A missing `config.toml` is an explicit first-use state. Read-only status may
detect both known harness installations, but neither harness is enabled until
the user runs `harness enable`. Read-only commands do not synthesize or persist
default policy.

`instructions.claude_mode` accepts:

- `symlink` — create `CLAUDE.md` as a symlink to `AGENTS.md`.
- `import` — create a managed `CLAUDE.md` containing `@AGENTS.md`.

`updates.mode` accepts:

- `off` — do not check automatically.
- `check` — check and report without applying.
- `apply-safe` — apply updates that require no user decision.

The update interval is used only when the optional daemon is enabled.

### `inventory.toml`

`inventory.toml` is the human-readable desired state for the computer.

It contains:

- Registered marketplaces.
- Installed plugins.
- Installed standalone skills.
- Managed instruction locations.
- Target harnesses for each resource.
- Explicit component inclusions and exclusions.
- Project paths containing locally managed native configuration.
- Per-resource update policy and version pins.

Every entry has a stable logical `ResourceId` and a concrete scope. Together
they form the exact `ResourceKey` used by inventory, state, dependencies, and
planned operations. The same logical identifier may exist independently in
global and multiple project scopes. Project paths are canonical absolute paths.
Authentication material is prohibited.

### `state.json`

`state.json` is machine-written provenance and apply state.

It records:

- Schema version.
- Native identifiers.
- Provenance and source identity.
- Materialization ownership.
- Content fingerprints.
- Installed and available versions or source revisions.
- Requested Git refs and resolved commit SHAs.
- Last update check.
- Last successful observation and application timestamps.
- Native harness versions.
- Per-resource apply results.

Users and agents may inspect `state.json`, but do not edit it as desired configuration.

Fresh declared/effective observations, capability-profile evidence, and health
findings are ephemeral. `status` and `adopt` do not persist their snapshots.
Successful mutation workflows may update provenance, fingerprints, revisions,
harness versions, timestamps, and apply results after re-observation.

### `managed/`

`managed/` contains skilltap-owned copies of materialized plugins and directly installed skills when a native package lifecycle is unavailable.

Managed artifacts never contain authentication secrets copied from harness configuration.

## Harness Commands

```text
skilltap harness list [--json]
skilltap harness enable <harness> [--binary <path>] [--json]
skilltap harness disable <harness> [--json]
```

`harness list` reports whether each harness is enabled and reachable, its detected version and adapter compatibility, its native configuration paths, and any blocking health issue.

`harness enable` creates the configuration directory when needed and enables one harness. It does not adopt or modify native configuration.

`harness disable` stops reconciliation for that harness. It does not uninstall
native resources or remove retained provenance and apply history.

## Adoption

```text
skilltap adopt [--from <target>] [--project [<path>] | --all-scopes] [--json]
```

`adopt` observes native resources and adds them to desired inventory. When `--from` is omitted, skilltap adopts from all enabled harnesses. Without a scope flag it adopts global resources. `--project` adopts the resolved project scope. `--all-scopes` adopts global resources and every project already recorded in inventory.

Adoption:

- Does not modify harness configuration.
- Preserves native source and scope.
- Records the source harness as provenance.
- Coalesces resources only when their identities and semantics match.
- Reports conflicts rather than choosing a winner.
- Leaves conflicting entries unadopted.
- May adopt all non-conflicting resources in the same invocation.
- Observes project-shared Claude declarations as health evidence but does not
  adopt them into personal project scope; the CLI has no shared-scope adoption
  selector.

Adopting a native resource makes it managed by reconciliation. It does not transfer the resource to other harnesses until `sync` is run.

## Status

```text
skilltap status [--target <target>] [--project [<path>] | --all-scopes] [--json]
```

`status` works before skilltap configuration exists.

It reports:

- Installed and enabled harnesses.
- Registered native marketplaces.
- Installed native plugins and standalone skills.
- Instruction health.
- Desired and unmanaged observed resources.
- Missing or drifted managed resources.
- Compatibility warnings.
- Broken or conflicting symlinks.
- Materialized resources and their source provenance.
- Available, pinned, blocked, or unreachable updates.
- Partial previous applications.
- Daemon health and last run when enabled.

Human-readable status ends with `healthy`, `changes needed`, `user decision required`, or `unhealthy`.

Status does not mutate skilltap or harness state.

## Planning

```text
skilltap plan [--target <target>] [--project [<path>] | --all-scopes] [--json]
```

`plan` compares desired inventory, last-applied state, fresh native
observations, and the capabilities selected from a verified compiled profile
for each concrete scope. Runtime probes may preserve or narrow compiled
support; they never grant mutation authority. Unknown harness versions remain
observe-only.

Every planned operation includes:

- Operation identifier.
- Target harness.
- Exact scope-bearing resource or component selector.
- Action and reason.
- Compatibility classification.
- Provenance.
- Files or native commands affected.
- Reversibility.
- Required acknowledgment.

Plan actions include marketplace lifecycle, plugin lifecycle, skill lifecycle, materialization, instruction repair, unmanaged conflicts, updates, and blocked incompatibilities.

Plans are not persisted for later blind application. `sync` observes the environment and computes a fresh plan.

## Synchronization

```text
skilltap sync
  [--target <target>]
  [--project [<path>] | --all-scopes]
  [--yes]
  [--include <resource-or-component>...]
  [--exclude <resource-or-component>...]
  [--json]
```

`sync` computes and applies the current reconciliation plan.

Without `--yes`:

- Native operations apply when they are fully supported.
- Faithful equivalents apply.
- Partial or lossy resource operations remain blocked.
- Unrelated safe operations may still apply.
- Output explains every blocked operation.

With `--yes`, partial operations may apply using the exact component set shown in the plan. `--yes` acknowledges the reported loss; it does not make unsupported components functional.

`--include` and `--exclude` constrain the operation set. Input selectors may
address a logical resource or one component within the command's selected
scopes. Each resulting operation carries an exact `ResourceKey`, and its
selector scope must equal its operation scope. Exclusion takes precedence over
inclusion.

A resource with unsupported required components remains blocked even with `--yes`. A component is required when the native manifest declares it as a dependency or omitting it prevents the remaining resource from functioning.

Synchronization is idempotent. Running it again without external changes produces an empty plan.

## Marketplace Lifecycle

```text
skilltap marketplace add <source> [--target <target>] [--project [<path>] | --all-scopes] [--name <name>] [--json]
skilltap marketplace remove <name> [--target <target>] [--project [<path>] | --all-scopes] [--json]
skilltap marketplace update [<name>] [--target <target>] [--project [<path>] | --all-scopes] [--json]
skilltap marketplace list [--target <target>] [--project [<path>] | --all-scopes] [--json]
```

Marketplace sources are explicit and may be a GitHub repository shorthand, Git URL, local directory, or remote catalog URL when supported natively by the target harness.

`marketplace add` updates desired inventory and registers the marketplace through each target harness's native lifecycle.

A source containing native catalogs for both harnesses may register both. A source supporting only one harness is not automatically translated into a marketplace for the other harness.

`marketplace list` lists registered marketplaces and their health. It does not list or search the plugins available inside them.

Removing a marketplace is blocked when doing so would implicitly remove managed plugins unless those plugins are included in the same requested operation.

## Plugin Lifecycle

```text
skilltap plugin install <plugin>@<marketplace>
  [--target <target>]
  [--project [<path>] | --all-scopes]
  [--yes]
  [--include <component>...]
  [--exclude <component>...]
  [--json]

skilltap plugin remove <plugin>@<marketplace> [--target <target>] [--project [<path>] | --all-scopes] [--json]
skilltap plugin update [<plugin>] [--target <target>] [--project [<path>] | --all-scopes] [--yes] [--json]
skilltap plugin list [--target <target>] [--project [<path>] | --all-scopes] [--json]
```

The caller provides an exact plugin selector. skilltap does not resolve fuzzy names.

For each target, installation preference is:

1. Install the target's native plugin from its registered marketplace.
2. Use a documented faithful native equivalent.
3. Materialize a compatible plugin from accessible source components.
4. Report a partial plan and require `--yes`.
5. Block the operation.

A target harness is eligible when it can faithfully load complete skill
directories and MCP configuration from documented global and project surfaces
that skilltap can observe. Marketplace registration and plugin lifecycle are
capabilities, not prerequisites. When they are unavailable, skilltap owns the
managed artifact and its source, revision, update, drift, and removal lifecycle.
It never substitutes writes to undocumented caches.

`plugin list` lists installed and desired plugins only. It does not expose an available-plugin catalog.

Native plugin dependencies remain under the native harness lifecycle. Materialized plugin dependencies are recorded explicitly in skilltap state.

Removing a materialized plugin removes only skilltap-owned files. Removing an adopted or native plugin uses the harness's supported removal lifecycle.

For every managed plugin, skilltap records the installed version or source revision per harness, marketplace and upstream identity, last update check, available version, pin, update policy, provenance, and compatibility result.

`plugin update` refreshes registered marketplace metadata before resolving updates. Native plugins update through the harness lifecycle. Materialized plugins regenerate only when the new component set remains faithful or the caller supplies the required foreground acknowledgment.

## Standalone Skill Model

A skill is a directory containing a top-level `SKILL.md`.

The complete directory is the skill, including scripts, references, templates, assets, configuration, and other files referenced by `SKILL.md`. skilltap never extracts `SKILL.md` from the rest of the directory.

A source directory with `SKILL.md` at its root resolves directly as one skill. When a repository contains a skill in a subdirectory, the caller identifies that directory explicitly with `--path`. skilltap does not search the repository and ask the caller to choose among discovered skills.

The complete skill directory is installed, linked, fingerprinted, checked for drift, and updated as one artifact.

## Standalone Skill Lifecycle

```text
skilltap skill install <source>
  [--name <name>]
  [--target <target>]
  [--project [<path>] | --all-scopes]
  [--ref <git-ref>]
  [--path <subdirectory>]
  [--yes]
  [--json]

skilltap skill remove <skill> [--target <target>] [--project [<path>] | --all-scopes] [--json]
skilltap skill update [<skill>] [--target <target>] [--project [<path>] | --all-scopes] [--yes] [--json]
skilltap skill list [--target <target>] [--project [<path>] | --all-scopes] [--json]
```

A standalone skill source must be an explicit local directory, Git repository URL, Git repository plus explicit subdirectory, or GitHub repository shorthand.

The source must resolve to one skill directory. The directory must contain `SKILL.md` at its top level.

Standalone skills use `.agents/skills/<skill-name>/` as their canonical managed representation when compatible. Harness adapters link or copy the complete skill directory only where required. A skill installed as part of a native plugin remains owned by that plugin and is not duplicated as a standalone skill.

For a Git-backed skill, state records the source URL, requested ref, and resolved commit SHA. A branch, tag, or default branch has an available update when it resolves to a different SHA. An explicitly pinned commit SHA does not update automatically.

Local edits to a managed Git-backed skill produce drift and block automatic replacement. A local-path skill is monitored for drift but has no remote update lifecycle.

Skill update output includes the old SHA, new SHA, compatibility change, and affected harnesses.

## Skill Compatibility

skilltap reads standard `SKILL.md` frontmatter, declared compatibility text and metadata, harness-specific companion metadata, referenced path variables, declared tool and MCP dependencies, and files required by the skill.

Compatibility is classified as:

- `compatible`
- `target-specific`
- `unknown`
- `incompatible`

An explicit target exclusion is incompatible. Known use of another harness's exclusive variables or components is target-specific. A standard skill with no target-specific requirements is compatible.

Unrecognized compatibility declarations are unknown. Native installation may proceed, but cross-harness materialization of an unknown skill requires `--yes`.

Compatibility warnings identify the evidence that produced the classification.

## Instruction Lifecycle

```text
skilltap instructions setup [--project [<path>] | --all-scopes] [--mode <mode>] [--yes] [--json]
skilltap instructions status [--project [<path>] | --all-scopes] [--json]
skilltap instructions repair [--project [<path>] | --all-scopes] [--yes] [--json]
```

Without a scope flag, instruction commands operate on global instructions. `--project` resolves the current project, `--project <path>` resolves another project, and `--all-scopes` operates on global instructions plus all managed project instruction locations.

Global canonical instructions live at `~/AGENTS.md`. Project canonical instructions live in `AGENTS.md` at the selected project root. Nested `AGENTS.md` files may be tracked when explicitly adopted.

Enabled harnesses bridge from their native global instruction locations to `~/AGENTS.md` when they do not load it directly. The bridges are `${CODEX_HOME:-$HOME/.codex}/AGENTS.md -> ~/AGENTS.md` and `~/.claude/CLAUDE.md -> ~/AGENTS.md`. `CODEX_HOME` relocates Codex-native state only; it never relocates canonical global instructions.

Claude integration uses the configured mode:

- `symlink` creates the appropriate `CLAUDE.md` symlink.
- `import` creates a managed file containing `@AGENTS.md`.

Setup behavior:

- If the canonical file and native bridges do not exist, create an empty canonical `AGENTS.md` and the required bridges.
- If the canonical `AGENTS.md` exists, preserve it and create missing bridges.
- If only a native instruction file exists, block and instruct the caller to adopt or reconcile its content.
- If a native instruction file contains content that differs from the canonical file, block.
- If the expected bridge exists, make no change.
- Never replace user-authored instruction content without `--yes` and a recoverable backup.

`repair` fixes only resources already recorded as skilltap-managed.

## Update Daemon

```text
skilltap daemon enable [--interval <duration>] [--json]
skilltap daemon disable [--json]
skilltap daemon status [--json]
skilltap daemon run [--json]
```

The daemon is optional and disabled until explicitly enabled.

`daemon enable` installs and starts a user-level `launchd` service on macOS or `systemd --user` service on Linux. `daemon disable` stops and removes that service. `daemon run` is the foreground process invoked by the service manager and is available for diagnostics.

On each interval, the daemon:

1. Refreshes registered marketplaces.
2. Resolves managed plugin versions and Git-backed skill SHAs.
3. Records available updates.
4. Applies updates when policy is `apply-safe` and no decision is required.
5. Re-observes affected harnesses.
6. Records results for `status`.

The same update lifecycle may manage the skilltap binary itself. Binary
updates default to the latest compatible release, can be disabled by policy,
and never auto-apply a major-version change unless the user explicitly opts
in. Plugin guidance and the website use this lifecycle rather than inventing a
separate updater.

The daemon never supplies `--yes`, accepts partial materialization, overwrites local drift, resolves conflicts, or modifies unmanaged resources. An update requiring judgment remains pending for foreground review.

Per-resource update policy and pins override global update policy.

## Ownership and Removal

skilltap never deletes an unmanaged resource merely because it is absent from inventory.

A resource becomes managed when it is installed through skilltap, adopted explicitly, materialized by skilltap, or created by an instruction setup operation.

Every removal plan states which native command or filesystem path is affected.

Symlinks are inspected without following them for ownership decisions. Targets are resolved separately for health checks.

## Mutation Safety

Only one mutating skilltap process may hold the configuration lock.

Before mutation, skilltap observes affected targets, validates required binaries and paths, computes the complete plan, checks compatibility and acknowledgment requirements, acquires the state lock, and re-observes fingerprints that could have changed.

File writes use atomic replacement where supported. Existing user files changed by an approved operation receive recoverable backups under the configuration directory.

Native CLI operations are not treated as globally transactional. After a failure, skilltap records completed and failed operations, stops dependent operations, re-observes native state, and reports the recovery plan.

## Output

Default output is concise plain text suitable for a terminal or agent transcript.

JSON output uses one document per command:

```json
{
  "schema": 1,
  "command": "status",
  "result": "attention_required",
  "summary": {},
  "resources": [],
  "operations": [],
  "warnings": [],
  "errors": [],
  "next_actions": []
}
```

Blocked operations include a `next_actions` entry that tells an invoking agent what must be explained to the user and which confirmation form permits continuation.

JSON output contains no ANSI formatting or incidental prose outside the document.

## Exit Codes

- `0` — operation completed and desired state is satisfied.
- `1` — invalid input, invalid configuration, or operational failure before mutation.
- `2` — drift, planned changes, or a user decision requires attention.
- `3` — mutation partially completed and recovery is required.

For `plan`, a non-empty plan exits `2`. For `status`, unhealthy state or required changes exit `2`. For `sync`, blocked operations exit `2` when no mutation failed. A failed or partial mutation exits `3`.

## Platform Contract

skilltap supports macOS and Linux.

It requires a filesystem with symbolic-link support when instruction mode is `symlink`, Git for Git-backed direct sources, and the native harness CLI for native lifecycle operations.

Windows is not part of the supported platform contract.

## Validation

All configuration, inventory, state, native command output, plugin manifests, skill frontmatter, marketplace metadata, and daemon service definitions are validated at their boundaries.

Unknown native fields are preserved when skilltap edits a documented configuration file.

A malformed managed file is an error. A malformed unmanaged native resource appears as a status health issue and is not rewritten automatically.
