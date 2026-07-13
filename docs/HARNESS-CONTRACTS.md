# Harness Contracts

This document defines the native capabilities skilltap relies on for every
supported target harness, along with the mappings skilltap considers faithful.

A harness contract describes current supported behavior. Verified compiled
profiles grant mutation authority for known versions and scopes. Runtime
detection binds that profile to one executable and may narrow its support
before skilltap mutates anything.

## Contract Rules

1. Native lifecycle commands are preferred over direct file mutation.
2. Machine-readable native output is preferred over human-output parsing.
3. Harness caches are observed but never used as undocumented write APIs.
4. Project scope is personal to the skilltap user unless the native artifact is inherently project-shared.
5. A native file changed by skilltap appears explicitly in the reconciliation plan.
6. Unknown harness versions are observe-only and never gain mutation authority
   from runtime probing.
7. Unsupported or absent native lifecycle behavior may be materialized only
   through documented load paths with explicit skilltap ownership.
8. Every cross-harness mapping has an explicit compatibility classification.
9. Observation findings expose only registered codes, authored summaries, typed
   subjects, and bounded typed scalar fields; raw native payloads never cross
   the adapter boundary.

## Common Capability Model

Adapters report capabilities rather than a single supported or unsupported value.

```text
marketplace.observe
marketplace.add
marketplace.update
marketplace.remove

plugin.observe
plugin.install
plugin.update
plugin.remove
plugin.enable
plugin.disable

skill.observe
skill.install
skill.update
skill.remove

instructions.observe
instructions.bridge

component.skill
component.mcp
component.hook
component.agent
component.app
component.connector
component.lsp
component.command
component.output_style
component.theme
component.monitor
component.executable
component.settings
```

Capabilities may vary by harness version and scope. A compiled verified profile
is the mutation allowlist. Runtime probes may preserve or narrow its support;
they never grant an undocumented capability or widen an unverified or
unsupported capability.

## Scope Mapping

skilltap has three scope selections:

```text
global
project(<canonical path>)
all-scopes
```

`all-scopes` expands into global scope plus every project recorded in inventory. Adapters receive one concrete global or project scope at a time.

Global resources are personal and available across projects. Project scope applies to one canonical project root.

A logical `ResourceId` plus one concrete scope forms the exact `ResourceKey`.
The same logical identifier may coexist globally and in multiple projects;
dependencies, observations, state records, and mutation selectors name the
exact key rather than inferring scope from the identifier.

skilltap does not write its own metadata into the project. Native harness files, skill directories, `AGENTS.md`, and `CLAUDE.md` may be created or changed when they are the documented representation of the desired resource.

Project-scoped mutations list every affected project file before application.

## Global Instructions

The canonical global instruction file is:

```text
~/AGENTS.md
```

Harness-native global instruction files bridge to the canonical file when the harness does not load it directly:

```text
${CODEX_HOME:-$HOME/.codex}/AGENTS.md -> ~/AGENTS.md
~/.claude/CLAUDE.md -> ../AGENTS.md
```

Claude import mode uses a managed `~/.claude/CLAUDE.md` containing:

```markdown
@~/AGENTS.md
```

Existing user-authored native instruction files are conflicts until their content is reconciled explicitly.

## Expanded Target Set

The intended direct adapter set, in addition to Codex and Claude Code, is:

```text
Factory Droid
Qwen Code
GitHub Copilot CLI
Gemini CLI
Junie
Kimi Code CLI
OpenCode
Kilo Code
Mistral Vibe
Kiro CLI
Amp
```

Each direct adapter must provide documented global and project skill roots,
global and project MCP configuration, effective-state observation, a verified
version profile, ownership-safe update and removal, and the common acceptance
tests described under Adding Another Harness. Native marketplace and plugin
lifecycle capabilities are preferred where available but are not admission
requirements.

Cursor, Zoo Code, and ZCode are boundary-validation candidates. They remain
observe-only until isolated validation identifies their exact supported global
and project write files, verifies reload and precedence, and establishes that
skilltap does not need to mutate an editor or extension cache.

