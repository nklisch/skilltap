---
description: Diagnose and auto-repair skilltap environment issues. Checks git, config file, disk space, and agent symlinks. Use --fix to auto-repair or --json for CI.
---

# Doctor

`skilltap doctor` is a diagnostic command that checks your environment, configuration, and installed state. Run it when something isn't working — it surfaces problems before they become cryptic errors during install or update.

```bash
skilltap doctor
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Auto-repair issues where safe to do so |
| `--json` | Machine-readable JSON output (for CI/scripting) |

## What It Checks

Doctor runs a series of independent checks sequentially. A failure in one doesn't skip the rest. Each check produces a **pass** (◇), **warning** (⚠), or **failure** (✗).

### 1. Git

Verifies `git` is on your PATH and is a recent enough version.

- **Fail**: git not found → install it from [git-scm.com](https://git-scm.com)
- **Warn**: git < 2.25 → shallow clone (`--filter`) may not work correctly

### 2. Config File

Checks that `~/.config/skilltap/config.toml` exists, is valid TOML, and passes schema validation.

- **Fail**: parse error or invalid values
- **Warn**: file doesn't exist → run `skilltap config` to create one
- **Fix** (`--fix`): missing config → creates a default config file

If the loader rejects the file because the schema doesn't match, the failure message points at `skilltap migrate`.

### 3. Directories

Checks that all expected directories exist: `~/.config/skilltap/`, `cache/`, `taps/`, and `~/.agents/skills/`.

- **Warn**: missing directory
- **Fix** (`--fix`): creates missing directories

### 4. State (skill records)

Validates the canonical state file that tracks installed skills, plugins, and MCP servers: `~/.config/skilltap/state.json` (global) and `<project>/.agents/state.json` (project).

- **Fail**: file is corrupt (bad JSON or failed schema validation)
- **Pass** with detail `N skills (G global, P project)` when records are present
- **Fix** (`--fix`): backs up the corrupt file to `<file>.bak` and creates a fresh empty state

### 5. Skill Integrity

For each tracked skill, verifies the skill directory actually exists on disk in the correct location (`~/.agents/skills/` for global skills, `.agents/skills/` inside the project for project-scoped skills). Also scans each skills directory for untracked entries.

- **Warn — orphan record**: skill is in state but directory is missing
- **Warn — orphan directory**: directory exists on disk but isn't tracked in state
- **Warn — broken link**: a linked skill's target directory no longer exists
- **Fix** (`--fix`): removes orphan records and broken link entries from `state.json`; does not delete orphan directories (they might be manually placed)

### 6. Agent Symlinks

For each skill with `also` entries, checks that symlinks exist and point to the correct target. Global-scoped skills are checked in `~/.claude/skills/` (etc.); project-scoped skills are checked in `{projectRoot}/.claude/skills/` (etc.).

- **Warn**: missing or broken symlink
- **Fix** (`--fix`): recreates missing or incorrect symlinks

### 7. Taps

For each configured tap, verifies the local clone is intact and the tap index (`tap.json` or `.claude-plugin/marketplace.json`) is valid.

- **Warn**: tap directory missing, tap index invalid, or `.git/` missing
- **Fix** (`--fix`): re-clones missing tap directories from the configured URL

### 8. Agent CLIs

Detects available agent CLIs (Claude Code, Gemini CLI, Codex, Ollama, etc.). If a specific agent is configured for semantic scanning, verifies it's available.

- **Warn**: configured agent not found on PATH (semantic scan will fail at runtime)
- **Warn**: no agents detected (informational only — not everyone uses semantic scan)

### 9. npm (conditional)

Only checked if any installed skills use an `npm:` source.

- **Warn**: npm not found on PATH

### Project-level checks (10–15)

These run when there's a project root (a `.git` directory or `skilltap.toml`) below the cwd:

**10. manifest drift** — Compares `skilltap.toml` declared dependencies against `state.json` records. Warns about declared-but-not-installed and installed-but-not-declared entries (drift items themselves aren't fixable — manifest edits are user responsibility; run `skilltap sync` or edit the file). **If `skilltap.toml` itself fails to parse**, the issue is fixable: `--fix` backs the corrupt file up to `skilltap.toml.bak` and writes a fresh empty manifest. The same recovery is invoked automatically when you run `skilltap install` in interactive mode against a corrupt manifest (see [Installing skills — Recovering from a broken skilltap.toml](/guide/installing-skills#recovering-from-a-broken-skilltap-toml)).

**11. lockfile drift** — Compares `skilltap.lock` against `state.json` SHAs. Warns on stale (lockfile entry has no state record) or orphan (state record has no lockfile entry); `--fix` regenerates missing lockfile entries from state. **If `skilltap.lock` itself fails to parse**, the issue is fixable: `--fix` backs the corrupt file up to `skilltap.lock.bak` and writes a fresh empty lockfile.

**12. plugin manifests** — Validates every `.skilltap/<name>.toml` publish manifest in the working tree. Warns on parse errors or missing required fields.

**13. mcp consistency** — Compares `state.json::mcpServers[]` against each agent's MCP config (Claude Code's `.claude/settings.json`, etc.). Warns on entries in state that aren't in the agent config (missing — needs fresh inject) or orphan agent-config entries with `skilltap:` prefix that have no state record. `--fix` prunes the orphans.

**14. legacy file orphans** — Detects leftover `installed.json` / `plugins.json` files. `--fix` renames each orphan file to `<file>.v1.bak` and points you at `skilltap migrate` if a config translation is needed.

## Output

### Default (interactive)

```
┌ skilltap doctor
│
◇ git: /usr/bin/git (2.44.0) ✓
◇ config: ~/.config/skilltap/config.toml ✓
◇ dirs: ~/.config/skilltap/ ✓
◇ installed: 7 skills (2 global, 5 project) ✓
◇ skills: 7 installed, 7 on disk ✓
◇ symlinks: 8 symlinks, 8 valid ✓
◇ taps: 2 configured, 2 valid ✓
◇ agents: 3 detected (claude, gemini, ollama) ✓
│
└ ✓ Everything looks good!
```

When issues are found:

```
┌ skilltap doctor
│
◇ git: /usr/bin/git (2.44.0) ✓
◇ config: ~/.config/skilltap/config.toml ✓
◇ dirs: ~/.config/skilltap/ ✓
◇ installed: 7 skills (2 global, 5 project) ✓
⚠ skills: 7 installed, 6 on disk
│  commit-helper: recorded in state.json but directory missing at ~/.agents/skills/commit-helper
⚠ symlinks: 8 symlinks, 6 valid
│  commit-helper: missing symlink at ~/.claude/skills/commit-helper
│  code-review: symlink at ~/.claude/skills/code-review points to wrong target
◇ taps: 2 configured, 2 valid ✓
⚠ agents: configured agent 'codex' not found on PATH
│
└ ⚠ 3 issues found. Run 'skilltap doctor --fix' to auto-fix where possible.
```

### With `--fix`

```
┌ skilltap doctor
│
...
⚠ skills: 5 installed, 4 on disk
│  commit-helper: directory missing — removed from state.json ✓
⚠ symlinks: 8 symlinks, 6 valid
│  commit-helper: removed (skill no longer installed) ✓
│  code-review: recreated symlink ✓
◇ taps: 2 configured, 2 valid ✓
⚠ agents: configured agent 'codex' not found on PATH
│  (cannot auto-fix — install codex or change scanner.agent_cli in config)
│
└ ✓ Fixed 3 of 4 issues. 1 requires manual action.
```

### With `--json`

```json
{
  "ok": false,
  "checks": [
    { "name": "git", "status": "pass", "detail": "/usr/bin/git (2.44.0)" },
    { "name": "config", "status": "pass", "detail": "~/.config/skilltap/config.toml" },
    { "name": "dirs", "status": "pass" },
    { "name": "installed", "status": "pass", "detail": "5 skills" },
    {
      "name": "skills",
      "status": "warn",
      "issues": [
        { "message": "commit-helper: directory missing", "fixable": true }
      ]
    },
    {
      "name": "symlinks",
      "status": "warn",
      "issues": [
        { "message": "commit-helper: missing symlink", "fixable": true },
        { "message": "code-review: wrong target", "fixable": true }
      ]
    },
    { "name": "taps", "status": "pass", "detail": "2 configured, 2 valid" },
    {
      "name": "agents",
      "status": "warn",
      "issues": [
        { "message": "configured agent 'codex' not found on PATH", "fixable": false }
      ]
    }
  ]
}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All checks pass, or only warnings (also exits 0 after `--fix` resolves all blocking failures) |
| `1` | One or more unfixed failures remain (corrupt files, missing git, etc.) |

Warnings alone produce exit code 0. After `--fix` runs, doctor re-classifies fixed checks as passes and exits 0 if no failures remain. Only hard, unfixed failures trigger exit 1, so you can chain `skilltap doctor --fix && …` in CI scripts.

### `--json` payload

Each check entry includes:

| Field | Description |
|---|---|
| `name` | Stable check identifier (`git`, `config`, `state`, `manifest-drift`, …) |
| `status` | `"pass"` \| `"warn"` \| `"fail"` |
| `detail` | Human-readable summary (e.g. path, version, count) |
| `info` | Extra structured context (paths inspected, scope-by-scope breakdown) |
| `issues[]` | Per-issue array; each entry has `message`, `fixable`, and optional `fixDescription` describing exactly what `--fix` would do |

## When to Run

- **Something's broken**: Run `skilltap doctor` to get a clear picture of what's wrong before digging into logs.
- **After moving your home directory**: Symlinks and paths may need updating — `--fix` handles most of this.
- **In CI setup scripts**: Use `--json` to parse results programmatically, or just check the exit code.
- **After a system reinstall**: Verify git and agents are still on PATH and your config survived.
