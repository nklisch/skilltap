# Design: Phase 31b — HTTP Registry Adapter Removal

## Overview

Remove the HTTP registry adapter from skilltap. After this phase:

- `core/src/registry/` directory is gone (~400 lines of code + 3 test files).
- `taps.ts` no longer has any HTTP code branches.
- `UpdateTapResult.http` field is removed (dead-loaded; zero production consumers).
- `core/src/index.ts` no longer re-exports Registry* schemas, `detectTapType`, `fetchSkillList`, `fetchSkillDetail`.
- CLI `tap add --type http` errors with a migration hint; `tap list` shows all taps as "git".
- Existing user configs with `[[taps]] type = "http"` parse cleanly (schema kept compatible) but the entries are silently filtered with a one-time stderr warning.

What's kept untouched: MCP server `type: "http"` (different feature — HTTP transport for MCP, not HTTP tap registries). The `auth_token` and `auth_env` config fields stay parseable in the v1.0 schema (inert — never read post-removal).

## Autonomous Decisions

The autopilot mandate forbids asking the user. These ambiguities resolved:

### D1. Schema kept loose for backward compatibility

**Question**: Narrow `type: z.enum(["git", "http"])` to `z.literal("git")` in `schemas/config.ts`?

**Decision**: Keep the enum unchanged. Existing user configs with HTTP taps must continue to parse. Narrowing would crash `loadConfig()` for any v1.0 user who hasn't run `skilltap migrate`, breaking every CLI command on first invocation. v2.0 code defensively filters HTTP entries at the call site instead.

### D2. HTTP-tap-in-config behavior: filter + warn (not error)

**Question**: When `loadTaps()` / `updateTap()` / etc. encounter `tap.type === "http"`, what happens?

**Decision**: Filter out (skip the entry) and emit a one-time stderr warning the first time per process. Hard-erroring would block users from interacting with their git taps if they happen to also have an HTTP tap in their config. The migration command and the soft v1 startup hint already direct users toward fixing this.

Warning message: `↑  HTTP tap '<name>' ignored — HTTP support removed in v2.0. Use a git tap or run 'skilltap migrate'.`

### D3. `UpdateTapResult.http` field dropped (breaking)

**Question**: Drop the `http: string[]` field from `UpdateTapResult`, or keep as always-empty for backward compat?

**Decision**: Drop. Agent #2 confirmed zero production consumers — only the registry/__tests__/integration.test.ts asserts on it (and that file is being deleted). External library consumers of `@skilltap/core` could theoretically depend on it, but v2.0 is a major release, so a breaking change to a dead field is acceptable.

### D4. `auth_token` and `auth_env` config fields kept parseable

**Question**: Delete `auth_token` and `auth_env` from the tap schema?

**Decision**: Keep both as `z.string().optional()` in the v1.0 tap schema. They're inert post-removal but allow existing configs to parse. v2 cleanup of these fields lands when the v1 schema is fully retired (Phase 31c+).

### D5. `--type` flag dropped from `tap add`

**Question**: Keep `--type` flag as no-op for compat, or remove entirely?

**Decision**: Remove the flag definition. If a user passes `--type http`, citty errors with "unknown flag" — a clearer signal than silent acceptance. If a user passes `--type git`, citty errors too — they can drop the flag and re-run. Prefer one breaking change over two semi-supported behaviors.

## Implementation Units

### Unit 1 — Filter helper + warning utility in `taps.ts`

**File**: `packages/core/src/taps.ts` (additions near the top, before `addTap`)

```typescript
type ConfigTap = Config["taps"][number];

let httpWarningEmittedFor = new Set<string>();

function isGitTap(tap: ConfigTap): boolean {
  return tap.type !== "http";
}

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

**Implementation Notes**:
- Module-level `Set` so the warning fires once per tap name per process. Acceptable for short-lived CLI runs.
- Uses raw ANSI codes (matching the project's `cli/src/index.ts` startup-hint pattern) to avoid importing UI helpers from the cli package into core.
- `isGitTap` is a small predicate used by the few sites that need a one-shot test.

**Acceptance Criteria**:
- [ ] `filterAndWarnHttpTaps([{name:"x",url:"...",type:"git"}])` returns the input unchanged with no stderr writes.
- [ ] `filterAndWarnHttpTaps([{name:"x",url:"...",type:"http"}])` returns `[]` and writes a single warning to stderr.
- [ ] Calling `filterAndWarnHttpTaps` twice with the same HTTP tap emits the warning only once.
- [ ] Mixed git+http input filters out the http and keeps the git, warning once.

---

### Unit 2 — Strip HTTP branches from `taps.ts`

**File**: `packages/core/src/taps.ts`

Edits (current line numbers from the working file):

**a. Imports (line 7–8)**: Replace
```typescript
import type { RegistrySource } from "./registry";
import { detectTapType, fetchSkillList } from "./registry";
```
with
```typescript
// (delete both lines — registry module is removed)
```

**b. `registrySourceToRepo()` helper (if present in the file)**: Delete.

**c. `UpdateTapResult` (lines 33–38)**: Remove the `http` field.

```typescript
export type UpdateTapResult = {
  /** Git taps: skill counts after pull. */
  updated: Record<string, number>;
};
```

**d. `addTap()` signature (line ~148–152)**: Drop `typeOverride` parameter.

```typescript
export async function addTap(
  name: string,
  url: string,
): Promise<Result<{ skillCount: number; type: "git" }, UserError | GitError>>
```

**e. `addTap()` body (lines 175–187)**: Remove the `tapType` resolution and HTTP branch.

Replace:
```typescript
const tapType = typeOverride ?? (await detectTapType(url));

