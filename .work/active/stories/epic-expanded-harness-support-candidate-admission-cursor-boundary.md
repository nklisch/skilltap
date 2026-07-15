---
id: epic-expanded-harness-support-candidate-admission-cursor-boundary
kind: story
stage: done
tags: [testing]
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-gate]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/cursor-skills.md
  - .research/attestation/cursor-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Validate Cursor Boundaries

## Checkpoint

Close Cursor's Agent Skills path and reload gaps while revalidating the already
attested MCP files in an isolated editor/CLI profile. Record exact source and
native evidence in this story body and conclude `admitted`, `observe_only`, or
`blocked`.

Validation retains `~/.cursor/mcp.json`, `<project>/.cursor/mcp.json`, and the
`cursor-agent mcp list`/tool-observation surface, then establishes exact
documented global/project skill roots, complete siblings and executable intent,
project/global precedence, update/reload visibility, and whether editor and CLI
consume the same promised skills. It also pins exact `cursor-agent` version
output and profile identity, MCP schema/transports, unknown/unowned entry
preservation, ownership-safe update/removal, and OAuth/extension/cache
boundaries.

Create `crates/harnesses/tests/candidate_cursor_boundary.rs` only after all
Cursor roots are redirectable away from operator state.

## Acceptance evidence

- [ ] Exact version bytes and both documented skill roots are reproduced in an
      isolated profile.
- [ ] Whole skill trees remain visible after sibling/content/mode updates and
      obey documented scope precedence in both editor and CLI where promised.
- [ ] Both MCP files preserve unrelated fields/servers, expose same-name
      precedence, and match fresh CLI list/tool state after reload.
- [x] OAuth and extension registration remain native/user-owned and absent from
      skilltap evidence.
- [ ] Owned removal and immediate repeat are idempotent and cache-independent.
- [x] Known MCP behavior alone cannot produce `admitted` while the skill
      boundary or editor/CLI equivalence remains unresolved.

## Ordering

Runs after the shared gate and before Cursor's admission checkpoint. It does not
add a registry entry or mutating port.

## Boundary validation evidence

### Safety boundary and method

Validation ran on Linux at `2026-07-15T02:08:45Z`. The shared admission gate was
already `stage: done`. Z.ai search/fetch tools were not exposed in this worker,
so current official Cursor sources were discovered through Cursor's own
`https://cursor.com/llms.txt` index and fetched directly over HTTPS as Markdown.
No browser, editor, authentication flow, OAuth flow, UI, screen scraper, MCP
server, or Cursor process was started.

The native preflight stopped before installation or mutation because all likely
Cursor executables were absent and the official profile contract is incomplete:

```text
agent=absent
cursor-agent=absent
cursor=absent
code=absent
cursor-server=absent
```

Current official docs relocate `cli-config.json` with `CURSOR_CONFIG_DIR` (and
`XDG_CONFIG_HOME` on Linux/BSD), but do not say either override relocates global
skills, global MCP, credentials, logs, caches, editor user data, or editor
extensions. They also do not document a complete disposable editor profile.
Changing `HOME` was not treated as proof of an officially supported complete
profile boundary. Per the checkpoint guardrail, no installer or native command
was run after that gap was established.

### Current official sources

All responses below returned HTTP 200 as `text/plain; charset=utf-8` on
2026-07-15. SHA-256 identifies the exact fetched bytes.

