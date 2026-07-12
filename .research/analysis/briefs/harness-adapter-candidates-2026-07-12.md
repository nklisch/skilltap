---
provenance: agent-synthesis
status: current
updated: 2026-07-12
temporal_contract: re-engage-on-trigger
scope: terminal coding/agent harnesses with extension, plugin, skill, and instruction surfaces
---

# Harness adapter candidates

## Decision

{inferred: candidate decision} Three additional harnesses meet the full
skilltap adapter bar today:

1. **Factory Droid** — native plugin marketplaces and lifecycle commands,
   user/project scopes, complete skills, AGENTS.md, and Git-commit update
   semantics.
2. **Qwen Code** — native marketplace-source and extension lifecycle commands,
   user/project scopes, complete skills, AGENTS.md, and direct Claude/Gemini
   marketplace compatibility.
3. **GitHub Copilot CLI** — native plugin marketplaces and lifecycle commands,
   user/repository configuration, complete skills, AGENTS.md, and Claude
   marketplace compatibility.

{inferred: bounded candidate} **Pi** is a viable adapter for the portable
skill/package plane, but not a full marketplace-parity candidate: it has a
native npm/Git package manager and package catalog, yet no user-registered
marketplace source primitive. Include it only if skilltap models Pi resources
as packages/sources rather than pretending its package catalog is a native
marketplace.

Gemini CLI has enough extension and skill primitives for a future bounded
adapter, but it currently lacks a native custom-marketplace registration and
skill-update protocol. Junie, Kimi Code, OpenCode, Goose, Kilo Code, Mistral
Vibe, and Kiro fail at least one hard gate below and should not be presented as
supported candidates in the first expansion.

## Gate used

An adapter is a full candidate only when the harness exposes all of these in a
faithful, observable way:

- a native or directly inspectable source/marketplace registration model;
- deterministic install, update, remove, list, and enable/disable operations
  where the harness supports enablement;
- complete skill directories with top-level `SKILL.md` and sibling resources;
- explicit global/user and project/workspace scopes;
- inspectable instruction/configuration state, including AGENTS.md or a
  documented equivalent;
- lifecycle operations that do not require skilltap to mutate an opaque cache;
- enough output/state to plan, reconcile, and report partial compatibility.

The gate is intentionally stricter than “the agent can read a skill.” A
harness that only supports copying files, an interactive marketplace screen,
or startup-time package resolution is not a full candidate.

## Candidate matrix

