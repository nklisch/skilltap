# Pattern: Agent Mode Branching

CLI commands that behave differently in agent mode fork early into `runAgentMode()` vs `runInteractiveMode()` after loading policy.

## Rationale

When an AI agent runs skilltap (not a human), the CLI must: use plain text (no ANSI/spinners), auto-accept prompts, hard-fail on security warnings, and output machine-parseable status lines. Rather than littering every callback with `if (agentMode)` checks, each command splits into two clean functions with completely separate UX logic.

## Examples

### Example 1: Early fork in run()
**File**: `packages/cli/src/commands/install.ts:92`
```typescript
async run({ args }) {
  const { config, policy } = await loadPolicyOrExit({ /* flags */ });

  if (policy.agentMode) return runAgentMode(args, config, policy);
  return runInteractiveMode(args, config, policy);
}
```

### Example 2: Agent mode — auto-accept, hard-fail, plain output
**File**: `packages/cli/src/commands/install.ts:112`
```typescript
async function runAgentMode(args, config, policy): Promise<void> {
  const result = await installSkill(args.source, {
    scope: policy.scope as "global" | "project",
    skipScan: false,
    onWarnings: async (warnings) => { agentSecurityBlock(warnings, []); process.exit(1); return false; },
    onSelectSkills: async (skills) => skills.map((s) => s.name),       // auto-select all
    onSelectTap: async (matches) => matches[0] ?? null,                 // auto-select first
    onSemanticWarnings: async (warnings) => { agentSecurityBlock([], warnings); process.exit(1); return false; },
  });

  if (!result.ok) { agentError(result.error.message); process.exit(1); }
  for (const record of result.value.records) {
    agentSuccess(record.name, installDir, record.ref);                  // plain text: "OK: Installed ..."
  }
}
```

### Example 3: Interactive mode — spinners, prompts, ANSI colors
**File**: `packages/cli/src/commands/install.ts:170`
```typescript
async function runInteractiveMode(args, config, policy): Promise<void> {
  intro("skilltap");
  const s = spinner();
  s.start("Resolving source...");
  // ...
  const result = await installSkill(args.source, {
    onWarnings: async (warnings, skillName) => {
      s.stop(); printWarnings(warnings, skillName);
      if (policy.onWarn === "fail") return false;
      return confirmInstall(skillName);                                 // interactive prompt
    },
    onSelectSkills: async (skills) => { s.stop(); return selectSkills(skills); },
    onSelectTap: async (matches) => { s.stop(); return selectTap(matches); },
    // ...
  });
  outro("Done!");
}
```

### Example 4: Agent output functions (no ANSI)
**File**: `packages/cli/src/ui/agent-out.ts:3`
```typescript
export function agentSuccess(name: string, path: string, ref?: string | null) {
  const refStr = ref ? ` (${ref})` : "";
  process.stdout.write(`OK: Installed ${name} → ${path}${refStr}\n`);
}

export function agentSecurityBlock(staticWarnings, semanticWarnings) {
  process.stderr.write("SECURITY ISSUE FOUND — INSTALLATION BLOCKED\n");
  process.stderr.write("DO NOT install this skill without manual review.\n\n");
  // ... list warnings as plain text
}
```

## When to Use

- Any command where agent mode behavior differs from interactive behavior (install, update)
- The branch happens after `loadPolicyOrExit()` — use `policy.agentMode` to decide

## When NOT to Use

- Commands with no interactive elements (list, info, find --json) — same output either way
- Don't add agent mode branching to a command unless SPEC.md requires distinct agent behavior

## Common Violations

- Using `if (agentMode)` inside callback implementations instead of having two separate function branches
- Calling `intro()`, `spinner()`, or `@clack/prompts` functions inside `runAgentMode()` — agent mode must be silent
- Forgetting `process.exit(1)` after `agentSecurityBlock()` — agent mode hard-fails on warnings
- Using `agentError()` in interactive mode or `errorLine()` in agent mode — each branch uses its own output helpers
