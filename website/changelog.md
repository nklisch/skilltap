---
title: Changelog
description: Release notes for every notable version of skilltap.
---

# Changelog

## v2.2.0 — V2 cutover: simplified security, deleted scaffolding

**BREAKING.** Config schema upgraded to V2. Run `skilltap migrate` once on
each machine. `loadConfig` hard-fails on legacy shapes with a hint pointing
at `migrate` — no silent fallback, no aliases. Migration is safe to re-run.

This release finishes the v2.0 redesign: removes the per-mode security
split, deletes the agent-mode plumbing for good, makes scope a typed flag,
adds typed positionals everywhere, lands plugin Capture with explicit
flags, and tracks standalone MCPs in the project manifest.

### Config changes

- `[security]` is now flat with three keys: `scan` (`semantic | static | none`), `on_warn` (`prompt | fail | install`), `trust` (glob array matched against tap name or source URL).
- `[scanner]` is a new sibling block holding operational config: `agent_cli`, `ollama_model`, `threshold`, `max_size`. Policy ("what should happen") and operation ("how to run the scan") are now cleanly separated.
- Removed: `[security.human]`, `[security.agent]`, `[[security.overrides]]`, `preset = ...`, `require_scan`, `[agent-mode]`, `[agent]`, `[registry].allow_npm`.
- Enum translations: `scan = "off"` → `"none"`; `on_warn = "allow"` → `"install"`.

### CLI changes

- `--scope project|global` replaces the `--project` / `--global` boolean pair on every lifecycle command (`install`, `remove`, `update`, `adopt`, `move`). `info` and `status` keep the boolean pair as a deliberate carve-out.
- `--also` is repeatable (`--also a --also b`); the comma-separated form is no longer supported.
- `--no-strict` is removed. Set `on_warn = "install"` in config (or pass `--strict` for a one-shot hard-fail) to control warn handling.
- `try <type> <source>` mirrors `install` and `remove` — every command takes a typed positional. `try <source>` (no type) is no longer accepted.
- `install <type> <source>` — typed positional everywhere. `install <source>` (no type) returns an error with a hint.
- Multi-plugin sources: `install plugin user/repo:plugin-name` selects one published plugin from a multi-plugin repo; `install plugin user/repo:*` installs every publishable plugin from that repo. Composes with `@ref` and URL forms.
- Plugin capture flags: `--force-capture` (auto-capture every same-source match, safe with `--yes`) and `--no-capture` (disable capture entirely; standalones stay independent). Mutually exclusive.
- `verify` removed — use `doctor skill <path>` (or `doctor plugin <path>`).
- `config security` flag set: `--scan`, `--on-warn`, `--trust-add`, `--trust-remove`, `--trust-list`. Old `--preset`, `--mode`, `--trust tap:n=preset`, `--remove-trust` flags are gone.
- `install mcp` honors smart-scope outside a git repo (defaults to `global`), matching `install skill` and `install plugin`.
- Smart-scope reporting: install prints `→ scope: <project|global> (inferred from cwd)` after the inference resolves, so the resolved scope is always visible.

### Manifest changes

- `skilltap.toml` and `skilltap.lock` track standalone MCP installs in `[[mcps]]` and `[[mcps.lock]]` tables. `skilltap sync` reconciles all three state types (skills, plugins, mcps) and writes manifest+lockfile entries from `install mcp` / `remove mcp`.

### Removed legacy command hints

`verify`, `link`, `unlink`, `enable`, `disable`, and `skills` each exit with
an explicit replacement hint (no more falling through to citty's generic
"unknown command"). Use `doctor`, `adopt`, `remove`, `toggle`, and the
top-level `info`/`adopt`/`move`/`remove` respectively.

### Removed code (no fallback, no alias)

- `policy-v2/` promoted to `policy/`, replacing the legacy `policy.ts`. `composeV2` → `composePolicy` (renamed; agent-mode plumbing stripped).
- `schemas/config-v2.ts` merged into `schemas/config.ts` — the V2 schema **is** the schema.
- `httpAdapter` — HTTP taps were already removed; the adapter was unreachable.
- `linkSkill` core function — its CLI surface was already gone.
- `searchSkillsRegistry`, `marketplaceSourceToRepo` deprecated wrappers.
- `info.ts` legacy `installed.json` + `plugins.json` fallback — the canonical store is `state.json` only.

### Bug fixes shipped in the same wave

- Lifecycle drift closed: `update`, `move`, `adopt`, `disable`/`enable`, plugin `toggle` all write `skilltap.toml` + `skilltap.lock` when present. Previously these wrote state but left the manifest desynced.
- `migrate` preserves existing `state.mcpServers` (no longer overwrites with `[]`).
- `try` loads config and threads `default_git_host`, so the `owner/repo` shorthand resolves through your configured host instead of silently defaulting to GitHub.
- `install mcp` honors smart-scope outside a git repo, no longer requiring an explicit `--scope` flag for the global case.
- `sync` no longer reports a spurious `ref-mismatch` for inline-table manifest entries (`{ ref = "main" }`) when the lockfile range is `*`.
- TUI fixes: Dashboard tab keys `1-4` switch tabs; Adopt screen handles Enter to confirm; Toggle screen renders the name step and the focus index correctly.
- `doctor --fix` exits 0 when fixes succeed; `--json` includes `info`, `fixDescription`, and `detail` fields per check.