| Harness | Marketplace/source lifecycle | Plugin/package lifecycle | Skills | Instructions | Cache/state boundary | Verdict |
|---|---|---|---|---|---|---|
| Factory Droid | Native marketplace add/list/update/remove | Native `droid plugin install/uninstall/update/list`, enablement, user/project scope | Complete `SKILL.md` directories, personal/workspace roots | `AGENTS.md`, including `~/.factory/AGENTS.md` | Native CLI owns cached content; skilltap must not edit it | **Full candidate** |
| Qwen Code | Native marketplace sources add/list/update/remove; Claude and Gemini sources | Native install/uninstall/enable/disable/update, `--scope project`, `--all` | Complete user/project/extension skills | `AGENTS.md` plus QWEN.md compatibility | Native extension directory and CLI lifecycle | **Full candidate** |
| GitHub Copilot CLI | Native marketplace add/list/browse/remove; declarative known marketplaces | Native install/uninstall/update/list/enable/disable | Complete project/personal skills and `.agents/skills` | `AGENTS.md`, plus CLAUDE/GEMINI roots | Installed plugins and marketplace caches are documented; native CLI owns them | **Full candidate** |
| Pi | npm/Git/local package sources and public package catalog; no registered marketplace primitive | Native install/remove/list/update, global/project settings | Agent Skills, `.agents/skills`, package skills | `AGENTS.md` and `CLAUDE.md` discovery | Inspectable settings plus package checkouts | **Bounded package candidate** |
| Gemini CLI | Direct Git/local extension sources and web gallery; no custom marketplace registration | Native extension install/update/uninstall/list/enable/disable | User/workspace `.agents/skills`, extension skills; direct skill install/link/uninstall but no documented skill update | `GEMINI.md`, user/project settings | Native extension copy and update command | **Not full candidate** |
| Junie | Native/Claude marketplace manifests and user/project state | Install/update/remove are documented through `/extensions` UI; content is cached | Extension skills and skill locations | Project/user configuration | Cache is documented, but deterministic shell lifecycle is not | **Not candidate** |
| Kimi Code CLI | Official/third-party/custom marketplace JSON | Slash/TUI lifecycle; Git/commit sources; user-only installs | Plugin `SKILL.md` directories | Separate configuration system | Managed copy; project scope not supported | **Not candidate** |
| OpenCode | No marketplace registration; npm/config plugin sources | One install command and startup Bun resolution; no full native update/remove/list lifecycle | Excellent `.agents/skills` and Claude-compatible discovery | Config instructions | Startup dependency cache is not a write API | **Not candidate** |
| Goose | Web skills marketplace and external `npx skills` flow | No native skill/plugin lifecycle documented | `.agents/skills` and plugin skills | Goose config/context files | External manager and plugin model | **Not candidate** |
| Kilo Code | Marketplace UI and remote `index.json` skill URLs | No deterministic native skill install/update/remove CLI documented | `.kilo/skills` and `.agents/skills` | Kilo config/instructions | UI/remote index, no reconcile-friendly lifecycle | **Not candidate** |
| Mistral Vibe | No marketplace/plugin lifecycle in CLI docs | Configurable skill paths and filters only | `.vibe/skills`, `.agents/skills`, `SKILL.md` | Vibe config/instructions | Plain config, no package provenance/update protocol | **Not candidate** |
| Kiro CLI | IDE/web-only Powers; no CLI package lifecycle | CLI has MCP/config commands, not Powers/skills management | Powers use `POWER.md`, not the shared `SKILL.md` contract | Kiro steering/agents | IDE-managed installation | **Not candidate** |

## Full candidate findings

### Factory Droid

Factory is a direct fit for the native-plugin lane. Its shell commands are
explicitly documented for scripting, and every plugin operation accepts a
scope. The plugin package can carry skills, commands, agents, hooks, MCP, and
Claude-compatible content. Updates resolve the latest marketplace Git commit,
which aligns with skilltap's requirement to observe source revision changes.
[factory-plugins]{1}

Its skill and instruction contracts also line up with the portable control
plane: a skill is a complete directory with `SKILL.md`, workspace and personal
roots are distinct, and `~/.factory/AGENTS.md` is a documented personal
override. [factory-skills]{2} [factory-agents]{3}

**Adapter posture:** call `droid plugin ...` for lifecycle; treat the plugin
cache as native-owned; project standalone skills to `.factory/skills`; bridge
`~/AGENTS.md` to `~/.factory/AGENTS.md`; record commit hashes as observations.

**Compatibility warning:** Factory's plugin manifest and component semantics
are not the Claude manifest even though Claude plugins can be installed. The
adapter must inspect the component set and classify hooks, droids, and MCP as
faithful, materialized, partial, or blocked rather than assuming all Claude
plugins are behaviorally identical.

### Qwen Code

Qwen is a particularly useful bridge harness because its documented extension
manager accepts Claude marketplaces and Gemini extensions directly. It has
native source registration (`sources add|list|update|remove`), extension
install/uninstall/enable/disable/update, and explicit user/project scopes.
Extensions can contain skills, subagents, commands, MCP, and context files;
Git/local installs are copied and then updated through the native command.
[qwen-extensions]{4}

Its standalone skill contract is a complete `SKILL.md` directory in user or
project roots, and Qwen reads existing `AGENTS.md` files alongside its own
instruction files. [qwen-skills]{5} [qwen-memory]{6}

**Adapter posture:** register Qwen marketplace sources natively; use Qwen's
extension commands for native bundles; project portable skills to `.qwen/skills`
or `.agents/skills` where the installed version supports that alias; observe
`AGENTS.md` and `QWEN.md` precedence; preserve the source and resolved revision
in skilltap state.