if (tapType === "http") { ... }
```

with: just continue into the git path (which begins at the existing `// Git tap` comment around line 189). The `config.taps.push` call writes `type: "git"` explicitly — keep that.

**f. `removeTap()` (lines 251–253)**: Remove the conditional skip; always rm the local dir.

Replace:
```typescript
if (tap.type !== "http") {
  await rm(tapDir(name), { recursive: true, force: true });
}
```

with:
```typescript
await rm(tapDir(name), { recursive: true, force: true });
```

**g. `updateTap()` body**: Two edits.

Around line 264–265: remove `const http: string[] = [];` and remove `http` from every `return ok({ updated, http })` callsite (lines 290, 350).

Around line 308: filter targets through `filterAndWarnHttpTaps`:
```typescript
const allTaps = name
  ? config.taps.filter((t) => t.name === name)
  : config.taps;
const targets = filterAndWarnHttpTaps(allTaps);
```

Then delete the inner `if (tap.type === "http")` block (lines 322–325).

**h. `loadTaps()` (lines 447–493)**: Replace the dual-branch loop with the git-only branch and a filter pass.

```typescript
for (const tap of filterAndWarnHttpTaps(config.taps)) {
  const dir = tapDir(tap.name);
  const tapResult = await loadTapJson(dir, tap.name, tap.url);
  if (!tapResult.ok) continue;
  for (const skill of tapResult.value.skills) {
    entries.push({ tapName: tap.name, skill });
  }
  for (const plugin of tapResult.value.plugins ?? []) {
    entries.push({
      tapName: tap.name,
      skill: {
        name: plugin.name,
        description: plugin.description,
        repo: tap.url,
        tags: plugin.tags,
        plugin: true,
      },
      tapPlugin: plugin,
    });
  }
}
```

**i. `getTapInfo()` (lines 582–592)**: Remove the HTTP branch entirely. After the `tap` lookup, if `tap.type === "http"` return a UserError pointing at the migration hint.

```typescript
if (tap.type === "http") {
  return err(
    new UserError(
      `Tap '${name}' is an HTTP tap — HTTP support was removed in v2.0.`,
      "Convert to a git tap or remove with 'skilltap tap remove'.",
    ),
  );
}
```

This path is preferred over silent filter for `getTapInfo` because the user explicitly asked about that tap by name; a silent return would be confusing.

**j. `TapInfo` type (around line 541)**: Remove `"http"` from the type field's union.

```typescript
export type TapInfo = {
  name: string;
  type: "git" | "builtin";
  url: string;
  // ...
};
```

**Acceptance Criteria**:
- [ ] `taps.ts` has no remaining references to `fetchSkillList`, `detectTapType`, `RegistrySource`, `registrySourceToRepo`.
- [ ] `loadTaps()` returns only git-tap entries; HTTP entries in config are silently filtered with one warning.
- [ ] `addTap("name", "https://example.com/api")` no longer attempts auto-detection; treats input as git URL.
- [ ] `updateTap()` returns `{ updated }` only — no `http` field.
- [ ] `getTapInfo("http-tap-name")` returns a UserError with hint.
- [ ] `removeTap()` cleans up the local dir for any tap (no skip).

---

### Unit 3 — Delete `core/src/registry/`

**Files to delete**:
- `packages/core/src/registry/client.ts`
- `packages/core/src/registry/types.ts`
- `packages/core/src/registry/index.ts`
- `packages/core/src/registry/__tests__/client.test.ts`
- `packages/core/src/registry/__tests__/integration.test.ts`
- `packages/core/src/registry/__tests__/schemas.test.ts`

Use `rm -rf packages/core/src/registry`.