| Official source | SHA-256 | Source-direct evidence |
| --- | --- | --- |
| `https://cursor.com/docs/skills.md` | `32d6836220da99f9f851a39031a188a887b6ea038a2730a288e8a3b820ca4064` | Names project roots `.agents/skills/` and `.cursor/skills/`, global roots `~/.agents/skills/` and `~/.cursor/skills/`, recursive `SKILL.md` discovery, and optional `scripts/`, `references/`, and `assets/`. It says referenced scripts may be executed by the agent, but does not define global/project collision precedence or a deterministic CLI discovery/list command. |
| `https://cursor.com/docs/mcp.md` | `1bbd67fb28afd37b34ac961d63c0e1577f3ec4fe94ecab08b1bdefa4c751715d` | Names project `.cursor/mcp.json` and global `~/.cursor/mcp.json`; defines `mcpServers`, stdio `command`/`args`/`env`/`envFile`, remote `url`/`headers`, static OAuth, and stdio/SSE/Streamable HTTP. OAuth and `vscode.cursor.mcp.registerServer()` remain native/user-owned boundaries. |
| `https://cursor.com/help/customization/mcp.md` | `932c12854b115fed5b5227a7a820441535fa4a5cb63937d45834c5a27de8026d` | States both MCP files are merged, project same-name configuration takes priority, and a saved edit requires restarting Cursor. It does not specify unknown-field preservation or an editor-free reload probe. |
| `https://cursor.com/docs/cli/using.md` | `f61dc3ecf537c503064a33e9c40789ade8f3f05998953b954dc0032d8f3a95f8` | Says the CLI automatically detects and respects the same `mcp.json` configuration as the editor. It does not make the same explicit editor/CLI promise for Agent Skills. |
| `https://cursor.com/docs/cli/reference/parameters.md` | `963d15eb1edf8407dfcda3d3334e23f89c6aa3593a7827b9fe53dc1c83112989` | Current executable spelling is `agent`; `-v`/`--version` prints version. Documents `agent mcp list` and `agent mcp list-tools <identifier>`. `login` is explicitly separate. This materially narrows the older `cursor-agent` command assumption, but no local executable exists to pin bytes. |
| `https://cursor.com/docs/cli/reference/configuration.md` | `d6921fd7a44cf73c0e09063d42aeebd01695fdc1848505df515b573dbf8d579d` | Documents `~/.cursor/cli-config.json`, project `.cursor/cli.json`, `CURSOR_CONFIG_DIR`, and Linux/BSD `XDG_CONFIG_HOME`; the override is stated only for CLI configuration. |
| `https://cursor.com/docs/cli/reference/authentication.md` | `c578539ee1ae3d574a75937230b22cea360f28e17da37529ddfceeedd04d2a1e` | Says `agent login` opens a browser and credentials are stored locally without naming a redirectable credential root. It was not invoked. |
| `https://cursor.com/docs/cli/installation.md` | `50435bbdfe5f3640484df508440b1c6274d8c57f85c2f91ac78ee15caa5a1d3c` | Gives `agent --version` as verification and states the CLI auto-updates by default. The installer was not run because installation cannot close the editor-profile isolation gap and would not provide a stable pinned profile by itself. |

The existing MCP attestation remains directionally correct for files and
transports. The current source adds explicit same-name project precedence. The
current CLI reference uses `agent`, not the previously assumed `cursor-agent`;
that correction is retained here rather than changing shared attestation.

### Commands and observations

Only read-only network and shell-resolution commands were used:

```text
curl --fail --silent --show-error --location --max-time 30 https://cursor.com/llms.txt
curl --fail --silent --show-error --location --max-time 30 <official-markdown-url>
curl --fail --silent --show-error --location --max-time 30 <official-markdown-url> | sha256sum
curl --fail --silent --show-error --location --max-time 30 --head <official-markdown-url>
command -v agent
command -v cursor-agent
command -v cursor
command -v code
command -v cursor-server
date -u '+%Y-%m-%dT%H:%M:%SZ'
uname -srm
```

The five `command -v` probes returned no path. Consequently there are no exact
native version bytes: neither `agent --version` nor any alias was runnable. No
`agent mcp` command was attempted, because the executable was absent and the
complete profile redirect could not be proven first.

### Admission-check evidence

The gate is strict: a documented claim alone does not count as an exercised
check where the story requires isolated effective behavior.