**Compatibility warning:** Qwen performs format conversion when importing
Claude or Gemini extensions. That is a native transfer, not proof of semantic
equivalence. Inspect the converted manifest and component list, and require
the normal partial acknowledgment for unsupported tool, agent, hook, or MCP
behavior.

### GitHub Copilot CLI

Copilot CLI has a complete native plugin marketplace surface: add/list/browse/
remove marketplaces and install/uninstall/update/list/enable/disable plugins.
It accepts marketplace, GitHub, Git URL, and local sources. Its marketplace
format is recognized in `.github/plugin/` and `.claude-plugin/`, which gives it a
direct compatibility path for Claude-oriented marketplaces. [copilot-plugin-ref]{7}

The plugin model bundles agents, skills, hooks, MCP, and LSP, with a required
root `plugin.json`. The skill contract is a complete directory with
`SKILL.md`, and project/personal roots include `.agents/skills`. Copilot's
instruction surface explicitly loads `AGENTS.md` and supports repository and
personal configuration. [copilot-plugins]{8} [copilot-skills]{9}
[copilot-instructions]{10}

**Adapter posture:** use native plugin and marketplace commands; keep
`~/.copilot/settings.json`, `.github/copilot/settings.json`, and local settings
as declared state; use native CLI lifecycle for installed plugins; use
`copilot skill` for standalone skills; bridge canonical `~/AGENTS.md` into the
documented Copilot instruction roots only when the operation is non-divergent.

**Compatibility warning:** Copilot's manifest is `plugin.json`, not Codex's
`.codex-plugin/plugin.json` or Claude's optional `.claude-plugin/plugin.json`.
Hooks, LSP, custom-agent fields, and enterprise policy can be unsupported in a
target harness; keep those as explicit partial outcomes.

## Bounded package candidate

### Pi

Pi's package manager is unusually compatible with skilltap's portable core. It
installs packages from npm, Git, URLs, and local paths; lists/removes/updates
them; writes explicit global or project settings; and reconciles Git checkouts.
Packages can bundle extensions, skills, prompts, and themes. [pi-packages]{11}

Pi also loads complete Agent Skills from global/project `.agents/skills`, can
point at Claude and Codex skill roots, and discovers `AGENTS.md`/`CLAUDE.md`
context. [pi-skills]{12} [pi-settings]{13}

The missing piece is important: Pi documents a package catalog and package
sources, not a user-registered marketplace source with add/update/remove
semantics. Therefore Pi is safe to support as a **package/source adapter** if
skilltap labels the resource accordingly. It should not be reported as having
Claude/Codex-style marketplace parity until Pi exposes that primitive.

## Near misses and why they are excluded

### Gemini CLI — extension-capable, marketplace-incomplete

Gemini has native extension install/update/uninstall/list/enable/disable and
bundled `skills/<name>/SKILL.md`; its skill utility supports user/workspace
scope and `.agents/skills`. [gemini-extensions]{14} [gemini-skills]{15}
The current public contract does not document custom marketplace registration,
and standalone skill update/provenance is not defined. It can be a future
direct-source adapter, but not a full marketplace candidate under the gate.

### Junie — marketplace-capable, automation-incomplete

Junie supports native and Claude marketplace manifests, user/project
`extensions.json`, and extension update/remove semantics. [junie-extensions]{16}
However, the current
documentation routes these operations through the interactive `/extensions`
manager, while content is stored in a user cache. [junie-extensions]{16}
skilltap cannot safely replace that native control plane with cache writes, so
Junie is deferred until a deterministic shell/API lifecycle is documented.

### Kimi Code CLI — marketplace-capable, scope-incomplete

Kimi has plugin skills, marketplace JSON, commit-addressable Git sources, and
install/list/enable/disable/remove slash commands. Its current docs explicitly
state that plugins are installed per-user and project-level installation is not
supported; the documented management surface is the TUI/slash command path.
[kimi-plugins]{17} That fails skilltap's project/global and non-interactive
requirements.

