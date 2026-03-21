# Design: Supporting marketplace.json as a Tap Format

## Overview

Enable skilltap to recognize Claude Code marketplace repos (`.claude-plugin/marketplace.json`) as taps, so users can `skilltap tap add owner/repo` with any Claude Code plugin marketplace and get its skills listed in `skilltap search`.

This is a read-only integration: skilltap reads marketplace.json and adapts it to the existing `Tap` type. No changes to `tap.json`, the config schema, or the install flow. The scanner update for `plugins/*/skills/*/SKILL.md` (Step 2.6) is already complete.

## Implementation Units

### Unit 1: Marketplace Schema

**File**: `packages/core/src/schemas/marketplace.ts`

```typescript
import { z } from "zod/v4";

// Plugin source can be a relative path string or an object with type-specific fields
const MarketplacePluginSourceSchema = z.union([
  z.string(), // relative path like "./plugins/my-plugin"
  z.object({
    source: z.literal("github"),
    repo: z.string(),
    ref: z.string().optional(),
  }),
  z.object({
    source: z.literal("url"),
    url: z.string(),
    ref: z.string().optional(),
  }),
  z.object({
    source: z.literal("git-subdir"),
    url: z.string(),
    path: z.string(),
    ref: z.string().optional(),
  }),
  z.object({
    source: z.literal("npm"),
    package: z.string(),
    version: z.string().optional(),
  }),
]);

const MarketplacePluginSchema = z.object({
  name: z.string(),
  source: MarketplacePluginSourceSchema,
  description: z.string().optional(),
  version: z.string().optional(),
  category: z.string().optional(),
  tags: z.array(z.string()).optional(),
});

const MarketplaceOwnerSchema = z.object({
  name: z.string(),
  email: z.string().optional(),
});

const MarketplaceMetadataSchema = z.object({
  description: z.string().optional(),
  version: z.string().optional(),
  pluginRoot: z.string().optional(),
}).optional();

export const MarketplaceSchema = z.object({
  name: z.string(),
  owner: MarketplaceOwnerSchema,
  metadata: MarketplaceMetadataSchema,
  plugins: z.array(MarketplacePluginSchema),
});

export type MarketplacePluginSource = z.infer<typeof MarketplacePluginSourceSchema>;
export type MarketplacePlugin = z.infer<typeof MarketplacePluginSchema>;
export type Marketplace = z.infer<typeof MarketplaceSchema>;
```

**Implementation Notes**:
- The schema validates the Claude Code marketplace.json format permissively — we only need `name`, `owner.name`, and `plugins[]` with `name` + `source`.
- Extra fields (strict, commands, agents, hooks, mcpServers, lspServers) are silently dropped — Zod strips unknown keys by default.
- The union for `source` covers all 5 documented source types.

**Acceptance Criteria**:
- [ ] Valid marketplace.json parses successfully
- [ ] Invalid marketplace.json (missing name, missing owner) returns parse error
- [ ] Extra fields (strict, hooks, mcpServers) are silently dropped
- [ ] All 5 source types parse correctly (relative path, github, url, git-subdir, npm)

---

### Unit 2: Marketplace-to-Tap Adapter

**File**: `packages/core/src/marketplace.ts`

```typescript
import type { Tap, TapSkill } from "./schemas/tap";
import type { Marketplace, MarketplacePlugin, MarketplacePluginSource } from "./schemas/marketplace";

/**
 * Convert a marketplace plugin source to a TapSkill.repo string
 * that the source adapter chain can resolve.
 *
 * @param source - The plugin source from marketplace.json
 * @param tapUrl - The git URL of the marketplace repo itself (for relative paths)
 */
export function marketplaceSourceToRepo(
  source: MarketplacePluginSource,
  tapUrl: string,
): string | null;

/**
 * Adapt a parsed marketplace.json into a skilltap Tap object.
 *
 * @param marketplace - Parsed marketplace data
 * @param tapUrl - The git URL of the marketplace repo (used to resolve relative paths)
 */
export function adaptMarketplaceToTap(
  marketplace: Marketplace,
  tapUrl: string,
): Tap;
```

**Implementation Notes**:

`marketplaceSourceToRepo()` mapping:
- **String (relative path)**: Return `tapUrl` — the marketplace's own URL. When installed via tap name, skilltap clones the marketplace repo and the scanner's Step 2.6 (`plugins/*/skills/*/SKILL.md`) discovers the skills within it. The `metadata.pluginRoot` prefix is NOT prepended here because the scanner already traverses `plugins/` recursively.
- **`github`**: Return `"owner/repo"` (the `repo` field) — GitHub shorthand the adapter chain handles.
- **`url`**: Return the `url` field directly — a full git URL.
- **`git-subdir`**: Return the `url` field — skilltap will clone the full repo, and the scanner will find skills inside it. The specific subdirectory path is not preserved (noted as a limitation).
- **`npm`**: Return `"npm:<package>"` — matches the npm adapter prefix.
- **Unknown/unsupported**: Return `null` — skip silently.

`adaptMarketplaceToTap()`:
- Map `marketplace.name` → `tap.name`
- Map `marketplace.metadata?.description` → `tap.description`
- For each plugin in `marketplace.plugins`:
  - Call `marketplaceSourceToRepo()` to get the repo string
  - If `null`, skip the plugin
  - Create a `TapSkill` with:
    - `name`: plugin.name
    - `description`: plugin.description ?? `"Plugin from ${marketplace.name} marketplace"`
    - `repo`: the resolved repo string
    - `tags`: plugin.tags ?? plugin.category ? [plugin.category] : []
  - Deduplicate by name (first occurrence wins) when multiple plugins map to the same marketplace URL

**Acceptance Criteria**:
- [ ] Relative path source maps to the tap's own git URL
- [ ] GitHub source `{ source: "github", repo: "owner/repo" }` maps to `"owner/repo"`
- [ ] URL source maps to the URL string directly
- [ ] git-subdir source maps to the URL (path not preserved)
- [ ] npm source `{ source: "npm", package: "@org/pkg" }` maps to `"npm:@org/pkg"`
- [ ] Plugins with `null` repo (unknown source type) are skipped
- [ ] Description falls back to a default string when missing
- [ ] Tags include category if present

---

### Unit 3: Modify `loadTapJson()` to Fall Back to marketplace.json

**File**: `packages/core/src/taps.ts`

Modify the existing `loadTapJson()` function:

```typescript
async function loadTapJson(
  dir: string,
  name?: string,
  tapUrl?: string,      // NEW: the git URL for this tap (for marketplace relative path resolution)
): Promise<Result<Tap, UserError>> {
  const label = name ? `tap '${name}'` : dir;

  // 1. Try tap.json (canonical format)
  const tapFile = Bun.file(join(dir, "tap.json"));
  if (await tapFile.exists()) {
    let raw: unknown;
    try {
      raw = await tapFile.json();
    } catch (e) {
      return err(new UserError(`Invalid JSON in tap.json in ${label}: ${e}`));
    }
    return parseWithResult(TapSchema, raw, `tap.json in ${label}`);
  }

  // 2. Fall back to .claude-plugin/marketplace.json
  const marketplaceFile = Bun.file(join(dir, ".claude-plugin", "marketplace.json"));
  if (await marketplaceFile.exists()) {
    let raw: unknown;
    try {
      raw = await marketplaceFile.json();
    } catch (e) {
      return err(new UserError(`Invalid JSON in marketplace.json in ${label}: ${e}`));
    }
    const parsed = parseWithResult(MarketplaceSchema, raw, `marketplace.json in ${label}`);
    if (!parsed.ok) return parsed;
    return ok(adaptMarketplaceToTap(parsed.value, tapUrl ?? ""));
  }

  return err(new UserError(`No tap.json or marketplace.json found in ${label}`));
}
```

**Implementation Notes**:
- The `tapUrl` parameter is added as an optional third argument — existing callers that don't need marketplace support can omit it.
- Call sites that have access to the tap's URL (from config or from `addTap()`) pass it through.
- The error message is updated: `"No tap.json or marketplace.json found"` instead of `"tap.json not found"`.
- `loadTapJson()` is private (not exported) — changes are internal.

**Modified call sites:**

1. `addTap()` (line ~169): Pass `url` as the tapUrl parameter.
2. `updateTap()` (lines ~259, ~274, ~317): Pass `BUILTIN_TAP.url` or `tap.url` as tapUrl.
3. `loadTaps()` (lines ~334, ~366): Pass `BUILTIN_TAP.url` or `tap.url` as tapUrl.
4. `getTapInfo()` (lines ~444, ~472): Pass the appropriate URL.
5. `isBuiltinTapCloned()` (line ~99): This checks for `tap.json` existence directly — update to also check for `.claude-plugin/marketplace.json`.