| Check | Result | Evidence or exact missing evidence |
| --- | --- | --- |
| `ExactInstallationIdentity` | missing | Official detection command is now `agent --version`; no `agent`, `cursor-agent`, or editor binary is installed, so exact stdout/stderr bytes and a version-pinned profile are unavailable. |
| `DocumentedGlobalSkillRoot` | missing (source established) | Official roots are `~/.agents/skills/` and `~/.cursor/skills/`; neither could be reproduced in an isolated native profile. |
| `DocumentedProjectSkillRoot` | missing (source established) | Official roots are `.agents/skills/` and `.cursor/skills/`; neither could be reproduced in an isolated native profile. |
| `CompleteSkillSiblings` | missing (source established) | Official docs promise scripts, references, and assets, but native sibling reads, content updates, nested references, and executable intent were not observable. |
| `SkillPrecedenceAndReload` | missing | Current docs do not define global/project same-name skill precedence, a deterministic reload, a CLI skill-list probe, or explicit editor/CLI equivalence. |
| `DocumentedGlobalMcpFile` | missing (source established) | `~/.cursor/mcp.json` is official, but was not read by an isolated Cursor process. |
| `DocumentedProjectMcpFile` | missing (source established) | `.cursor/mcp.json` is official, but was not read by an isolated Cursor process. |
| `McpSchemaAndPrecedence` | missing (source established) | `mcpServers`, transports, and project-over-global same-name precedence are official; isolated merge behavior and tool exposure were not reproduced. |
| `EffectiveReloadObservation` | missing | Official surfaces are Cursor restart plus `agent mcp list`/`list-tools`; no isolated editor/CLI profile or executable was available. |
| `UnknownFieldAndSiblingPreservation` | missing | No supported Cursor writer or isolated effective decoder was exercised, so unrelated top-level fields, unknown server fields, and unowned servers were not proven preserved. |
| `OwnershipSafeUpdateAndRemoval` | missing | No owned native identity was created; update/removal could not be tested without crossing the unproven profile boundary. |
| `CacheIndependentBoundary` | missing | `CURSOR_CONFIG_DIR` only establishes CLI-config relocation. Global skills/MCP, auth, editor profile, extension state, and caches lack one proven redirectable boundary. |
| `SharedAdapterAcceptance` | missing | Detection, both scope matrices, effective reload, preservation, removal, OAuth exclusion, and cache non-mutation could not run. No candidate test was created. |
| `ImmediateRepeatNoChange` | missing | No safe native mutation was authorized, so no apply/reload/removal operation existed to repeat. |

OAuth, credentials, extension registration, and cache state were neither read nor
written. No production adapter, registry entry, test, target attestation,
operator configuration, or native state changed.

## Implementation notes

- Execution capability: direct source fetch and inert local preflight only; the
  caller forbade nested agents and required a hard stop at an unproven profile.
- Review weight: not applicable; this is a child-story boundary checkpoint and
  child stories advance directly to done.
- Files changed: this story only.
- Tests added/removed: none; creating a candidate test was explicitly contingent
  on a proven redirectable Cursor profile.
- Simplification: none.
- Discrepancies from design: current official CLI docs use `agent`, not
  `cursor-agent`; exact roots are now documented, but safe effective validation
  remains blocked by missing binaries and incomplete profile redirection.
- Adjacent issues parked: none.

## Missing checks

`ExactInstallationIdentity`, `DocumentedGlobalSkillRoot`,
`DocumentedProjectSkillRoot`, `CompleteSkillSiblings`,
`SkillPrecedenceAndReload`, `DocumentedGlobalMcpFile`,
`DocumentedProjectMcpFile`, `McpSchemaAndPrecedence`,
`EffectiveReloadObservation`, `UnknownFieldAndSiblingPreservation`,
`OwnershipSafeUpdateAndRemoval`, `CacheIndependentBoundary`,
`SharedAdapterAcceptance`, and `ImmediateRepeatNoChange`.

## Original mutation disposition

blocked

## Relaxed registry disposition — 2026-07-15

The original `blocked` result remains binding for mutation and effective-state
claims: no exact installed profile, skill precedence/equivalence, reload,
ownership, removal, or cache-independent mutation boundary was established.
The relaxed gate asks only for reliable target identity plus safe documented
reads for registry admission. Cursor satisfies that narrower threshold through
the current official executable name `agent`, its bounded one-line `--version`
observation contract, the documented global/project Agent Skills roots, and the
attested global/project `.cursor/mcp.json` files with project-over-global MCP
precedence.

The registry disposition is therefore **`observe_only`**, with unresolved
`skill_precedence`, `editor_cli_skill_equivalence`, and `effective_reload`
reported explicitly. No auth/login/browser/editor/cache boundary was used or
promoted, and the prior missing mutation checks above are intentionally
preserved.
