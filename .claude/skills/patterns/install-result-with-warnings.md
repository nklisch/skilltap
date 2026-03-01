# Pattern: InstallResult with Warnings

`installSkill()` returns both `records` (what was installed) and `warnings` (security findings), with an optional `onWarnings` callback for per-skill interactive interception before placement.

## Rationale

Security scan results are not errors — skills may still be installed despite warnings. Aggregating warnings into the return value (rather than passing them through side effects) keeps the flow pure and testable. The `onWarnings` callback allows the CLI layer to prompt the user per-skill without core knowing anything about terminal UI.

## Examples

### Example 1: InstallResult type
**File**: `packages/core/src/install.ts:33`
```typescript
export type InstallResult = {
  records: InstalledSkill[];
  warnings: StaticWarning[];
};
```

### Example 2: InstallOptions — scan-related fields
**File**: `packages/core/src/install.ts:19`
```typescript
export type InstallOptions = {
  scope: "global" | "project";
  // ...
  skipScan?: boolean;
  /** Called before placement if warnings are found. Return true to proceed, false to abort. */
  onWarnings?: (
    warnings: StaticWarning[],
    skillName: string,
  ) => Promise<boolean>;
};
```

### Example 3: Per-skill scan and callback in runSecurityScan
**File**: `packages/core/src/install.ts:69`
```typescript
async function runSecurityScan(
  selected: ScannedSkill[],
  onWarnings?: InstallOptions["onWarnings"],
): Promise<Result<StaticWarning[], ScanError | UserError>> {
  const allWarnings: StaticWarning[] = [];
  for (const skill of selected) {
    const scanResult = await scanStatic(skill.path);
    if (!scanResult.ok) return scanResult;
    if (scanResult.value.length > 0) {
      allWarnings.push(...scanResult.value);
      if (onWarnings) {
        const proceed = await onWarnings(scanResult.value, skill.name);
        if (!proceed) return err(new UserError("Install cancelled."));
      }
    }
  }
  return ok(allWarnings);
}
```

### Example 4: Security check gated by skipScan
**File**: `packages/core/src/install.ts:169`
```typescript
if (!options.skipScan) {
  const scanResult = await runSecurityScan(selected, options.onWarnings);
  if (!scanResult.ok) return scanResult;
  allWarnings.push(...scanResult.value);
}
```

### Example 5: Warnings in the successful result
**File**: `packages/core/src/install.ts` (end of function)
```typescript
return ok({ records: newRecords, warnings: allWarnings });
```

## When to Use

- `InstallResult` is the return type for `installSkill()` — always return both fields
- Use `skipScan: true` in tests that don't need security checks (avoids false positives on fixture content)
- Use `onWarnings` in CLI commands to show an interactive prompt; in non-interactive contexts, pass `undefined` to auto-proceed with warnings accumulated in the result

## When NOT to Use

- Don't treat non-empty `warnings` as a failure — the install succeeded; warnings are advisory
- Don't put UI logic (printing, prompting) inside `onWarnings` implementations in core — that belongs in the CLI layer

## Common Violations

- Ignoring `result.value.warnings` in CLI output — users should see security warnings even when install succeeds
- Not passing `skipScan: true` in tests — fixture content may contain patterns that trigger false positives, causing tests to behave unexpectedly
- Calling `scanStatic` outside of `runSecurityScan` in install flows — keeps the scan + abort logic in one place
