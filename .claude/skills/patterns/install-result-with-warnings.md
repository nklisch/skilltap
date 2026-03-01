# Pattern: InstallResult with Warnings

`installSkill()` returns both `records` (what was installed) and `warnings`/`semanticWarnings` (security findings), with optional callbacks for per-skill interactive interception before placement.

## Rationale

Security scan results are not errors — skills may still be installed despite warnings. Aggregating warnings into the return value keeps the flow pure and testable. The callback-driven options (`onWarnings`, `onSemanticWarnings`, `onOfferSemantic`) let the CLI layer decide how to present findings and whether to proceed, without core knowing about terminal UI.

## Examples

### Example 1: InstallResult type
**File**: `packages/core/src/install.ts:58`
```typescript
export type InstallResult = {
  records: InstalledSkill[];
  warnings: StaticWarning[];
  semanticWarnings: SemanticWarning[];
};
```

### Example 2: Warning-related callbacks in InstallOptions
**File**: `packages/core/src/install.ts:24`
```typescript
export type InstallOptions = {
  // ...data fields...
  skipScan?: boolean;
  onWarnings?: (warnings: StaticWarning[], skillName: string) => Promise<boolean>;
  onSemanticWarnings?: (warnings: SemanticWarning[], skillName: string) => Promise<boolean>;
  onOfferSemantic?: () => Promise<boolean>;
  onSemanticProgress?: (completed: number, total: number) => void;
};
```

### Example 3: Per-skill scan and callback in runSecurityScan
**File**: `packages/core/src/install.ts` (inside security scan helper)
```typescript
if (scanResult.value.length > 0) {
  allWarnings.push(...scanResult.value);
  if (onWarnings) {
    const proceed = await onWarnings(scanResult.value, skill.name);
    if (!proceed) return err(new UserError("Install cancelled."));
  }
}
```

### Example 4: Two-phase scan pipeline in install flow
**File**: `packages/core/src/install.ts` (install flow)
```typescript
// Phase 1: Static scan (unless skipScan)
if (!options.skipScan) {
  const scanResult = await runSecurityScan(selected, options.onWarnings);
  if (!scanResult.ok) return scanResult;
  allWarnings.push(...scanResult.value);
}

// Phase 2: Semantic scan (if agent available and enabled)
if (shouldRunSemantic && options.agent) {
  const semResult = await scanSemantic(skill.path, options.agent, { threshold });
  if (semResult.ok && semResult.value.length > 0) {
    allSemanticWarnings.push(...semResult.value);
    if (options.onSemanticWarnings) {
      const proceed = await options.onSemanticWarnings(semResult.value, skill.name);
      if (!proceed) return err(new UserError("Install cancelled."));
    }
  }
}
```

### Example 5: Warnings in the successful result
**File**: `packages/core/src/install.ts` (end of function)
```typescript
return ok({ records: newRecords, warnings: allWarnings, semanticWarnings: allSemanticWarnings });
```

## When to Use

- `InstallResult` is the return type for `installSkill()` — always return all three fields
- Use `skipScan: true` in tests that don't need security checks (avoids false positives on fixture content)
- Use `onWarnings`/`onSemanticWarnings` in CLI commands to show interactive prompts; in non-interactive contexts, pass `undefined` to auto-proceed with warnings accumulated in the result

## When NOT to Use

- Don't treat non-empty `warnings` as a failure — the install succeeded; warnings are advisory
- Don't put UI logic inside callback implementations in core — that belongs in the CLI layer

## Common Violations

- Ignoring `result.value.warnings` or `result.value.semanticWarnings` in CLI output — users should see security warnings even when install succeeds
- Not passing `skipScan: true` in tests — fixture content may contain patterns that trigger false positives
- Forgetting to return both `warnings` and `semanticWarnings` arrays (even if empty) in the result
