# Pattern: Policy Composition

A pure function composes config values and CLI flags into a single `EffectivePolicy` object consumed by commands, centralizing all precedence rules and validation.

## Rationale

Install and update commands need to reconcile config file settings with CLI flags, handling conflicts (e.g., `--skip-scan` vs `require_scan = true`) and mode overrides (agent mode forces specific values). Rather than scattering this logic across commands, `composePolicy()` is a single pure function that returns `Result<EffectivePolicy, UserError>` — testable, readable, and a single point of truth for "what should this command do?"

## Examples

### Example 1: Policy types
**File**: `packages/core/src/policy.ts:4`
```typescript
export type CliFlags = {
  strict?: boolean;
  noStrict?: boolean;
  skipScan?: boolean;
  yes?: boolean;
  semantic?: boolean;
  project?: boolean;
  global?: boolean;
};

export type EffectivePolicy = {
  yes: boolean;
  onWarn: "prompt" | "fail" | "allow";
  requireScan: boolean;
  skipScan: boolean;
  scanMode: "static" | "semantic" | "off";
  scope: "global" | "project" | "";
  also: string[];
  agentMode: boolean;
};
```

### Example 2: The composition function
**File**: `packages/core/src/policy.ts:29`
```typescript
export function composePolicy(
  config: Config,
  flags: CliFlags,
): Result<EffectivePolicy, UserError> {
  const agentMode = config["agent-mode"].enabled;

  // Select per-mode settings: security.agent when agent mode, security.human otherwise
  const modeSec = agentMode ? config.security.agent : config.security.human;

  if (agentMode) {
    // Agent mode: forces yes=true, reads scan/onWarn/requireScan from config.security.agent
    if (flags.skipScan && modeSec.require_scan) {
      return err(new UserError("Security scanning is required by config."));
    }
    return ok({ yes: true, onWarn: modeSec.on_warn, requireScan: modeSec.require_scan, ... });
  }

  // Normal mode: CLI flags > config.security.human > defaults
  if (flags.skipScan && modeSec.require_scan) {
    return err(new UserError("Security scanning is required by config."));
  }
  // ...compose remaining fields with precedence rules
}
```

### Example 3: CLI helper that loads config + composes policy
**File**: `packages/cli/src/ui/policy.ts:11`
```typescript
export async function loadPolicyOrExit(flags: CliFlags) {
  const configResult = await loadConfig();
  if (!configResult.ok) { errorLine(configResult.error.message); process.exit(1); }
  const config = configResult.value;

  const policyResult = composePolicy(config, flags);
  if (!policyResult.ok) {
    if (config["agent-mode"].enabled) agentError(policyResult.error.message);
    else errorLine(policyResult.error.message, policyResult.error.hint);
    process.exit(1);
  }
  return { config, policy: policyResult.value };
}
```

### Example 4: Command consuming policy for early branching
**File**: `packages/cli/src/commands/install.ts:92`
```typescript
async run({ args }) {
  const { config, policy } = await loadPolicyOrExit({
    strict: args.strict,
    noStrict: args["no-strict"],
    skipScan: args["skip-scan"],
    yes: args.yes,
    semantic: args.semantic,
    project: args.project,
    global: args.global,
  });

  if (policy.agentMode) return runAgentMode(args, config, policy);
  return runInteractiveMode(args, config, policy);
}
```

## When to Use

- Any command that needs config + CLI flag reconciliation (install, update)
- Testing policy precedence rules — `composePolicy` is pure, no I/O needed
- Adding new config/flag interactions — add to `composePolicy`, not to individual commands

## When NOT to Use

- Simple commands that don't have flag/config interactions (list, info, remove)
- Commands that only read config without reconciling flags

## Common Violations

- Scattering flag/config precedence logic across command files instead of centralizing in `composePolicy()`
- Forgetting to validate conflicting flag combinations (e.g., `--skip-scan` + `require_scan`)
- Not using `loadPolicyOrExit()` in CLI commands — it handles error display and exit codes
