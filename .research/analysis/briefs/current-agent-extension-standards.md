---
provenance: agent-synthesis
status: current
updated: 2026-07-10
temporal_contract: supersedes-prior
supersedes: .research/.import-holding/claude-code-marketplace-findings.md
refresh_verification:
  delta_reverified:
    - codex-build-plugins
    - codex-plugins
    - codex-skills
    - codex-agents-md
    - codex-config
    - codex-config-reference
    - claude-plugins-reference
    - claude-plugin-marketplaces
    - claude-skills
    - claude-settings
    - claude-memory
    - agentskills-spec
    - agentskills-client-implementation
  grandfathered_prior_verified: []
---

# Current agent extension standards

## Decision

{inferred: architecture decision} skilltap should model Codex and Claude Code as distinct native control planes
joined by a small portable skill contract. It should register and operate each
harness's marketplaces and plugins through native interfaces when those
interfaces are deterministic, but it should own portable standalone skills as
complete directories and project them into each harness's supported locations.

{inferred: product boundary} Native plugin formats are not interchangeable
packages. The faithful common denominator is the Agent Skills directory format;
plugin transfer beyond that denominator is an explicit compatibility operation,
not a transparent conversion. [codex-build-plugins]{1}
[claude-plugins-reference]{10} [agentskills-spec]{20}

## Contract map

The native columns below summarize attested contracts. The final column is
{inferred: skilltap posture} design guidance composed from those contracts.

| Resource | Portable or canonical form | Codex native contract | Claude Code native contract | skilltap posture |
|---|---|---|---|---|
| Standalone skill | Whole directory with top-level `SKILL.md` [agentskills-spec]{20} | User/project `.agents/skills`; directory symlinks supported [codex-skills]{3} | User/project `.claude/skills`; directory symlinks supported [claude-skills]{12} | Own the whole directory in `.agents/skills` when compatible; create adapter links or copies only where required |
| Plugin | No shared plugin schema | Required `.codex-plugin/plugin.json`; Codex component model [codex-build-plugins]{1} | Optional `.claude-plugin/plugin.json`; Claude component model [claude-plugins-reference]{10} | Preserve native identity and lifecycle; never relabel one manifest as the other |
| Marketplace | No shared catalog schema | `.agents/plugins/marketplace.json` [codex-build-plugins]{1} | `.claude-plugin/marketplace.json` [claude-plugin-marketplaces]{11} | Register selected sources natively; do not expose search or recommendation |
| Global instructions | skilltap policy: `~/AGENTS.md` | Reads `~/.codex/AGENTS.md` or override [codex-agents-md]{4} | Reads `~/.claude/CLAUDE.md`, not `AGENTS.md` [claude-memory]{14} | Bridge both native paths to the canonical file and diagnose precedence blockers |
| Project instructions | Repository `AGENTS.md` hierarchy | Reads layered `AGENTS.md` files natively [codex-agents-md]{4} | Reads `CLAUDE.md`; supports import or symlink bridge [claude-memory]{14} | Keep `AGENTS.md` primary and install the narrowest faithful Claude bridge |
| Update state | skilltap machine state | Catalog source, installed cache, enablement, config [codex-build-plugins]{1} | Marketplace, installed cache, enablement, settings [claude-plugins-reference]{10} [claude-settings]{13} | Observe native state and retain source/revision/ownership separately |

The whole-directory skill boundary is normative in Agent Skills and native in
both harnesses: scripts, references, assets, and other siblings can be part of
the skill and cannot be reconstructed from `SKILL.md` alone.
[agentskills-spec]{20} [codex-skills]{3} [claude-skills]{12}

## Portable skill contract

The conforming portable core is a directory whose name matches its `name` field
and whose root contains exact `SKILL.md`. Required frontmatter is `name` and
`description`; optional portable fields are `license`, `compatibility`,
`metadata`, and experimental `allowed-tools`. The standard does not define an
installation path, marketplace, plugin, source provenance, dependency solver,
lockfile, or update protocol. [agentskills-spec]{20}

{inferred: canonical placement} `.agents/skills/` is an interoperability convention rather than a normative
path in the open standard. It is nevertheless an appropriate canonical managed
placement for skilltap because Codex consumes it directly and the official
Agent Skills implementation guide identifies it as a cross-client convention.
Claude's native personal/project skill roots accept symlinked complete skill
directories, so an adapter can project the same managed directory without
discarding resources. [agentskills-client-implementation]{21}
[codex-skills]{3} [claude-skills]{12}

