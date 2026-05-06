---
title: Changelog
description: Release notes for every notable version of skilltap.
---

# Changelog

## v2.1.0 — Canonical store + transparent v0.x migration (unreleased)

The v2.1 release retires the dual-source-of-truth state model from v2.0.
`state.json` is now the canonical store for installed skills and plugins.
Existing v0.x users transition transparently — your data is read once from
`installed.json`/`plugins.json` and written into `state.json` on the next
install/update/remove. No explicit `skilltap migrate` is required, though
the command remains available.

### Changed

- **`state.json` is the canonical state file.** `install`, `update`, `remove`,
  `disable`, `enable`, `move`, `adopt`, `link`, plugin operations all now
  write directly to `state.json` instead of `installed.json`/`plugins.json`.
  Existing v0.x users get a one-time transparent read fallback so no data
  is lost. The next save populates `state.json` and the fallback stops
  firing for that scope.
- **`--agent` flag and `SKILLTAP_AGENT=1` env var work as documented.** v2.0
  advertised both as the modern way to enter agent mode, but the CLI only
  honored the legacy `[agent-mode]` config block. `composePolicy` now
  resolves agent mode with proper precedence: `flags.agent` >
  `SKILLTAP_AGENT=1` > config block. CLI startup checks (telemetry notice,
  update hint, skill-update reminder) also short-circuit on the env var.

### Added

- **Doctor `v0.x file orphans` check.** `skilltap doctor` now runs 15 checks
  total. The new check detects when `state.json` is populated AND legacy
  `installed.json`/`plugins.json` are still on disk (a common state after
  a transparent migration). `--fix` renames each orphan to `<file>.v1.bak`.
  Pre-migration users (empty state, populated legacy file) are intentionally
  not flagged — their fallback is still active.
- **`skilltap doctor` runs from real git repos correctly.** Fixed a
  `Bun.file('.git').exists()` bug in the project-root detection that made
  `skilltap status`/`doctor` always report "no project root" when run inside
  a real git repo. Replaced with `lstat`.

### Internals

- **Module-graph cleanup.** Extracted `getConfigDir`/`ensureDirs` to a leaf
  module (`core/src/dirs.ts`) and `SKILLTAP_AGENT` env-var check to its own
  helper (`core/src/agent-env.ts`). The previous circular import between
  `config.ts` ↔ `state/save.ts` (which had been worked around with dynamic
  `await import()` calls) is gone — all callers use clean static imports.
- **Net –355 lines of code.** The dual-write scaffolding from the early
  v2.1 cutover (sync-from-v1.ts, read-bridge.ts) is dead code now that
  `state.json` is canonical. Deleted.

### Known gaps

- **v0.x schema deletion** — `schemas/installed.ts` and `schemas/plugins.ts`
  are still imported (mostly for type re-exports — `InstalledSkill`,
  `PluginRecord` shapes are reused as-is in `state.json`). Wholesale deletion
  is deferred to v2.2 after a release window for users to clear orphans
  via `skilltap doctor --fix`.

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

- **Security config** — single `[security]` block with three keys: `scan`
  (`semantic`/`static`/`none`), `on_warn` (`prompt`/`fail`/`install`),
  `trust = []` (glob allowlist of taps and source URLs that skip scanning).
  Removed the `[security.human]`/`[security.agent]` per-mode split, the
  preset table (`none`/`relaxed`/`standard`/`strict`), and
  `[[security.overrides]]`. `migrate` translates v0.x configs.
- **Default `on_warn` = `install`** — security warnings are reported but no
  longer block by default. Set `on_warn = "fail"` to restore strict
  behavior, or `prompt` for the v0.x interactive style.
- **Single state file** — `installed.json` and `plugins.json` consolidated
  into `state.json` per scope. v0.x users see a soft startup hint pointing
  at `skilltap migrate`.

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
- Translates `[security.human]`/`[security.agent]` → flat `[security]` block.
- Translates `[agent-mode] enabled = true` → `[agent].default = true`.
- Translates `[[security.overrides]] preset = "none"` → `[security].trust`
  entries. Other presets are dropped with a warning (less expressive in v2.0).
- Errors with a list if any HTTP taps are configured (must be converted to
  git or removed before re-running).

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