Pi is a conditional compound target. A mutable Pi profile requires the Pi
runtime plus compatible user-installed MCP and Claude Code hook-compatibility
extensions. The adapter observes the core runtime and each companion extension
separately, reports missing or incompatible companions as capability-specific
health findings, remains observe-only until both companions are healthy, and
never attributes extension behavior to Pi core. Pre-existing companion
extensions remain user-owned and are not adopted implicitly.

Goose, Windsurf/Devin Desktop Cascade, and Cline remain outside the target set
until they document ambient project-scoped MCP configuration. Roo Code remains
excluded because it is retired.

## Codex Contract

### Detection

The Codex adapter locates the configured `codex` binary and reads its version.

The adapter selects a verified compiled profile for exact Codex version
`0.144.1` and
scope. Help and JSON probes may narrow that profile before use. Native plugin
installation is available only when the profile includes the operation and the
installed CLI still exposes:

```text
codex plugin add
codex plugin list --json
codex plugin remove
```

Marketplace lifecycle is available only when the installed CLI exposes:

```text
codex plugin marketplace add
codex plugin marketplace list --json
codex plugin marketplace upgrade
codex plugin marketplace remove
```

If an installed Codex release lacks one of these commands, the corresponding capability is unavailable. skilltap does not substitute cache mutation.

This profile does not grant plugin update: Codex `0.144.1` exposes add, list,
and remove but no plugin update command. Marketplace upgrade is a distinct
catalog-source operation.

### Native paths

Codex uses documented locations including:

```text
${CODEX_HOME:-$HOME/.codex}/config.toml
${CODEX_HOME:-$HOME/.codex}/AGENTS.md
~/.agents/skills/<skill>/SKILL.md
~/.agents/plugins/marketplace.json
${CODEX_HOME:-$HOME/.codex}/plugins/
${CODEX_HOME:-$HOME/.codex}/plugins/cache/
```

Project resources use documented locations including:

```text
<project>/AGENTS.md
<project>/.agents/skills/<skill>/SKILL.md
<project>/.agents/plugins/marketplace.json
<project>/.codex/config.toml
```

Nested `AGENTS.md` and `.agents/skills/` directories may apply below the project root according to Codex discovery rules.

### Marketplaces

Global marketplace registration uses the native marketplace CLI when available.

Codex also discovers personal and repository marketplace files. skilltap may write a managed marketplace entry only when the native CLI lacks the required project-scoped lifecycle and the documented marketplace file is the supported representation.

Marketplace files are validated before replacement. Unknown entries are preserved.

Codex marketplace catalogs are not translated from Claude catalogs automatically. A source must contain a compatible Codex marketplace or participate in explicit plugin materialization.

### Plugins

A Codex plugin contains `.codex-plugin/plugin.json`.

Documented Codex plugin components include skills, hooks, apps and connectors, MCP servers, and assets.

Native global installation uses `codex plugin add` only when the verified
compiled profile grants that scoped operation and runtime evidence has not
narrowed it.

When Codex lacks an explicit project-scoped install operation, skilltap may
own a project marketplace registration and project complete plugin skills into
`<project>/.agents/skills/` plus portable MCP definitions into
`<project>/.codex/config.toml`. Those effective destinations are verified
fresh after mutation. A copied plugin directory is not installation evidence;
the marketplace registration and each projected load surface retain separate
ownership evidence.

Plugin caches are read only for observation, provenance, and drift detection.

### Skills

Global standalone skills use `~/.agents/skills/<skill>/`. Project standalone skills use `<project>/.agents/skills/<skill>/`.

The complete directory is the skill. `SKILL.md` is its required entry point.

When the canonical managed skill already occupies the Codex load path, no additional copy or symlink is created.

### Instructions

Codex's native global instruction path is `${CODEX_HOME:-$HOME/.codex}/AGENTS.md`. skilltap bridges that path to canonical `~/AGENTS.md`; changing `CODEX_HOME` never relocates the canonical file.

Project and nested instructions use `AGENTS.md` at their natural directory scope and require no Codex-specific bridge.

### Configuration editing

User configuration lives in `${CODEX_HOME:-$HOME/.codex}/config.toml`. Project configuration may live in `<project>/.codex/config.toml`.

Direct TOML editing is used only for documented settings without a native lifecycle command. Unknown keys and tables are preserved.

## Claude Code Contract

### Detection