{inferred: compatibility rule} Format validity and behavioral portability are
separate results. Claude-specific invocation, model, tool, subagent, and hook
fields can materially change execution, while experimental Agent Skills fields
do not guarantee uniform enforcement. skilltap should therefore report both
strict format conformance and per-target behavioral compatibility.
[claude-skills]{12} [agentskills-spec]{20}

## Native plugin and marketplace contracts

Codex packages require `.codex-plugin/plugin.json`; Claude's
`.claude-plugin/plugin.json` is optional and has different discovery and
component semantics. Both systems keep components at the plugin root, but the
similar directory shape does not make their manifests, resource kinds, or
lifecycle state equivalent. [codex-build-plugins]{1}
[claude-plugins-reference]{10}

{confidence: current-public-docs} Codex documents non-interactive marketplace source management with
`codex plugin marketplace add|list|upgrade|remove`, but its documented end-user
plugin installation surface is the interactive `/plugins` browser. The current
public contract does not establish a deterministic plugin install, uninstall,
or update command or stable structured output for those operations.
[codex-build-plugins]{1} [codex-plugins]{2}

Claude documents non-interactive marketplace and plugin lifecycle commands,
including install, uninstall, enable, disable, update, and JSON-capable list
operations. Plugin identity is qualified by both plugin and marketplace name,
and native settings distinguish user, project, local, and managed scopes.
[claude-plugin-marketplaces]{11} [claude-plugins-reference]{10}
[claude-settings]{13}

{inferred: adapter rule} skilltap should call native Claude lifecycle commands
and observe their outputs. For Codex, it may manage marketplace sources
natively, but it must capability-probe plugin mutations and stop with an
actionable plan when the installed host exposes only an interactive surface.
Direct cache writes are not a faithful fallback for either harness because the
caches are installed runtime artifacts with native version and ownership
semantics. [codex-build-plugins]{1} [claude-plugins-reference]{10}

Registering a marketplace is not discovery. {inferred: product boundary}
skilltap can remember a user-selected marketplace source and reconcile that
registration without listing, searching, ranking, or recommending its remote
inventory.

## Instructions and scope

The skilltap-wide global policy `~/AGENTS.md` is not itself either harness's
documented native global path. Codex reads `~/.codex/AGENTS.md` unless a
non-empty `AGENTS.override.md` takes precedence. Claude reads
`~/.claude/CLAUDE.md` and explicitly does not read `AGENTS.md` directly.
[codex-agents-md]{4} [claude-memory]{14}

{inferred: faithful bridge} Global reconciliation should make
`~/.codex/AGENTS.md` and `~/.claude/CLAUDE.md` symlinks to `~/AGENTS.md` when
their native files contain no distinct content. If Claude-specific instructions
must coexist, its bridge may instead import `~/AGENTS.md`. An effective Codex
override, an existing divergent native file, or a broken link is drift that
must be planned and reported rather than overwritten silently.
[codex-agents-md]{4} [claude-memory]{14}

For project scope, Codex natively layers `AGENTS.md` from repository root toward
the working directory. Claude instead loads project `CLAUDE.md` files and
documents an import or symlink bridge to `AGENTS.md`. The state model should
therefore record the canonical project instructions separately from each
adapter's bridge representation. [codex-agents-md]{4} [claude-memory]{14}

The default command scope can remain global while `--project` selects the
current repository and `--project <path>` selects another repository. That is a
skilltap UX policy, not a native standard. Native adapters must still honor
Codex project trust and Claude's project/local/user settings distinctions before
claiming a projected configuration is effective. [codex-config]{5}
[claude-settings]{13}

## Reconciliation and updates

{inferred: state model} Reconciliation needs three separate observations:

1. desired skilltap state, including managed harnesses, source selections,
   scope, ownership, and accepted compatibility consequences;
2. native declared state, including marketplace registration, settings, and
   enablement; and
3. effective installed state, including materialized skill directories and
   native cached plugin versions.

{inferred: state separation} Collapsing these layers would incorrectly treat a shared Claude project plugin
declaration as proof that every user installed it, or a refreshed Codex
marketplace as proof that cached plugins were upgraded. Claude retains per-user
trust and consent for project declarations; current Codex documentation does
not specify that marketplace upgrade updates installed plugins.
[claude-settings]{13} [codex-build-plugins]{1}

Claude resolves plugin update identity from manifest version, marketplace-entry
version, or Git commit SHA in that order. An unchanged explicit version can
therefore suppress an update even when a repository moves. Codex catalogs can
pin Git refs or SHAs and npm selectors, while installed plugins occupy versioned
cache paths. [claude-plugins-reference]{10}
[claude-plugin-marketplaces]{11} [codex-build-plugins]{1}

