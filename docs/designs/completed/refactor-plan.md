# Refactor Plan: CLI Boilerplate & Core Simplification

## Summary

The codebase is architecturally sound — the monorepo split, Result pattern, adapter strategy, and Zod boundary approach are all working well. However, as the CLI has grown to 30+ commands, significant boilerplate has accumulated. The same 5-10 line blocks are copy-pasted across most commands: agent-mode error handling, project root resolution, JSON output, scope determination, and config loading. Meanwhile, core has a few large files (doctor.ts at 873 lines) and repeated safeParse+prettifyError sequences.

This plan prioritizes deduplication and simplification over aesthetic changes. Each step is independently shippable.

## Refactor Steps

### Step 1: Use `exitWithError` consistently across all commands

**Priority**: High
**Risk**: Low
**Files**: 12 command files + `ui/agent-out.ts`

**Current State**: `exitWithError(agentMode, msg, hint)` already exists in `ui/agent-out.ts:37-45` but only 3 commands use it. The other 12 commands inline the same 4-line pattern:
```typescript
if (agentMode) {
  agentError(result.error.message);
  process.exit(1);
}
errorLine(result.error.message, result.error.hint);
process.exit(1);
```

**Target State**: All 12 commands use `exitWithError(policy.agentMode, result.error.message, result.error.hint)`.

**Approach**: Mechanical find-and-replace in each command file. No logic changes.

**Verification**:
- `bun test` passes
- Grep for `agentError(result.error` returns 0 hits outside agent-out.ts

---

### Step 2: Extract `tryFindProjectRoot()` helper

**Priority**: High
**Risk**: Low
**Files**: `ui/resolve.ts` + 15 command files

**Current State**: `await findProjectRoot().catch(() => undefined)` appears 15 times across CLI commands. One variant uses `null` instead of `undefined`.

**Target State**: Single helper in `ui/resolve.ts`:
```typescript
export async function tryFindProjectRoot(): Promise<string | undefined> {
  return findProjectRoot().catch(() => undefined);
}
```
All 15 call sites import and use it.

**Approach**: Add helper, then find-and-replace all `.catch(() => undefined)` and `.catch(() => null)` call sites.

**Verification**:
- `bun test` passes
- Grep for `findProjectRoot().catch` returns 0 hits

---

### Step 3: Extract `outputJson()` helper

**Priority**: Medium
**Risk**: Low
**Files**: `ui/format.ts` or `ui/agent-out.ts` + ~12 command files

**Current State**: `process.stdout.write(\`${JSON.stringify(data, null, 2)}\n\`)` appears ~12 times in production code.

**Target State**: Single helper:
```typescript
export function outputJson(data: unknown): void {
  process.stdout.write(`${JSON.stringify(data, null, 2)}\n`);
}
```

**Approach**: Add to `ui/agent-out.ts` (already the agent-mode output module), replace all call sites.

**Verification**:
- `bun test` passes
- Grep for `JSON.stringify.*null.*2` in command files returns 0 hits (test files are fine)

---

### Step 4: Merge enable.ts and disable.ts into a single toggle command file

**Priority**: Medium
**Risk**: Low
**Files**: `commands/skills/enable.ts`, `commands/skills/disable.ts`

**Current State**: These two files are nearly character-for-character identical — 53 lines each, differing only in: function called (`enableSkill` vs `disableSkill`), command name/description, and output label ("Enabled" vs "Disabled").

**Target State**: A single shared factory or a combined file that exports both commands using a parameterized helper:
```typescript
function makeToggleCommand(action: "enable" | "disable") {
  const coreFn = action === "enable" ? enableSkill : disableSkill;
  const label = action === "enable" ? "Enabled" : "Disabled";
  return defineCommand({ ... });
}
export const enableCommand = makeToggleCommand("enable");
export const disableCommand = makeToggleCommand("disable");
```

**Approach**: Create `commands/skills/toggle.ts` exporting both, update parent subcommand registration to import from the new file, delete the old files.

**Verification**:
- `bun test` passes
- `bun run dev skills enable <name>` and `bun run dev skills disable <name>` work identically to before

---

### Step 5: Extract `parseWithResult()` Zod helper in core

**Priority**: Medium
**Risk**: Low
**Files**: `core/src/schemas/index.ts` + `config.ts`, `taps.ts`, `doctor.ts`, `registry/client.ts`

**Current State**: The same 5-line safeParse + prettifyError + Result wrapping pattern appears 9 times in core:
```typescript
const result = SomeSchema.safeParse(raw);
if (!result.success) {
  const details = z.prettifyError(result.error);
  return err(new UserError(`Invalid ${label}: ${details}`));
}
return ok(result.data);
```

**Target State**: Single helper in `schemas/index.ts`:
```typescript
export function parseWithResult<T>(
  schema: z.ZodType<T>,
  data: unknown,
  label: string,
): Result<T, UserError> {
  const result = schema.safeParse(data);
  if (!result.success) {
    return err(new UserError(`Invalid ${label}: ${z.prettifyError(result.error)}`));
  }
  return ok(result.data);
}
```

Note: Not all safeParse sites are identical — `scanner.ts` and `validate.ts` collect warnings rather than returning errors, and `doctor.ts` emits issues rather than returning Results. Only replace the sites that match this exact pattern (config.ts x2, taps.ts, registry/client.ts x3, and doctor.ts x2 where it returns early with the check result).

