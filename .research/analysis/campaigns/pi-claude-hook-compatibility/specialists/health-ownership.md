---
provenance: agent-synthesis
updated: 2026-07-12
facet: pi-hooks-health-ownership-scope
temporal_contract: supersedes-prior
campaign: pi-claude-hook-compatibility
---

# Pi compound profile: installed-state health, scope, ownership, and MCP-adapter interaction

This specialist brief covers facet 3 of the `pi-claude-hook-compatibility`
campaign: how a controller can observe the installed version, enablement,
scope, runtime health, update/removal identity, trust behavior, and ownership
boundaries of `@hsingjui/pi-hooks`, and how it interacts with `pi-mcp-adapter`
in a compound Pi profile. The load-bearing distinction throughout is
**package presence versus effective health**: the extension can be installed
and loaded while producing no behavioral effect, and Pi's observation surfaces
report presence without reporting version or health.

## Source map

Proposed `harness-candidates` corpus continuation. Numbers 56–60 are this
specialist's proposed registrations pending orchestrator INDEX reconciliation;
existing Pi sources already registered at 11, 12, 13, 30, 55 are cited as-is.

| Number | Handle | URL / path |
|---:|---|---|
| 11 | `pi-packages` | `https://pi.dev/docs/latest/packages` *(existing)* |
| 13 | `pi-settings` | `https://pi.dev/docs/latest/settings` *(existing)* |
| 55 | `pi-mcp-adapter` | `https://pi.dev/packages/pi-mcp-adapter` *(existing)* |
| 56 | `pi-hooks-health-installed-package` | `~/.pi/agent/npm/node_modules/@hsingjui/pi-hooks/` |
| 57 | `pi-hooks-health-npm-registry` | `https://registry.npmjs.org/@hsingjui%2Fpi-hooks` |
| 58 | `pi-hooks-health-github-source` | `https://github.com/hsingjui/pi-hooks` |
| 59 | `pi-hooks-health-pi-runtime` | `pi list`, `pi --help`, `~/.pi/agent/settings.json` (live) |
| 60 | `pi-hooks-health-mcp-adapter-installed` | `~/.pi/agent/npm/node_modules/pi-mcp-adapter/` |

## Installed version identity

The installed package identifies itself in its manifest as
`@hsingjui/pi-hooks@0.0.2`, MIT-licensed, with repository
`git+https://github.com/hsingjui/pi-hooks.git` and a single Pi extension entry
at `./src/pi-hooks.ts`. It peer-depends on `@earendil-works/pi-coding-agent`
with an unbounded `*` range.
[pi-hooks-health-installed-package]{56}

The npm registry is the authoritative version surface: `dist-tags.latest` is
`0.0.2`, and only two versions exist (`0.0.1` 2026-04-02, `0.0.2` 2026-05-08).
The installed version equals the registry latest. The GitHub repository has
**no release tags**, so npm is the sole citable release authority.
[pi-hooks-health-npm-registry]{57} [pi-hooks-health-github-source]{58}

The package's source spec in settings is the unversioned form
`npm:@hsingjui/pi-hooks`. Per Pi's package contract, versioned specs
(`npm:pkg@1.2.3`) are pinned and skipped by `pi update --all` / `--extensions`;
unversioned specs float to npm latest on update.
[pi-packages]{11}

## Observable version, scope, and health

`pi list` is the documented observation surface for installed packages, and
the `pi --help` text states explicitly that it lists "installed extensions
from settings." Its output per package is a source identifier and one resolved
checkout path — for `@hsingjui/pi-hooks`, the path
`~/.pi/agent/npm/node_modules/@hsingjui/pi-hooks`. It does not report version,
peer-dependency health, load status, or per-resource enable state.
[pi-hooks-health-pi-runtime]{59}

Because `pi list` is settings-derived and path-only, three facts that a
controller needs must be read from separate surfaces:

- **Version** is readable from the resolved checkout's own `package.json`
  (`version: 0.0.2`), which is an inspectable, configurable location under
  Pi's documented npm checkout root — not an opaque cache.
  [pi-hooks-health-installed-package]{56} [pi-packages]{11}
- **Currency** must be checked against the npm registry `dist-tags.latest`
  (currently `0.0.2`, matching installed), not against the git HEAD, because
  the GitHub default branch reads `0.0.1` while npm and the installed copy
  read `0.0.2` (see Contradictions).
  [pi-hooks-health-npm-registry]{57} [pi-hooks-health-github-source]{58}
- **Effective health** — whether the extension actually does anything — is
  determined by the presence and contents of a `hooks` key in
  `~/.pi/agent/settings.json` or `<cwd>/.pi/settings.json`, not by the
  package being listed. The extension's `loadSettings()` returns
  `{ settings: undefined }` and every registered callback finds zero groups
  when no `hooks` key exists. On this machine the package is present in the
  fourteen-entry `packages` array but the `hooks` key is **absent**, so the
  extension is loaded yet inert.
  [pi-hooks-health-installed-package]{56} [pi-hooks-health-pi-runtime]{59}

There is no documented `pi doctor` / `pi health` / `pi list --json` surface
that reports whether a loaded extension initialized successfully. A failed or
inert extension is not distinguished from a healthy one by `pi list`.
[pi-hooks-health-pi-runtime]{59}

## Scope, trust, and enablement

Install and remove write to settings by default at user scope
(`~/.pi/agent/settings.json`); `-l` writes to project scope
(`.pi/settings.json`). Packages may appear in both scopes; identity is by npm
package name (for npm sources), git URL without ref (git sources), or resolved
absolute path (local sources). When the same package appears in both global
and project settings, "the project entry wins unless the project entry has
`autoload: false`, in which case it is applied as a delta over the global
entry."
[pi-packages]{11} [pi-settings]{13}

Trust gates project-scope resource loading: "pi installs any missing packages
automatically on startup after the project is trusted," and a package can be
tried without installing via `pi -e` / `--extension` to a temporary directory
for the current run only. A controller materializing a project-scope
`.pi/settings.json` package entry therefore relies on prior project trust; it
cannot force-load a project package across the trust boundary.
[pi-packages]{11}

Resource enable/disable (extensions, skills, prompt templates, themes) is a
TUI-only surface: `pi config` starts in global settings and Tab switches to
project; `pi config -l` starts in project overrides. The `pi --help` text
describes `pi config` as the sole enable/disable mechanism, and no
non-interactive enable/disable flag is documented.
[pi-hooks-health-pi-runtime]{59} [pi-packages]{11}

## Update, removal, and ownership boundaries

Update and removal operate on the settings `packages` array via documented,
non-cache surfaces: `pi update <pkg>` / `pi update --all` / `--extensions`
for updates; `pi remove` (alias `pi uninstall`) for removal. The mutation
target is the inspectable settings file, not an opaque plugin cache; this
differs structurally from harnesses whose installed plugins live in a
versioned cache that is explicitly not an authoring or mutation surface.
[pi-packages]{11} [pi-hooks-health-pi-runtime]{59}

The `@hsingjui/pi-hooks` extension itself owns no persisted state. Its shared
context holds only in-memory fields: a `Set<string>` for session-start
dedupe, a transient `pendingUserPromptContext`, and a `stopHookActive` flag.
It reads `~/.pi/agent/settings.json` and `<cwd>/.pi/settings.json` via
`existsSync` + `readFileSync`; there is no write path in its config layer.
A controller that writes the `hooks` key therefore writes to a file the
extension only reads — the extension does not own or guard that key.
[pi-hooks-health-installed-package]{56}

## Interaction with `pi-mcp-adapter` in a compound profile

