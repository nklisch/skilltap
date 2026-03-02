# Pattern: Callback-Driven Options

Core functions accept typed option objects with async callback fields for UI decision points, keeping core logic pure while letting the CLI layer control interaction flow.

## Rationale

Core functions (`installSkill`, `updateSkill`) need user decisions at specific points (select skills, confirm warnings, show progress) but must not import CLI dependencies. Optional async callbacks in the options type let the CLI layer inject its UX (spinners, prompts, plain text) without core knowing about it. Omitting a callback means auto-proceed. This also simplifies testing — pass mock callbacks or omit them entirely.

## Examples

### Example 1: InstallOptions — 6 callback fields
**File**: `packages/core/src/install.ts:24`
```typescript
export type InstallOptions = {
  scope: "global" | "project";
  projectRoot?: string;
  skillNames?: string[];
  also?: string[];
  ref?: string;
  tap?: string | null;
  skipScan?: boolean;
  onWarnings?: (warnings: StaticWarning[], skillName: string) => Promise<boolean>;
  onSelectSkills?: (skills: ScannedSkill[]) => Promise<string[]>;
  onSelectTap?: (matches: TapEntry[]) => Promise<TapEntry | null>;
  agent?: AgentAdapter;
  semantic?: boolean;
  threshold?: number;
  onSemanticWarnings?: (warnings: SemanticWarning[], skillName: string) => Promise<boolean>;
  onOfferSemantic?: () => Promise<boolean>;
  onSemanticProgress?: (completed: number, total: number) => void;
};
```

### Example 2: UpdateOptions — progress + confirm callbacks
**File**: `packages/core/src/update.ts:18`
```typescript
export type UpdateOptions = {
  name?: string;
  yes?: boolean;
  strict?: boolean;
  projectRoot?: string;
  onProgress?: (skillName: string, status: "checking" | "upToDate" | "updated" | "skipped" | "linked") => void;
  onDiff?: (skillName: string, stat: DiffStat, fromSha: string, toSha: string) => void;
  onShowWarnings?: (warnings: StaticWarning[], skillName: string) => void;
  onConfirm?: (skillName: string, hasWarnings: boolean) => Promise<boolean>;
  agent?: AgentAdapter;
  semantic?: boolean;
  threshold?: number;
  onSemanticWarnings?: (warnings: SemanticWarning[], skillName: string) => void;
  onSemanticProgress?: (completed: number, total: number) => void;
};
```

### Example 3: Core calls callback at decision point
**File**: `packages/core/src/install.ts:140` (inside `runSecurityScan`)
```typescript
if (scanResult.value.length > 0) {
  allWarnings.push(...scanResult.value);
  if (onWarnings) {
    const proceed = await onWarnings(scanResult.value, skill.name);
    if (!proceed) return err(new UserError("Install cancelled."));
  }
}
```

### Example 4: Interactive CLI provides callback with spinner management
**File**: `packages/cli/src/commands/install.ts:240` (inside `runInteractiveMode`)
```typescript
onWarnings: async (warnings, skillName) => {
  s.stop();
  printWarnings(warnings, skillName);
  if (policy.onWarn === "fail") {
    log.error("Aborting due to --strict / on_warn=fail.");
    return false;
  }
  const proceed = await confirmInstall(skillName);
  s.start("Installing...");
  return proceed;
},
```

### Example 5: Agent mode provides auto-accept/hard-fail callbacks
**File**: `packages/cli/src/commands/install.ts:134`
```typescript
onWarnings: async (warnings) => {
  agentSecurityBlock(warnings, []);
  process.exit(1);
  return false;
},
onSelectSkills: async (skills) => skills.map((s) => s.name),
onSelectTap: async (matches) => matches[0] ?? null,
```

## When to Use

- Core functions that need external decisions (user prompts, progress display)
- Any new option type where the function has a decision point or output point
- Callback naming convention: `on` + verb — `onWarnings`, `onProgress`, `onConfirm`, `onSelectSkills`

## When NOT to Use

- Don't use callbacks for pure data transformations — pass the data as a parameter instead
- Don't make callbacks required — always handle the "no callback" case (auto-proceed or skip)
- Don't use events/EventEmitter — simple async callbacks are sufficient for this project

## Common Violations

- Making callbacks required instead of optional — breaks test ergonomics and forces callers to pass no-ops
- Putting UI logic (ANSI, spinners) in core callback invocations — core only calls the callback, CLI defines what it does
- Forgetting the `skipScan: true` shortcut in tests — without it, fixture content may trigger scan callbacks
- Not handling `isCancel` from `@clack/prompts` inside callbacks — clack cancel symbols need explicit handling