### Migration

Run `skilltap migrate`. It translates v0.x and pre-v2.2 configs:

- Per-mode `[security.<mode>]` → flat `[security]` (stricter mode wins; both originals reported in warnings).
- `[[security.overrides]]` `preset = "none"` → `security.trust` glob entry.
- `[[security.overrides]]` `preset = "relaxed" | "standard" | "strict"` → dropped with a warning naming the match string.
- Operational scanner keys (`agent_cli`, `ollama_model`, `threshold`, `max_size`) extracted from per-mode blocks → new sibling `[scanner]` block.
- `[agent-mode]` and `[agent]` blocks → dropped with warnings; non-interactive use is driven by TTY detection + `--yes` + `--json`.
- `[registry].allow_npm` → dropped with a warning.
- `scan = "off"` → `"none"`. `on_warn = "allow"` → `"install"`.
- `installed.json` + `plugins.json` → consolidated into `state.json`. Existing `state.mcpServers` is preserved.
- HTTP taps → error before any writes; lists affected taps for manual handling.

```bash
# One shot — translates config + consolidates state files
skilltap migrate

# Verify migration was clean
skilltap doctor

# If issues remain, auto-repair what's safe
skilltap doctor --fix
```

Originals are renamed to `*.v1.bak` (e.g., `config.toml.v1.bak`, `installed.json.v1.bak`). `migrate` is idempotent.

If you used `--agent` or `SKILLTAP_AGENT=1` in scripts:

```bash
# Before (v2.1)
skilltap install <source> --agent

# After (v2.2) — typed positional, smart-scope, --yes for no prompts
skilltap install skill <source> --yes | cat
```

---

## v2.1.0 — Canonical store, manifest safety, comprehensive test + doc coverage

The v2.1 release makes `state.json` the one canonical store for installed
skills and plugins (replacing the v0.x `installed.json` + `plugins.json` pair),
closes correctness gaps in `sync` and `install`, and ships a full end-to-end
test suite and doc audit for the v2.0 surface. v0.x users transition
transparently — your data is read once from the legacy files and written
into `state.json` on the next install/update/remove. `skilltap migrate`
remains available for explicit one-shot migration; `skilltap doctor --fix`
cleans up the orphan legacy files afterward.

### Changed

- **`state.json` is the canonical state file.** `install`, `update`, `remove`,
  `disable`, `enable`, `move`, `adopt`, `link`, plugin operations all now
  write directly to `state.json` instead of `installed.json`/`plugins.json`.
  Existing v0.x users get a one-time transparent read fallback so no data
  is lost. The next save populates `state.json` and the fallback stops
  firing for that scope.
- **`--agent` flag and `SKILLTAP_AGENT=1` env var work with proper
  precedence.** `composePolicy` resolves agent mode as: `flags.agent` >
  `SKILLTAP_AGENT=1` > `[agent-mode]` config block. CLI startup checks
  (telemetry notice, update hint, skill-update reminder) short-circuit on
  the env var. The flag is wired into `install`, `update`, `tap install`,
  `skills remove`, and `skills enable`/`disable`. The `isAgentMode()` helper
  used by read-only commands (`disable`, `enable`, `toggle`, `plugin info`,
  `skills info`, etc.) honors the flag via direct argv inspection, so the
  documented precedence applies uniformly across every command.
- **`skilltap install` refuses (or auto-recovers) when `skilltap.toml` is
  corrupt.** Previously, install silently swallowed a corrupt manifest and
  left the project in a half-managed state: `state.json` updated, skill
  files placed, but manifest unchanged and lockfile missing the new entry.
  Now: agent mode exits 1 with a pointer to `skilltap doctor --fix` (CI/scripts
  must never silently mutate user files); interactive mode auto-recovers
  (backs up the corrupt file to `skilltap.toml.bak`, resets to empty,
  continues the install).
- **`skilltap sync` now requires a project root.** Running `sync` outside
  any git repo or project previously reported a misleading "trivially
  in-sync" no-op (the `if (!projectRoot)` guard was dead code because
  `tryFindProjectRoot()` fell back to cwd). It now exits 1 with a clear
  error pointing the user to a directory containing `.git` or
  `skilltap.toml`. Replaced internally with a new `findManifestRoot()`
  helper that returns null when no manifest ancestor exists.

### Added