A compound Pi profile pairs `pi-hooks` (Claude hook compatibility) with
`pi-mcp-adapter` (MCP support). The two extensions are independently
versioned, independently installed/removed/updated, and independently
authored (`@hsingjui/pi-hooks` by `hsingjui`; `pi-mcp-adapter@2.11.0` by
Nico Bailon). `pi remove npm:@hsingjui/pi-hooks` removes only the hooks
package; the adapter remains, and vice versa.
[pi-hooks-health-mcp-adapter-installed]{60} [pi-hooks-health-pi-runtime]{59}

Their configuration-file ownership is disjoint. `pi-hooks` reads the `hooks`
key of `settings.json` family files; `pi-mcp-adapter` reads the `mcp.json`
family (`~/.config/mcp/mcp.json`, `~/.pi/agent/mcp.json`, project `.mcp.json`,
project `.pi/mcp.json`) and writes only to an observational metadata cache
(`~/.pi/agent/mcp-cache.json`). No file-config overlap exists for a controller
managing both. [pi-hooks-health-installed-package]{56}
[pi-hooks-health-mcp-adapter-installed]{60} [pi-mcp-adapter]{55}

There is, however, a behavioral coupling at the tool-event layer: `pi-hooks`
documents `mcp__.*` among the tool-name matchers for `PreToolUse` /
`PostToolUse` / `PostToolUseFailure`, so a configured tool hook can observe,
deny, or rewrite any tool whose Pi name begins with `mcp__` — i.e. tools
surfaced by the MCP adapter. In a compound profile, a `PreToolUse` hook with
matcher `mcp__.*` and `permissionDecision: "deny"` will gate the adapter's
MCP tools. {extends} A controller that manages both packages' configuration
must keep this coupling in view: hook config can block MCP-mediated actions,
including — by extension — the controller's own MCP tool calls.
[pi-hooks-health-installed-package]{56}

## Evidence that grants versus narrows compiled mutation authority

The facet-3 question for the campaign decision gate: which observations
**grant** a compiled Pi compound profile mutation authority, and which only
**narrow** the contract a controller must enforce?

**Granting evidence** {inferred: aggregates} (the package lifecycle is
observable and mutable through documented, inspectable, non-cache surfaces):

- Install/remove/update/list operate on `~/.pi/agent/settings.json` and
  `.pi/settings.json` — canonical settings files, not opaque caches.
  [pi-packages]{11} [pi-hooks-health-pi-runtime]{59}
- Identity is stable and documented: npm identity = package name, so
  `npm:@hsingjui/pi-hooks` and `npm:pi-mcp-adapter` are unambiguous handles
  for install/remove/update. [pi-packages]{11}
- The npm checkout root is inspectable and configurable (`PI_PACKAGE_DIR`
  override; version readable from the resolved `package.json`), giving a
  non-cache observation path for installed version.
  [pi-hooks-health-pi-runtime]{59} [pi-hooks-health-installed-package]{56}
- The two extensions have disjoint config-file ownership and independent
  update/removal — clean, non-conflicting ownership boundaries for a
  controller managing both. [pi-hooks-health-mcp-adapter-installed]{60}

**Narrowing evidence** {inferred: aggregates} (these constrain the contract
but do not themselves grant authority; a controller must enforce the
companion rules they imply):

- `pi list` reports presence + path only; it does not surface version, health,
  or enable state. {inferred: implies} The controller must read version from
  the resolved `package.json` and health from the `hooks` key separately.
  [pi-hooks-health-pi-runtime]{59}
- Package presence ≠ effective health: the extension is loaded yet inert when
  `hooks` is absent. {inferred: implies} Mutation authority over the
  `packages` array does not imply the hook behavior is active; the controller
  must also own the `hooks` key to assert effective hook health.
  [pi-hooks-health-installed-package]{56} [pi-hooks-health-pi-runtime]{59}
