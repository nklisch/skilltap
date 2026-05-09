# Design: Skill Disable / Enable

## Overview

Add the ability to temporarily disable skills so agents no longer see them, without deleting files or losing installed.json records. Enabling restores the skill to its previous state. This covers all managed skill types (installed, linked, adopted) and handles the fact that `.agents/skills/` itself is a discovery directory for some agents.

### Key Decisions

- **Managed only**: `disable`/`enable` only works on skills tracked in `installed.json`. Unmanaged skills must be `adopt`ed first.
- **List shows all**: `skilltap skills` always shows disabled skills with a `(disabled)` tag. `--disabled` / `--active` flags filter.
- **Disabled directory**: `.agents/skills/.disabled/<name>/` — dot-prefix hides from naive agent scanners while keeping skills co-located.
- **Scope preservation**: Project-scope disabled skills stay in the project (committable). Global disabled skills stay in `~/.agents/skills/.disabled/`.

### Mechanism

Disabling a skill does two things:
1. **Removes agent symlinks** (`.claude/skills/foo`, `.cursor/skills/foo`, etc.)
2. **Moves skill files** from `.agents/skills/<name>` to `.agents/skills/.disabled/<name>` — so agents scanning `.agents/skills/` directly won't find it

For **linked** skills, there's no move (skilltap doesn't own those files) — only symlinks are removed.

The `active` field on the `InstalledSkill` record tracks the state.

---

## Implementation Units

### Unit 1: Schema — Add `active` Field

**File**: `packages/core/src/schemas/installed.ts`

```typescript
export const InstalledSkillSchema = z.object({
  name: z.string(),
  description: z.string().default(""),
  repo: z.string().nullable(),
  ref: z.string().nullable(),
  sha: z.string().nullable().default(null),
  scope: z.enum(["global", "project", "linked"]),
  path: z.string().nullable(),
  tap: z.string().nullable().default(null),
  also: z.array(z.string()).default([]),
  installedAt: z.iso.datetime(),
  updatedAt: z.iso.datetime().default("1970-01-01T00:00:00.000Z"),
  trust: TrustInfoSchema.optional(),
  active: z.boolean().default(true),  // NEW
});
```

**Implementation Notes**:
- `.default(true)` ensures backwards compatibility — existing records without `active` parse as active.
- No migration needed; Zod's default handles it on read.

**Acceptance Criteria**:
- [ ] Existing `installed.json` files without `active` field parse successfully with `active: true`
- [ ] Records with `active: false` round-trip through save/load correctly
- [ ] `InstalledSkill` type includes `active: boolean`

---

### Unit 2: Path Helper — Disabled Skills Directory

**File**: `packages/core/src/paths.ts`

```typescript
export function skillDisabledDir(
  name: string,
  scope: "global" | "project",
  projectRoot?: string,
): string {
  const base =
    scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
  return join(base, ".agents", "skills", ".disabled", name);
}
```

**Implementation Notes**:
- Mirrors `skillInstallDir` but with `.disabled` segment inserted.
- Same scope logic — global uses `globalBase()`, project uses `projectRoot`.

**Acceptance Criteria**:
- [ ] Global: returns `~/.agents/skills/.disabled/<name>`
- [ ] Project: returns `<projectRoot>/.agents/skills/.disabled/<name>`

---

### Unit 3: Core — `disableSkill()` Function

**File**: `packages/core/src/disable.ts`

```typescript
import { mkdir, rename } from "node:fs/promises";
import { dirname } from "node:path";
import { loadInstalled, saveInstalled } from "./config";
import { skillDisabledDir, skillInstallDir } from "./paths";
import { removeAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export type DisableOptions = {
  scope?: "global" | "project" | "linked";
  projectRoot?: string;
};

export async function disableSkill(
  name: string,
  options: DisableOptions = {},
): Promise<Result<void, UserError>>;
```

**Implementation Notes**:

1. Load `installed.json`, find record by name (and optional scope filter).
2. If not found, return `UserError` with hint to run `skilltap skills`.
3. If already `active: false`, return `UserError("Skill '<name>' is already disabled.")`.
4. Remove agent symlinks via `removeAgentSymlinks(record.name, record.also, record.scope, options.projectRoot)`.
5. Move skill files to disabled directory:
   - For `scope === "linked"`: **skip move** — skilltap doesn't own linked files. Only symlinks are removed.
   - For `scope === "global" | "project"`:
     - Source: `skillInstallDir(name, scope, projectRoot)`
     - Dest: `skillDisabledDir(name, scope, projectRoot)`
     - `mkdir(dirname(dest), { recursive: true })` then `rename(source, dest)` (atomic on same filesystem).
