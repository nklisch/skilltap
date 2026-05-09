# Refactor Plan: After Phases 39–43

## Overview

Five phases of major reshape have landed: plugin capture (39), agent-mode demolition
(40), output abstraction (41), typed CLI surface (42), Claude Code adoption (43). Net code
delta is ~–4000 LOC. The codebase shape is fundamentally sound — no circular imports, no
TODO/FIXME debt, no Result-type violations, no throwing in core, no Zod-boundary leaks.

What's left is duplication that emerged from agents working in parallel and a few
naming/pattern inconsistencies. This refactor pass consolidates ~250 lines and aligns
patterns before Phase 44's TUI work compounds them.

**No public API breakage** in any step. Function renames go through deprecation aliases
where consumers might exist.

## Scope: in vs out

**In scope:**
- Extract `setupOutput(args)` helper to consolidate 46 `createOutput({ json, quiet })` boilerplate sites.
- Mirror `install/shared.ts` with `remove/shared.ts` for the three remove handlers.
- Migrate `install/mcp.ts` to use `install/shared.ts` (currently bypasses it).
- Replace inline scope-resolution ternaries in 5 commands with the existing `resolveScope()` helper.
- Extract `ui/picker.ts` for clack picker boilerplate (toggle + adopt).
- Rename for MCP/agent-plugin naming consistency: `installMcpOnly` → `installMcp`, `removeMcpInstall` → `removeMcp`, `adoptAgentPlugin` → `adoptPlugin`.
- Delete unused `onDeepScan` callback from `InstallOptions`.
- Replace one `node:fs/promises` import with `Bun.$` in `capture.ts`.
- Migrate 3 plugin test files from `mkdtemp()` to `createTestEnv()`.

**Out of scope:**
- Splitting long files (all >400-line files are well-factored single-responsibility).
- Removing the 13 unused output schema exports (intentional placeholders for follow-up tightening).
- Scope-enum SSoT extraction (acceptable as inline schema literals; no runtime drift).
- Test rewriting against contracts (no impl-driven tests found in audit).
- Touching the agent-plugins module structure (clean).

## Refactor Steps

### Step 1: Extract `setupOutput(args)` helper

**Priority**: High
**Risk**: Low
**Files**: New `packages/cli/src/ui/setup.ts`; modified ~46 files under `packages/cli/src/commands/`.

**Current State**:

Across 46 command files:
```typescript
const out = createOutput({ json: args.json, quiet: args.quiet });          // 24 sites
const out = createOutput({ json: args.json, quiet: false });               // 18 sites
const out = createOutput({ json: false, quiet: false });                   // 4 sites
```

**Target State**:

New helper:
```typescript
// packages/cli/src/ui/setup.ts
import { createOutput, type Output } from "../output";

export interface OutputArgs {
  json?: boolean;
  quiet?: boolean;
}

/**
 * Construct an Output from a citty args object. Defaults json/quiet to false.
 * Use at the top of every CLI command's run() body.
 */
export function setupOutput(args: OutputArgs): Output {
  return createOutput({
    json: args.json ?? false,
    quiet: args.quiet ?? false,
  });
}
```

Each command:
```typescript
import { setupOutput } from "../ui/setup";
// ...
const out = setupOutput(args);
```

**Implementation Notes**:
- Mechanical replacement. Run a project-wide find-and-replace.
- Commands without a `quiet` arg (e.g., `config get`, `tap remove`) still work — `args.quiet` is `undefined` and the helper coalesces to `false`.
- One commit. No need to migrate progressively.

**Acceptance Criteria**:
- [ ] `packages/cli/src/ui/setup.ts` exists and exports `setupOutput`.
- [ ] `grep -rn "createOutput(" packages/cli/src/commands/ --include="*.ts" | grep -v ".test.ts"` returns zero matches (all migrated to `setupOutput(args)`).
- [ ] Build passes; `bun test` passes.
- [ ] No behavioral regression — `--json`, `--quiet`, default modes all still work.

---

### Step 2: Extract `remove/shared.ts`

**Priority**: High
**Risk**: Low
**Files**: New `packages/cli/src/commands/remove/shared.ts`; modified `remove/skill.ts`, `remove/plugin.ts`, `remove/mcp.ts`.

**Current State**:

Each of the three remove handlers has:
```typescript
async run({ args }) {
  const out = createOutput({ json: args.json, quiet: false });
  const policyResult = await loadPolicyOrExit(args, out);
  if (!policyResult) return;
  const { config, policy } = policyResult;
  const projectRoot = await tryFindProjectRoot();
  const scope = resolveScope(args, config);
  // ... per-type body
}
```

`remove/skill.ts` (182 lines), `remove/plugin.ts` (80 lines), `remove/mcp.ts` (88 lines). Common prelude is ~40 lines × 3 = ~120 lines duplicated.

**Target State**:

```typescript
// packages/cli/src/commands/remove/shared.ts
import type { Output } from "@skilltap/core";
import { tryFindProjectRoot, type Config, type EffectivePolicy } from "@skilltap/core";
import { setupOutput } from "../../ui/setup";
import { loadPolicyOrExit } from "../../ui/policy";
import { resolveScope } from "../../ui/resolve";

export interface RemoveContext {
  out: Output;
  config: Config;
  policy: EffectivePolicy;
  projectRoot: string | null;
  scope: "global" | "project";
}

/**
 * Shared prelude for the three remove subcommands. Constructs Output, loads
 * policy, resolves project root + scope. Exits with error if any step fails.
 */
export async function setupRemoveContext(args: {
  json?: boolean;
  project?: boolean;
  global?: boolean;
}): Promise<RemoveContext | null> {
  const out = setupOutput(args);
  const policyResult = await loadPolicyOrExit(args, out);
  if (!policyResult) return null;
  const { config, policy } = policyResult;
  const projectRoot = await tryFindProjectRoot();
  const scope = resolveScope(args, config);
  return { out, config, policy, projectRoot, scope };
}
```

Each handler:
```typescript
async run({ args }) {
  const ctx = await setupRemoveContext(args);
  if (!ctx) return;
  const { out, scope, projectRoot } = ctx;
  // ... per-type body
}
```

**Implementation Notes**:
- Mirror the pattern from `install/shared.ts`'s `setupInstallContext`.
- Don't include the `--yes` confirmation prompt in the shared helper — that's per-type. Each handler handles its own confirmation flow.
- Test focus: each remove command's existing tests must still pass — same observable behavior.

**Acceptance Criteria**:
- [ ] `packages/cli/src/commands/remove/shared.ts` exports `setupRemoveContext`.
- [ ] Each `remove/*.ts` is shorter (target: 150 / 60 / 70 lines down from 182 / 80 / 88).
- [ ] Existing remove tests pass without modification.
- [ ] Build passes.

---

### Step 3: Migrate `install/mcp.ts` to `install/shared.ts`

**Priority**: High
**Risk**: Low
**Files**: `packages/cli/src/commands/install/mcp.ts`.

**Current State**:

`install/mcp.ts` rolls its own setup:
```typescript
async run({ args }) {
  const out = createOutput({ json: args.json, quiet: false });
  const policyResult = await loadPolicyOrExit(args, out);
  // ... 30 lines of context setup ...
  const scope = args.project ? "project" : args.global ? "global" : "project";
  // ... eventual installMcpOnly() call
}
```

While `install/skill.ts` and `install/plugin.ts` use `setupInstallContext()` from shared.

**Target State**:

```typescript
async run({ args }) {
  const ctx = await setupInstallContext(args);
  if (!ctx) return;
  const { out, scope, projectRoot, also } = ctx;
  // ... rest of MCP-specific install flow
}
```

**Implementation Notes**:
- Verify `install/shared.ts setupInstallContext()` returns everything `install/mcp.ts` needs. If it's missing fields specific to MCP install (e.g., `also` already parsed), extend the shared context type.
- The MCP flow doesn't need `--strict` / `--no-strict` / `--semantic` flags. If they're in shared but unused in MCP, that's fine (extra fields are ignored).

**Acceptance Criteria**:
- [ ] `install/mcp.ts` calls `setupInstallContext(args)` instead of constructing pieces manually.
- [ ] `install/mcp.ts` is shorter (target: ~70 lines down from 99).
- [ ] Existing `install mcp` tests pass.

---

### Step 4: Inline scope-resolution → `resolveScope()`

