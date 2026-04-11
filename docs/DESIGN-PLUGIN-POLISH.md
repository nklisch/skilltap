# Design: Phase 25 — Plugin Polish

## Overview

Final polish phase: integrate plugin awareness into the existing tap/find/status ecosystem, add a `plugin` flag to `TapSkillSchema`, update the status command, and write an e2e lifecycle test. Documentation updates are handled by a separate skill invocation after implementation.

## Key Design Decisions

### Add `plugin` flag to TapSkillSchema, not a separate TapPluginSchema

A boolean `plugin` field on `TapSkillSchema` (defaulting to `false`) is the simplest way to indicate that a tap entry is a plugin. The `adaptMarketplaceToTap` function can set this based on whether the marketplace entry has MCP/agent components. This avoids schema restructuring.

### Status command: add plugin count

The `skilltap status --json` command already reports `taps` count. Adding a `plugins` count is a one-line addition to both the plain text and JSON output.

### E2e test: full plugin lifecycle

A single lifecycle test exercises: install plugin → list plugins → toggle component → info → remove. This tests the integration between all Phase 20-24 work.

### Doc updates deferred to skill invocation

SPEC.md, ARCH.md, UX.md updates are best handled by the update-documentation skill after all code changes are committed. The design doesn't specify those manually.

---

## Implementation Units

### Unit 1: Add `plugin` flag to TapSkillSchema

**File**: `packages/core/src/schemas/tap.ts`

Add `plugin: z.boolean().default(false)` to `TapSkillSchema`:

```typescript
export const TapSkillSchema = z.object({
  name: z.string(),
  description: z.string(),
  repo: z.string(),
  tags: z.array(z.string()).default([]),
  trust: TapTrustSchema.optional(),
  plugin: z.boolean().default(false),  // NEW
});
```

**Acceptance Criteria**:
- [ ] Existing tap.json without `plugin` field still parses (defaults to false)
- [ ] `TapSkill` type includes `plugin: boolean`

---

### Unit 2: Update `adaptMarketplaceToTap` to detect plugins

**File**: `packages/core/src/marketplace.ts`

When building `TapSkill` entries from marketplace plugins, check if the marketplace entry has MCP or agent components (by checking for `.mcp.json`, `mcpServers`, or `agents` fields in the source). Since the marketplace format doesn't expose this directly, use a heuristic: if the marketplace plugin's source points to a repo that we know is a plugin (has `plugin.json`), mark it. 

Actually, the simplest approach: marketplace plugins are always `plugin: true` — the whole point of a marketplace is to list plugins, not bare skills. Set `plugin: true` on all entries produced by `adaptMarketplaceToTap`.

```typescript
skills.push({
  name: plugin.name,
  description: plugin.description ?? `Plugin from ${marketplace.name} marketplace`,
  repo,
  tags: plugin.tags ?? (plugin.category ? [plugin.category] : []),
  plugin: true,  // marketplace entries are always plugins
});
```

**Acceptance Criteria**:
- [ ] All entries from `adaptMarketplaceToTap` have `plugin: true`
- [ ] Entries from regular `tap.json` default to `plugin: false`

---

### Unit 3: Show plugin badge in `skilltap find`

**File**: `packages/cli/src/commands/find.ts`

When displaying tap results, if `skill.plugin` is `true`, show a `[plugin]` badge after the name or in the source column.

This is a display-only change in the `find` command's result formatting. Check how the current find output looks and add the badge.

**Acceptance Criteria**:
- [ ] Plugin entries show `[plugin]` in find output
- [ ] Non-plugin entries unchanged

---

### Unit 4: Add plugin count to `skilltap status`

**File**: `packages/cli/src/commands/status.ts`

Add a `plugins` field to both plain text and JSON output:

Plain text: `plugins: N` (after `taps: N`)
JSON: `"plugins": N`

Load plugins via `loadPlugins()` and count.

**Acceptance Criteria**:
- [ ] `skilltap status` shows plugin count
- [ ] `skilltap status --json` includes `plugins` number

---

### Unit 5: E2e lifecycle test

**File**: `packages/core/src/plugin/e2e-lifecycle.test.ts`

Full integration test exercising the complete plugin lifecycle:

```
describe("plugin lifecycle e2e")
  test("install → list → toggle → info → remove", async () => {
    // Setup: create a Claude plugin repo fixture
    // 1. Install plugin via installPlugin
    // 2. Verify plugins.json has record
    // 3. Verify skills placed on disk
    // 4. Verify MCP injected into agent config
    // 5. Toggle a skill off → verify moved to .disabled/
    // 6. Toggle it back on → verify restored
    // 7. Remove plugin → verify everything cleaned up
  });
```

Uses env isolation (SKILLTAP_HOME, XDG_CONFIG_HOME) and fixture repos.

**Acceptance Criteria**:
- [ ] Full lifecycle passes end-to-end
- [ ] All filesystem state verified at each step

---

## Implementation Order

1. **Unit 1**: Update `TapSkillSchema` with `plugin` field
2. **Unit 2**: Update `adaptMarketplaceToTap`
3. **Unit 3**: Update `find` command display
4. **Unit 4**: Update `status` command
5. **Unit 5**: E2e lifecycle test

---

## Testing

Unit 1-2 are covered by existing schema/marketplace tests (just verify they still pass + add one test for the new flag).

Unit 3-4 are display-only CLI changes — verified by manual inspection or lightweight subprocess tests.

Unit 5 is the main test output of this phase.

---

## Verification Checklist

```bash
bun test packages/core/src/schemas/tap.test.ts    # schema still passes
bun test packages/core/src/marketplace.test.ts     # marketplace adapter updated
bun test packages/core/src/plugin/e2e-lifecycle.test.ts  # lifecycle test
bun test  # full suite
```