6. Set `record.active = false` and `record.updatedAt = new Date().toISOString()`.
7. Save `installed.json`.

**Acceptance Criteria**:
- [ ] Disabling a global managed skill moves `.agents/skills/foo` to `.agents/skills/.disabled/foo`
- [ ] Disabling a project managed skill moves project `.agents/skills/foo` to project `.agents/skills/.disabled/foo`
- [ ] Disabling a linked skill only removes symlinks — no file move
- [ ] Agent symlinks (`.claude/skills/foo`, etc.) are removed
- [ ] `installed.json` record has `active: false` and updated `updatedAt`
- [ ] Disabling an already-disabled skill returns an error
- [ ] Disabling a non-existent skill returns an error with hint

---

### Unit 4: Core — `enableSkill()` Function

**File**: `packages/core/src/disable.ts` (same file)

```typescript
export type EnableOptions = {
  scope?: "global" | "project" | "linked";
  projectRoot?: string;
};

export async function enableSkill(
  name: string,
  options: EnableOptions = {},
): Promise<Result<void, UserError>>;
```

**Implementation Notes**:

1. Load `installed.json`, find record by name (and optional scope filter).
2. If not found, return `UserError`.
3. If already `active: true`, return `UserError("Skill '<name>' is already enabled.")`.
4. Move skill files back:
   - For `scope === "linked"`: **skip move**.
   - For `scope === "global" | "project"`:
     - Source: `skillDisabledDir(name, scope, projectRoot)`
     - Dest: `skillInstallDir(name, scope, projectRoot)`
     - `mkdir(dirname(dest), { recursive: true })` then `rename(source, dest)`.
5. Re-create agent symlinks via `createAgentSymlinks(record.name, dest, record.also, effectiveScope, projectRoot)`.
   - For linked skills, the symlink target is `record.path` (the external path).
6. Set `record.active = true` and `record.updatedAt = new Date().toISOString()`.
7. Save `installed.json`.

**Acceptance Criteria**:
- [ ] Enabling a disabled global skill moves `.agents/skills/.disabled/foo` back to `.agents/skills/foo`
- [ ] Enabling a disabled project skill moves it back within the project
- [ ] Enabling a linked skill re-creates symlinks pointing to `record.path`
- [ ] Agent symlinks are re-created for all agents in `record.also`
- [ ] `installed.json` record has `active: true` and updated `updatedAt`
- [ ] Enabling an already-enabled skill returns an error
- [ ] Enabling a non-existent skill returns an error with hint

---

### Unit 5: Core Barrel Export

**File**: `packages/core/src/index.ts`

Add:
```typescript
export * from "./disable";
```

**Acceptance Criteria**:
- [ ] `disableSkill`, `enableSkill`, `DisableOptions`, `EnableOptions` are importable from `@skilltap/core`

---

### Unit 6: CLI Command — `skilltap skills disable`

**File**: `packages/cli/src/commands/skills/disable.ts`

```typescript
import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "disable",
    description: "Temporarily disable a skill (hide from agents)",
  },
  args: {
    name: { type: "positional", description: "Skill name to disable", required: true },
    global: { type: "boolean", description: "Disable global skill", default: false },
    project: { type: "boolean", description: "Disable project skill", default: false },
  },
  async run({ args }) { /* ... */ },
});
```

**Implementation Notes**:

1. Resolve scope from `--global`/`--project` flags (same pattern as `remove.ts`).
2. Call `disableSkill(args.name, { scope, projectRoot })`.
3. On success: `log.success("Disabled skill 'foo'")` (clack) or plain text in agent mode.
4. On error: `log.error(result.error.message)` + hint, exit 1.

**Acceptance Criteria**:
- [ ] `skilltap skills disable foo` disables skill `foo`
- [ ] `--global` / `--project` flags correctly scope the operation
- [ ] Exit code 0 on success, 1 on error
- [ ] Agent mode outputs plain text

---

### Unit 7: CLI Command — `skilltap skills enable`

**File**: `packages/cli/src/commands/skills/enable.ts`

```typescript
import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "enable",
    description: "Re-enable a previously disabled skill",
  },
  args: {
    name: { type: "positional", description: "Skill name to enable", required: true },
    global: { type: "boolean", description: "Enable global skill", default: false },
    project: { type: "boolean", description: "Enable project skill", default: false },
  },
  async run({ args }) { /* ... */ },
});
```