The Claude adapter locates the configured `claude` binary and reads its version.

Native plugin lifecycle is available only when the verified compiled profile
for Claude Code `2.1.201` and
the exact Claude version and scope includes the operation and runtime evidence
has not narrowed it:

```text
claude plugin install
claude plugin list --json
claude plugin update
claude plugin uninstall
claude plugin enable
claude plugin disable
```

Claude marketplace and plugin list commands accept `--json` but not `--scope`.
Marketplace update also omits `--scope`; project selection is supplied by the
bounded working directory. Marketplace add/remove and plugin mutations receive
the exact `user` or `local` scope supported by their operation.

Native marketplace lifecycle follows the same compiled-authority and
narrowing-only rule for:

```text
claude plugin marketplace add
claude plugin marketplace list --json
claude plugin marketplace update
claude plugin marketplace remove
```

### Native paths

Claude Code uses documented locations including:

```text
~/.claude/settings.json
~/.claude/CLAUDE.md
~/.claude/skills/<skill>/SKILL.md
~/.claude/plugins/known_marketplaces.json
~/.claude/plugins/cache/
```

Project and local configuration use:

```text
<project>/.claude/settings.json
<project>/.claude/settings.local.json
<project>/CLAUDE.md
<project>/.claude/CLAUDE.md
<project>/.claude/skills/<skill>/SKILL.md
```

### Native scope mapping

skilltap global scope maps to Claude's `user` scope.

skilltap project scope maps to Claude's `local` scope for marketplace and plugin lifecycle. This keeps personal skilltap state from silently creating team-wide plugin requirements in `.claude/settings.json`.

A user may independently maintain project-shared Claude configuration. skilltap
observes it as declared state and reports its effect, but the current CLI has no
shared-scope adoption selector and never adopts it into personal project scope.

### Marketplaces

Native marketplace registration invokes Claude's marketplace lifecycle with `--scope user` for global scope and `--scope local` for project scope.

Marketplace update, removal, and JSON listing use native commands.

Claude marketplace catalogs are not translated into Codex marketplace catalogs. Individual plugins may still be evaluated for materialization.

### Plugins

A Claude plugin may contain `.claude-plugin/plugin.json` and convention-based component directories.

Supported Claude plugin components include skills, commands, agents, hooks, MCP servers, LSP servers, output styles, themes, monitors, executables, and plugin settings.

Global plugin lifecycle uses Claude's `user` scope. Personal project plugin lifecycle uses Claude's `local` scope.

Claude's plugin cache is observed but never written directly.

### Skills

Global standalone skills use `~/.claude/skills/<skill>/`. Project standalone skills use `<project>/.claude/skills/<skill>/`.

Claude treats the complete directory as the skill and `SKILL.md` as its entry point.

When skilltap's canonical `.agents/skills/` directory is not a native Claude load path for the installed version, the Claude adapter links or copies the complete skill directory into `.claude/skills/`.

### Instructions

Claude's native global instruction path is `~/.claude/CLAUDE.md`. skilltap bridges that path to canonical `~/AGENTS.md`.

Project Claude instructions use `CLAUDE.md` or `.claude/CLAUDE.md` according to the managed bridge location.

Claude supports two faithful bridges: a `CLAUDE.md` symlink targeting `AGENTS.md`, or a managed `CLAUDE.md` importing the canonical file.

## Standalone Skill Contract

A portable skill follows the Agent Skills directory model:

```text
<skill>/
├── SKILL.md
└── supporting files and directories
```

skilltap validates standard frontmatter fields including `name`, `description`, `license`, `compatibility`, `metadata`, and `allowed-tools`.

Harness-specific fields are preserved.

The standard `compatibility` value and `metadata` mapping are evidence, not an automatic guarantee. Adapter-specific fields, variables, tool names, and supporting files may narrow compatibility.

## Cross-Harness Component Matrix