- Resource enable/disable is TUI-only (`pi config`); no non-interactive
  enable/disable is documented. {inferred: implies} A deterministic controller
  cannot enable/disable a package resource; it can only add/remove the
  packages entry and edit the `hooks` key directly.
  [pi-hooks-health-pi-runtime]{59} [pi-packages]{11}
- Version identity is npm-anchored with a divergent git HEAD (main `0.0.1` vs
  npm/installed `0.0.2`; no git tags). Update detection must key on npm
  `dist-tags.latest`; a git-HEAD freshness check produces false drift.
  [pi-hooks-health-npm-registry]{57} [pi-hooks-health-github-source]{58}
- The installed entry is unversioned (`npm:@hsingjui/pi-hooks`), so it floats
  to npm latest on `pi update`. {inferred: implies} A controller wanting
  determinism must rewrite the spec to a versioned form.
  [pi-packages]{11}
- The manifest peer-depends on `@earendil-works/pi-coding-agent: *` (unbounded)
  — there is no declared host-compatibility range to test against.
  {inferred: implies} A controller cannot statically verify Pi-version
  compatibility from the manifest alone.
  [pi-hooks-health-installed-package]{56}
  [pi-hooks-health-npm-registry]{57}
- Hook config can gate MCP tools (`mcp__.*` matcher), so in a compound profile
  hook configuration is a coupling point that can block the adapter's (and the
  controller's own) MCP tool calls. {extends}
  [pi-hooks-health-installed-package]{56}
- Trust gates project-scope loading; {inferred: implies} a controller cannot
  force-load a project package across the trust boundary.
  [pi-packages]{11}

## Disconfirming analysis

| Load-bearing proposition tested | Disconfirming search | Outcome |
|---|---|---|
| `pi list` reports installed version | Ran `pi list`; output is source-id + path only; `--help` says "from settings." | Confirmed absent; version must be read from the resolved `package.json`. [pi-hooks-health-pi-runtime]{59} |
| Package presence implies hook behavior is active | Inspected live settings: `hooks` key absent while package is present; extension's `loadSettings` returns undefined settings. | Rejected; presence and effective health are separate facts. [pi-hooks-health-pi-runtime]{59} [pi-hooks-health-installed-package]{56} |
| A non-interactive command enables/disables a package | Searched `pi --help` and the packages doc; only `pi config` (TUI) documented. | Rejected; no non-interactive enable/disable surface found. [pi-hooks-health-pi-runtime]{59} [pi-packages]{11} |
| Git HEAD reflects npm release currency for npm-sourced packages | Compared GitHub `main` manifest (`0.0.1`) with npm latest and installed (`0.0.2`); tags endpoint empty. | Rejected; the npm registry governs version for this package, not the git HEAD. [pi-hooks-health-github-source]{58} [pi-hooks-health-npm-registry]{57} |
| `pi-hooks` and `pi-mcp-adapter` share a config file | Compared config-file ownership: `settings.json` `hooks` vs `mcp.json` family; checked installed sources. | Rejected; ownership is disjoint. [pi-hooks-health-installed-package]{56} [pi-hooks-health-mcp-adapter-installed]{60} |
| The extension persists its own state under `~/.pi/` | Read `src/config.ts` and `src/hook-context.ts`; only reads and in-memory fields. | Rejected; no persisted state owned by the extension. [pi-hooks-health-installed-package]{56} |
| A versioned spec is required for install | Packages doc: unversioned `npm:pkg` is the default; versioned specs pin and skip updates. | Rejected; unversioned is accepted (but floats on update). [pi-packages]{11} |

## Contradictions

**Version surface divergence between source-of-record and release authority.**
The GitHub default-branch `package.json` reads `version: 0.0.1` while the npm
tarball and the installed copy both read `0.0.2`. Relationship: `tension`,
handles `pi-hooks-health-github-source` and `pi-hooks-health-npm-registry`
(with `pi-hooks-health-installed-package` corroborating the installed value).
The two surfaces disagree on the current version because the `0.0.2` publish
did not leave the default branch manifest in step. No merger is offered: npm
is authoritative for installed/release version; git HEAD is unreliable for
this package. [pi-hooks-health-github-source]{58}
[pi-hooks-health-npm-registry]{57} [pi-hooks-health-installed-package]{56}

