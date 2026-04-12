---
title: Changelog
description: Release notes for every notable version of skilltap.
---

# Changelog

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