| Source component | Codex target | Claude target | Default classification |
|---|---|---|---|
| Standard skill directory | Native or linked | Native or linked | Faithful when compatibility passes |
| MCP server | Native plugin MCP | Native plugin MCP | Conditional on transport, auth, variables, and config semantics |
| Hook | Native hook | Native hook | Conditional on equivalent event and payload semantics |
| Claude command | Materialized skill | Native legacy component | Conditional; must preserve invocation behavior |
| Claude agent | No default plugin equivalent | Native | Unsupported for Codex until a faithful adapter exists |
| Claude LSP server | No default plugin equivalent | Native | Unsupported for Codex |
| Claude output style | No equivalent | Native | Target-specific |
| Claude theme | No equivalent | Native | Target-specific |
| Claude monitor | No equivalent | Native | Target-specific |
| Claude plugin executable | No assumed equivalent | Native | Target-specific unless explicitly referenced by a portable skill |
| Claude plugin settings | No generic equivalent | Native | Target-specific |
| Codex app or connector | Native | No generic equivalent | Unsupported unless reducible to a faithful MCP server |
| Codex asset metadata | Native | Not behavior-bearing | May be omitted only when it does not affect operation |

“Conditional” means skilltap analyzes the concrete component. It does not assume portability from the component type alone.

## MCP Mapping

An MCP server maps faithfully only when both harnesses support the same transport, command or URL, argument and environment handling, authentication assumptions, path-variable substitution, enablement, and tool-filter semantics.

Connector identity, hosted authorization, or custom UI is not reduced to plain MCP configuration unless the resulting behavior is demonstrably equivalent.

## Hook Mapping

Hooks map only when source and target lifecycle events have equivalent timing, payload, failure behavior, working directory, environment, and permission semantics.

Similar event names are insufficient evidence.

An unsupported optional hook may be omitted only through acknowledged partial materialization. An unsupported required hook blocks the plugin.

## Marketplace Identity

A marketplace's stable native lineage consists of its harness, native
marketplace name, normalized declared source and requested selector, and exact
scope. A resolved revision is mutable observation evidence, not identity.

Two marketplaces with similar names are not coalesced unless their normalized sources and identities match.

## Plugin Identity

A native plugin's stable lineage consists of its harness-native qualified name,
marketplace lineage, and exact scope. Its resolved version, revision, and
fingerprint are mutable observations, not identity.

Cross-harness entries are associated only when they originate from the same
declared source with compatible semantics or the user records an explicit
mapping. Matching names, similar URLs, equal fingerprints, or compatible
resolved versions alone do not establish identity.

## Version and Update Contract

Native plugin versions come from native structured output and marketplace metadata.

Git-backed resources store the requested ref and resolved commit SHA. An update is available when the same requested selector resolves to a different version or commit. A pinned commit SHA remains fixed.

A changed plugin manifest or skill tree is re-evaluated for compatibility before update application.

Unknown native version formats are preserved as opaque values and compared only through the owning adapter.

## Unknown Harness Versions

An unknown harness version may be observed when its structured output and documented files remain parseable.

Mutation requires a verified compiled capability profile for the exact version
and scope. Runtime probes may narrow that authority but cannot create it.

If verification fails, status remains available and mutation is blocked with a harness-contract error.

## Adding Another Harness

A harness is not supported until its adapter can provide reliable installation
detection, stable observation, explicit global and project scope behavior,
faithful complete-directory skill loading, MCP configuration and load
observation, update identity, complete fixture-based contract tests, and clear
unsupported-component reporting.

Native marketplace, plugin lifecycle, hooks, instructions, agents, and other
extension components are optional capabilities. When native lifecycle is
absent, skilltap may own acquisition, managed projection, update, drift, and
removal through documented skill and MCP load paths. Filesystem copying without
a supported load contract, effective-state observation, ownership tracking,
and idempotent reconciliation is not a harness integration.

## Authoritative References

- [Codex plugins and marketplace behavior](https://developers.openai.com/codex/plugins/build)
- [Codex customization, skills, and AGENTS.md](https://developers.openai.com/codex/concepts/customization)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Claude Code plugin reference](https://code.claude.com/docs/en/plugins-reference)
- [Claude Code marketplace lifecycle](https://code.claude.com/docs/en/plugin-marketplaces)
- [Claude Code settings and scopes](https://code.claude.com/docs/en/settings)
- [Claude Code instructions and AGENTS.md bridging](https://code.claude.com/docs/en/memory)
- [Claude Code skills](https://code.claude.com/docs/en/slash-commands)
- [Agent Skills specification](https://agentskills.io/specification)
