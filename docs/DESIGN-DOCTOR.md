# Design: `skilltap doctor`

Diagnostic command that checks skilltap's environment, configuration, and state integrity. Surfaces problems before they become cryptic errors during install/update.

## Command

```
skilltap doctor [flags]
```

**Options:**

| Flag | Type | Default | Description |
|---|---|---|---|
| `--json` | boolean | false | Output as JSON (for CI/scripting) |
| `--fix` | boolean | false | Auto-fix issues where possible |

## Checks

Doctor runs a sequence of checks, each producing a pass, warning, or failure. Checks are independent — a failure in one doesn't skip the rest.

### 1. Git

```
◇ git: /usr/bin/git (2.44.0) ✓
```

- Verify `git` is on PATH (`which git`)
- Get version (`git --version`)
- **Fail**: git not found → `✗ git not found on PATH. Install git: https://git-scm.com`
- **Warn**: git version < 2.25 → `⚠ git 2.25+ recommended (found 2.20). Shallow clone --filter may not work.`

### 2. Config File

```
◇ config: ~/.config/skilltap/config.toml ✓
```

- Check file exists
- Parse with smol-toml
- Validate with `ConfigSchema`
- **Fail**: parse error → `✗ config.toml is invalid TOML: {parse error at line N}`
- **Fail**: schema error → `✗ config.toml has invalid values: {prettified Zod error}`
- **Warn**: file doesn't exist → `⚠ No config.toml found. Run 'skilltap config' to create one.`
- **Fix** (`--fix`): missing config → create default config

### 3. Directories

```
◇ dirs: ~/.config/skilltap/ ✓
  cache/  taps/  config.toml  installed.json
```

- Check all expected directories exist: `~/.config/skilltap/`, `cache/`, `taps/`
- Check `~/.agents/skills/` exists (global install dir)
- **Warn**: missing directory → `⚠ Missing directory: ~/.config/skilltap/cache/`
- **Fix** (`--fix`): create missing directories

### 4. installed.json

```
◇ installed.json: 5 skills ✓
```

- Check file exists and parses
- Validate with `InstalledJsonSchema`
- **Fail**: parse error → `✗ installed.json is corrupt: {error}`
- **Warn**: file doesn't exist → `⚠ No installed.json found (no skills installed).` (not an error)
- **Fix** (`--fix`): corrupt file → back up to `installed.json.bak`, create fresh `{ version: 1, skills: [] }`

### 5. Installed Skills Integrity

```
◇ skills: 5 installed, 5 on disk ✓
```

For each skill in `installed.json`:

- **Orphan record**: skill directory doesn't exist on disk
  - `⚠ commit-helper: recorded in installed.json but directory missing at ~/.agents/skills/commit-helper/`
  - **Fix** (`--fix`): remove orphan record from installed.json
