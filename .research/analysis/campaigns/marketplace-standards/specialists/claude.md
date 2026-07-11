---
provenance: agent-synthesis
updated: 2026-07-10
facet: claude-code-native-contracts
temporal_contract: supersedes-prior
---

# Claude Code native extension contracts

## Source map

| Number | Handle | URL |
|---:|---|---|
| 10 | `claude-plugins-reference` | `https://code.claude.com/docs/en/plugins-reference` |
| 11 | `claude-plugin-marketplaces` | `https://code.claude.com/docs/en/plugin-marketplaces` |
| 12 | `claude-skills` | `https://code.claude.com/docs/en/slash-commands` |
| 13 | `claude-settings` | `https://code.claude.com/docs/en/settings` |
| 14 | `claude-memory` | `https://code.claude.com/docs/en/memory` |

## Native plugin contract

A Claude Code plugin is a self-contained directory. Default components live at the plugin root: skill directories, flat legacy commands, agents, hooks, MCP and LSP definitions, output styles, themes, monitors, executables, scripts, and limited default settings. `.claude-plugin/` is reserved for the optional `plugin.json` manifest; component directories do not belong beneath it. A plugin-root `CLAUDE.md` is not loaded as project context. [claude-plugins-reference]{10}

When `plugin.json` exists, `name` is its sole required field. Without a manifest, Claude Code derives the plugin name from the directory and discovers default component locations. The manifest can redirect component paths and carry metadata, dependency, and default-enablement fields; unknown top-level fields warn but do not prevent loading unless strict validation is requested. [claude-plugins-reference]{10}

Marketplace-installed plugins are copied into version-specific directories under Claude's plugin cache rather than executed from their source checkout. Paths cannot escape the installed plugin root. Native cache contents are therefore runtime artifacts, not a supported authoring or state-mutation surface. [claude-plugins-reference]{10}

Claude also recognizes a skills-directory plugin: a folder under a personal or project `.claude/skills/` tree containing `.claude-plugin/plugin.json`. It loads in place as `<name>@skills-dir`, without a marketplace or install step. Project-scope instances require workspace trust and have additional limits for executable components. [claude-plugins-reference]{10}

## Marketplace and lifecycle contract

A native marketplace is a catalog rooted at `.claude-plugin/marketplace.json`. Its required fields are `name`, `owner`, and `plugins`; every plugin entry requires a public plugin `name` and a `source`. Sources may identify relative content, GitHub or other git repositories, git subdirectories, npm packages, or supported settings declarations. Relative paths resolve from the marketplace root and cannot traverse above it. [claude-plugin-marketplaces]{11}

Claude Code exposes non-interactive native operations for marketplace add/list/update/remove and plugin install/uninstall/enable/disable/update/list. Marketplace and plugin list operations support JSON output; lifecycle mutations accept explicit user, project, or local scope where applicable. Removing a marketplace also uninstalls plugins installed from it. [claude-plugin-marketplaces]{11} [claude-plugins-reference]{10}

Update availability is determined by the resolved plugin version, in priority order: `plugin.json` version, marketplace-entry version, then git commit SHA for supported git-derived sources. An unchanged explicit version suppresses updates even when the repository changed; omitting both explicit versions makes a new source commit a new plugin version. [claude-plugins-reference]{10} [claude-plugin-marketplaces]{11}

The marketplace settings contract includes per-marketplace automatic update. Official Anthropic marketplaces default to auto-update enabled and others default to disabled when the setting is omitted. Managed allow/block policy is checked before marketplace/plugin network or filesystem work. [claude-settings]{13}

## Scope and configuration contract

Plugin installation scopes map to native settings files: user to `~/.claude/settings.json`, project to `.claude/settings.json`, local to `.claude/settings.local.json`, and managed to read-only managed settings. The default install/update scope is user. [claude-plugins-reference]{10}

Plugin state is represented in `enabledPlugins` as `plugin-name@marketplace-name: true|false`. `extraKnownMarketplaces` declares marketplace sources. A project declaration is shared configuration, but each collaborator still passes native trust and installation consent; a repository entry alone does not silently install external code for other users. [claude-settings]{13}

General settings precedence is managed, command-line, local, project, then user. This makes local settings the native per-machine override for repository-owned project settings, while managed policy remains non-overridable. [claude-settings]{13}

## Standalone skill contract

A standalone skill is the complete folder containing its required top-level `SKILL.md`; optional scripts, references, templates, and examples live beside it as supporting material. Native locations are `~/.claude/skills/<name>/SKILL.md` for personal scope and `.claude/skills/<name>/SKILL.md` for project scope. Plugin skills live at `<plugin>/skills/<name>/SKILL.md`. [claude-skills]{12}

Personal and project skill entries may be symlinks to complete directories. Claude watches existing skill trees for `SKILL.md` changes during a session, while changes to other plugin components require `/reload-plugins` or a restart. [claude-skills]{12}

Claude's `SKILL.md` frontmatter is an extension surface, not merely portable metadata. The documented optional fields include invocation controls, named arguments, tool allow/deny lists, model and effort overrides, forked subagent context, agent selection, and lifecycle hooks. Cross-harness transfer must therefore classify these fields rather than assume another harness implements them. [claude-skills]{12}

## Instruction contract

Claude Code's native global instruction path is `~/.claude/CLAUDE.md`; project instructions may be `./CLAUDE.md` or `./.claude/CLAUDE.md`, with `CLAUDE.local.md` for private project-local instructions. Files above the working directory load at startup and subordinate files load when Claude accesses their directories. [claude-memory]{14}