**Approach**: Add helper, replace matching call sites. Leave scanner/validate/discover sites that have different error handling.

**Verification**:
- `bun test` passes
- The helper has a unit test

---

### Step 6: Simplify agent-mode detection pattern in simple commands

**Priority**: Medium
**Risk**: Low
**Files**: ~8 simpler command files (adopt, info, move, link, tap/add, tap/info, verify, skills/index)

**Current State**: Commands that don't need the full `EffectivePolicy` still load config just to check agent mode:
```typescript
const configResult = await loadConfig();
const agentMode = configResult.ok && configResult.value["agent-mode"].enabled;
```

Meanwhile, commands that already use `loadPolicyOrExit` get `policy.agentMode` for free.

**Target State**: For commands that only need agent mode (not the full policy), extract a helper:
```typescript
export async function isAgentMode(): Promise<boolean> {
  const configResult = await loadConfig();
  return configResult.ok && configResult.value["agent-mode"].enabled;
}
```

For commands that already use `loadPolicyOrExit`, just use `policy.agentMode` — no change needed.

**Approach**: Add helper to `ui/policy.ts`, replace matching sites in the ~8 commands that load config only for agent-mode.

**Verification**:
- `bun test` passes
- Grep for `configResult.ok && configResult.value["agent-mode"]` returns 0 hits

---

### Step 7: Split doctor.ts into check modules

**Priority**: Medium
**Risk**: Medium
**Files**: `core/src/doctor.ts` → `core/src/doctor/` directory

**Current State**: `doctor.ts` is 873 lines containing 9 check functions, 3 file-existence helpers, and an orchestrator. Each check follows the same structure but is a self-contained block. The file is hard to navigate and the helpers (`resolvedDirExists`, `fileExists`, `isSymlinkAt`) are duplicated from `discover.ts`.

**Target State**:
```
core/src/doctor/
  index.ts          — runDoctor() orchestrator + types (~80 lines)
  checks/
    git.ts          — checkGit
    config.ts       — checkConfig
    directories.ts  — checkDirs
    installed.ts    — checkInstalled
    skills.ts       — checkSkills
    symlinks.ts     — checkSymlinks
    taps.ts         — checkTaps
    agents.ts       — checkAgents
    npm.ts          — checkNpm
```

Shared file helpers (`resolvedDirExists`, `fileExists`, `isSymlinkAt`) should be extracted to `core/src/fs.ts` (which already exists for base path helpers) and shared with `discover.ts`.

**Approach**:
1. Move file helpers to `fs.ts`, update imports in `discover.ts`
2. Create `doctor/` directory with orchestrator
3. Move each check into its own file
4. Re-export from `doctor/index.ts`
5. Update barrel export in `core/src/index.ts`

**Verification**:
- `bun test` passes (doctor.test.ts unchanged)
- `bun run dev doctor` works identically
- `wc -l` of each check file < 120 lines

---

### Step 8: Extract shared security scan runner for install/update

**Priority**: Low
**Risk**: Medium
**Files**: `core/src/install.ts`, `core/src/update.ts`, potentially new `core/src/security/run.ts`

**Current State**: Both `install.ts` and `update.ts` orchestrate static+semantic scans with similar callback invocations, warning collection, and policy-based gating. Install has a `runSecurityScan()` inner helper; update inlines similar logic.

**Target State**: A shared `runSecurityScan()` exported from `security/index.ts` that both install and update can call with their respective callbacks.

**Approach**: Extract install's `runSecurityScan()` to `security/run.ts`, generalize it to accept the callback signatures used by both install and update, then refactor update to use it.

**Verification**:
- `bun test` passes
- Install and update integration tests still pass
- The shared function has a unit test

---

### Step 9: Consolidate scope resolution

**Priority**: Low
**Risk**: Low
**Files**: `commands/skills/enable.ts`, `commands/skills/disable.ts` (after Step 4: toggle.ts), `commands/skills/remove.ts`

**Current State**: `resolveScope()` exists in `ui/resolve.ts` and is used by some commands, but `enable`/`disable`/`remove` inline `args.project ? "project" : args.global ? "global" : undefined` instead of using it.

**Target State**: All commands that determine scope from `--project`/`--global` flags use `resolveScope()`.

**Approach**: Replace inline ternaries with `resolveScope()` calls. May need to adjust `resolveScope` signature if it currently does more than just extract the flag (e.g., prompts).

**Verification**:
- `bun test` passes
- Grep for `args.project ? "project" : args.global ? "global"` returns 0 hits

---

## Non-Goals (Investigated but not worth the cost)

- **Command factory/middleware pattern**: A `createCommand()` wrapper that auto-loads config + policy + agentMode was considered. However, citty doesn't have a middleware model, and wrapping `defineCommand` would obscure the command structure and make it harder to read individual commands. The boilerplate reduction per command (~3 lines) doesn't justify the abstraction cost.

- **Merging link.ts and symlink.ts**: They serve different purposes (project-local symlinks vs agent-specific symlinks). The duplicated `clearForSymlink` logic is only ~10 lines. Not worth the coupling.

- **Merging adopt.ts and move.ts**: Different domain operations, different inputs. Sharing a few path helpers isn't worth the conceptual merge.

- **Generic `readAndParse<T>()`**: While the read→parse→validate pattern repeats, each site has enough variation (TOML vs JSON, error vs warning collection, different error contexts) that a generic function would need many parameters. The `parseWithResult()` helper in Step 5 captures the most common case without over-abstracting.