{inferred: update rule} For skilltap-managed Git skills, a changed resolved Git
SHA is the update signal. For native plugins, retain and explain the harness's
own resolved-version basis rather than replacing it with a universal SHA rule.
An optional daemon may automatically apply only plans classified as safe; any
new incompatibility, consent boundary, destructive removal, or unsupported
native mutation remains pending for an explicit operation.

## Compatibility outcomes

{inferred: compatibility taxonomy} Every cross-harness plan should classify each resource:

- **faithful** — the target has an equivalent native representation and the
  complete resource can be preserved;
- **materializable** — skilltap can reproduce the relevant files or settings,
  but the result is skilltap-owned rather than a native installation;
- **partial** — some behavior or resource has no faithful target equivalent;
- **blocked** — trust, policy, missing capability, or unresolved ownership
  prevents a safe deterministic operation.

{inferred: consent rule} `--yes` may approve ordinary already-disclosed changes,
but it must not erase compatibility consequences. Partial or destructive plans
should enumerate the affected resource and consequence so approval can remain
piecewise and operation-scoped.

## Disconfirming analysis

The Codex specialist searched current official OpenAI documentation for a
non-interactive plugin install command, installed-plugin update semantics, and
a direct `~/AGENTS.md` global instruction source. {confidence:
current-public-docs} The retrieved official material documents interactive
`/plugins`, non-interactive marketplace source commands, and global instructions
under Codex home; it does not settle the missing plugin mutation and update
contracts. This is a bounded documentation gap, not proof that no installed
Codex version exposes additional behavior. [codex-plugins]{2}
[codex-build-plugins]{1} [codex-agents-md]{4}

The Claude sources were checked for direct `AGENTS.md` loading and for a
literal global `~/.claude/CLAUDE.md -> ~/AGENTS.md` example. Claude explicitly
documents that it reads `CLAUDE.md`, not `AGENTS.md`, and documents symlink and
import bridges; {inferred: global bridge composition} applying that bridge at
the independently documented global Claude path is a composition of contracts,
not a verbatim native recipe. [claude-memory]{14}

The Agent Skills specification and official client guide were checked for
marketplace, provenance, package, dependency, lockfile, and update primitives.
{confidence: current-public-docs} None is defined in the attested current
format contract; the standard instead leaves placement and client lifecycle to
implementations. [agentskills-spec]{20}
[agentskills-client-implementation]{21}

## Contradictions and qualifications

### Strict conformance versus tolerant loading — `tension`

The Agent Skills specification gives strict naming and directory rules, while
its client guide recommends tolerating some invalid inputs for interoperability.
These claims concern authoring conformance and client consumption respectively.
{inferred: diagnostic recommendation} skilltap should expose both results instead of one ambiguous validity flag.
[agentskills-spec]{20} [agentskills-client-implementation]{21}

### Shared Claude declarations versus installed state — `qualifies`

Claude project settings can declare enabled plugins for collaborators, but
external plugins still require each user's trust and installation consent. A
{inferred: state interpretation} repository declaration is desired project state, not evidence of effective
machine installation. [claude-plugins-reference]{10} [claude-settings]{13}

### Native marketplace parity — `incommensurable`

Codex and Claude both use JSON marketplace catalogs and Git-backed sources, and
Codex can read a legacy Claude catalog path. That compatibility does not make
all entries or plugin manifests semantically interchangeable. {inferred:
adapter validation} Each adapter must
validate against its own native contract. [codex-build-plugins]{1}
[claude-plugin-marketplaces]{11}

## Unknowns and revisit triggers

- {confidence: current-public-docs} Codex's current public documentation does not settle non-interactive plugin
  mutation commands, installed-plugin update semantics, exact enablement schema,
  or stable JSON output. Revisit when OpenAI documents them.
- {confidence: current-public-docs} Neither native documentation set provides a versioned structured-output
  contract sufficient to skip runtime validation for every lifecycle command.
  [codex-build-plugins]{1} [claude-plugins-reference]{10}
- {confidence: current-public-docs} Agent Skills does not define distribution or lifecycle. Revisit if the open
  standard adds package, provenance, version, dependency, or update primitives.
- Re-run this engagement when native manifest schemas, skill frontmatter,
  global instruction locations, scope rules, or plugin update precedence change.

## Refresh delta

The superseded legacy notes centered Claude's marketplace as a portable
distribution model and treated skilltap as an inventory-discovery installer.
This refresh replaces that frame. The current evidence supports distinct native
plugin adapters, a narrow whole-directory Agent Skills bridge, registered-source
management without discovery, and explicit reconciliation of desired, declared,
and effective state.
