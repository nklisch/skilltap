---
id: epic-expanded-harness-support-native-coexistence-contract
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-native-coexistence
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Route Native and Managed Lifecycle by Evidence

## Checkpoint

Introduce the shared coexistence decision that every native-managed adapter
consumes. Replace project-managed-first routing with a pure, target-neutral
representation selector: existing target state pins updates/removals, a plugin
inherits its exact target-local marketplace representation, and only a fresh
marketplace compares adapter-authored native and managed component plans.

Generalize managed projection from project-only context to one concrete
`Scope`. This lets target adapters project to documented global and project
surfaces without duplicating acquisition, ownership, drift, acknowledgment,
execution, or state logic. Preserve Codex's current global-native/project-
managed behavior exactly.

## Contract

**Files**:

- `crates/core/src/lifecycle_representation.rs` (new),
  `crates/core/src/lib.rs`
- `crates/harnesses/src/native_distribution.rs` (new),
  `crates/harnesses/src/registry.rs`, `crates/harnesses/src/lib.rs`
- `crates/harnesses/src/managed_projection.rs`,
  `crates/harnesses/src/adapters/codex_managed.rs`
- `crates/cli/src/application.rs`,
  `crates/cli/src/application/lifecycle.rs`,
  `crates/cli/src/application/execution.rs`,
  `crates/cli/src/application/tests.rs`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleRepresentation { Native, Managed }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepresentationCandidate {
    pub representation: LifecycleRepresentation,
    pub plan: MaterializationPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepresentationEvidence {
    Existing(LifecycleRepresentation),
    Marketplace(LifecycleRepresentation),
    Fresh {
        native: Option<RepresentationCandidate>,
        managed: Option<RepresentationCandidate>,
    },
}

pub fn select_lifecycle_representation(
    evidence: RepresentationEvidence,
) -> Result<LifecycleRepresentation, LifecycleRepresentationError>;

pub fn applied_lifecycle_representation(
    state: &TargetResourceState,
) -> Result<LifecycleRepresentation, LifecycleRepresentationError>;
```

```rust
pub struct NativeDistributionContext<'a> {
    pub target: &'a HarnessId,
    pub scope: &'a Scope,
    pub checkout: &'a ResolvedSourceCheckout,
    pub requested_revision: Option<&'a RequestedRevision>,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
}