**Acceptance Criteria**:
- [ ] Directory `packages/core/src/registry` does not exist.
- [ ] `bun test packages/core/src/` does not pick up the deleted test files (no errors about missing files).

---

### Unit 4 — Strip registry exports from `core/src/index.ts`

**File**: `packages/core/src/index.ts`

Delete lines 29–41:

```typescript
// Export registry module — exclude names that conflict with ./schemas and ./skills-registry
export type {
  RegistryDetailResponse,
  RegistryListResponse,
} from "./registry/types";
export {
  RegistryDetailResponseSchema,
  RegistryListResponseSchema,
  RegistrySkillSchema,
  RegistryTrustSchema,
} from "./registry/types";
export type { RegistryAuth, FetchSkillListResult } from "./registry/client";
export { detectTapType, fetchSkillList, fetchSkillDetail } from "./registry/client";
```

**Acceptance Criteria**:
- [ ] `import { fetchSkillList } from "@skilltap/core"` fails to resolve at TypeScript level.
- [ ] `import { detectTapType } from "@skilltap/core"` fails to resolve.
- [ ] All other named exports from core/src/index.ts continue to work (verified by `bun test`).

---

### Unit 5 — Update `cli/src/commands/tap/add.ts`

**File**: `packages/cli/src/commands/tap/add.ts`

Edits:

**a. `args`**: Drop the `type` arg definition entirely (lines 23–26).

**b. URL description (line 20)**: Change `"URL of the tap (git repo or HTTP registry)"` → `"Git repository URL"`.

**c. Body**: Remove the `typeOverride` extraction and validation (lines 32–35). Drop the unused `agentMode` HTTP path.

**d. `addTap()` call**: Drop the third arg (was `typeOverride`):
```typescript
const result = await addTap(tapName, tapUrl);
```

**e. Output labels (lines 66, 85)**: Remove the ternary; always say "git":
```typescript
const typeLabel = "git";
```
…or just drop the variable and inline the literal in both places.

**Acceptance Criteria**:
- [ ] `skilltap tap add my-tap https://example.com/repo` succeeds and adds a git tap.
- [ ] `skilltap tap add my-tap https://example.com/repo --type http` errors with "Unknown argument --type".
- [ ] Help text (`skilltap tap add --help`) does not mention HTTP, registry, or `--type`.
- [ ] Existing `cli/src/commands/tap.test.ts` still passes (no HTTP-specific cases there).

---

### Unit 6 — Update `cli/src/commands/tap/list.ts`

**File**: `packages/cli/src/commands/tap/list.ts`

Edits:

**a. JSON output (lines 70–78)**: `tap.type` is now always `"git"`, so no change needed in JSON shape — values just collapse.

**b. Type column (line 97)**: Replace
```typescript
tap.type === "http" ? ansi.dim("http") : ansi.dim("git"),
```
with
```typescript
ansi.dim("git"),
```

**Acceptance Criteria**:
- [ ] `skilltap tap list` displays "git" for all configured taps regardless of historical type.
- [ ] `skilltap tap list --json` returns objects with `type: "git"` for all taps.
- [ ] No code path emits "http" as a type label.

---

### Unit 7 — Update `core/src/doctor/checks/taps.ts`

**File**: `packages/core/src/doctor/checks/taps.ts`