- **Doctor v2.0 checks (10–15).** `skilltap doctor` now runs 15 checks total
  (was 9). The six new v2.0 checks: `state.json` validity, manifest drift,
  lockfile drift, plugin manifest validity (`.skilltap/<name>.toml`), MCP
  injection consistency, and v0.x file orphans. **Manifest and lockfile
  corruption are now `fixable: true`** — `--fix` backs the corrupt file to
  `.bak` and writes a fresh empty file using the same `recoverManifest`/
  `recoverLockfile` helper the install preflight uses. The `v0.x file
  orphans` check detects when `state.json` is populated AND legacy
  `installed.json`/`plugins.json` are still on disk; `--fix` renames each
  orphan to `<file>.v1.bak`. Pre-migration users (empty state, populated
  legacy file) are intentionally not flagged — their fallback is still
  active.
- **`skilltap doctor` runs from real git repos correctly.** Fixed a
  `Bun.file('.git').exists()` bug in the project-root detection that made
  `skilltap status`/`doctor` always report "no project root" when run
  inside a real git repo. Replaced with `lstat`.
- **Comprehensive v2.0/v2.1 test coverage.** 50+ new CLI subprocess and
  unit tests for previously-uncovered surface: `resolveScope`
  smart-scope-default (all 5 branches), `sync` drift workflow with
  `--apply`/`--strict`/`--json`, `try` never-writes invariant (the safety
  property that makes try usable on untrusted code), `migrate` idempotency
  and HTTP-tap abort, doctor `--fix` repair workflow, agent-mode precedence
  chain, HTTP-tap stderr warning, install-never-writes-`installed.json`
  invariant, first-time global install (smart-default outside git),
  skill remove drops from manifest + lockfile, `status --json`.

### Documentation

