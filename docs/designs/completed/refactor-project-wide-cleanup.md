# Refactor Plan: Project-Wide Cleanup

## Overview

The skilltap codebase is healthy at the macro level — the architecture document describes module boundaries that the source roughly honors, established patterns are documented, and dead code / commented-out blocks / stale TODOs are absent. The opportunities are at the *seam* level: orchestration logic that has accumulated across lifecycle commands, hardcoded literals that bypass the centralized constants the project has already created for them, two long files (`install.ts` 1021, `update.ts` 839) that conflate three or four phases each, and a handful of Zod-boundary holes where external JSON enters core via `JSON.parse + as`.

The refactor is purely structural — no behavior changes, no public-API breaks for `@skilltap/core` consumers, no schema changes. Every step is independently testable against the existing source-mode and binary-mode test suites.

The plan is **13 steps**, ordered so that helpers land before their consumers. Steps 1–7 are pure mechanical extractions; steps 8–11 unblock the larger decompositions; step 12 is the only step with real reach, and it depends on the helpers from earlier steps existing first.

**Estimated impact:**
- ~250–500 lines eliminated from production code.
- `install.ts` shrinks from 1021 → ~400 lines split across 3–4 files.
- `update.ts` shrinks from 839 → ~300 lines split per adapter.
- 6 new Zod schemas plug external-JSON validation gaps.
- Single-source-definitions extended to plugin-format dirs and the default agent ID.
- 200+ ad-hoc `process.exit(1)` paths in CLI commands collapse to a single `exitOnError(result, out)` helper.

## Refactor Steps

### Step 1: Centralize plugin-format directory constants

**Priority**: High
**Risk**: Low
**Files**: `packages/core/src/manifest/paths.ts`, `packages/core/src/plugin/detect.ts`, `packages/core/src/plugin/parse-claude.ts`, `packages/core/src/plugin/parse-codex.ts`, `packages/core/src/taps.ts`

`PUBLISH_DIR = ".skilltap"` already lives in `manifest/paths.ts`, but the sibling values `.claude-plugin` and `.codex-plugin` are hardcoded across 8+ call sites. The pattern doc on single-source-definitions calls this exact case out.

**Current State**:

```typescript
// packages/core/src/plugin/detect.ts:70-75
if (await Bun.file(join(dir, ".claude-plugin", "plugin.json")).exists()) {
  return parseClaudePlugin(dir);
}
if (await Bun.file(join(dir, ".codex-plugin", "plugin.json")).exists()) {
  return parseCodexPlugin(dir);
}

// packages/core/src/plugin/parse-claude.ts:16
const manifestPath = join(pluginDir, ".claude-plugin", "plugin.json");

// packages/core/src/plugin/parse-codex.ts:15
const manifestPath = join(pluginDir, ".codex-plugin", "plugin.json");

// packages/core/src/taps.ts:84
join(dir, ".claude-plugin", "marketplace.json"),
```

**Target State**:

```typescript
// packages/core/src/manifest/paths.ts (extended)
export const PUBLISH_DIR = ".skilltap";
export const CLAUDE_PLUGIN_DIR = ".claude-plugin";
export const CODEX_PLUGIN_DIR = ".codex-plugin";

export function claudePluginManifestPath(repoRoot: string): string {
  return join(repoRoot, CLAUDE_PLUGIN_DIR, "plugin.json");
}
export function codexPluginManifestPath(repoRoot: string): string {
  return join(repoRoot, CODEX_PLUGIN_DIR, "plugin.json");
}
export function marketplaceManifestPath(repoRoot: string): string {
  return join(repoRoot, CLAUDE_PLUGIN_DIR, "marketplace.json");
}
```

```typescript
// packages/core/src/plugin/detect.ts:70-75 (after)
if (await Bun.file(claudePluginManifestPath(dir)).exists()) {
  return parseClaudePlugin(dir);
}
if (await Bun.file(codexPluginManifestPath(dir)).exists()) {
  return parseCodexPlugin(dir);
}
```

