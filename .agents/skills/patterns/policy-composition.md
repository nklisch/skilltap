# Pattern: Policy Composition

A pure function composes config values and CLI flags into a single `EffectivePolicy` object consumed by commands, centralizing all precedence rules.

## Rationale

Install and update commands need to reconcile config file settings with CLI flags (e.g., `--strict` overrides `on_warn`, `--skip-scan` overrides scan config, `--scope` overrides the default). Rather than scattering this logic across commands, `composePolicy()` is a single pure function — testable, readable, and the single source of truth for "what should this command do?"

## Examples

### Example 1: EffectivePolicy and CliFlags types
**File**: `packages/core/src/policy/types.ts:1`
```typescript
export type EffectivePolicy = {
  yes: boolean;
  scope: "global" | "project" | "";
  also: string[];
  scanMode: "semantic" | "static" | "none";
  onWarn: "prompt" | "fail" | "install";
  /** True when --skip-scan was passed. */
  skipScan: boolean;
  /** True when source matched a trust glob — scanMode forced to "none". */
  trusted: boolean;
};

export type CliFlags = {
  yes?: boolean;
  noYes?: boolean;
  strict?: boolean;    // → onWarn: "fail"
  skipScan?: boolean;
  deep?: boolean;      // → scanMode: "semantic"
  scope?: "project" | "global";
};
```

### Example 2: composePolicy — the pure function
**File**: `packages/core/src/policy/compose.ts:36`
```typescript
export function composePolicy(
  config: Config,
  flags: CliFlags,
): Result<EffectivePolicy, UserError> {
  return ok({
    yes: resolveYes(flags),           // flags.noYes overrides flags.yes
    scope: resolveScope(config, flags), // flags.scope > config.defaults.scope
    also: config.defaults.also,
    scanMode: resolveScanMode(config, flags), // flags.deep → "semantic"; else config.security.scan
    onWarn: resolveOnWarn(config, flags),     // flags.strict → "fail"; else config.security.on_warn
    skipScan: flags.skipScan === true,
    trusted: false,  // always false from base compose; set to true by composePolicyForSource
  });
}
```

### Example 3: composePolicyForSource — per-source trust overlay
**File**: `packages/core/src/policy/compose.ts:51`
```typescript
export function composePolicyForSource(
  config: Config,
  flags: CliFlags,
  source: SourceForPolicy,  // { tapName?, sourceUrl }
): Result<EffectivePolicy, UserError> {
  const base = composePolicy(config, flags);
  if (!base.ok) return base;

  if (isTrusted(config.security.trust, source)) {
    return ok({ ...base.value, trusted: true, scanMode: "none" });
  }
  return base;
}
```
When the source matches a trust glob, `trusted: true` and `scanMode: "none"` — scanning is skipped entirely.

### Example 4: loadPolicyOrExit — CLI-layer wrapper
**File**: `packages/cli/src/ui/policy.ts:10`
```typescript
export async function loadPolicyOrExit(flags: CliFlags) {
  const configResult = await loadConfig();
  if (!configResult.ok) { out.error(configResult.error.message); process.exit(1); }

  const policyResult = composePolicy(configResult.value, flags);
  if (!policyResult.ok) { out.error(policyResult.error.message); process.exit(1); }

  return { config: configResult.value, policy: policyResult.value };
}
```
Used by 4 commands: `install/shared.ts`, `install/mcp.ts`, `update.ts`, `remove/shared.ts`. The fifth call site (`find.ts`) calls `composePolicy` directly to avoid the exit behavior.

### Example 5: Command consuming policy
**File**: `packages/cli/src/commands/install/shared.ts:81`
```typescript
const { config, policy } = await loadPolicyOrExit({
  strict: args.strict,
  skipScan: args["skip-scan"],
  yes: args.yes,
  scope: scopeFlag,   // "project" | "global" | undefined
});

// policy.scope, policy.scanMode, policy.onWarn, policy.skipScan used downstream
const scope = await resolveScope(policy, ...);
```

## When to Use

- Any command that needs config + CLI flag reconciliation (install, update, remove)
- Testing policy precedence rules — `composePolicy` is pure, no I/O needed
- Adding new config/flag interactions — add to `composePolicy`, not to individual commands

## When NOT to Use

- Commands that don't have flag/config interactions (list, info, doctor)
- Per-source trust decisions — use `composePolicyForSource` after resolving the source

## Common Violations

- Scattering flag/config precedence logic across command files instead of centralizing in `composePolicy`
- Directly comparing `policy.scanMode === "semantic"` vs `policy.skipScan` at call sites — check `policy.trusted` first (trusted sources skip scanning entirely)
- Not using `loadPolicyOrExit()` in CLI commands — it handles error display and exit codes correctly
- Confusing `"none"` (current `scanMode` off value) with old `"off"` — the enum is `"semantic" | "static" | "none"`
