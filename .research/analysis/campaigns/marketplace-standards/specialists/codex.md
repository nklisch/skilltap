---
provenance: agent-synthesis
updated: 2026-07-10
status: current
temporal_contract: supersedes-prior
facet: codex-native-contracts
---

# Codex native extension contracts

## Finding

Codex exposes two distinct native management planes. Local skills and
instructions are discovered from documented filesystem locations, while
distributed plugins are installed from marketplace catalogs and carry their own
manifest, cache, enablement, component, and policy lifecycle. Codex CLI and the
IDE share layered TOML configuration. [codex-skills]{3}
[codex-agents-md]{4} [codex-build-plugins]{1} [codex-config]{5}

### Plugins and marketplaces

A plugin is an installable bundle whose documented components include skills,
apps or connectors, MCP servers, hooks, and other workflow capabilities. New
sessions are required before newly installed skills or tools become available.
[codex-plugins]{2}

Every package has `.codex-plugin/plugin.json` as its entry point. Component
directories and files remain at the plugin root, and manifest paths are
root-relative `./` paths contained by that root. [codex-build-plugins]{1}

The canonical local catalogs are
`$REPO_ROOT/.agents/plugins/marketplace.json` and
`~/.agents/plugins/marketplace.json`. Catalog entries can resolve local, Git
repository, Git subdirectory, or npm sources; Git sources can select a ref or
SHA, and npm sources can select a version, range, or tag. The desktop host
installs a versioned copy under
`~/.codex/plugins/cache/$MARKETPLACE_NAME/$PLUGIN_NAME/$VERSION/` and stores
plugin enablement in `~/.codex/config.toml`. [codex-build-plugins]{1}

The documented non-interactive marketplace source lifecycle uses [codex-build-plugins]{1}:
`codex plugin marketplace add`, `list`, `upgrade`, and `remove`. `add` accepts
GitHub shorthand, Git URLs, SSH URLs, or a local root, and supports Git ref and
sparse-checkout controls. [codex-build-plugins]{1}

The documented end-user plugin install surface in Codex CLI is the interactive
`/plugins` browser, which supports install, uninstall, enable, and disable.
[codex-plugins]{2}

### Skills

A Codex skill is the complete directory containing top-level `SKILL.md`, not
the Markdown file alone. `SKILL.md` requires `name` and `description`; scripts,
references, assets, and Codex metadata are optional sibling content.
[codex-skills]{3}

Repository skills are discovered in `.agents/skills` directories from the
working directory upward to the repository root; user skills live in
`$HOME/.agents/skills`; administrator skills live in `/etc/codex/skills`.
Codex follows symlinked skill directories and does not merge same-named skills.
[codex-skills]{3}

Codex automatically detects filesystem skill changes, with restart as the
documented fallback. A skill can be disabled without deletion using a
`[[skills.config]]` entry in `~/.codex/config.toml`, addressed by its
`SKILL.md` path. [codex-skills]{3}

### Instructions and configuration

Codex's documented global instruction location is the Codex home directory:
`~/.codex/AGENTS.override.md` wins when non-empty, otherwise
`~/.codex/AGENTS.md` is read. `CODEX_HOME` can relocate that scope.
[codex-agents-md]{4}

Project instructions are selected one file per directory from repository root
through the working directory, preferring `AGENTS.override.md`, then
`AGENTS.md`, then configured fallback names. Root-to-leaf concatenation makes
nearer guidance later and therefore overriding. [codex-agents-md]{4}

User configuration is `~/.codex/config.toml`; trusted projects can layer
`.codex/config.toml` from repository root to working directory. CLI overrides
have highest precedence, then closest project config, selected profile, user,
system, and built-in defaults. Untrusted projects do not load project `.codex`
configuration, hooks, or rules. [codex-config]{5}

Plugin-bundled MCP servers can be enabled, disabled, tool-filtered, and assigned
approval behavior through plugin-scoped TOML without changing the plugin.
Administrators can independently constrain permitted marketplace sources and
plugin MCP identities. [codex-config-reference]{6}

## Adapter implications

- {inferred: applies} Treat skills, instructions, marketplace sources, and
  installed plugins as different resource kinds. A marketplace upgrade refreshes
  a catalog source; the fetched documentation does not establish that it also
  upgrades every installed plugin.