**Implementation Notes**:
- Add the three helper functions and three constants to `manifest/paths.ts`.
- Replace every callsite found by `grep -rn '\.claude-plugin\|\.codex-plugin' packages/core/src --include="*.ts"`.
- Doc-comment references to `.claude-plugin/` and `.codex-plugin/` in JSDoc remain as plain strings (they're prose, not literals).
- Riskiest part: missing a callsite. Mitigation: grep again after the edit; CI catches anything that breaks.

**Acceptance Criteria**:
- [ ] `bun run build` succeeds.
- [ ] `bun test` passes (plugin detect tests cover both branches).
- [ ] No `".claude-plugin"` or `".codex-plugin"` literals remain in `packages/core/src/**/*.ts` outside `manifest/paths.ts` and prose comments.

---

### Step 2: Add DEFAULT_AGENT_ID + use it for plugin/MCP defaults

**Priority**: High
**Risk**: Low
**Files**: `packages/core/src/symlink.ts`, `packages/core/src/mcp-install.ts`, `packages/core/src/schemas/plugins.ts`, `packages/core/src/plugin/install.ts`, `packages/core/src/plugin/lifecycle.ts`, `packages/core/src/plugin/state.ts`

`"claude-code"` appears as a literal default in 7 places. Some uses are correct (the agent-definitions feature only targets Claude Code today), but they should still resolve through a named constant so a future second target is one edit instead of seven.

**Current State**:

```typescript
// packages/core/src/mcp-install.ts:143-146
const agents =
  options.agents && options.agents.length > 0
    ? options.agents
    : ["claude-code"];

// packages/core/src/schemas/plugins.ts:46
platform: z.string().default("claude-code"),

// packages/core/src/plugin/install.ts:287-291
const dest = agentDefPath(
  component.name,
  "claude-code",
  scope,
  projectRoot,
);
```

**Target State**:

```typescript
// packages/core/src/symlink.ts (extended)
export const DEFAULT_AGENT_ID = "claude-code" as const;

// packages/core/src/mcp-install.ts:143-146
const agents =
  options.agents && options.agents.length > 0
    ? options.agents
    : [DEFAULT_AGENT_ID];

// packages/core/src/schemas/plugins.ts:46
platform: z.string().default(DEFAULT_AGENT_ID),

// packages/core/src/plugin/install.ts:287-291
const dest = agentDefPath(
  component.name,
  DEFAULT_AGENT_ID,
  scope,
  projectRoot,
);
```

**Implementation Notes**:
- Export `DEFAULT_AGENT_ID` from `symlink.ts` (already houses `AGENT_PATHS`, `AGENT_LABELS`, `VALID_AGENT_IDS`).
- The five `plugin/lifecycle.ts` literals at 127, 133, 263, 269 are also defaults for the same agent-defs lookup — replace those too.
- Schemas: `z.string().default(DEFAULT_AGENT_ID)` — re-export from index where needed.
- Riskiest part: a literal `"claude-code"` that semantically means "the user explicitly chose Claude Code" gets accidentally replaced with `DEFAULT_AGENT_ID` — but those live in tests and adopters' opt-in lists, which this step does not touch.

**Acceptance Criteria**:
- [ ] `bun test` passes.
- [ ] Outside `symlink.ts`, the `agent-plugins/claude-code.ts` scanner identity, `plugin/parse-claude.ts:format`, and tests, no `"claude-code"` literal remains as a *default* value.
- [ ] `grep '"claude-code"' packages/core/src --include="*.ts"` returns only those allowed-literal sites.

---

### Step 3: Extract `currentSkillDir(record, scope, projectRoot)`

**Priority**: Medium
**Risk**: Low
**Files**: `packages/core/src/paths.ts`, `packages/core/src/install.ts`, `packages/core/src/update.ts`, `packages/core/src/adopt.ts`, `packages/core/src/move.ts`, `packages/core/src/disable.ts`

The conditional `record.active === false ? skillDisabledDir(...) : skillInstallDir(...)` is repeated 8 times across lifecycle modules. The disabled-skill branch is the kind of thing that's easy to forget when adding a new lifecycle command.

**Current State**:

```typescript
// packages/core/src/update.ts:271-275 (and 7 similar blocks)
const scope = record.scope as "global" | "project";
const destDir =
  record.active === false
    ? skillDisabledDir(name, scope, projectRoot)
    : skillInstallDir(name, scope, projectRoot);
```

**Target State**:

```typescript
// packages/core/src/paths.ts (extended)
import type { InstalledSkill } from "./schemas/installed";

export function currentSkillDir(
  record: Pick<InstalledSkill, "name" | "scope" | "active">,
  projectRoot?: string,
): string {
  const scope = record.scope as "global" | "project";
  return record.active === false
    ? skillDisabledDir(record.name, scope, projectRoot)
    : skillInstallDir(record.name, scope, projectRoot);
}
```

```typescript
// packages/core/src/update.ts:271 (after)
const destDir = currentSkillDir(record, projectRoot);
```

**Implementation Notes**:
- The helper takes a `Pick<InstalledSkill, ...>` type so it doesn't require the full record at every callsite — important for tests.
- `paths.ts` already imports nothing schema-side; the new import adds a one-way dependency from paths.ts → schemas/installed.ts. That direction is fine (paths is a leaf).
- Riskiest part: a callsite that intentionally wants the active path even on a disabled record (none currently exist; verify with `grep` before edit).

**Acceptance Criteria**:
- [ ] `bun test` passes.
- [ ] No callsite in `packages/core/src` outside `paths.ts` ternaries between `skillDisabledDir` and `skillInstallDir` based on `active === false`.

---

### Step 4: Extract orphan-purge helper

**Priority**: High
**Risk**: Low
**Files**: `packages/core/src/orphan.ts`, `packages/core/src/install.ts`, `packages/core/src/update.ts`

The pre-install/pre-update orphan-purge sequence is duplicated. `update.ts` purges twice per call (global scope + project scope), so the same 30-line block actually runs three times in three places.

**Current State**:

```typescript
// packages/core/src/install.ts:529-547
if (options.onOrphansFound) {
  const orphans = await findOrphanRecords(installed, options.projectRoot);
  if (orphans.length > 0) {
    const namesToPurge = await options.onOrphansFound(orphans);
    if (namesToPurge.length > 0) {
      const toPurge = orphans.filter((o) =>
        namesToPurge.includes(o.record.name),
      );
      await purgeOrphanRecords(toPurge, installed, fileRoot);
      const purgedNames = new Set(namesToPurge);
      installed.splice(
        0,
        installed.length,
        ...installed.filter((s) => !purgedNames.has(s.name)),
      );
    }
  }
}

// packages/core/src/update.ts:717-756 (twice — global + project)
// (near-identical block, with `globalInstalled.skills = ...filter(...)`
// instead of in-place splice)
```

**Target State**:

```typescript
// packages/core/src/orphan.ts (extended)
export async function purgeOrphansWithCallback(
  installed: InstalledSkill[],
  fileRoot: string | undefined,
  projectRoot: string | undefined,
  onOrphansFound: ((orphans: OrphanRecord[]) => Promise<string[]>) | undefined,
): Promise<InstalledSkill[]> {
  if (!onOrphansFound) return installed;
  const orphans = await findOrphanRecords(installed, projectRoot);
  if (orphans.length === 0) return installed;
  const namesToPurge = await onOrphansFound(orphans);
  if (namesToPurge.length === 0) return installed;
  const toPurge = orphans.filter((o) => namesToPurge.includes(o.record.name));
  await purgeOrphanRecords(toPurge, installed, fileRoot);
  const purgedNames = new Set(namesToPurge);
  return installed.filter((s) => !purgedNames.has(s.name));
}
```

```typescript
// packages/core/src/install.ts:529-547 (after)
const purged = await purgeOrphansWithCallback(
  installed,
  fileRoot,
  options.projectRoot,
  options.onOrphansFound,
);
installed.splice(0, installed.length, ...purged); // preserve install.ts's
                                                  // mutate-in-place contract
```

**Implementation Notes**:
- Returning a new array (not mutating) is the cleaner shape. `install.ts` has an in-place mutation contract that downstream code in the same function relies on — splice the result back in.
- `update.ts` calls it twice (once per scope) and reassigns the field — clean.
- Riskiest part: a subtle ordering bug where downstream code in `installSkill` reads the `installed` reference after the splice. Verify with the existing orphan tests (`packages/core/src/orphan.test.ts`, `packages/core/src/orphan-integration.test.ts`).

**Acceptance Criteria**:
- [ ] `bun test packages/core/src/orphan*` passes.
- [ ] `bun test packages/core/src/install.test.ts packages/core/src/update.test.ts` passes.
- [ ] No `findOrphanRecords` call exists outside `orphan.ts` and the new helper.

---

### Step 5: Extract `validateScopeArg(scopeArg, out)` for CLI commands

**Priority**: Medium
**Risk**: Low
**Files**: `packages/cli/src/ui/resolve.ts`, `packages/cli/src/commands/move.ts`, `packages/cli/src/commands/adopt.ts`

`resolveScope()` already lives in `cli/ui/resolve.ts` and handles the runtime resolution, but two commands hand-roll a *separate* validation step before calling it (or instead of calling it).

**Current State**:

```typescript
// packages/cli/src/commands/move.ts:34-44
const scopeArg = args.scope as string | undefined;
if (scopeArg === undefined) {
  out.error("Specify target scope: --scope project|global");
  process.exit(1);
}
if (scopeArg !== "project" && scopeArg !== "global") {
  out.error(
    `Invalid --scope value '${scopeArg}'. Use 'project' or 'global'.`,
  );
  process.exit(1);
}

// packages/cli/src/commands/adopt.ts:74-84
const scopeArg = args.scope as string | undefined;
if (
  scopeArg !== undefined &&
  scopeArg !== "project" &&
  scopeArg !== "global"
) {
  out.error(
    `Invalid --scope value '${scopeArg}'. Use 'project' or 'global'.`,
  );
  process.exit(1);
}
```

**Target State**:

```typescript
// packages/cli/src/ui/resolve.ts (extended)
export function validateScopeArg(
  scopeArg: string | undefined,
  out: Output,
  options: { required?: boolean } = {},
): "project" | "global" | undefined {
  if (scopeArg === undefined) {
    if (options.required) {
      out.error("Specify target scope: --scope project|global");
      process.exit(1);
    }
    return undefined;
  }
  if (scopeArg !== "project" && scopeArg !== "global") {
    out.error(
      `Invalid --scope value '${scopeArg}'. Use 'project' or 'global'.`,
    );
    process.exit(1);
  }
  return scopeArg;
}
```

```typescript
// packages/cli/src/commands/move.ts:34-44 (after)
const scope = validateScopeArg(args.scope as string | undefined, out, {
  required: true,
});

// packages/cli/src/commands/adopt.ts:74-84 (after)
const scope = validateScopeArg(args.scope as string | undefined, out);
```

**Implementation Notes**:
- The CLI test suite already exercises both error paths — exact message comparison must be preserved.
- This step does NOT change `resolveScope()` itself; it sits ahead of it as the validation layer.
- Riskiest part: the `process.exit(1)` calls inside the helper run before any CLI cleanup. Existing inline code does the same — equivalent behavior preserved.

**Acceptance Criteria**:
- [ ] `bun test packages/cli/src/commands/move*` passes (the move command CLI tests verify the exact error messages).
- [ ] `bun test packages/cli/src/commands/adopt*` passes.
- [ ] `validateScopeArg` is exported from `cli/ui/resolve.ts` and re-exported as needed.

---

### Step 6: Extract `exitOnError(result, out)` for CLI command handlers

**Priority**: Medium
**Risk**: Low
**Files**: `packages/cli/src/ui/resolve.ts` (or new `packages/cli/src/ui/exit.ts`), all `packages/cli/src/commands/**/*.ts`

`if (!result.ok) { out.error(result.error.message, result.error.hint); process.exit(1); }` repeats 200+ times across CLI commands. A dedicated helper standardizes the message format and trims about 4 lines per callsite.

**Current State**:

```typescript
// packages/cli/src/commands/move.ts:70-73 (and ~50 similar)
if (!result.ok) {
  out.error(result.error.message, result.error.hint);
  process.exit(1);
}

// packages/cli/src/commands/adopt.ts:149-153 (and many similar with progress.fail)
if (!result.ok) {
  progress.fail("Scan failed");
  out.error(result.error.message, result.error.hint);
  process.exit(1);
}
```

**Target State**:

```typescript
// packages/cli/src/ui/exit.ts (new)
import type { Output } from "@skilltap/core";
import type { Result } from "@skilltap/core";

interface SkilltapErrorLike {
  message: string;
  hint?: string;
}

export function exitOnError<T, E extends SkilltapErrorLike>(
  result: Result<T, E>,
  out: Output,
  options: { onError?: () => void } = {},
): asserts result is { ok: true; value: T } {
  if (!result.ok) {
    options.onError?.();
    out.error(result.error.message, result.error.hint);
    process.exit(1);
  }
}
```

```typescript
// packages/cli/src/commands/move.ts:70-73 (after)
exitOnError(result, out);

// packages/cli/src/commands/adopt.ts:149-153 (after)
exitOnError(result, out, { onError: () => progress.fail("Scan failed") });
```

**Implementation Notes**:
- Use `asserts result is { ok: true; value: T }` so callers below can dereference `result.value` with no extra type guard.
- Spread the conversion across files in 5-10 commits if review burden matters; the test suite catches regressions on each.
- Many sites also call `out.json({ kind: "error", ... })` before exit — those are distinct from this helper and stay inline.
- Riskiest part: a callsite that does post-error cleanup beyond `progress.fail` — those still work via the `onError` hook. Verify by reading every match before converting.

**Acceptance Criteria**:
- [ ] `bun test` passes (full CLI suite — error paths are well-tested).
- [ ] `bun run verify:binary:tests` passes.
- [ ] At least 50% of the duplicated blocks are converted (some sites with custom error formatting are deliberately left inline).
- [ ] CLI test files validating error exit codes and messages still pass without modification.

---

### Step 7: Replace `process.stderr.write` in core via warning callback

**Priority**: Medium
**Risk**: Low
**Files**: `packages/core/src/taps.ts`, callers of `loadTaps`/`filterAndWarnHttpTaps`

`taps.ts:48` writes ANSI-colored output directly to `process.stderr`. The Output-interface pattern requires all output to flow through a CLI-controlled abstraction — this is the single remaining direct write in core.

**Current State**:

```typescript
// packages/core/src/taps.ts:38-57
const httpWarningEmittedFor = new Set<string>();

function filterAndWarnHttpTaps(taps: readonly ConfigTap[]): ConfigTap[] {
  const result: ConfigTap[] = [];
  for (const tap of taps) {
    if (tap.type === "http") {
      if (!httpWarningEmittedFor.has(tap.name)) {
        httpWarningEmittedFor.add(tap.name);
        const DIM = "\x1b[2m";
        const RESET = "\x1b[0m";
        process.stderr.write(
          `${DIM}↑  HTTP tap '${tap.name}' ignored — HTTP support removed in v2.0. Use a git tap or run 'skilltap migrate'.${RESET}\n`,
        );
      }
      continue;
    }
    result.push(tap);
  }
  return result;
}
```

**Target State**:

```typescript
// packages/core/src/taps.ts (after)
export type LoadTapsOptions = {
  onHttpTapIgnored?: (tapName: string) => void;
};

const httpWarningEmittedFor = new Set<string>();

function filterAndWarnHttpTaps(
  taps: readonly ConfigTap[],
  onWarn?: (name: string) => void,
): ConfigTap[] {
  const result: ConfigTap[] = [];
  for (const tap of taps) {
    if (tap.type === "http") {
      if (!httpWarningEmittedFor.has(tap.name) && onWarn) {
        httpWarningEmittedFor.add(tap.name);
        onWarn(tap.name);
      }
      continue;
    }
    result.push(tap);
  }
  return result;
}

// loadTaps signature gains the option:
export async function loadTaps(
  options: LoadTapsOptions = {},
): Promise<Result<TapEntry[], UserError>> { ... }
```

```typescript
// packages/cli/src/commands/sync.ts and other loadTaps callsites
const tapsResult = await loadTaps({
  onHttpTapIgnored: (name) =>
    out.warn(
      `HTTP tap '${name}' ignored — HTTP support removed in v2.0. Use a git tap or run 'skilltap migrate'.`,
    ),
});
```

**Implementation Notes**:
- Removing the ANSI codes is fine — `Output.warn` already applies its own styling per output mode.
- The `httpWarningEmittedFor` Set stays inside taps.ts; that's de-duplication state, not output.
- Callers that don't pass the callback (e.g. internal `loadTaps()` invocations from other core modules) silently drop the warning — same end-user behavior since the CLI's outer `loadTaps` call is the one users see.
- Riskiest part: a CLI test that asserts on the literal warning text in stderr — after this change that text comes from the CLI's `out.warn`, which may format differently. Update the test if so.

**Acceptance Criteria**:
- [ ] `bun test` passes.
- [ ] No `process.stdout.write` or `process.stderr.write` calls remain in `packages/core/src/**/*.ts` (verify with grep).
- [ ] HTTP-tap CLI tests still observe the warning.

---

### Step 8: Plug Zod-boundary holes in external-JSON parsers

**Priority**: High
**Risk**: Medium
**Files**: `packages/core/src/npm-registry.ts`, `packages/core/src/skills-registry.ts`, `packages/core/src/self-update.ts`, `packages/core/src/trust/verify-github.ts`, `packages/core/src/trust/verify-npm.ts`, `packages/core/src/plugin/mcp-inject.ts`, `packages/core/src/doctor/checks/mcp-consistency.ts`, `packages/core/src/schemas/` (new schemas)

Six call sites parse external JSON via `JSON.parse + as <Type>`, bypassing the Zod-boundary pattern. Each is an opportunity for malformed external data to enter core logic.

**Current State**:

```typescript
// packages/core/src/trust/verify-github.ts:64
const parsed = JSON.parse(raw) as GhAttestationResult[];

// packages/core/src/trust/verify-npm.ts:170
const intoto = JSON.parse(decoded) as InTotoStatement;

// packages/core/src/plugin/mcp-inject.ts:138
const cfg = JSON.parse(text);
// ... subsequent type-unsafe property access

// packages/core/src/npm-registry.ts:110
return data as Record<string, unknown>;
// ... downstream `(data as any).versions[...]`

// packages/core/src/self-update.ts:46
const cache = (await f.json()) as UpdateCache;

// packages/core/src/skills-registry.ts:67
return (await res.json()) as RegistryApiResponse;
```

**Target State**:

For each, define a Zod schema and use `parseWithResult`. Example:

```typescript
// packages/core/src/schemas/external/npm-registry.ts (new)
import { z } from "zod/v4";
export const NpmPackageMetadataSchema = z.object({
  name: z.string(),
  versions: z.record(z.string(), z.object({
    version: z.string(),
    dist: z.object({
      tarball: z.string(),
      integrity: z.string().optional(),
      shasum: z.string().optional(),
    }),
  })),
  "dist-tags": z.record(z.string(), z.string()).default({}),
}).passthrough();

// packages/core/src/npm-registry.ts (after)
const data: unknown = await response.json();
const parsed = NpmPackageMetadataSchema.safeParse(data);
if (!parsed.success) {
  return err(new NetworkError(
    `Malformed npm registry response: ${z.prettifyError(parsed.error)}`,
  ));
}
return ok(parsed.data);
```

Equivalent schemas to add (one per file under `schemas/external/` or alongside existing schemas):
- `NpmPackageMetadataSchema` — npm registry response
- `RegistryApiResponseSchema` — skills.sh-shape registry response
- `UpdateCacheSchema` — self-update cache file
- `GhAttestationSchema` — `gh attestation verify --format=json` array
- `InTotoStatementSchema` — npm provenance statement
- `McpClientConfigSchema` — `.mcp.json` shape (covers both inject and consistency check paths)

**Implementation Notes**:
- All schemas use `.passthrough()` — these are external responses we don't fully own; we validate the *fields we use* and ignore the rest.
- Each conversion is independent. Land them as 6 small commits or one larger commit; the test suite covers each path.
- Riskiest part: a schema that's stricter than reality and rejects valid responses in production. Mitigation: passthrough + only mark explicitly-required fields, leave optional ones optional.

**Acceptance Criteria**:
- [ ] `bun test` passes (existing tests cover the npm install, update-check, trust verification, MCP inject paths).
- [ ] No `JSON.parse(...) as <Type>` and no `(await ...json()) as <Type>` patterns remain in `packages/core/src/**/*.ts` (verify with grep).
- [ ] Each new schema has at least one positive and one negative unit test.

---

### Step 9: Deduplicate tap-plugin install branches

**Priority**: Medium
**Risk**: Low
**Files**: `packages/core/src/install.ts`

`install.ts:587-619` and `install.ts:624-649` are two branches of the same tap-plugin install — one with the user's `onPluginDetected` callback, one without. The two `installPlugin(...)` calls take *almost* the same options, and both wrap the result with an *identical* return shape.

**Current State**:

```typescript
// install.ts:580-650 (paraphrased)
if (options.onPluginDetected) {
  const decision = await options.onPluginDetected(manifestResult.value);
  if (decision === "cancel") return err(new UserError("Install cancelled."));
  if (decision !== "skills-only") {
    const result = await installPlugin(tapDirPath, manifestResult.value, {
      ...common,
      onWarnings: options.onWarnings ? async (w, n) => options.onWarnings?.(w, "plugin-static", n) : undefined,
      onConfirm: options.onConfirmInstall ? async (m) => options.onConfirmInstall?.("plugin", m) : undefined,
      onCaptureConfirm: options.onPluginCaptureConfirm,
      onCaptureConflict: options.onPluginCaptureConflict,
      ...
    });
    if (!result.ok) return result;
    return ok({
      records: [], warnings: result.value.warnings, semanticWarnings: [],
      updates: [], pluginRecord: result.value.record, captured: result.value.captured,
    });
  }
} else {
  const result = await installPlugin(tapDirPath, manifestResult.value, {
    ...common,
    onCaptureConfirm: options.onPluginCaptureConfirm,
    onCaptureConflict: options.onPluginCaptureConflict,
    ...
  });
  if (!result.ok) return result;
  return ok({
    records: [], warnings: result.value.warnings, semanticWarnings: [],
    updates: [], pluginRecord: result.value.record, captured: result.value.captured,
  });
}
```

**Target State**:

```typescript
// install.ts (refactored)
async function installTapPluginFromMatch(
  tapDirPath: string,
  manifest: PluginManifest,
  match: TapEntry,
  tapName: string,
  options: InstallOptions,
): Promise<Result<InstallResult, ...>> {
  const result = await installPlugin(tapDirPath, manifest, {
    scope: options.scope,
    projectRoot: options.projectRoot,
    also: options.also ?? [],
    skipScan: options.skipScan,
    onWarnings: options.onWarnings
      ? async (w, n) => options.onWarnings?.(w, "plugin-static", n)
      : undefined,
    onConfirm: options.onConfirmInstall
      ? async (m) => options.onConfirmInstall?.("plugin", m)
      : undefined,
    onCaptureConfirm: options.onPluginCaptureConfirm,
    onCaptureConflict: options.onPluginCaptureConflict,
    skipCapture: options.pluginSkipCapture,
    repo: match.skill.repo ?? null,
    ref: null,
    sha: null,
    tap: tapName,
  });
  if (!result.ok) return result;
  return ok({
    records: [],
    warnings: result.value.warnings,
    semanticWarnings: [],
    updates: [],
    pluginRecord: result.value.record,
    captured: result.value.captured,
  });
}

// then the orchestration becomes:
if (match?.tapPlugin) {
  // ... build manifestResult ...
  if (options.onPluginDetected) {
    const decision = await options.onPluginDetected(manifestResult.value);
    if (decision === "cancel") return err(new UserError("Install cancelled."));
    if (decision !== "skills-only") {
      return installTapPluginFromMatch(tapDirPath, manifestResult.value, match, tapPluginRef.tapName, options);
    }
    // skills-only: fall through to skill-install path
  } else {
    return installTapPluginFromMatch(tapDirPath, manifestResult.value, match, tapPluginRef.tapName, options);
  }
}
```

**Implementation Notes**:
- The `onWarnings`/`onConfirm` adapters always pass through; the only meaningful difference between the two branches was that the no-callback path omitted them. Including them in both is correct: they're optional, so they no-op if undefined.
- Cuts ~30 lines and removes one source of drift.
- Riskiest part: the existing code's no-callback branch is *intentional* about not wiring `onWarnings`/`onConfirm` (e.g. to enforce auto-acceptance). Verify by reading the install plugin tests.

**Acceptance Criteria**:
- [ ] `bun test packages/core/src/install*` passes.
- [ ] Tap-plugin install path is exercised by at least one integration test that still passes (`installFromTap` / `installPluginViaTap` tests).

---

### Step 10: Decompose `installSkill` into phases

**Priority**: High
**Risk**: Medium
**Files**: `packages/core/src/install.ts` (split into `install/index.ts`, `install/resolve.ts`, `install/scan.ts`, `install/place.ts`)

`installSkill()` is 510 lines and 8 distinct responsibilities. The function's length is the dominant readability issue in the codebase. Steps 1-9 land helpers that this step uses.

**Current State**:

```
installSkill()                                      // 511-1021
├── orphan purge                                    // 529-547
├── tap pre-resolution                              // 549-558
├── tap-plugin resolution + branch                  // 561-654
├── source resolve + git check                      // 656-668
├── content fetch (clone / npm / local)             // 670-735
├── plugin detect + branch                          // 739-785
├── skill scan                                      // 787-816
├── already-installed conflict                      // 818-879
├── security scan                                   // 881-919
├── trust resolve                                   // 921-940
├── place + symlink                                 // 945-975
├── state save                                      // 979-989
└── manifest sync                                   // 990-1001
```

**Target State**:

Module layout:

```
packages/core/src/install/
├── index.ts                  // re-exports installSkill (existing public API)
├── orchestrate.ts            // installSkill() — top-level coordinator, ~150 lines
├── resolve.ts                // resolveAndFetch() — tap, source, clone, plugin-detect
├── scan.ts                   // scanAndSelect() — discover, scan, select
├── place.ts                  // placeAndRecord() — copy, symlinks, state, manifest
└── tap-plugin.ts             // installTapPluginFromMatch() (from Step 9)
```

```typescript
// install/orchestrate.ts (skeleton)
export async function installSkill(
  source: string,
  options: InstallOptions,
): Promise<Result<InstallResult, UserError | GitError | ScanError | NetworkError>> {
  // 1. Load existing state + purge orphans (uses Step 4 helper)
  const installedResult = await loadSkillState(...);
  if (!installedResult.ok) return installedResult;
  const installed = await purgeOrphansWithCallback(...);

  // 2. Resolve and fetch (delegates tap-plugin short-circuit to Step 9 helper)
  const fetched = await resolveAndFetch(source, options, installed);
  if (!fetched.ok) return fetched;
  if (fetched.value.kind === "tap-plugin-installed") return ok(fetched.value.result);
  const { contentDir, resolved, plugin, cleanup } = fetched.value;

  try {
    // 3. Branch on plugin-detect outcome
    if (plugin) {
      const decision = await options.onPluginDetected?.(plugin) ?? "install";
      if (decision === "cancel") return err(new UserError("Install cancelled."));
      if (decision === "install") {
        return await installPluginInline(contentDir, plugin, options);
      }
      // "skills-only" falls through to skill scan
    }

    // 4. Scan + select (sub-phase; reads contentDir, returns selected SkillEntry[])
    const scanned = await scanAndSelect(contentDir, options, installed);
    if (!scanned.ok) return scanned;

    // 5. Place + record (sub-phase; copies, symlinks, writes state, syncs manifest)
    return await placeAndRecord(scanned.value, resolved, options, installed);
  } finally {
    await cleanup();
  }
}
```

**Implementation Notes**:
- Split is an atomic operation in code-organization terms — moving a function to a new file. Tests don't change; imports update. Land it as one commit.
- Each new file is ~100-200 lines, all from existing code, no logic changes.
- Riskiest part: the cleanup-tmp-dir try/finally that wraps most of the function. Preserving it across the split is the single thing easiest to break — explicitly verify by exercising the failure-path tests (e.g. `install.npm.test.ts` — invalid scope, missing skill, security warning rejected).
- Imports: each new file imports `from "../types"`, `from "../paths"`, etc.; the public re-export from `src/index.ts` continues to point at `installSkill` from `install/orchestrate.ts` (or a barrel `install/index.ts`).
- Rollback path: revert the directory structure; the original `install.ts` was a single file in git.

**Acceptance Criteria**:
- [ ] `bun test packages/core/src/install*` passes — the entire install test suite.
- [ ] `bun run verify:binary:tests` passes (compiled-binary path).
- [ ] No file in `install/` is over 250 lines.
- [ ] Public API surface unchanged: `import { installSkill } from "@skilltap/core"` resolves to the same function.

---

### Step 11: Decompose `update.ts` per adapter

**Priority**: Medium
**Risk**: Medium
**Files**: `packages/core/src/update.ts` (split into `update/index.ts`, `update/git.ts`, `update/npm.ts`, `update/orchestrate.ts`)

`update.ts` is 839 lines with three adapter-specific paths (git multi-skill, git standalone, npm) and a top-level coordinator. Splitting per adapter makes the per-file size manageable and makes adding a new adapter (e.g. http tarball) a localized change.

**Current State**:

```
update.ts                                           // 839 lines
├── updateSkill() coordinator                       // 696-839
├── updateGitSkill() (standalone)                   // 380-419
├── updateGitSkillGroup() (multi-skill, 216 lines)  // 420-592
├── updateNpmSkill()                                // 220-340
├── updateLocalSkill()                              // (small)
├── runUpdateSemanticScan() helper                  // 172-188
└── various fetch/diff/scan helpers
```

**Target State**:

```
packages/core/src/update/
├── index.ts                  // public re-exports
├── orchestrate.ts            // updateSkill() top-level (loads state, dispatches by adapter)
├── git.ts                    // updateGitSkill + updateGitSkillGroup
├── npm.ts                    // updateNpmSkill
├── local.ts                  // updateLocalSkill
└── shared.ts                 // runUpdateSemanticScan, diff helpers, trust resolution
```

The split is strictly mechanical: each adapter function moves to its own file, shared helpers move to `shared.ts`, the top-level coordinator stays as `orchestrate.ts`. The mutable-container antipattern (`installed: { skills: [...] }` passed for in-place mutation) stays as-is — refactoring it is a separate, larger conversation; this step preserves the existing API.

**Implementation Notes**:
- Use Step 4's `purgeOrphansWithCallback` (already landed by this point) to remove the duplicated orphan logic from `updateSkill`.
- `updateGitSkillGroup` (216-line function) does NOT get split in this step — it's complex enough that an internal refactor is a separate exercise.
- Each new file is ~100-250 lines.
- Rollback path: revert the split; `update.ts` was a single file in git.

**Acceptance Criteria**:
- [ ] `bun test packages/core/src/update*` passes.
- [ ] `bun run verify:binary:tests` passes.
- [ ] No file in `update/` is over 350 lines.
- [ ] Public exports (`updateSkill`) unchanged.

---

### Step 12: Add `applySkillStateChange()` for lifecycle commands

**Priority**: Medium
**Risk**: Medium
**Files**: `packages/core/src/state/apply.ts` (new), `install.ts`, `remove.ts`, `move.ts`, `adopt.ts`, `disable.ts`

Five lifecycle command modules implement the same sequence: load state → mutate → save state → (best-effort) update manifest. Centralizing this prevents future drift in how manifest sync is wired into each operation.

**Current State**:

```typescript
// install.ts:979-1001 (paraphrased)
const saveResult = await saveSkillState(installed, fileRoot);
if (!saveResult.ok) return saveResult;
if (options.projectRoot) {
  for (const record of newRecords) {
    await addSkillToManifest(options.projectRoot, record).catch((e) => {
      out?.warn?.(`Failed to update skilltap.toml for '${record.name}': ${e}`);
    });
  }
}

// remove.ts:103-113 (similar shape, removeSkillFromManifest)
// move.ts:168-210 (similar — multi-step)
// adopt.ts:36-74 (similar — has its own helpers, syncAdoptToManifest)
// disable.ts:... (similar)
```

**Target State**:

```typescript
// packages/core/src/state/apply.ts (new)
export interface ApplyChangeOptions<TRecord> {
  projectRoot?: string;
  scope: "global" | "project";
  // returns the new array; null means "abort the change"
  mutate: (current: InstalledSkill[]) => InstalledSkill[] | null;
  manifestSync?: {
    onAdded?: (record: InstalledSkill, projectRoot: string) => Promise<void>;
    onRemoved?: (record: InstalledSkill, projectRoot: string) => Promise<void>;
  };
}

export async function applySkillStateChange(
  opts: ApplyChangeOptions<InstalledSkill>,
): Promise<Result<InstalledSkill[], UserError>> {
  const fileRoot = opts.scope === "project" ? opts.projectRoot : undefined;
  const loadResult = await loadSkillState(fileRoot);
  if (!loadResult.ok) return loadResult;
  const before = loadResult.value;

  const after = opts.mutate(before);
  if (after === null) return ok(before); // mutate aborted

  const saveResult = await saveSkillState(after, fileRoot);
  if (!saveResult.ok) return saveResult;

  if (opts.manifestSync && opts.projectRoot) {
    const beforeNames = new Set(before.map((r) => r.name));
    const afterNames = new Set(after.map((r) => r.name));
    const added = after.filter((r) => !beforeNames.has(r.name));
    const removed = before.filter((r) => !afterNames.has(r.name));
    for (const r of added) {
      await opts.manifestSync.onAdded?.(r, opts.projectRoot).catch(() => {});
    }
    for (const r of removed) {
      await opts.manifestSync.onRemoved?.(r, opts.projectRoot).catch(() => {});
    }
  }

  return ok(after);
}
```

**Implementation Notes**:
- The helper computes the diff (added/removed) automatically rather than asking each lifecycle command to track it.
- `install.ts` is the trickiest consumer because it already has a custom mutation flow (orphan purge, multi-skill loop, conflict resolution). For install, this helper is invoked at the *end* of the existing flow, not as a wrapper around it.
- `remove.ts`, `move.ts`, `adopt.ts`, `disable.ts` benefit most — those are short modules where the helper does most of the work.
- Riskiest part: the manifest-sync diff logic — duplicate names across scopes don't currently confuse anything, but the diff in this helper would. Mitigation: add a unit test.
- This step is genuinely atomic — once added, all lifecycle commands should converge. But it can land incrementally: add the helper first (commit 1), then convert one consumer at a time (commits 2-5).

**Acceptance Criteria**:
- [ ] Unit tests for `applySkillStateChange` cover: normal add, normal remove, mutate-returns-null abort, manifest-sync skipped when projectRoot undefined.
- [ ] At least 3 lifecycle commands converted to use the helper.
- [ ] `bun test` full suite passes.
- [ ] No regressions in lifecycle.test.ts and lifecycle-edge-cases.test.ts.

---

### Step 13: Inline pure-barrel files where they add no value

**Priority**: Low
**Risk**: Low
**Files**: `packages/core/src/policy/index.ts`, `packages/core/src/adapters/types.ts`

Two files are pure re-export shims that add an indirection layer with no benefit:

- `policy/index.ts` (3 lines) — re-exports `composePolicy` and types from `compose.ts`. Move the export onto `compose.ts` and delete the index.
- `adapters/types.ts` (8 lines) — defines a single `SourceAdapter` type used by 4 sibling files. Inline into `adapters/index.ts` (which already re-exports it).

**Current State**:

```typescript
// packages/core/src/policy/index.ts
export { composePolicy, composePolicyForSource } from "./compose";
export type { EffectivePolicy, CliFlags } from "./types";
```

**Target State**:

```typescript
// packages/core/src/policy.ts (was policy/compose.ts; index.ts deleted)
// — exports moved here directly; consumers import from "../policy" unchanged
```

**Implementation Notes**:
- Delete `policy/index.ts` and `policy/types.ts`; rename `policy/compose.ts` → `policy.ts`; merge `types.ts` content into it.
- For consumers, the path is `from "../policy"` — which resolves to `policy.ts` after the rename. No import-path changes needed.
- For `adapters/types.ts`: the 8-line type moves into `adapters/index.ts`. All files that import `from "./types"` (within `adapters/`) update to import from `./index` or have the type colocated.
- Riskiest part: a circular import that the index file currently breaks. Mitigation: build before commit; circulars surface immediately.
- This step is genuinely low-value — defer if other steps demand more attention.

**Acceptance Criteria**:
- [ ] `bun run build` succeeds.
- [ ] `bun test` passes.
- [ ] No file `policy/index.ts` or `adapters/types.ts` exists.

---

## Implementation Order

The dependency graph for these steps is shallow. Most can run in parallel; only steps 4, 9, 10, 11 have real dependencies:

```
Step 1: plugin-format dirs        ──┐
Step 2: DEFAULT_AGENT_ID          ──┤
Step 3: currentSkillDir helper    ──┼── independent, land in any order
Step 5: validateScopeArg          ──┤
Step 6: exitOnError               ──┤
Step 7: stderr→callback in taps   ──┤
Step 8: Zod boundary holes        ──┘

Step 4: orphan-purge helper       ── unblocks ──> Step 10, Step 11
Step 9: tap-plugin dedup          ── unblocks ──> Step 10
Step 10: installSkill split       ── depends on Step 4, Step 9
Step 11: update.ts split          ── depends on Step 4
Step 12: applySkillStateChange    ── independent (best after Step 10)
Step 13: barrel inlining          ── independent
```

**Suggested chronological order:**

1. Step 1 — plugin-format dirs (15 min, mechanical)
2. Step 2 — DEFAULT_AGENT_ID (15 min, mechanical)
3. Step 3 — currentSkillDir (15 min, mechanical)
4. Step 4 — orphan-purge helper (30 min, includes 2 callsites)
5. Step 5 — validateScopeArg (15 min)
6. Step 6 — exitOnError (1-2 hours, broad reach)
7. Step 7 — stderr callback (30 min)
8. Step 8 — Zod boundary (1-2 hours, 6 schemas)
9. Step 9 — tap-plugin dedup (30 min)
10. Step 10 — installSkill split (2-4 hours)
11. Step 11 — update.ts split (1-2 hours)
12. Step 12 — applySkillStateChange (2-3 hours including consumer migrations)
13. Step 13 — barrel inlining (15 min)

Total: roughly 10-15 focused hours. Each step is a single PR / single commit.

## Safety Net

Every step's acceptance criteria are gated by:

- `bun test` — full source-mode suite (~60 sec)
- `bun run build` — compile must succeed
- `bun run verify:binary` — binary smokes
- `bun run verify:binary:tests` — full CLI suite against compiled binary (~80 sec)

For steps 10–12 (the three with non-trivial reach), running `bun run verify:binary:tests` before the commit is mandatory — the dynamic-import boundary in the install/update split could regress only in the compile path.

## What This Plan Does NOT Do

- **Does not change public API.** All exports from `@skilltap/core` keep their names and signatures. The bump script never needs to advance major.
- **Does not change Zod schemas for on-disk formats.** `state.json`, `skilltap.toml`, `.skilltap/<name>.toml`, `tap.json`, `marketplace.json`, `plugin.json`, SKILL.md frontmatter — all unchanged.
- **Does not address the 216-line `updateGitSkillGroup` function.** That's a deeper internal refactor (mutable-container, nested loops); a separate plan is appropriate.
- **Does not address callback explosion.** Step 12 normalizes one specific lifecycle pattern; the broader callback-driven-options pattern is documented and working — it doesn't need refactoring.
- **Does not touch the TUI (`packages/cli/src/tui/`).** The TUI has its own established structure (Ink + reducers) and the patterns there are intentional.
- **Does not regenerate the patterns skill.** New helpers introduced (currentSkillDir, applySkillStateChange) will need pattern docs added later — that's a follow-up to keep the skill in sync, not part of this plan.