- **Comprehensive website audit.** All 15 website pages (11 guides + 4
  reference) audited for v2.1 consistency. 4 Blocker fixes and ~10
  High-priority gaps closed. Key fixes: removed the obsolete scope-prompt
  UI mock from getting-started (smart-scope-default infers scope silently);
  fixed `update --all` (doesn't exist, 4 occurrences) in teams guide;
  corrected `[security]` block usage in teams + config-options worked
  examples (per-mode keys belong in `[security.human]`/`[security.agent]`,
  not the shared `[security]` block); corrected the false claim that
  `install` auto-migrates v0.x state. Added complete CLI reference sections
  for `try`, `migrate`, `toggle`/`enable`/`disable`, `mcp:` source format,
  bare `skilltap` status. Doctor check table updated to 15 (was 9). Added
  missing config keys (`builtin_tap`, `default_git_host`,
  `updates.show_diff`).
- **Foundation doc updates.** SPEC.md §v2.0 Sync gained the project-root
  requirement; §`skilltap install` gained a Manifest Preflight callout;
  §v2.0 Doctor Upgrades documents the new fixability for manifest and
  lockfile checks. UX.md install + sync flows updated with the corrupt-
  manifest handling and the no-project-root error case. CLI hint
  correctness: `skilltap config set agent-mode.enabled true` now emits a
  hint mentioning the `--agent` flag and `SKILLTAP_AGENT=1` env var
  alongside the persistent wizard.
- **End-to-end test design document.** New `docs/designs/completed/e2e-v2.md`
  enumerates 27 golden-path tests across 13 user journeys plus 20
  adversarial tests across 5 failure-mode categories, with spec
  citations, fixture requirements, and prioritized implementation order.
  ~18 of the highest-priority tests landed in this release; the rest
  documented for a follow-up pass.

### Internals

- **Module-graph cleanup.** Extracted `getConfigDir`/`ensureDirs` to a leaf
  module (`core/src/dirs.ts`) and `SKILLTAP_AGENT` env-var check to its
  own helper (`core/src/agent-env.ts`). The previous circular import
  between `config.ts` ↔ `state/save.ts` (worked around with dynamic
  `await import()` calls) is gone — all callers use clean static imports.
- **Net –355 lines of code.** Dual-write scaffolding from the early v2.1
  cutover (`sync-from-v1.ts`, `read-bridge.ts`) deleted now that
  `state.json` is canonical.
- **`manifest/recover.ts`** — shared `recoverManifest()` /
  `recoverLockfile()` helpers used by both install preflight and doctor
  `--fix`. Backup-then-replace pattern mirrors the existing `state-v2`
  doctor fixer.
- **Lint: 104 errors / 279 warnings → 0 / 0.** Project-wide `bun run
  check` safe-fix pass + targeted unsafe-fix passes for unused
  imports/variables/templates + per-line documentation of every remaining
  `!` non-null assertion with its runtime guard. macOS `/tmp` →
  `/private/tmp` symlink bug in `makeTmpDir` fixed mid-cleanup, resolving
  6 long-standing test failures.

### Known gaps (deferred to v2.2)

- **v0.x schema deletion** — `schemas/installed.ts` and `schemas/plugins.ts`
  are still imported (mostly for type re-exports — `InstalledSkill`,
  `PluginRecord` shapes are reused as-is in `state.json`). Wholesale
  deletion is deferred to v2.2 after a release window for users to clear
  orphans via `skilltap doctor --fix`.
- **`[agent-mode]` config block schema retirement** — the block is read
  for back-compat but won't be the documented entry point in v2.2.
- **Plugin authoring guide** — `creating-skills.md` doesn't yet have a
  section on publishing plugins via `.skilltap/<name>.toml` (`[[skills]]`,
  `[[servers]]`, `[[agents]]`, `publish = true`). Tracked for a focused
  doc pass.

---

## v2.0.0-rc.1 — Tooling-surface redesign

The v2.0 release reshapes how you manage skills and plugins around a project
manifest, simplifies the security config, drops "agent mode" as a separate
mental model, and adds Claude Desktop as an MCP target. Existing v0.x setups
keep working — run `skilltap migrate` when ready.

### Added

- **Project manifest (`skilltap.toml` + `skilltap.lock`)** — declare your
  project's skill and plugin dependencies; commit the files; teammates run
  `skilltap sync --apply` for parity. `install` and `remove` keep both files
  in sync automatically.
- **`skilltap sync`** — show drift between manifest, lockfile, and installed
  state. `--apply` executes the plan via existing install/remove machinery.
  `--strict` stops at the first failure.
- **`skilltap status`** (also bare `skilltap`) — text dashboard of skills,
  plugins, MCP injection per agent, taps, drift. `--json` for scripting.
- **`skilltap try <source>`** — read-only preview of any source. Clones to a
  temp dir, parses manifests, runs static security scan, prints summary,
  cleans up. `--skip-scan` and `--json` flags supported.
- **`skilltap migrate`** — one-shot upgrade from v0.x state. Reads
  `installed.json` + `plugins.json` + v0.x config keys, writes consolidated
  `state.json` and v2 config layout, renames originals to `.v1.bak`. Refuses
  to migrate HTTP taps (no longer supported) with a list of which to fix.
- **Top-level `toggle` / `enable` / `disable` `<plugin>[:component]`** —
  direct component addressing. Bare plugin name opens a multiselect picker
  (toggle) or applies bulk action (enable/disable). Fully back-compat:
  existing `skilltap plugin toggle` keeps working.
- **`.skilltap/<plugin>.toml`** — native v2.0 plugin manifest format. Place
  one or more files in `.skilltap/` with `publish = true` to make a repo
  installable as one or more plugins. Multi-plugin repos supported via
  `user/repo:plugin-name` syntax. Existing `.claude-plugin/plugin.json` and
  `.codex-plugin/plugin.json` formats continue to be read.
- **Claude Desktop MCP target** — added alongside `claude-code`, `cursor`,
  `codex`, `gemini`, `windsurf`. Resolves to the OS-specific config path on
  macOS and Linux; Windows deferred.
- **Smart scope default** — inside a git repo, `install` defaults to
  `--project`; outside, `--global`. Removes the scope prompt for the common
  case. Override explicitly with `--global` / `--project`.
- **Component-ref syntax** — `skilltap toggle dev-toolkit:test-generator`
  addresses one component directly without going through a picker. Same
  pattern for `enable` and `disable`.
- **`--agent` flag (and `SKILLTAP_AGENT=1` env var)** — non-interactive mode
  for AI agents, CI, scripts. Replaces (but stays compatible with) the
  config-only `[agent-mode]` block.
- **Doctor v2 checks** — `skilltap doctor` now also checks `state.json`
  validity, manifest/lockfile drift, `.skilltap/<plugin>.toml` validity, and
  MCP injection consistency (with auto-fix for orphan agent-config entries).
- **`skilltap install mcp:<source>` / `skilltap remove mcp:<source>`** —
  install or remove MCP servers from any source (git, local, npm) without
  going through the skill machinery. Useful for one-off MCP-only repos.
  Servers land under `state.json::mcpServers[]` namespaced as
  `skilltap:<slug>:<server-name>`; remove prunes both state and the target
  agents' MCP configs.

### Simplified

- **Single state file** — `installed.json` and `plugins.json` consolidated
  into `state.json` per scope. v0.x users see a soft startup hint pointing
  at `skilltap migrate`; the legacy files remain readable as a one-time
  fallback for unmigrated setups.
- **Top-level command surface** — daily-use commands (`install`, `remove`,
  `update`, `list`, `info`, `enable`, `disable`, `toggle`, `sync`,
  `status`) sit at the root; `skills/`, `plugin/`, `tap/` subgroups are
  retained for less-common operations.

> **Note on security config:** The original v2.0 design called for a
> collapsed single `[security]` block with three keys (`scan`/`on_warn`/`trust`)
> and `on_warn = "install"` as the default — these features were
> documented in earlier release-note drafts but were **deferred** during
> Phase 31c-c-2. Shipped v2.0/v2.1 retains the v0.x per-mode structure
> (`[security.human]` and `[security.agent]`) with `--agent` activating
> the agent-mode block. The preset table, `[[security.overrides]]`,
> and `require_scan` are all still active. See
> [SPEC.md → Security Scanning](https://github.com/nklisch/skilltap/blob/main/docs/SPEC.md#security-scanning)
> for the canonical v2.2 schema (per-mode blocks, presets, `require_scan`,
> and `--agent` were all removed in v2.2).

### Removed

- **HTTP registry tap adapter** — taps are git-only. Existing config entries
  with `type = "http"` are silently filtered with a one-time warning;
  `skilltap migrate` lists them as needing manual conversion or removal.
- **`UpdateTapResult.http`** — dead-loaded field with no production
  consumers; dropped from the public API.

### Changed

- **CLI command surface** — top-level shortcuts for `sync`, `status`, `try`,
  `migrate`, `toggle`, `enable`, `disable`. Existing paths (`skilltap plugin
  toggle`, `skilltap skills remove`, etc.) still work as silent aliases.
  Shell completion scripts (bash, zsh, fish) updated.

### Migration

```bash
skilltap migrate
```

The command:

- Consolidates `installed.json` + `plugins.json` → `state.json`.
- Translates v0.x's old top-level `[security]` keys (`scan`, `on_warn`,
  `require_scan`) **into** per-mode `[security.human]` + `[security.agent]`
  blocks. (Per-mode structure was kept; the original "collapse to flat
  [security]" design was deferred.) Existing per-mode configs pass through
  unchanged.
- Keeps `[agent-mode]` intact (the originally-planned `[agent]` block with
  `default`/`block` was deferred). The new `--agent` flag and
  `SKILLTAP_AGENT=1` env var are alternative per-invocation entry points.
- Keeps `[[security.overrides]]` intact (the originally-planned
  `trust = []` glob array was deferred).
- Renames originals to `<file>.v1.bak` (e.g., `installed.json.v1.bak`).
- Aborts before any writes with a list if any HTTP taps are configured
  (must be converted to git or removed before re-running).

If you're not ready to migrate, v2.0 also reads v0.x state — manifest writes
and v2 commands work alongside the legacy paths. The full cutover (v0.x path
retirement) lands in v2.1+.

### Known gaps

- **v0.x reader retirement** — `installed.json`/`plugins.json` are still
  read AND written by install/update/remove for back-compat. The full
  cutover to state.json reads — and retirement of the `[agent-mode]` config
  block — lands in v2.1.

---

## v0.10.8

### Fixes

- **Plugin detection for direct install** — `skilltap install nklisch/skills` (or any
  direct git/GitHub source) now detects plugin manifests and prompts whether to install
  as a plugin. Previously the plugin manifest was silently ignored and only skills were
  installed. Tap-based resolution was unaffected.

- **HTTP MCP servers** — plugins that declare HTTP MCP servers now have them fully
  injected into target agent configs as `{ url, headers? }` entries. Previously HTTP
  servers were parsed from the manifest but silently dropped at install time. Toggle and
  remove also work correctly for HTTP servers.

---

## v0.10.7

### Fixes

- **Shell completions — plugin command** — `plugin` / `plugins` was registered in the
  CLI but missing from all shell completion scripts. Bash, zsh, and fish now complete
  `plugin list`, `plugin info`, `plugin toggle` (with `--skills`, `--mcps`, `--agents`),
  and `plugin remove`.
- **Shell completions — missing flags** — several flags were absent from completions:
  `verify --all`, `config --reset`, `skills info --json` (and the `info` alias),
  `tap add --type`, `tap remove --yes`, `tap list --json`.
- **Shell completions — status description** — corrected to "Show agent mode status and
  configuration" across zsh and fish.

---

## v0.10.4

### Fixes

- **Nested frontmatter metadata** — `skilltap verify` no longer fails with
  "expected record, received string" when a skill's frontmatter contains a
  nested `metadata:` block. The parser now correctly preserves nested
  key-value pairs as an object instead of flattening them to the top level.
  (Fixes [#14](https://github.com/nklisch/skilltap/issues/14))

---

## v0.10.1

### Fixes

- **macOS self-update quarantine** — downloaded binary now has the `com.apple.quarantine`
  extended attribute stripped before replacing the running binary, preventing Gatekeeper
  from killing the updated binary on macOS.
- **macOS code signature** — ad-hoc sign (`codesign -s -`) the binary after download in
  both self-update and install script. Cross-compiled Mach-O binaries from CI lack any
  code signature, which Apple Silicon rejects with "load code signature error 4".

---

## v0.10.0

### Plugin Support

- **Plugin detection and parsing** — skilltap now reads Claude Code (`.claude-plugin/plugin.json`)
  and Codex (`.codex-plugin/plugin.json`) plugin formats, extracting skills, MCP server configs,
  and agent definitions into a normalized manifest.
- **Plugin install** — `skilltap install <repo>` auto-detects plugins and installs the portable
  subset (skills + MCP servers + agent definitions) across all target agent platforms. MCP servers
  are injected into each agent's config file (Claude Code, Cursor, Codex, Gemini, Windsurf) with
  namespacing (`skilltap:<plugin>:<server>`) and automatic backup.
- **Plugin management** — new `skilltap plugin` command group: `list`, `info`, `toggle`
  (enable/disable individual skills, MCPs, or agents within a plugin), and `remove`.
- **Agent definitions** — plugin agent `.md` files are placed in `.claude/agents/` (Claude
  Code-only for now, extensible to other platforms).
- **plugins.json** — installed plugins tracked in `~/.config/skilltap/plugins.json` (global)
  or `{project}/.agents/plugins.json` with per-component active/inactive state.

### Tap-Defined Plugins

- **Inline plugin definitions in tap.json** — taps can now define plugins in a `plugins` array
  with inline skills, MCP servers, and agent definitions. Content files live in the tap repo itself.
- **`skilltap install tap-name/plugin-name`** — new install syntax for tap-defined plugins. If
  the first path segment matches a configured tap name, resolves the plugin from that tap directly.
- **Marketplace plugin auto-detection** — when using a Claude Code marketplace repo as a tap,
  plugins with relative-path sources and `.claude-plugin/plugin.json` are automatically treated
  as full plugins (not just skill entries).

### Improvements

- `skilltap find` shows `[plugin]` badge for plugin entries in search results.
- `skilltap status` / `--json` includes plugin count.
- `tap.json` `plugin: true` flag on skill entries marks them as plugins for badge display.

---

## v0.9.14
- Fixed `skilltap update` auto-removing disabled skills as "stale records". Disabled
  skills are now skipped entirely during orphan detection — missing install directories
  and symlinks are expected for disabled skills and should not trigger removal.

---

## v0.9.12
- Fixed `skilltap update` crashing with `cp: cannot stat '...': No such file or directory`
  when a skill's subdirectory was removed from an upstream multi-skill repo. The update
  now detects this and offers to remove the stale skill record instead of crashing.
- Fixed `skilltap update` and `install` crashing or showing false "already installed"
  conflicts when an installed.json record exists but the skill directory has been manually
  deleted. Stale records are now detected and cleaned up before the operation proceeds.
- Fixed `skilltap remove` silently succeeding when the skill directory was already gone;
  now reports that only the record was cleaned up.
- Added support for Claude Code marketplace repos (`.claude-plugin/marketplace.json`) as
  taps. `skilltap tap add owner/repo` now works with any Claude Code plugin marketplace.
- Scanner now discovers skills in `plugins/*/skills/*/SKILL.md` (Claude Code plugin layout)
  without requiring a deep scan.

---

## v0.9.11
- Fixed scanner not detecting skills placed directly at `skills/SKILL.md` (flat layout).
  Previously only `skills/<name>/SKILL.md` (subdirectory layout) was recognized.
- Fixed `SKILLTAP_INSTALL` env var not being passed through to the install script.

---

## v0.9.10
- Fixed skill install failing on macOS with "no such file or directory" during copy.
  The scanner's symlink resolution (`/tmp` → `/private/tmp`) caused a path mismatch
  with the install cache, so copied paths pointed at the deleted temp dir.

---

## v0.9.9
- Fixed scanner incorrectly matching lowercase `skill.md` as `SKILL.md` on macOS
  (case-insensitive filesystem). All SKILL.md existence checks now use `readdir` +
  exact string matching instead of `Bun.file().exists()`.
- Root `SKILL.md` no longer short-circuits scanning — repos with both a root skill
  and nested skills under `.agents/skills/` now discover all skills correctly.

---

## v0.9.7
- Fixed `skilltap tap install` failing with "Skill not found in repo" on macOS when
  installing skills from repos that use `.agents/skills/` layout. Bun.Glob silently
  skips dot-prefixed directories in cross-compiled macOS binaries; replaced with
  `readdir`-based scanning that works reliably across platforms.
- Added debug logging to the skill scanner (enable with `SKILLTAP_DEBUG=1`).

---

## v0.9.6
- Fixed `skilltap update` crashing on skills installed from local git paths whose source directory was later deleted. Records with a local filesystem path as `repo` now gracefully skip with `"local"` status instead of hard-failing on git fetch.

---

## v0.9.5
- Fixed `skilltap install` erroring on conflict when no `onAlreadyInstalled` callback is provided; previously silently swallowed the conflict. Callers must now explicitly handle the case via the callback. Agent mode now auto-updates on reinstall (matching `--yes` behavior).
- Fixed `skilltap install <local-path>` failing when the path is not a git repository. Non-git local directories are now copied directly and recorded with `repo: null` (skipped during update). Git-based local paths continue to clone and support updates.
- Added comprehensive lifecycle test suite (24 tests) covering the full skill journey — install → update → disable → enable → move → remove — across git, npm, local, adopted, and linked sources.

---

## v0.9.4
- Fixed `skilltap update` erroring with "git fetch failed: fatal: not a git repository" on adopted local skills that have no git remote. These skills are now silently skipped during update with a "local (no remote)" status.
- Fixed Homebrew tap name in install instructions.

---

## v0.9.3
- Fixed `install`, `find`, `tap list`, and `skills info` failing with "No taps
  configured" on a fresh install. The built-in tap is now cloned on first use
  in all commands that resolve tap names, not just `tap install`.

---

## v0.9.2
- Fixed `skills adopt` (and skill discovery in general) crashing with ENOENT when a
  broken symlink exists in an agent skills directory. Broken symlinks are now silently
  skipped during discovery.

---

## v0.9.1
- Fixed symlink creation to gracefully handle existing paths: replaces stale symlinks
  pointing to the wrong target, real directories, and leftover files — instead of
  failing with EEXIST.
- Symlink creation is now idempotent when the path already points to the correct target.

---

## v0.8.0
**Security config redesign and `skills` command group.**

### Features
- Added `skilltap skills` command group with three subcommands:
  - `skills list` — unified view of all installed skills across global and project scopes.
  - `skills adopt` — bring an existing skill directory under skilltap management.
  - `skills move` — relocate a skill between scopes or agents.
- Added `skilltap config security` command and interactive wizard for configuring security settings.
- Redesigned security configuration: per-mode settings (interactive vs. agent), presets (`strict`, `standard`, `permissive`), and per-tap trust overrides replace the previous single-level config.
- Updated the config wizard and agent-mode wizard to surface the new per-mode security options.
- Added security configuration section to the Getting Started guide.

### Internal
- Added `core/src/skills/discover`, `adopt`, and `move` modules backing the new command group.
- Added shell completions for `config security` and the `skills` subcommands.
- Added tests for security config redesign and skills command group spec coverage.

---

## v0.7.16
- Removed `npx` references from docs; fixed GitHub Releases URL.

## v0.7.15
**Authentication overhaul.**
- Replaced direct GitHub API HTTP calls with `git` CLI auth — installation now uses your existing git credentials (SSH keys, git credential helpers, etc.) with no extra token setup.
- Added `default_git_host` config key to set a preferred git host.

## v0.7.13
- Added `skilltap tap info <name>` command to inspect a registered tap and its available skills.

## v0.7.12
- Added 8-second timeout to registry fetch to prevent test and install hangs on slow networks.

## v0.7.8 – v0.7.9
**Multi-skill remove improvements.**
- `remove` now accepts a `--global` flag to remove globally-installed skills instead of project-scoped ones.
- Fixed `projectRoot` scoping so removal targets the correct installed.json.
- Disambiguation prompt added when a skill name matches across multiple taps in interactive picker.

## v0.7.7
- Fixed a bug where installing from multiple sources in a single command would duplicate the first source's skills.

## v0.7.5 – v0.7.6
**Multi-source install.**
- `skilltap install` now accepts multiple source arguments in one invocation.
- When `skillNames` are specified they take precedence over the interactive picker.
- Updated docs and shell completions for multi-source usage.

## v0.7.3
- Fixed `tap install` incorrectly showing stale cached skills; now pre-filters to the requested skill and clears the cache before resolving.

## v0.7.2
- Fixed semantic scanning not wiring through `find`, `tap install`, and install-update code paths — warnings were silently dropped in these flows.

## v0.7.1
- Fixed `self-update` incorrectly reporting "already up to date" when the update cache was absent or stale.

## v0.7.0
**Install UX and architectural cleanup.**
- Refactored toward single-source definitions across agent metadata and config enum arrays.
- Install now shows per-step progress UI.
- `tap install` refreshes the tap on update so you always see current skill versions.
- `doctor` no longer falsely flags linked skills as orphans or wrong-target.
- `tap install` pre-selects already-installed skills so deselecting one removes it.
- Fixed zsh multi-remove completion; dropped phantom update flags from completions.

---

## v0.6.6
- Semantic scan progress is now shown during `update` — no more silent wait on large skill sets.

## v0.6.5
**Agent selection and install-on-existing behavior.**
- `find` and `tap install` now prompt for agent selection when no default agent is configured.
- Installing a skill that already exists triggers an update instead of failing.
- Update refreshes symlinks even when the skill is already at the latest commit.

## v0.6.4
- Fixed a Bun crash when the terminal resizes during the config wizard.

## v0.6.3
- Added anonymous install telemetry (opt-in).
- Added a persistent footer bar with context-aware keybinding hints to interactive prompts.

---

## v0.5.9
- Added `skilltap config edit` command to open the config file in `$EDITOR`.

## v0.5.7
- Fixed 11 open issues identified via agent-mode audit: edge cases in install, remove, update, and scan flows.

## v0.5.5 – v0.5.6
**Per-project doctor.**
- `doctor` now checks the project-local `installed.json` in addition to the global one.
- `installed.json` is designed to be committed to version control, enabling team-wide skill tracking.
- Fixed `self-update` binary replacement to use `cp + rm` instead of `mv` to avoid cross-device failures.

## v0.5.1
- Fixed `verify`: bare skill name resolution, `--all` flag, and YAML block scalar parsing.

## v0.5.0
**Tap install, multi-select, and GitHub shorthand.**
- Added `skilltap tap install` subcommand — browse and install skills from a registered tap interactively.
- `find` search prompt now supports multi-select mode.
- `tap add` now accepts GitHub shorthand (`owner/repo`) in addition to full URLs.
- `remove` accepts multiple skill names in one command; shows interactive multiselect when no name is given.
- Added `--force` flag to `self-update` to bypass the update cache.

---

## v0.4.0 – v0.4.4
**Interactive search improvements.**
- `find -i` shows descriptions alongside skill names in search results.
- Reactive fuzzy search prompt using fzf-style matching.
- Results show source and install count.
- Fixed cross-device binary replacement (`mv` → `cp + rm`).
- Added debug logging for shell command errors.

---

## v0.3.9
- Reactive search prompt with fzf fuzzy matching for `find -i`.
- Reduced static scanner false positives; warning output now includes surrounding context.
- Picker items show source and install count.

## v0.3.8
- `find` is now TTY-aware: runs interactive search flow in a terminal, falls back to plain output in pipes/scripts.

## v0.3.5 – v0.3.7
**Find overhaul and registry switch.**
- `find` supports multi-word queries and sorts results by install count.
- Added `--local` flag to search only locally-installed skills (no network).
- Replaced npm search with the [skills.sh registry](https://skills.sh).
- Added `config get` / `config set` commands for reading and writing individual config keys from the CLI.
- Added first-run telemetry consent prompt; installs now emit an anonymous event.
- Shell completions now document `config get/set`, registry config, and install flags.

## v0.3.3 – v0.3.4
**Telemetry, find scoring, and PATH setup.**
- `find` automatically falls back to npm search and uses scored multi-token ranking.
- Interactive `find` uses fzf for real-time filtering.
- Installer script now writes a `PATH` export to the user's shell profile automatically.
- Added `telemetry` command and `[telemetry]` config section.
- Added team/org messaging to landing page and docs.

## v0.3.2
**Self-update, status, and install UX.**
- Added `skilltap self-update` to update the binary in-place from GitHub Releases.
- Added `skilltap status` to show currently installed skills and their state.
- Improved install conflict handling: clearer prompts, agent selection during install.
- Moved skills to `.agents/skills/`; symlinks to `.claude/skills/` and `CLAUDE.md`.
- Added [nklisch/skilltap-skills](https://github.com/nklisch/skilltap-skills) as the official tap.
- Linked docs to the official Agent Skills specification.
- Added interactive PTY test harness for `@clack/prompts` flows.

## v0.3.1
- Added install confirmation prompts.
- Fixed inaccuracies found by implementation audit of docs/spec.
- Improved terminal demo: accurate security warning format, interactive scope/agent prompts.

## v0.3.0
**Shell completions, doctor, create/verify, and first npm release.**
- Added shell completions for bash, zsh, and fish (`skilltap completions`).
- Added `skilltap doctor` to diagnose broken symlinks, missing installs, and config issues.
- Added `skilltap create` (scaffold a new skill) and `skilltap verify` (validate SKILL.md).
- Published to npm; added install script and Homebrew formula.
- Added cross-platform release workflow and semantic scan landing page animations.

---

## v0.2.0
**Registry taps, npm skills, trust system, and distribution.**
- Added HTTP registry tap support — point a tap at a hosted `tap.json` index.
- Added npm source adapter — install skills published to npm with `npm:<package>`.
- Added trust resolution system: skills from trusted taps skip re-confirmation.
- Added interactive agent selection prompt during install when multiple agents are detected.
- Added install script, Homebrew formula, and GitHub Releases workflow.
- Added full CI pipeline.
- Security: overlap chunks to catch cross-boundary split attacks; hardened semantic prompt.

---

## v0.1.0
**Initial release.**
- Core install/remove/update flow for agent skills from any git source.
- Static security scanner with 7 pattern detectors (trojan source, homoglyphs, hidden Unicode, suspicious base64, etc.).
- Semantic scan via local agent CLI (Claude Code, Cursor, Ollama, etc.).
- Source adapters: git URL, GitHub shorthand, local path.
- Agent symlinks: Claude Code → `.claude/skills/`, Cursor → `.cursor/skills/`, etc.
- Tap management: `tap add`, `tap list`, `tap remove`, `tap update`.
- `find` command for skill discovery.
- Config wizard with TOML config file.
- Agent mode for non-interactive use in automation.