**Priority**: Medium
**Risk**: Low
**Files**: `commands/adopt.ts`, `commands/status.ts`, `commands/move.ts`, `commands/info.ts`, possibly `install/mcp.ts` (if Step 3 doesn't catch it).

**Current State**:

Inline ternary chains in 5+ files:
```typescript
// adopt.ts:118
const scope = args.project ? "project" : args.global ? "global" : "project";
// status.ts:77-83
const scope = args.global
  ? "global"
  : args.project
    ? "project"
    : config.defaults.scope || "project";
// move.ts:33-43
// (similar)
```

Meanwhile `cli/src/ui/resolve.ts` exports `resolveScope(args, config)` that does this exact resolution canonically.

**Target State**:

```typescript
import { resolveScope } from "../ui/resolve";
// ...
const scope = resolveScope(args, config);
```

**Implementation Notes**:
- The canonical `resolveScope` already exists. This is a "use the helper" step.
- Each call site needs the local `config` in scope. If a command doesn't load config currently, add the load.
- One file at a time. Run targeted tests after each: `bun test packages/cli/src/commands/<name>.test.ts`.

**Acceptance Criteria**:
- [ ] `grep -rn "args.project ? \\|args.global ?" packages/cli/src/commands/ --include="*.ts" | grep -v ".test.ts"` returns no matches outside the canonical `resolveScope` impl.
- [ ] Each migrated command's tests pass.
- [ ] Behavior unchanged: `--project` and `--global` flags still work; default scope still inferred from project root.

---

### Step 5: Extract `ui/picker.ts` for clack picker boilerplate

**Priority**: Medium
**Risk**: Low
**Files**: New `packages/cli/src/ui/picker.ts`; modified `commands/toggle.ts`, `commands/adopt.ts`.

**Current State**:

`runTogglePicker()` (toggle.ts:307–400, 94 lines) and `runAdoptPicker()` (adopt.ts:111–180, 70 lines) both:
1. Build a list of `{ value, label, hint? }` options.
2. Call clack `select({ message, options })`.
3. Handle `isCancel` → emit cancellation message.
4. Dispatch on selected value.

**Target State**:

```typescript
// packages/cli/src/ui/picker.ts
import { isCancel, select } from "@clack/prompts";
import type { Output } from "@skilltap/core";

export interface PickerOption<T> {
  value: T;
  label: string;
  hint?: string;
}

/**
 * Wrap clack's select() with cancel handling. Returns the picked value or
 * null on cancel. Caller emits the cancel message via out if needed.
 */
export async function pickOne<T>(opts: {
  message: string;
  options: PickerOption<T>[];
  emptyMessage?: string;
  out: Output;
}): Promise<T | null> {
  if (opts.options.length === 0) {
    opts.out.info(opts.emptyMessage ?? "Nothing to pick.");
    return null;
  }
  const choice = await select({
    message: opts.message,
    options: opts.options.map((o) => ({
      value: o.value,
      label: o.label,
      hint: o.hint,
    })),
  });
  if (isCancel(choice)) {
    opts.out.info("Cancelled.");
    return null;
  }
  return choice as T;
}
```

Each picker:
```typescript
const choice = await pickOne({
  message: "What do you want to toggle?",
  options: [
    { value: "skill", label: "Skill" },
    { value: "plugin", label: "Plugin" },
    { value: "mcp", label: "MCP server" },
  ],
  out,
});
if (!choice) return;
// dispatch on choice
```

**Implementation Notes**:
- Phase 44 will replace these clack pickers with Ink TUI screens. Step 5 makes that easier — there's one `pickOne()` to swap out, not two.
- The existing pickers may have multi-step flows (pick type → pick name → multiselect components). The first-step `select` migrates cleanly. Multi-select is its own helper if needed; for Phase 5 we focus on `pickOne`.
- A `multiSelect()` helper can follow as Step 5b if there's value (toggle.ts has a multiselect for components).

**Acceptance Criteria**:
- [ ] `packages/cli/src/ui/picker.ts` exists.
- [ ] `commands/toggle.ts` and `commands/adopt.ts` use `pickOne()` for their first-level select.
- [ ] Existing tests for toggle and adopt pickers pass (or are updated for the new API).
- [ ] No behavioral regression — picker UX is unchanged.

---

### Step 6: Naming consistency for MCP and adopted-plugin functions

**Priority**: Medium
**Risk**: Medium (rename touches many call sites; carefully sequenced)
**Files**: `core/src/mcp-install.ts`, `core/src/adopt.ts`, all importers.

**Current State**:

```typescript
// core/src/mcp-install.ts
export async function installMcpOnly(...);

// somewhere
export async function removeMcpInstall(...);     // (or removeMcpServers — verify)

// core/src/adopt.ts (Phase 43)
export async function adoptAgentPlugin(...);
```

Inconsistency:
- `installMcpOnly` has an `Only` suffix the others lack.
- `removeMcpInstall` has `Install` in the middle (probably from "remove an mcp install").
- `adoptAgentPlugin` has the `Agent` qualifier; the skill version is `adoptSkill`.

**Target State**:

```typescript
export async function installMcp(...);     // was installMcpOnly
export async function removeMcp(...);       // was removeMcpInstall
export async function adoptPlugin(...);     // was adoptAgentPlugin
```

**Implementation Notes**:
- Each rename is one commit. Three commits total.
- Per rename:
  1. Rename the function.
  2. Add a deprecated alias: `export const installMcpOnly = installMcp;` so any external consumers (none expected; the codebase is pre-release) still work for one cycle.
  3. Update all internal call sites to the new name.
  4. Update tests asserting on internal function names (rare).
- After all three renames are merged, a follow-up commit can delete the deprecated aliases (out of scope for this refactor pass; queue for Phase 46 polish).

**Acceptance Criteria**:
- [ ] Three rename commits, one per function.
- [ ] Each commit: function renamed; deprecated alias exported; all internal call sites updated; tests pass.
- [ ] After all three: `grep -rn "installMcpOnly\|removeMcpInstall\|adoptAgentPlugin" packages/ --include="*.ts" | grep -v ".test.ts" | grep -v "^.*export const"` returns no matches (all callers migrated).

---

### Step 7: Delete unused `onDeepScan` callback

**Priority**: Low
**Risk**: Low
**Files**: `packages/core/src/install.ts`, `packages/cli/src/commands/install/shared.ts` (if it's set there).

**Current State**:

```typescript
// core/src/install.ts InstallOptions
onDeepScan?: (count: number) => Promise<boolean>;   // line 91
```

Defined but **never invoked anywhere** in core. CLI may or may not set it (verify); regardless, it's dead.

**Target State**:

Field removed from `InstallOptions`. All call sites that set it have the line deleted.

**Implementation Notes**:
- Verify no test asserts it's set.
- Search: `grep -rn "onDeepScan" packages/ --include="*.ts"`. Should find: 1 declaration in core/install.ts, possibly some setting in CLI, 0 invocations.

**Acceptance Criteria**:
- [ ] `grep -rn "onDeepScan" packages/ --include="*.ts"` returns 0 matches.
- [ ] Build passes; tests pass.

---

### Step 8: Migrate plugin test files to `createTestEnv()`

**Priority**: Low
**Risk**: Low
**Files**: `packages/core/src/plugin/e2e-lifecycle.test.ts`, `plugin/install.test.ts`, `plugin/install.capture.test.ts`.

**Current State**:

Three plugin test files use `mkdtemp()` directly:
```typescript
const tmp = await mkdtemp(join(tmpdir(), "skilltap-test-"));
process.env.SKILLTAP_HOME = tmp;
// ...
afterEach(async () => {
  await rm(tmp, { recursive: true, force: true });
});
```

While other tests use the canonical `createTestEnv()`:
```typescript
let env: TestEnv;
beforeEach(async () => { env = await createTestEnv(); });
afterEach(async () => { await env.cleanup(); });
```

**Target State**:

All three files migrated to `createTestEnv()`. Same isolation guarantees, more consistent shape.

**Implementation Notes**:
- `createTestEnv()` returns `{ homeDir, configDir, cleanup }`. Adapt test code to use `homeDir` instead of the locally-named tmpdir.
- One file at a time; verify tests still pass.

**Acceptance Criteria**:
- [ ] All three files use `createTestEnv()`.
- [ ] No `mkdtemp()` calls in `packages/core/src/plugin/*.test.ts` (other plugin tests already use it; just these three).
- [ ] Tests pass.

---

### Step 9: Replace `node:fs/promises` `rm` with `Bun.$`

**Priority**: Low
**Risk**: Low
**Files**: `packages/core/src/plugin/capture.ts`.

**Current State**:

```typescript
import { rm } from "node:fs/promises";
// ...
await rm(targetPath, { recursive: true, force: true });
```

**Target State**:

```typescript
import { $ } from "bun";
// ...
await $`rm -rf ${targetPath}`.quiet();
```

**Implementation Notes**:
- Single-site change.
- The `Bun.$` template handles paths safely (no shell injection).
- `quiet()` matches the project's git/shell pattern.

**Acceptance Criteria**:
- [ ] `grep -rn "from \"node:fs/promises\"" packages/core/src/plugin/capture.ts` returns no match.
- [ ] Tests in `plugin/capture.test.ts` pass.

---

### Step 10: Cleanup pass — delete deprecated aliases from Step 6

**Priority**: Low
**Risk**: Low (atomic rename, but deprecated aliases protect during the migration cycle)
**Files**: `core/src/mcp-install.ts`, `core/src/adopt.ts`, `core/src/index.ts`.

**Current State** (after Step 6 lands):

```typescript
export async function installMcp(...);
export const installMcpOnly = installMcp;   // deprecated alias

export async function removeMcp(...);
export const removeMcpInstall = removeMcp;  // deprecated alias

export async function adoptPlugin(...);
export const adoptAgentPlugin = adoptPlugin; // deprecated alias
```

**Target State**:

The deprecated aliases removed; only the canonical names exist.

**Implementation Notes**:
- This step lands AFTER Step 6 — needs the renames merged first. Could be deferred to Phase 46 polish if any external consumers exist; for this pre-release codebase, immediate is fine.
- Verify no test or external consumer references the old name first.

**Acceptance Criteria**:
- [ ] Aliases removed.
- [ ] `grep -rn "installMcpOnly\|removeMcpInstall\|adoptAgentPlugin" packages/` returns 0 matches.
- [ ] Build passes; tests pass.

---

## Implementation Order

Order by dependency and risk:

1. **Step 1** — `setupOutput(args)` helper. Highest value, lowest risk. Touches the most files but mechanically.
2. **Step 7** — Delete `onDeepScan` (small, isolated cleanup; safe to run anywhere).
3. **Step 9** — `Bun.$` for `rm` (one-site, isolated).
4. **Step 8** — Test isolation cleanup (3 test files; independent of source changes).
5. **Step 4** — Inline scope-resolution → `resolveScope()` (independent; uses existing helper).
6. **Step 3** — `install/mcp.ts` to `install/shared.ts` (uses existing helper; simple).
7. **Step 2** — `remove/shared.ts` extraction (new helper module; touches 3 commands).
8. **Step 5** — `pickOne()` extraction (new helper; touches 2 commands).
9. **Step 6a-c** — Rename commits (one per function; sequential).
10. **Step 10** — Delete deprecated aliases (after Step 6 batch).

Steps 1–4 are independent and could parallelize; sequential agent execution is simpler.

## Pre-mortem

**Riskiest step**: Step 6 (renames). Many call sites need updating in lockstep. Mitigation: deprecated aliases preserve compatibility during the migration cycle; tests catch any missed import.

**Riskiest assumption**: That `install/shared.ts setupInstallContext()` (referenced by Step 3) actually returns everything `install/mcp.ts` needs. If it doesn't, Step 3 grows to extending shared first. Mitigation: Step 3's implementation begins by reading `install/shared.ts`; if a gap exists, the agent adds the missing field.

**Fallback**: Each step is a separate commit. If Step 5 (picker extraction) reveals more multiselect complexity than expected, defer to Phase 44 where the TUI rewrite handles it natively. Skip without losing other progress.

## Verification Checklist (project-wide, after all steps)

```bash
# 1. Build
bun run build

# 2. Source-side checks
grep -rn "createOutput(" packages/cli/src/commands/ --include="*.ts" | grep -v ".test.ts"
# Expect: empty after Step 1.

grep -rn "args.project ? \\|args.global ?" packages/cli/src/commands/ --include="*.ts" | grep -v ".test.ts"
# Expect: empty after Step 4.

grep -rn "from \"node:fs/promises\"" packages/core/src/plugin/ --include="*.ts"
# Expect: empty after Step 9.

grep -rn "onDeepScan" packages/ --include="*.ts"
# Expect: empty after Step 7.

grep -rn "installMcpOnly\|removeMcpInstall\|adoptAgentPlugin" packages/ --include="*.ts" | grep -v ".test.ts"
# Expect: empty after Step 10.

# 3. Tests
bun test
```

Expected outcome: ~250 lines net deletion across the codebase, full suite green, patterns more uniform for Phase 44's TUI work.