pub trait NativeDistributionPort: Sync {
    fn assess(
        &self,
        context: &NativeDistributionContext<'_>,
    ) -> Result<Option<NativeDistributionAssessment>, NativeDistributionError>;
}
```

Amend `ManagedProjectionContext` to carry `scope: &Scope` instead of
`project: &AbsolutePath`, and replace `plan_managed_project_lifecycle` /
`ManagedProjectPlanContext` with concrete-scope equivalents. Adapters derive all
native roots; core/CLI do not match target ids.

## Required behavior

- Harness-owned native/adopted state routes native. Skilltap-owned materialized
  state with a valid managed manifest routes managed. Contradictory evidence
  fails closed.
- Fresh plugin install follows its target-local marketplace representation.
- Fresh marketplace selection rejects blocked required components. Faithful
  native wins. Managed wins only when native is absent or managed includes a
  strict superset without adding required blocks. Equal partial plans prefer
  native; incomparable partial plans block and expose both consequence sets.
- Once native is selected, an unknown version or narrowed capability blocks the
  native operation; it cannot reselect managed as an authority bypass.
- One resolved checkout is borrowed by native/managed assessors. There is no
  recursive discovery, second clone, or source mutation.
- Existing execution ports, operation dependencies, configuration lock,
  revalidation, rollback, final observation, and target-local state refresh are
  reused.
- `NativeDistributionPort` is an assessment boundary, not a universal plugin
  schema. It emits existing normalized component/materialization evidence while
  each adapter retains its native parser and semantics.

## Acceptance evidence

- Pure tests cover state pinning, marketplace inheritance, native preference,
  managed strict-superset selection, equal/incomparable partial plans, required
  blocks, absent candidates, and contradictory state.
- Mixed-target tests prove one resource may be native on one target and managed
  on another without coalescing ids, revisions, ownership, or journals.
- Codex project managed lifecycle retains operation ids, projection bytes,
  state, removal, pending recovery, and immediate-repeat behavior; Codex global
  lifecycle remains native.
- Managed projection can receive global and project scopes without a target-id
  branch.
- `git grep` finds no new Droid/Qwen/Copilot behavior dispatch in core or CLI.

## Ordering

This is the foundation checkpoint. The Factory, Qwen, and Copilot adapter
stories depend on it; no target profile or registry entry should land before
this route is usable.

## Contract evidence preflight

Evidence refreshed 2026-07-14 against official documentation, official release
metadata, and isolated homes/projects under `/tmp`; no operator harness state or
repository source was used as a probe target. The delegated tool surface did not
expose the requested Z.ai tools, so the URLs below were fetched directly from
their official hosts. This is evidence for the parallel source implementation; implementation
closure for this routing story is recorded below.

### Factory Droid

**Official sources**

- https://docs.factory.ai/cli/getting-started/quickstart.md
- https://docs.factory.ai/cli/configuration/plugins.md
- https://docs.factory.ai/cli/configuration/skills.md
- https://docs.factory.ai/cli/configuration/mcp.md
- https://registry.npmjs.org/droid/latest
- https://registry.npmjs.org/@factory%2fcli-linux-x64/0.171.0
- https://registry.npmjs.org/@factory/cli-linux-x64/-/cli-linux-x64-0.171.0.tgz
- https://github.com/Factory-AI/factory-plugins

**Exact release and probes.** The official `droid` package and Linux x64
platform package are `0.171.0` at Git commit
`81c7977d27c48c57d0d27aa9000b8182f626f6f3`; the downloaded platform tarball
matched the registry SHA-1 `16bd6a55ff2e8ff60cab79503dc4d70febe22006`.
With isolated `HOME`, XDG roots, and working directory:

```text
$ droid --version
0.171.0

$ droid plugin list --scope user
No plugins installed in user scope.

$ droid plugin list --scope project
No plugins installed in project scope.

$ droid plugin list --scope user --json
error: unknown option '--json'                       # exit 1
```

The exact lifecycle argv exposed by `0.171.0` is
`plugin marketplace add <url>|remove <name>|list|update [name]` and
`plugin install|uninstall|update|list`, with `-s|--scope user|project` on every
plugin operation. Marketplace operations are unscoped. Neither plugin nor
marketplace list has structured output. Against an isolated clone of the
official Factory marketplace, native add/install/list/update/uninstall/remove
succeeded. Installed list rows were exactly:

```text
Installed plugins:
Active:
  security-engineer@factory-marketplace  [user]  e8801fa
```

Native state retained the 12-character commit `e8801fa1020f`, exact scope,
qualified plugin/marketplace identity, and cache install path in
`~/.factory/plugins/installed_plugins.json`; user and project enablement lived
in their respective `.factory/settings.json`. Plugin skills remained children
under the native cache rather than appearing in standalone skill roots. A
second uninstall returned exit 1, so reconciliation must observe and suppress
an already-satisfied removal rather than replay it.

**Scoped component contract.** Official docs currently name personal/workspace
skills as `~/.factory/skills/<name>/` and `<project>/.factory/skills/<name>/`,
with complete sibling files. Skills require a restart/rescan. MCP uses
`~/.factory/mcp.json` plus ancestor/project `.factory/mcp.json`; user wins over
folder, then project. Project servers are removed by editing the project file,
whereas CLI additions are user-scoped. MCP file changes auto-reload. Org
`mcpPolicy` can retain a declaration while preventing it from running, so that
is policy health, not drift.

**Disposition.** Implementation can proceed for an exact `0.171.0` profile only
with version-gated human-output fixtures and no invented JSON shape. Current
docs advertise marketplace `--ref`/`--sha`, but the exact binary rejects
`--ref` as an unknown option and does not show either flag in help. Do not grant
pin mutation authority to this profile; preserve a requested pin through a
faithful managed representation or block it.

### Qwen Code

**Official sources**

- https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/
- https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/
- https://qwenlm.github.io/qwen-code-docs/en/users/features/mcp/
- https://qwenlm.github.io/qwen-code-docs/en/users/configuration/trusted-folders/
- https://github.com/QwenLM/qwen-code/releases/tag/v0.19.10
- https://registry.npmjs.org/@qwen-code/qwen-code/0.19.10

**Exact release and probes.** The official package is `0.19.10`, Git commit
`095bd160918086a3a33192133e7923635f08f973`, and `qwen --version` emits exact
stdout `0.19.10\n`. The exact lifecycle is `extensions sources
add|list|update|remove`, plus `extensions install|uninstall|list|update|enable|
disable`; install accepts `--scope user|project|workspace`, where workspace is
a project alias. All extension/source/MCP list commands are human-only:
`qwen mcp list --json` exits 1 with `Unknown argument: json`.

An isolated complete native extension produced a multi-line list record with
name/version, copied path, source/type, `Enabled (User)`, `Enabled (Workspace)`,
and enumerated context files, commands, skills, and MCP servers. User installs
set user enablement; project installs set user false/workspace true. CLI changes
require session restart; only the interactive `/extensions` manager hot-reloads.
The native store separates copied artifacts under `~/.qwen/extensions/` from
activation in `~/.qwen/extension-store/state.json`, including canonical
workspace overrides.

A configured Claude marketplace source listed as:

```text
claude-market
 Source: /tmp/.../qwen-claude-marketplace (Type: local)
 Last updated: <timestamp>