Find the HTTP-fast-path block (per agent #1, around lines 31–35):

```typescript
if (tap.type === "http") {
  validCount++;
  info.push(`${tap.name} (http): ok`);
  continue;
}
```

Delete it. Doctor now treats every configured tap as git and runs git-clone + tap.json validation. HTTP-typed taps in config are filtered upstream by `loadTaps`, so doctor will see them as missing dirs (which is fine — they'll appear under the doctor's existing "missing tap dir" warning).

**Acceptance Criteria**:
- [ ] `core/src/doctor/checks/taps.ts` has no `tap.type === "http"` references.
- [ ] If a user has an HTTP tap in config, `skilltap doctor` reports it (likely as missing local clone dir or invalid tap.json).

---

### Unit 8 — Test maintenance

**a. Delete entirely**: The three `core/src/registry/__tests__/*.test.ts` files (handled by Unit 3's `rm -rf`).

**b. `core/src/taps.test.ts`** (1191 lines): No HTTP tests per agent #2's report. **No change needed.** Run after edits to confirm.

**c. `cli/src/commands/tap.test.ts`**: No HTTP-specific tests. **No change needed.** Confirm post-edit.

**d. New test file**: `packages/core/src/taps.http-removal.test.ts`

Add a small test confirming the filter behavior:

```typescript
import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { writeFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { loadTaps } from "./taps";

describe("HTTP tap filtering (Phase 31b)", () => {
  let env: TestEnv;
  beforeEach(async () => {
    env = await createTestEnv();
    await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  });
  afterEach(async () => {
    await env.cleanup();
  });

  test("loadTaps silently skips HTTP entries in config", async () => {
    // Synthesize a config with an HTTP tap and a git tap (without local clone)
    const cfgPath = join(env.configDir, "skilltap", "config.toml");
    await writeFile(
      cfgPath,
      `
builtin_tap = false

[[taps]]
name = "http-tap"
url = "https://example.com/api"
type = "http"

[[taps]]
name = "git-tap"
url = "https://example.com/repo.git"
type = "git"
`,
    );

    const result = await loadTaps();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // Both taps lack local clones, so entries is empty either way.
    // The important check: HTTP didn't error, and code path completed.
    expect(Array.isArray(result.value)).toBe(true);
  });
});
```

This test verifies that HTTP entries in config don't crash `loadTaps`. The deeper "warning emitted once" assertion is hard to test cleanly without stubbing stderr; skip for now.

**Acceptance Criteria**:
- [ ] All 204 existing v2.0 tests still pass.
- [ ] All existing v1.0 tap tests (`packages/core/src/taps.test.ts`, `cli/src/commands/tap.test.ts`) still pass.
- [ ] New `taps.http-removal.test.ts` passes.

---

### Unit 9 — Verify no orphan imports of the deleted module

**Verification step (not a code change)**: After all edits, run:

```bash
grep -rn "from.*registry/" packages/core/src/ packages/cli/src/ 2>/dev/null
grep -rn "fetchSkillList\|detectTapType\|fetchSkillDetail" packages/core/src/ packages/cli/src/ 2>/dev/null
grep -rn "RegistrySkillSchema\|RegistryListResponseSchema\|RegistryDetailResponseSchema\|RegistryTrustSchema\|RegistryAuth\|FetchSkillListResult" packages/core/src/ packages/cli/src/ 2>/dev/null
```

All three should return zero results (or only matches inside the design doc / changelog).

---

## Implementation Order

1. **Unit 1** — add filter helper to taps.ts. Compiles standalone.
2. **Unit 2** — strip HTTP branches from taps.ts. Now references the helper from Unit 1.
3. **Unit 4** — strip registry exports from `core/src/index.ts`. After taps.ts no longer imports them.
4. **Unit 3** — delete `core/src/registry/` directory. After all references are gone.
5. **Unit 5** — update `cli/src/commands/tap/add.ts`. After `addTap()` signature in Unit 2.
6. **Unit 6** — update `cli/src/commands/tap/list.ts`. Independent of CLI add changes.
7. **Unit 7** — update `core/src/doctor/checks/taps.ts`. Independent.
8. **Unit 8** — write `taps.http-removal.test.ts`. Independent.
9. **Unit 9** — orphan-import grep verification. Last.

Order respects dependencies: types/helpers before consumers, dead code deletions after their last reference is removed.

## Testing

### Verification Checklist

After implementation, run:

```bash
# 1. Compile & test the v2.0 baseline (must remain green)
bun test packages/core/src/manifest/ packages/core/src/state/ packages/core/src/migrate/ packages/core/src/sync/ packages/core/src/plugin-v2/ packages/core/src/plugin/detect.test.ts packages/core/src/schemas/config-v2.test.ts packages/core/src/policy-v2/ packages/core/src/status/

# 2. Tap-related tests (existing)
bun test packages/core/src/taps.test.ts
bun test packages/cli/src/commands/tap.test.ts

# 3. New filter test
bun test packages/core/src/taps.http-removal.test.ts

# 4. Orphan import grep (Unit 9)
grep -rn "registry/" packages/core/src/ packages/cli/src/
grep -rn "fetchSkillList\|detectTapType" packages/core/src/ packages/cli/src/

# 5. Sanity: bare CLI run doesn't error from missing registry imports
SKILLTAP_NO_STARTUP=1 bun packages/cli/src/index.ts --version
```

Expected outcomes:
- Baseline: 204+ tests pass.
- Tap tests: pass unchanged.
- New filter test: pass.
- Greps: zero matches (exception: design doc itself, this PROGRESS.md).
- CLI version prints cleanly.

## Out of Scope

- **Removing `auth_token`/`auth_env` config fields** — kept parseable; cleanup with v1 schema retirement (Phase 31c+).
- **Hard-erroring on HTTP taps** — chose filter+warn for backward compat.
- **Schema narrow to `z.literal("git")`** — would crash existing user configs.
- **CLI `--type` flag deprecation period** — chose hard removal; v2.0 is the breaking-change boundary.
- **Migration command updates** — already handled in Phase 27 (rejects HTTP taps with friendly error).