Claude Code does not read `AGENTS.md` directly. Anthropic documents two bridges: a `CLAUDE.md` file that imports `AGENTS.md` with `@path` syntax, or a symlink when no Claude-specific content is required. [claude-memory]{14}

## Adapter implications

The following are design inferences rather than native-source claims:

- {inferred: preserves} A Claude adapter should invoke `claude plugin marketplace ...` and `claude plugin ...` for native marketplace/plugin lifecycle, then observe state through JSON-capable list commands. Editing the cache would bypass the documented ownership and version model.
- {inferred: distinguishes} The adapter needs separate resource kinds for marketplace-installed plugins, skills-directory plugins, and plain skills because their ownership, caching, trust, and removal semantics differ.
- {inferred: models} Plugin identity should preserve both plugin name and marketplace name. A source URL alone cannot replace the qualified identity used by `enabledPlugins` and lifecycle commands.
- {inferred: tracks} Update provenance should retain the native resolved-version basis—manifest version, marketplace version, or commit SHA—so a plan can explain why upstream repository movement does or does not constitute an available update.
- {inferred: avoids} Project settings should be treated as declarations that may prompt collaborators, not proof that a plugin is installed on the current machine.
- {inferred: validates} A portable skill transfer should copy or link the entire skill directory and inspect Claude-only frontmatter fields. Unsupported behavioral fields must produce a compatibility result rather than being silently discarded.
- {inferred: bridges} If `~/AGENTS.md` is the control plane's canonical global instruction file, the faithful Claude bridge is `~/.claude/CLAUDE.md` as a symlink or import shim. The latter is required when Claude-specific additions must coexist.
- {inferred: defers} Trust prompts and managed marketplace restrictions are native policy boundaries. An adapter should report them as blocked or pending native consent, not work around them by materializing cache files.

## Disconfirming analysis

| Load-bearing proposition tested | Disconfirming search | Outcome |
|---|---|---|
| Plugins require a manifest | The plugin reference explicitly documents manifest-free auto-discovery. | Rejected; the brief states `plugin.json` is optional. [claude-plugins-reference]{10} |
| Project `enabledPlugins` means installation is complete | The settings reference says external project declarations do not install for other users and retain trust/consent prompts. | Rejected; project state is a shared declaration, not machine installation evidence. [claude-settings]{13} |
| Repository movement always creates an update | Both version references state that explicit unchanged versions suppress updates. | Rejected; update tracking must retain resolved-version basis. [claude-plugins-reference]{10} [claude-plugin-marketplaces]{11} |
| A plugin may reuse arbitrary parent files | Cache/path rules prohibit traversal outside the copied plugin root. | Rejected; installed plugins must be self-contained after permitted symlink handling. [claude-plugins-reference]{10} |
| `AGENTS.md` is natively read by Claude | The memory reference expressly says Claude reads `CLAUDE.md`, not `AGENTS.md`. | Rejected; a symlink or import bridge is necessary. [claude-memory]{14} |
| A skill can be represented by `SKILL.md` alone | The skills guide calls `SKILL.md` required but explicitly supports adjacent scripts, references, templates, and examples. | Rejected; the directory is the managed resource boundary. [claude-skills]{12} |

## Contradictions

No direct source contradiction was found among the attested Anthropic pages.

One qualification matters operationally: the plugin reference says project-scope install writes `enabledPlugins` and makes the plugin available to repository collaborators, while the settings reference clarifies that external project declarations do not install automatically and each user still receives trust/install prompts. Relationship: `qualifies`, handles `claude-plugins-reference` and `claude-settings`. The shared file expresses desired availability; observed machine installation remains separate. [claude-plugins-reference]{10} [claude-settings]{13}

The general memory hierarchy and plugin packaging rules address different frames: a project `CLAUDE.md` can load from the working tree, but a `CLAUDE.md` bundled at a marketplace plugin root is not loaded as plugin context. Relationship: `incommensurable`, handles `claude-memory` and `claude-plugins-reference`; project traversal and cached plugin component discovery are distinct loaders. [claude-memory]{14} [claude-plugins-reference]{10}

## Unknowns

- {ambiguous: native-list-output-schema} The documentation confirms JSON output for marketplace and plugin lists but does not provide a versioned JSON schema. Adapter parsers should validate observed output and preserve unknown fields.
- {ambiguous: noninteractive-trust-completion} The documentation establishes trust/consent prompts but does not define a general automation flag that grants project plugin trust. A deterministic controller should stop and report required native consent unless a separately documented native mechanism applies.
- {ambiguous: global-AGENTS-symlink} Anthropic explicitly demonstrates an `AGENTS.md` symlink bridge at project scope and separately defines `~/.claude/CLAUDE.md` as user scope. Using `~/.claude/CLAUDE.md -> ~/AGENTS.md` is a direct composition of those contracts, but the exact global example is not stated verbatim.

## Revisit if

- Anthropic publishes versioned JSON schemas for plugin or marketplace list output.
- Claude Code changes the `@skills-dir` plugin mechanism or its trust boundaries.
- `AGENTS.md` becomes a directly loaded Claude instruction filename.
- Plugin lifecycle adds a documented non-interactive trust/consent mechanism.
- Manifest, marketplace, or `SKILL.md` schemas change in a way that affects resource identity or update detection.

## Acquisition candidates

None. The load-bearing contract claims were available in current official primary documentation.