**Declared license versus detected license.** The npm `package.json` declares
`"license": "MIT"`; GitHub repo metadata reports `license: null` (no LICENSE
file detected, and the `files` field would exclude one from the tarball).
Relationship: `qualifies`, handles `pi-hooks-health-installed-package` and
`pi-hooks-health-github-source`. The declarative license stands at the
manifest tier; there is no license file the registry or repo can detect.
[pi-hooks-health-installed-package]{56} [pi-hooks-health-github-source]{58}

**Package-entry merge rule versus hook-entry merge rule.** Pi's package
contract states that when the same package appears in global and project
settings, "the project entry wins" (override, or delta when `autoload: false`).
The `pi-hooks` extension's own `mergeHooks` concatenates global and project
`hooks` arrays per event so that **both scopes' hooks fire**. Relationship:
`incommensurable`, handles `pi-packages` and `pi-hooks-health-installed-package`.
The two rules govern different keys of the same settings file (`packages` vs
`hooks`) and cannot be reduced to a single merge semantics. A controller
managing a compound profile must apply project-wins to the `packages` array
and concatenate-and-merge to the `hooks` key.
[pi-packages]{11} [pi-hooks-health-installed-package]{56}

## Unknowns

- {ambiguous: enable-disable-persistence} `pi config` is the documented
  enable/disable surface but the underlying persisted representation of
  per-resource enable/disable state (the exact settings key/shape it writes)
  is not stated in the packages or settings documentation attested here.
  [pi-packages]{11} [pi-settings]{13}
- {ambiguous: load-failure-observation} No documented surface reports whether
  a loaded extension initialized successfully. A controller cannot distinguish
  a healthy extension from one that failed to register its hooks.
  [pi-hooks-health-pi-runtime]{59}
- {ambiguous: pi-list-json} `pi list` has no documented JSON output mode in
  the attested sources; a controller must parse the human-readable
  source-id + path lines.
  [pi-hooks-health-pi-runtime]{59}
- {ambiguous: peer-range-compat} The unbounded `*` peer dependency on
  `@earendil-works/pi-coding-agent` declares no compatibility range, so
  manifest-level Pi-version compatibility is untestable.
  [pi-hooks-health-installed-package]{56}

## Revisit if

- Pi adds a non-interactive enable/disable command or a `pi list --json`
  surface exposing version/health/enable state.
- `pi-hooks` begins persisting its own state files, or starts writing the
  `hooks` key programmatically (today it only reads).
- The GitHub repository gains release tags, or the default-branch manifest is
  brought into step with npm `dist-tags.latest`, changing which surface is
  authoritative for version.
- The package's peer dependency gains a bounded range against
  `@earendil-works/pi-coding-agent`.
- `pi-mcp-adapter` and `pi-hooks` add a shared configuration file or a
  documented ordering dependency between extension load order.
- Pi documents the persisted shape of per-resource enable/disable state.

## Acquisition candidates

- The `pi-mcp-adapter` npm registry document (`https://registry.npmjs.org/pi-mcp-adapter`)
  would let the orchestrator attest version-currency for the adapter half of
  the compound profile from the registry directly, rather than only from the
  installed manifest. (Acquisition is grounded: the installed `pi-mcp-adapter`
  version and the registry `dist-tags.latest` were both observed at the same
  version during this engagement; a dedicated registry attestation would
  harden the currency claim.)
- The Pi settings reference (`https://pi.dev/docs/latest/settings`) fetched in
  full beyond the existing terse `pi-settings` attestation would resolve the
  `enable-disable-persistence` unknown by exposing the exact settings key
  `pi config` writes.