### OpenCode — skill-compatible, lifecycle-incomplete

OpenCode's skill discovery is attractive: it reads complete `SKILL.md`
directories from OpenCode, Claude, and `.agents/skills` roots. [opencode-skills]{20}
Its plugin CLI only documents `opencode plugin <module>` to install and update
config; npm packages are resolved by Bun at startup and cached under
`~/.cache/opencode`. [opencode-cli]{18} [opencode-plugins]{19} There is no
documented marketplace registration or full native plugin update/remove/list
contract, so direct cache/config manipulation would not meet the gate.

### Goose — portable skills, external lifecycle

Goose discovers `.agents/skills` and plugin-provided skills, but its own guide
delegates installation to the external `npx skills` CLI and a Summon extension.
[goose-skills]{21} The web Skills Marketplace is a browsing/submission surface,
not a deterministic native install/update/reconcile API. It is not a candidate.

### Kilo Code — promising UI marketplace, no reconcile surface

Kilo has global/project skills, `.agents/skills`, and a remote `index.json`
mechanism. [kilo-skills]{22} Its marketplace documentation describes UI
installation/removal and Kilo-specific destinations, but does not document a
native non-interactive skill/plugin lifecycle suitable for state reconciliation.
[kilo-marketplace]{23}

### Mistral Vibe — skill paths without package provenance

Vibe supports `SKILL.md`, `.vibe/skills`, `.agents/skills`, and config filters.
[mistral-skills]{24} Its current CLI documentation has no marketplace, package source, or
install/update/remove lifecycle. [mistral-skills]{24} It is a materialization
target for portable skills only, not a managed harness candidate.

### Kiro CLI — different package contract and IDE-only Powers

Kiro's Powers use `POWER.md` plus MCP and hooks and are documented as one-click
IDE/web installations; the FAQ says CLI support is planned. [kiro-powers]{25}
The CLI exposes MCP/config commands, but not a Power/skill package lifecycle.
[kiro-cli-commands]{26} It does not meet the shared `SKILL.md` and native CLI
requirements.

## Recommended rollout

{inferred: sequencing recommendation} Add adapters in this order:

1. Factory Droid — validates a conventional native marketplace/plugin adapter
   with explicit commit-based updates and simple scope semantics.
2. Qwen Code — exercises cross-marketplace conversion and source compatibility
   with both Claude and Gemini.
3. GitHub Copilot CLI — adds a widely used marketplace with declarative
   repository settings, enterprise policy, and a distinct `plugin.json` model.
4. Pi package adapter — add after the resource model can distinguish package
   sources from registered marketplaces; do not fake marketplace operations.

Defer Gemini, Junie, Kimi, OpenCode, Goose, Kilo, Vibe, and Kiro. Re-open a
deferred harness only when the missing hard gate is documented and testable.

## Adapter acceptance tests

Before calling any adapter complete, run an isolated fixture matrix:

- register a source and repeat the command; the second plan is a no-op;
- install a package/plugin at user and project scope;
- update a Git source after changing its resolved SHA;
- disable/enable and verify the declared state without touching caches;
- remove and verify both native state and skilltap provenance;
- import a Claude/Codex-compatible plugin with one faithful skill and one
  unsupported component, asserting a partial plan and `--yes` acknowledgment;
- introduce divergent instruction files and verify canonical-wins warnings;
- corrupt or delete a native cache and verify status reports drift rather than
  repairing by direct cache mutation;
- run all operations with harness binaries absent or unauthenticated and
  verify fail-fast, actionable output.

## Disconfirming analysis

I searched for counterexamples to the candidate decision in each harness's
current official CLI/reference pages: missing uninstall/update commands,
missing project scope, source registration hidden behind an interactive UI,
and cache-only installation. That search found the near misses above rather
than a fourth full candidate. The absence of a documented command is not proof
that an unreleased binary cannot have one; it is a bounded public-contract
finding and should be rechecked before implementation.

The candidate decision is composed across the attested source contracts and is
marked `{inferred: candidate decision}`. Individual command, path, and format
claims remain source-bound above.