**Implementation Notes**:
- Same pattern as disable command but calls `enableSkill()`.

**Acceptance Criteria**:
- [ ] `skilltap skills enable foo` enables a disabled skill
- [ ] `--global` / `--project` flags correctly scope the operation
- [ ] Exit code 0 on success, 1 on error
- [ ] Agent mode outputs plain text

---

### Unit 8: Register Subcommands

**File**: `packages/cli/src/commands/skills/index.ts`

Add to `subCommands`:
```typescript
subCommands: {
  // ... existing
  disable: () => import("./disable").then((m) => m.default),
  enable: () => import("./enable").then((m) => m.default),
},
```

**Acceptance Criteria**:
- [ ] `skilltap skills disable` and `skilltap skills enable` are routable commands

---

### Unit 9: Update `skilltap skills` List Display

**File**: `packages/cli/src/commands/skills/index.ts`

Add filter flags and display logic:

```typescript
args: {
  // ... existing
  disabled: { type: "boolean", description: "Show only disabled skills", default: false },
  active: { type: "boolean", description: "Show only active skills", default: false },
},
```

**Implementation Notes**:

1. After loading discovered skills, cross-reference with `installed.json` to check `active` status.
2. Apply `--disabled` / `--active` filters.
3. In the managed table, add visual indicator for disabled skills:
   - Interactive: `ansi.dim("disabled")` status label instead of `ansi.green("managed")`.
   - Agent mode: `DISABLED` status in plain text output.
4. Disabled skills sort to the bottom of their section.

The `discoverSkills()` function returns all managed skills regardless of `active` status (it reads from `installed.json` which has the records). The `active` field on the record is used for display and filtering.

**Acceptance Criteria**:
- [ ] Disabled skills appear in default `skilltap skills` output with `disabled` status
- [ ] `--disabled` shows only disabled skills
- [ ] `--active` shows only active (non-disabled) skills
- [ ] Agent mode includes `DISABLED` status label
- [ ] JSON output includes `active` field from record

---

### Unit 10: Guard Disable-Aware Commands

**File**: Multiple — `packages/core/src/update.ts`, `packages/cli/src/commands/skills/remove.ts`

**Implementation Notes**:

Commands that operate on skills should be aware of disabled state:

- **`skilltap update`**: Skip disabled skills during bulk update (`skilltap update` with no name). When targeting a specific disabled skill by name, warn and proceed (the user explicitly asked).
- **`skilltap skills remove`**: Works on disabled skills (you should be able to fully remove a disabled skill). The skill files are in `.disabled/` so `removeSkill()` needs to check `active` to know which path to `rm -rf`.

Changes to `removeSkill()` in `packages/core/src/remove.ts`:
```typescript
// When building installPath, check if skill is disabled
const installPath =
  record.scope === "linked" && record.path !== null
    ? record.path
    : record.active === false
      ? skillDisabledDir(record.name, record.scope === "linked" ? "global" : record.scope, options.projectRoot)
      : skillInstallDir(record.name, record.scope === "linked" ? "global" : record.scope, options.projectRoot);
```

Changes to `updateSkill()` in `packages/core/src/update.ts`:
- In bulk update mode (no name specified): filter out records where `active === false`.
- When a specific name is given and it's disabled: proceed normally (update the files in `.disabled/`).

**Acceptance Criteria**:
- [ ] `skilltap skills remove foo` works when `foo` is disabled (removes from `.disabled/`)
- [ ] `skilltap update` (bulk) skips disabled skills
- [ ] `skilltap update foo` works on a disabled skill (updates files in place)

---

### Unit 11: Doctor Check for Disabled Skills

**File**: `packages/core/src/doctor.ts`

**Implementation Notes**:

Add awareness of `.disabled/` directory to the existing skill integrity check. The doctor should:
- Not flag disabled skills as "missing from install dir" (they're in `.disabled/`).
- Verify that disabled skills exist in `.disabled/` and enabled skills exist in the normal dir.
- Flag orphaned directories in `.disabled/` that have no `installed.json` record.

This is a minor adjustment to the existing `checkSkillIntegrity` function, not a new check.

**Acceptance Criteria**:
- [ ] Doctor does not report false positives for disabled skills
- [ ] Doctor detects disabled skill whose `.disabled/` directory is missing
- [ ] Doctor detects orphaned directories in `.disabled/` with no record

---

### Unit 12: Shell Completions Update

**File**: `packages/cli/src/completions/generate.ts`

**Implementation Notes**:

Add `disable` and `enable` to the `skills` subcommand completions. The `--get-completions` endpoint for `disabled-skills` should return names of disabled skills (for `enable` completion), and `active-skills` should return names of active skills (for `disable` completion).

**Acceptance Criteria**:
- [ ] Tab-completion suggests `disable` and `enable` under `skills`
- [ ] `skilltap skills disable <TAB>` completes with active skill names
- [ ] `skilltap skills enable <TAB>` completes with disabled skill names

---

## Implementation Order

1. **Unit 1** — Schema (`active` field) — everything depends on this
2. **Unit 2** — Path helper (`skillDisabledDir`) — needed by core functions
3. **Unit 3** — `disableSkill()` core function
4. **Unit 4** — `enableSkill()` core function
5. **Unit 5** — Barrel export
6. **Unit 10** — Guard existing commands (`removeSkill`, `updateSkill`)
7. **Unit 11** — Doctor awareness
8. **Unit 6** — CLI `disable` command
9. **Unit 7** — CLI `enable` command
10. **Unit 8** — Register subcommands
11. **Unit 9** — List display update
12. **Unit 12** — Shell completions

Units 1-2 are pure additions with no behavioral changes. Units 3-5 are core logic. Unit 10 must be done before CLI commands to prevent bugs if someone disables then removes. Units 6-9 and 12 are CLI layer.

---

## Testing

### Unit Tests: `packages/core/src/disable.test.ts`

**Setup**: Uses `makeTmpDir`, env var isolation (`SKILLTAP_HOME`, `XDG_CONFIG_HOME`), fixture repos.

```
describe("disableSkill")
  test("disables a managed global skill — moves to .disabled/, removes symlinks, sets active=false")
  test("disables a managed project skill — moves within project .agents/skills/.disabled/")
  test("disables a linked skill — removes symlinks only, no file move")
  test("errors on already-disabled skill")
  test("errors on non-existent skill")

describe("enableSkill")
  test("enables a disabled global skill — moves back from .disabled/, recreates symlinks, sets active=true")
  test("enables a disabled project skill — moves within project")
  test("enables a linked skill — recreates symlinks from record.path")
  test("errors on already-enabled skill")
  test("errors on non-existent skill")
```

### Schema Tests: `packages/core/src/schemas/installed.test.ts`

Add to existing:
```
test("defaults active to true when field is missing")
test("preserves active: false through round-trip")
```

### Path Tests: `packages/core/src/paths.test.ts`

Add:
```
test("skillDisabledDir returns correct global path")
test("skillDisabledDir returns correct project path")
```

### Integration — Remove disabled skill: `packages/core/src/remove.test.ts`

Add:
```
test("removeSkill works on a disabled skill (finds in .disabled/)")
```

### CLI Tests: `packages/cli/src/commands/skills/disable.test.ts`

```
describe("skilltap skills disable")
  test("disables a skill and prints success")
  test("errors on unknown skill")
  test("--project flag scopes to project")

describe("skilltap skills enable")
  test("enables a disabled skill and prints success")
  test("errors on unknown skill")
```

Use `runSkilltap` (non-interactive, pipe mode) — these commands produce plain text output via `log.success`/`log.error`, no clack spinners needed.

### List Display Tests: `packages/cli/src/commands/skills/index.test.ts`

Add to existing:
```
test("shows disabled skills with 'disabled' status label")
test("--disabled flag filters to disabled only")
test("--active flag filters to active only")
```

---

## Verification Checklist

```bash
# Run all tests
bun test

# Run focused tests
bun test packages/core/src/disable.test.ts
bun test packages/core/src/schemas/installed.test.ts
bun test packages/cli/src/commands/skills/disable.test.ts

# Manual smoke test
skilltap install github:nklisch/test-skill --global --also claude-code
skilltap skills                    # shows skill as "managed"
skilltap skills disable test-skill # disables
ls ~/.agents/skills/.disabled/     # skill dir here
ls ~/.claude/skills/               # symlink gone
skilltap skills                    # shows "disabled" status
skilltap skills enable test-skill  # re-enables
ls ~/.agents/skills/test-skill     # skill dir restored
ls ~/.claude/skills/               # symlink restored
skilltap skills                    # shows "managed" again
```