**Acceptance Criteria**:
- [ ] Git tap with `tap.json` works exactly as before
- [ ] Git tap with `.claude-plugin/marketplace.json` (no tap.json) loads and adapts to Tap
- [ ] Git tap with both files uses `tap.json` (takes precedence)
- [ ] Git tap with neither file returns error mentioning both formats
- [ ] HTTP taps are unaffected (they don't use `loadTapJson`)
- [ ] `isBuiltinTapCloned()` still works correctly

---

### Unit 4: Re-export from schemas/index.ts

**File**: `packages/core/src/schemas/index.ts`

Add the new schema and types to the barrel export:

```typescript
export { MarketplaceSchema } from "./marketplace";
export type { Marketplace, MarketplacePlugin, MarketplacePluginSource } from "./marketplace";
```

**Implementation Notes**:
- Follow existing pattern — schemas/index.ts re-exports all schemas.

**Acceptance Criteria**:
- [ ] `MarketplaceSchema` is importable from `@skilltap/core`

---

## Implementation Order

1. **Unit 1: Marketplace Schema** — no dependencies, defines the types other units need
2. **Unit 4: Re-export** — wire up the barrel export
3. **Unit 2: Marketplace-to-Tap Adapter** — depends on schema types
4. **Unit 3: Modify loadTapJson()** — depends on the adapter function

## Testing

### Schema Tests: `packages/core/src/schemas/marketplace.test.ts`

```typescript
describe("MarketplaceSchema", () => {
  test("parses valid marketplace.json with relative path source");
  test("parses valid marketplace.json with github source");
  test("parses valid marketplace.json with npm source");
  test("parses valid marketplace.json with url source");
  test("parses valid marketplace.json with git-subdir source");
  test("rejects missing name");
  test("rejects missing owner");
  test("rejects missing plugins array");
  test("strips unknown fields (hooks, mcpServers, etc.)");
  test("handles empty plugins array");
  test("handles optional metadata");
});
```

### Adapter Tests: `packages/core/src/marketplace.test.ts`

```typescript
describe("marketplaceSourceToRepo", () => {
  test("relative path string returns tapUrl");
  test("github source returns owner/repo");
  test("url source returns the URL");
  test("git-subdir source returns the URL");
  test("npm source returns npm:package");
});

describe("adaptMarketplaceToTap", () => {
  test("maps marketplace name to tap name");
  test("maps metadata.description to tap description");
  test("maps plugins to TapSkills with correct repo");
  test("skips plugins with null repo");
  test("defaults description when missing");
  test("includes category in tags");
  test("deduplicates plugins that share the same marketplace URL");
});
```

### Integration Tests: `packages/core/src/taps.test.ts` (additions)

```typescript
describe("loadTapJson — marketplace.json fallback", () => {
  test("loads marketplace.json when tap.json is absent");
  test("prefers tap.json when both exist");
  test("returns error when neither exists, message mentions both formats");
});

describe("addTap — marketplace repo", () => {
  test("adds a tap from a marketplace repo (no tap.json, has .claude-plugin/marketplace.json)");
  test("skill count reflects marketplace plugins");
});

describe("loadTaps — marketplace taps", () => {
  test("marketplace tap skills appear in loadTaps() results");
  test("marketplace tap skills are searchable via searchTaps()");
});
```

### CLI Subprocess Test: `packages/cli/src/commands/tap.test.ts` (addition)

```typescript
test("tap add succeeds with marketplace repo", async () => {
  // Create a fixture repo with .claude-plugin/marketplace.json
  // Run: skilltap tap add test-marketplace <path>
  // Assert: exit 0, output includes skill count
});
```

## Verification Checklist

```bash
# Unit tests
bun test packages/core/src/schemas/marketplace.test.ts
bun test packages/core/src/marketplace.test.ts

# Integration tests (taps)
bun test packages/core/src/taps.test.ts

# CLI tests
bun test packages/cli/src/commands/tap.test.ts

# Full suite
bun test

# Manual: add the anthropics/skills marketplace
skilltap tap add anthropics/skills
skilltap search pdf
```
