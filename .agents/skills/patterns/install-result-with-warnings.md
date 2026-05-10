# Pattern: InstallResult with Warnings

`installSkill()` returns both `records` (what was installed) and `warnings`/`semanticWarnings` (security findings), with optional callbacks for per-skill interactive interception before placement.

## Rationale

Security scan results are not errors — skills may still be installed despite warnings. Aggregating warnings into the return value keeps the flow pure and testable. The callback-driven options (`onWarnings`, `onSemanticWarnings`, `onOfferSemantic`) let the CLI layer decide how to present findings and whether to proceed, without core knowing about terminal UI.

## Examples

### Example 1: InstallResult type
**File**: `packages/core/src/install/types.ts:105`
```typescript
export type InstallResult = {
  records: InstalledSkill[];
  warnings: StaticWarning[];
  semanticWarnings: SemanticWarning[];
  /** Names of skills that were already installed and the user chose to update. */
  updates: string[];
  /** Plugin record if a plugin was installed. */
  pluginRecord?: PluginRecord;
  /** Components captured from standalone state into the plugin (if capture occurred). */
  captured?: {
    skills: string[];
    mcpServers: string[];
    forcedCrossSource: { skills: string[]; mcpServers: string[] };
  };
};
```

### Example 2: Warning callback in InstallOptions — unified signature
**File**: `packages/core/src/install/types.ts:41`
```typescript
export type InstallOptions = {
  // ...data fields...
  skipScan?: boolean;
  /** Unified callback for static AND semantic warnings. Return false to abort. */
  onWarnings?: (
    warnings: StaticWarning[] | SemanticWarning[],
    kind: "skill-static" | "plugin-static" | "skill-semantic",
    name: string,
  ) => Promise<boolean>;
};
```
Static and semantic warnings share a single callback (unlike the old `onWarnings`/`onSemanticWarnings` split). The `kind` discriminator lets callers format messages differently.

### Example 3: Unified onWarnings callback at the call site
**File**: `packages/cli/src/ui/install-callbacks.ts`
```typescript
onWarnings: async (warnings, kind, name) => {
  p.pause();
  if (kind === "skill-static" || kind === "plugin-static") {
    printWarnings(warnings as StaticWarning[], name);
  } else {
    printSemanticWarnings(warnings as SemanticWarning[], name);
  }
  if (policy.onWarn === "fail") return false;
  return confirmInstall(name);
},
```
The `kind` discriminator lets the callback format static vs semantic warnings differently, with a single callback rather than two.

### Example 4: Warnings in the successful result
**File**: `packages/core/src/install/orchestrate.ts`
```typescript
return ok({
  records: newRecords,
  warnings: allWarnings,
  semanticWarnings: allSemanticWarnings,
  updates: updatedNames,
});
```

## When to Use

- `InstallResult` is the return type for `installSkill()` — always populate all required fields
- Use `skipScan: true` in tests that don't need security checks (avoids false positives on fixture content)
- Use the unified `onWarnings(warnings, kind, name)` callback in CLI commands; pass `undefined` to auto-proceed

## When NOT to Use

- Don't treat non-empty `warnings` as a failure — the install succeeded; warnings are advisory
- Don't put UI logic inside callback implementations in core — that belongs in the CLI layer

## Common Violations

- Ignoring `result.value.warnings` or `result.value.semanticWarnings` in CLI output — users should see security warnings even when install succeeds
- Not passing `skipScan: true` in tests — fixture content may contain patterns that trigger false positives
- Implementing separate `onWarnings`/`onSemanticWarnings` callbacks — the unified `onWarnings(warnings, kind, name)` handles both