```

For `0.19.10`, `extensions install claude-market:claude-fixture` nevertheless
failed with `Install source not found: claude-market`; the documented
path/URL-plus-plugin form (`/tmp/.../qwen-claude-marketplace:claude-fixture`)
succeeded. Conversion copied the whole Claude source but generated a
`qwen-extension.json` containing only name/version/description. Native list
exposed the converted command and skill, but not the copied hook, agent, or
`.mcp.json`; the copied MCP server was absent from `qwen mcp list`. A successful
conversion process is therefore not fidelity evidence: component comparison
must decide native versus managed.

**Scoped component contract.** Skills are complete directories at
`~/.qwen/skills/<name>/` and `<project>/.qwen/skills/<name>/`; changes require a
restart. MCP merges `mcpServers` from `~/.qwen/settings.json` and
`<project>/.qwen/settings.json`, with the project definition winning a name
collision. Isolated `mcp list` displayed project entries as `Pending approval`;
`qwen mcp approve <name>` emitted `Approved MCP server "<name>" (bound to its
current config). Approved servers connect in your next interactive session.`
Changing the bound definition must therefore invalidate effective approval.
Folder trust is disabled by default; when enabled, an untrusted workspace
ignores project settings and prevents extension install/update/uninstall.

**Disposition.** Implementation can proceed for exact `0.19.10` with a
version-gated text parser, config-bound MCP approval health, restart-aware
verification, and component-by-component conversion assessment. It must not
claim structured lifecycle output, source-name installation, or fidelity from a
zero exit status.

### GitHub Copilot CLI

**Official sources**

- https://docs.github.com/en/enterprise-cloud@latest/copilot/reference/copilot-cli-reference/cli-plugin-reference
- https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills
- https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers
- https://docs.github.com/en/copilot/reference/mcp-allowlist-enforcement
- https://docs.github.com/en/copilot/concepts/agents/about-enterprise-plugin-standards
- https://github.com/github/copilot-cli/releases/tag/v1.0.70
- https://registry.npmjs.org/@github/copilot/1.0.70
- https://github.com/github/copilot-cli/releases/download/v1.0.70/SHA256SUMS.txt

**Exact release and probes.** The official Linux x64 release tarball matched
SHA-256 `4edee3cd005254960789329181968b209b17cab47f43ee13c9e071b1f7e33095`.
Exact version stdout is:

```text
GitHub Copilot CLI 1.0.70.
Run 'copilot update' to check for updates.
```

`1.0.70` exposes global `plugin install|list|uninstall|update` and marketplace
`add|browse|list|remove|update`; none accepts a scope and neither list accepts
`--json`. Although the current reference table still names plugin enable and
disable, the exact binary omits both commands; `plugin enable --help` falls back
to parent help. Empty native outputs are `No plugins installed.` and two
included marketplace rows (`copilot-plugins`, `awesome-copilot`).

MCP does have structured observation. Empty `copilot mcp list --json` is
`{"mcpServers":{}}`. With isolated user/workspace fixtures it returns a
`mcpServers` object keyed by server name; each value carries normalized
`tools`, type, command/URL fields, `source`, and workspace `sourcePath`.
`copilot mcp get <name> --json` wraps the same record under the requested name.
`<project>/.mcp.json` overrides same-named user entries and is selected instead
of `<project>/.github/mcp.json`; when `.mcp.json` is absent, `.github/mcp.json`
is loaded. The native JSON included raw fixture environment values, so adapter
normalization must redact before findings or state.

**Scoped component contract.** Current skill precedence is exact first-found:
project `.github/skills`, `.agents/skills`, `.claude/skills`, inherited parent
roots, then personal `~/.copilot/skills`, `~/.agents/skills`, then plugin skills.
`/skills reload` reloads a running session. MCP loads user
`~/.copilot/mcp-config.json`, one workspace file, plugin definitions, then
`--additional-mcp-config`; workspace wins user collisions and MCP additions are
available immediately. Enterprise MCP policy is name/ID based, applies to local
and remote servers, and is explicitly bypassable by file editing today; report
policy effectiveness separately. Enterprise plugin standards are public
preview and can inject known marketplaces/default plugins at authentication.

**Disposition.** Managed whole-skill and MCP implementation can proceed for
`1.0.70`, using the structured MCP schema above. The planned native plugin
profile cannot proceed as written: there is no project-scoped native lifecycle,
no structured plugin/marketplace list, and no enable/disable argv in the exact
binary. Do not register a profile claiming those capabilities.

### Routing disposition

The target-neutral coexistence selector can proceed: absent or narrowed native
candidates are first-class evidence and must not prevent a faithful managed
candidate from being compared on a fresh install. Existing native state remains
pinned native. Factory `0.171.0` and Qwen `0.19.10` can supply narrowly verified
native candidates through version-gated text fixtures; Copilot `1.0.70` can
supply managed skill/MCP candidates and structured MCP observation, but not the
planned scoped native plugin candidate.

## Blocker

GitHub Copilot CLI `1.0.70` materially contradicts the requested native profile:
its plugin and marketplace list commands are unstructured and unscoped, and its
binary omits the documented enable/disable commands. The Copilot adapter story
must either narrow native authority to exact global install/list/update/remove
with version-gated human parsing, or wait for and validate a release that
actually exposes the scoped/structured contract. Factory marketplace pin flags
are also documentation-only for exact `0.171.0`; they must remain disabled in
that profile. Finally, this delegated pass could not satisfy the requested Z.ai
transport because no Z.ai tool was exposed; a parent context with that tool must
re-fetch the listed URLs if transport provenance is a hard gate.

## Implementation closure

- Added the pure `LifecycleRepresentation` selector and target-local state
  inference. Native/adopted bindings remain native, materialized skilltap
  bindings remain managed, and native-plus-managed evidence fails closed.
  Materialized marketplace state may have an empty projection manifest; its
  provenance/ownership pair is the valid managed pin.
- Added the adapter-owned `NativeDistributionPort` and concrete assessment
  context. No Factory, Qwen, or Copilot registry entries or behavior dispatch
  were added.
- Shared orchestration now pins existing state and target-local marketplace
  representation, compares fresh native/managed plans through one resolved
  checkout, and never falls back from selected native evidence after profile
  narrowing or unknown-version detection.
- Generalized managed lifecycle planning/execution names and plumbing to
  concrete-scope lifecycle ports while retaining Codex's global-native and
  project-managed adapter behavior.
- Added pure selection, mixed-target/global-project state, and assessment-port
  boundary tests. Existing Codex, Gemini, OpenCode, native failure, recovery,
  idempotence, and compiled lifecycle acceptance tests remain green.
- Verification passed: `cargo test --workspace --all-targets`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- This story intentionally leaves dependent Factory, Qwen, and Copilot adapter
  registration and dispatch to their child stories.