- **Orphan directory**: directory exists in `~/.agents/skills/` but no record in installed.json
  - `⚠ unknown-skill: directory exists at ~/.agents/skills/unknown-skill/ but not tracked in installed.json`
  - (no auto-fix — could be manually placed, don't delete)
- **Broken link**: linked skill's target doesn't exist
  - `⚠ my-local-skill: symlink target /home/nathan/dev/old-project/my-skill does not exist`
  - **Fix** (`--fix`): remove broken link record and symlink

### 6. Agent Symlinks

```
◇ symlinks: 8 symlinks, 8 valid ✓
```

For each skill with `also` entries:

- Check symlink exists at the agent-specific path
- Check symlink target points to the correct canonical path
- **Warn**: missing symlink → `⚠ commit-helper: missing symlink at ~/.claude/skills/commit-helper/`
- **Warn**: broken symlink → `⚠ commit-helper: symlink at ~/.claude/skills/commit-helper/ points to wrong target`
- **Fix** (`--fix`): recreate missing/broken symlinks

### 7. Taps

```
◇ taps: 2 configured, 2 valid ✓
  home       3 skills   https://gitea.example.com/nathan/my-tap
  community  12 skills  https://github.com/someone/awesome-tap
```

For each tap in config:

- Check tap directory exists at `~/.config/skilltap/taps/{name}/`
- Check `tap.json` exists and validates with `TapSchema`
- Check `.git/` exists (it's a cloned repo)
- **Warn**: tap directory missing → `⚠ tap 'home': directory missing. Run 'skilltap tap update home' to re-clone.`
- **Warn**: invalid tap.json → `⚠ tap 'home': tap.json is invalid: {error}`
- **Warn**: config references tap not on disk → `⚠ tap 'community': in config but directory missing`
- **Fix** (`--fix`): missing tap directory → re-clone from config URL

### 8. Agent CLIs

```
◇ agents: 3 detected
  claude     /home/nathan/.local/bin/claude (Claude Code)
  gemini     /usr/local/bin/gemini (Gemini CLI)
  ollama     /usr/local/bin/ollama (Ollama — 2 models)
```

- Run detection for all known agent adapters (same as `detectAgents()`)
- Show which are available and their paths
- If `security.agent` is configured, verify that specific agent is available
- **Warn**: configured agent not found → `⚠ Configured agent 'claude' not found on PATH. Semantic scan will fail.`
- **Warn**: no agents found → `⚠ No agent CLIs found. Semantic scanning unavailable.` (informational — not everyone uses semantic scan)

### 9. npm (conditional)

Only checked if any installed skills have `npm:` source or if npm adapter features are used:

```
◇ npm: /usr/bin/npm (10.8.0) ✓
  registry: https://registry.npmjs.org
  logged in as: nathan
```

- Check `npm` is on PATH
- Check registry is reachable (`npm ping`)
- Check auth status (`npm whoami`) — warn if not logged in (only matters for `publish`)
- **Warn**: npm not found → `⚠ npm not found. Install Node.js for npm skill support.` (only if npm skills are installed)

## Output

### Interactive (default)

```
$ skilltap doctor

┌ skilltap doctor
│
◇ git: /usr/bin/git (2.44.0) ✓
◇ config: ~/.config/skilltap/config.toml ✓
◇ dirs: ~/.config/skilltap/ ✓
◇ installed.json: 5 skills ✓
◇ skills: 5 installed, 5 on disk ✓
◇ symlinks: 8 symlinks, 8 valid ✓
◇ taps: 2 configured, 2 valid ✓
◇ agents: 3 detected (claude, gemini, ollama) ✓
│
└ ✓ Everything looks good!
```

With issues:

```
$ skilltap doctor

┌ skilltap doctor
│
◇ git: /usr/bin/git (2.44.0) ✓
◇ config: ~/.config/skilltap/config.toml ✓
◇ dirs: ~/.config/skilltap/ ✓
◇ installed.json: 5 skills ✓
⚠ skills: 5 installed, 4 on disk
│  commit-helper: directory missing at ~/.agents/skills/commit-helper/
⚠ symlinks: 8 symlinks, 6 valid
│  commit-helper: missing symlink at ~/.claude/skills/commit-helper/
│  code-review: symlink points to wrong target
◇ taps: 2 configured, 2 valid ✓
⚠ agents: configured agent 'codex' not found on PATH
│
└ ⚠ 3 issues found. Run 'skilltap doctor --fix' to auto-fix where possible.
```

### With `--fix`

```
$ skilltap doctor --fix

┌ skilltap doctor
│
◇ git: /usr/bin/git (2.44.0) ✓
◇ config: ~/.config/skilltap/config.toml ✓
◇ dirs: ~/.config/skilltap/ ✓
◇ installed.json: 5 skills ✓
⚠ skills: 5 installed, 4 on disk
│  commit-helper: directory missing — removed from installed.json ✓
⚠ symlinks: 8 symlinks, 6 valid
│  commit-helper: removed (skill no longer installed) ✓
│  code-review: recreated symlink ✓
◇ taps: 2 configured, 2 valid ✓
⚠ agents: configured agent 'codex' not found on PATH
│  (cannot auto-fix — install codex or change security.agent in config)
│
└ ✓ Fixed 3 of 4 issues. 1 requires manual action.
```

### JSON (`--json`)

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
        { "skill": "commit-helper", "issue": "directory missing", "fixable": true }
      ]
    },
    {
      "name": "symlinks",
      "status": "warn",
      "issues": [
        { "skill": "commit-helper", "path": "~/.claude/skills/commit-helper/", "issue": "missing", "fixable": true },
        { "skill": "code-review", "path": "~/.claude/skills/code-review/", "issue": "wrong target", "fixable": true }
      ]
    },
    { "name": "taps", "status": "pass", "detail": "2 configured, 2 valid" },
    {
      "name": "agents",
      "status": "warn",
      "issues": [
        { "agent": "codex", "issue": "not found on PATH", "fixable": false }
      ]
    }
  ]
}
```

## Exit Codes

| Code | Meaning |
|---|---|
| 0 | All checks pass (or all issues fixed with `--fix`) |
| 1 | One or more failures (not just warnings) |

Warnings alone produce exit code 0. Only hard failures (corrupt files, missing git) produce exit code 1. This lets CI scripts use `skilltap doctor` as a health check without false positives from missing optional features.

## Implementation

### Core

```typescript
// packages/core/src/doctor.ts

interface DoctorCheck {
  name: string;
  status: "pass" | "warn" | "fail";
  detail?: string;
  issues?: DoctorIssue[];
}

interface DoctorIssue {
  message: string;
  fixable: boolean;
  fix?: () => Promise<void>;  // called when --fix is passed
}

interface DoctorResult {
  ok: boolean;          // true if no failures
  checks: DoctorCheck[];
}

function runDoctor(options?: {
  fix?: boolean;
  onCheck?: (check: DoctorCheck) => void;  // callback for streaming output
}): Promise<DoctorResult>
```

Each check is a standalone function that returns a `DoctorCheck`. The orchestrator runs them sequentially (some checks depend on earlier results — e.g., symlink check needs installed.json to have been parsed).

### CLI

```
packages/cli/src/commands/doctor.ts
```

Uses the `onCheck` callback to print each result as it completes (streaming output, not batch).

## New Files

```
packages/core/src/doctor.ts            # Check functions + orchestrator
packages/cli/src/commands/doctor.ts    # CLI command
```

## Testing

- **Unit tests**: each check function with valid/invalid/missing state
- **Integration test**: `skilltap doctor` on a healthy environment (all pass)
- **Integration test**: `skilltap doctor` with deliberately broken state (orphan records, broken symlinks, corrupt config)
- **Integration test**: `skilltap doctor --fix` repairs broken state
- **Integration test**: `skilltap doctor --json` output format