- {inferred: recommends} Use `codex plugin marketplace` commands for tracked
  source lifecycle. For a local catalog owned by the user, reconcile the JSON
  catalog itself while preserving fields that Codex documents but skilltap does
  not interpret. Never write the plugin cache as an API.
- {inferred: constrains} Do not assume a non-interactive plugin-install command
  from the fetched docs. Probe the installed Codex capability and version; use a
  native install command only when the host exposes a suitable deterministic
  interface. Otherwise report the interactive-only native surface or plan an
  explicitly acknowledged faithful materialization.
- {inferred: applies} Manage standalone skills as whole directories in
  `$HOME/.agents/skills` or the applicable repository `.agents/skills`; symlink
  adapters are native-compatible for Codex.
- {inferred: applies} A machine-wide canonical `~/AGENTS.md` is not itself a
  documented Codex global location. To implement that policy faithfully, bridge
  `~/.codex/AGENTS.md` to the canonical file and report an existing non-empty
  `AGENTS.override.md` as the effective higher-priority source.
- {inferred: constrains} Preserve unknown TOML and JSON fields, account for
  project trust before claiming project configuration is effective, and model
  plugin hook trust separately from plugin install and enablement.
- {inferred: recommends} Track both catalog provenance (Git ref/SHA or npm
  selector) and installed plugin manifest version. Those values represent
  different lifecycle points in the documented cache-and-catalog model.

## Unknowns

- {ambiguous: official install surface} The fetched official docs show
  interactive CLI plugin installation and non-interactive marketplace source
  management, but do not document a non-interactive `codex plugin install` or
  uninstall command.
- {ambiguous: installed update semantics} `marketplace upgrade` is documented
  as refreshing configured marketplaces; automatic reinstallation or upgrade of
  already cached plugins is not specified.
- {ambiguous: enablement schema} The docs state that enablement is stored in
  `~/.codex/config.toml` but the fetched sources do not give the exact top-level
  plugin enablement key.
- {ambiguous: machine output} Structured output or stable exit-code contracts
  for marketplace and plugin lifecycle commands are not described in the
  fetched sources.
- {ambiguous: marketplace schema evolution} The marketplace and manifest pages
  document fields and path rules but do not publish a schema-version or unknown
  field round-trip contract.

## Disconfirming analysis

Official-site searches were run for a non-interactive `codex plugin install`,
plugin update behavior, and a direct `~/AGENTS.md` global instruction source.
No fetched official page displaced the attested model: the plugin user guide
documents `/plugins`; the build guide documents non-interactive marketplace
source commands; the instruction guide locates global guidance under Codex
home. This is evidence of a documentation gap, not proof that unlisted commands
cannot exist in a particular Codex release. {confidence: current-public-docs}

The configuration reference was also checked against the build guide. It
supports plugin-scoped MCP policy and administrator marketplace restrictions,
but does not establish a stable direct-write contract for plugin installation
state. [codex-config-reference]{6}

## Contradictions

No direct contradiction was found among the attested OpenAI sources.

Two qualifications matter:

- The build guide calls `.agents/plugins/marketplace.json` canonical while also
  accepting a repository `.claude-plugin/marketplace.json` as
  legacy-compatible. That compatibility read path does not establish semantic
  equivalence between all Codex and Claude plugin fields.
  [codex-build-plugins]{1}
- The skill guide says filesystem changes are detected automatically, while
  config changes and plugin installs call for restart or a new session. These
  statements concern different activation paths rather than conflicting update
  guarantees. [codex-skills]{3} [codex-plugins]{2}

## Revisit if

- OpenAI documents a non-interactive plugin install, uninstall, or upgrade
  command with structured output.
- The plugin or marketplace manifests gain a published schema/version contract.
- Codex changes its user skill, global instruction, marketplace, cache, or
  configuration locations.
- Marketplace upgrade semantics are documented to include installed plugin
  upgrades.

## Bibliography mapping

1. `codex-build-plugins` — https://learn.chatgpt.com/docs/build-plugins
2. `codex-plugins` — https://learn.chatgpt.com/docs/plugins
3. `codex-skills` — https://learn.chatgpt.com/docs/build-skills
4. `codex-agents-md` — https://learn.chatgpt.com/docs/agent-configuration/agents-md
5. `codex-config` — https://learn.chatgpt.com/docs/config-file/config-basic
6. `codex-config-reference` — https://learn.chatgpt.com/docs/config-file/config-reference

Numbers 7-9 remain unassigned.
