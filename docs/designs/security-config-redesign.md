# Design: Security Configuration Redesign

## Overview

Redesign skilltap's security configuration system to support:

1. **Named presets** (`none`, `relaxed`, `standard`, `strict`) for quick one-command setup
2. **Independent per-mode settings** — human and agent modes each have their own scan/on_warn config, with no enforced minimums on agent mode
3. **Per-trust-tier overrides** — configure security level per named tap and per source type (git URL, npm, local path), with tier overrides beating mode defaults
4. **Dedicated `config security` subcommand** — interactive wizard + non-interactive flag-based scripting
5. **End-of-wizard summary** showing effective policy for both modes before saving

### Current Problems

- Security settings are a flat `[security]` section that applies globally, with agent mode silently overriding at policy composition time
- No way to say "trust my internal tap" without `--skip-scan` on every install
- Agent mode forces `on_warn=fail` and `require_scan=true` with no user control
- Setup wizard buries security across scattered prompts with no preset-based fast path
- `on_warn` only has `prompt`/`fail` — no `allow` option to ignore warnings

---

## Implementation Units

### Unit 1: New Config Schema

**File**: `packages/core/src/schemas/config.ts`

Replace the flat `SecurityConfigSchema` with a structured schema supporting per-mode settings and trust tier overrides.

```typescript
// --- New constants ---

export const SECURITY_PRESETS = ["none", "relaxed", "standard", "strict"] as const;
export const ON_WARN_MODES = ["prompt", "fail", "allow"] as const; // add "allow"
export const SOURCE_TYPES = ["tap", "git", "npm", "local"] as const;

// --- Per-mode security settings ---

export const SecurityModeSchema = z.object({
  scan: z.enum(SCAN_MODES).default("static"),
  on_warn: z.enum(ON_WARN_MODES).default("prompt"),
  require_scan: z.boolean().default(false),
});

// --- Per-trust-tier override ---

export const TrustOverrideSchema = z.object({
  /** Named tap or source type this override applies to */
  match: z.string(),
  /** What kind of match: a specific tap name, or a source type */
  kind: z.enum(["tap", "source"]),
  /** Security preset to apply for this tier */
  preset: z.enum(SECURITY_PRESETS),
});

// --- Top-level security schema ---

export const SecurityConfigSchema = z.object({
  // Per-mode settings
  human: SecurityModeSchema.prefault({}),
  agent: SecurityModeSchema.prefault({
    scan: "static",
    on_warn: "fail",
    require_scan: true,
  }),

  // Shared settings (not per-mode)
  agent_cli: z.string().default(""),           // renamed from "agent" to avoid confusion with agent mode
  threshold: z.number().int().min(0).max(10).default(5),
  max_size: z.number().int().default(51200),
  ollama_model: z.string().default(""),

  // Trust tier overrides — evaluated in order, first match wins
  overrides: z.array(TrustOverrideSchema).default([]),
});
```

**Preset definitions** (new constant, maps preset name to SecurityMode values):

```typescript
export const PRESET_VALUES: Record<
  (typeof SECURITY_PRESETS)[number],
  { scan: "static" | "semantic" | "off"; on_warn: "prompt" | "fail" | "allow"; require_scan: boolean }
> = {
  none:     { scan: "off",      on_warn: "allow",  require_scan: false },
  relaxed:  { scan: "static",   on_warn: "allow",  require_scan: false },
  standard: { scan: "static",   on_warn: "prompt", require_scan: false },
  strict:   { scan: "semantic", on_warn: "fail",   require_scan: true  },
};
```

**Migration**: The old flat `security.scan` / `security.on_warn` / `security.require_scan` / `security.agent` fields must be migrated to the new schema on first load. See Unit 3.

**Acceptance Criteria**:
- [ ] `SecurityModeSchema` validates per-mode fields independently
- [ ] `TrustOverrideSchema` validates match/kind/preset
- [ ] `PRESET_VALUES` maps all 4 presets to concrete mode settings
- [ ] `ON_WARN_MODES` includes `"allow"`
- [ ] `SecurityConfigSchema` has `human`, `agent`, `overrides` fields
- [ ] Old `security.agent` field renamed to `security.agent_cli` to avoid ambiguity
- [ ] All constants are exported for use by CLI and tests

---

### Unit 2: Policy Composition Update

**File**: `packages/core/src/policy.ts`

Update `composePolicy` to use per-mode settings and resolve trust tier overrides.

```typescript
export type EffectivePolicy = {
  yes: boolean;
  onWarn: "prompt" | "fail" | "allow";       // add "allow"
  requireScan: boolean;
  skipScan: boolean;
  scanMode: "static" | "semantic" | "off";
  scope: "global" | "project" | "";
  also: string[];
  agentMode: boolean;
};

/**
 * Resolve trust-tier override for a given install source.
 * Returns the preset name if a matching override exists, null otherwise.
 */
export function resolveOverride(
  overrides: TrustOverride[],
  source: { tapName?: string; sourceType: "tap" | "git" | "npm" | "local" },
): (typeof SECURITY_PRESETS)[number] | null;

/**
 * Compose effective security policy from config + CLI flags.
 * Uses per-mode settings (human vs agent). No enforced minimums on agent mode.
 */
export function composePolicy(
  config: Config,
  flags: CliFlags,
): Result<EffectivePolicy, UserError>;

/**
 * Compose effective policy with a trust-tier override applied.
 * Called per-source during install when the source is known.
 */
export function composePolicyForSource(
  config: Config,
  flags: CliFlags,
  source: { tapName?: string; sourceType: "tap" | "git" | "npm" | "local" },
): Result<EffectivePolicy, UserError>;
```

**Implementation Notes**:

- `composePolicy` reads `config.security.human` or `config.security.agent` based on `config["agent-mode"].enabled`
- Agent mode no longer forces any overrides — it simply uses `config.security.agent.*` settings as-is
- Agent mode still sets `yes: true` and `agentMode: true` (behavioral, not security)
- `composePolicyForSource` calls `resolveOverride` first; if a match is found, the preset values replace the mode defaults. CLI flags (`--strict`, `--semantic`, `--skip-scan`) still override on top.
- Override resolution order: named tap match (exact) → source type match → mode default
- `--skip-scan` is rejected only when the effective `requireScan` is true (from mode config OR override)

**Acceptance Criteria**:
- [ ] Agent mode uses `config.security.agent.*` directly, no hardcoded overrides
- [ ] Human mode uses `config.security.human.*` directly
- [ ] `resolveOverride` returns correct preset for tap name match, source type match, and no match
- [ ] Named tap overrides take priority over source type overrides
- [ ] CLI flags override trust tier preset values
- [ ] `--skip-scan` + `requireScan: true` returns `UserError` regardless of mode
- [ ] `on_warn: "allow"` propagates correctly through the policy

---

### Unit 3: Config Migration

**File**: `packages/core/src/config.ts`

Add a migration step in `loadConfig` that converts old flat security config to the new per-mode structure.

```typescript
/**
 * Migrate v1 flat security config to v2 per-mode structure.
 * Called during loadConfig before Zod validation.
 *
 * v1 shape:
 *   [security]
 *   scan = "static"
 *   on_warn = "prompt"
 *   require_scan = false
 *   agent = "claude"
 *
 * v2 shape:
 *   [security]
 *   agent_cli = "claude"
 *   [security.human]
 *   scan = "static"
 *   on_warn = "prompt"
 *   require_scan = false
 *   [security.agent]
 *   scan = "static"
 *   on_warn = "fail"
 *   require_scan = true
 */
export function migrateSecurityConfig(raw: Record<string, unknown>): Record<string, unknown>;
```

**Implementation Notes**:
- Detect v1 by checking for `security.scan` as a top-level string (not nested under `human`/`agent`)
- Copy v1 values into `security.human`
- Set `security.agent` to strict defaults (`scan: v1.scan === "off" ? "static" : v1.scan`, `on_warn: "fail"`, `require_scan: true`) to preserve current agent-mode behavior for existing users
- Rename `security.agent` (the CLI path string) to `security.agent_cli`
- Migration is idempotent — running it on v2 config is a no-op
- Config is saved back after migration so the TOML file is updated

**Acceptance Criteria**:
- [ ] v1 config with flat `security.scan` migrates to `security.human.scan` + `security.agent.scan`
- [ ] `security.agent` (string, CLI path) moves to `security.agent_cli`
- [ ] Agent mode defaults preserve current strict behavior for existing users
- [ ] Already-migrated v2 config passes through unchanged
- [ ] Missing v1 fields get schema defaults

---

### Unit 4: `config security` Subcommand — Interactive Wizard

**File**: `packages/cli/src/commands/config/security.ts`

New subcommand registered in `packages/cli/src/commands/config.ts`.

```typescript
import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "skilltap config security",
    description: "Configure security settings",
  },
  args: {
    // Non-interactive flags (Unit 5)
    preset: { type: "string", description: "Apply a named preset: none, relaxed, standard, strict" },
    mode: { type: "string", description: "Which mode to configure: human, agent, both" },
    scan: { type: "string", description: "Scan level: static, semantic, off" },
    "on-warn": { type: "string", description: "Warning behavior: prompt, fail, allow" },
    "require-scan": { type: "boolean", description: "Block --skip-scan" },
    trust: { type: "string", description: "Add trust override: tap:name=preset or source:type=preset" },
    "remove-trust": { type: "string", description: "Remove a trust override by match name" },
  },
  async run({ args }) {
    // If any flags provided → non-interactive mode (Unit 5)
    // Otherwise → interactive wizard
  },
});
```

**Interactive wizard flow:**

```
┌  Security Configuration
│
◆  Configure which mode?
│  ○ Human (when you run skilltap)
│  ○ Agent (when AI agents run skilltap)
│  ○ Both (same settings for both)
│
◆  Security preset? (or "Custom" to set individually)
│  ○ None — no scanning
│  ○ Relaxed — static scan, ignore warnings
│  ○ Standard — static scan, ask on warnings (Recommended)
│  ○ Strict — static + semantic scan, block on warnings
│  ○ Custom
│
│  [If Custom:]
◆  Scan level?
│  ○ Static only
│  ○ Static + Semantic
│  ○ Off
│
◆  When warnings are found?
│  ○ Ask me (prompt)
│  ○ Always block (fail)
│  ○ Ignore warnings (allow)
│
◆  Require scanning? (block --skip-scan)
│  ○ Yes  ○ No
│
│  [If semantic chosen, for either mode:]
◆  Agent CLI for semantic scanning?
│  ○ Claude Code (detected)
│  ○ Gemini CLI (detected)
│  ○ Other — enter path
│
◆  Configure trust overrides?
│  ○ Yes  ○ No, keep current overrides
│
│  [If yes:]
◆  Add override for:
│  ○ A specific tap
│  ○ All git URL sources
│  ○ All npm sources
│  ○ All local path sources
│  ○ Done adding overrides
│
│  [If tap:]
◇  Tap name: my-company-tap
│
◆  Security preset for "my-company-tap"?
│  ○ None / Relaxed / Standard / Strict
│
│  [Loop back to "Add override for:" until "Done"]
│
◇  Summary
│
│  ┌─────────────────────────────────────────────┐
│  │  Human mode:  standard (static + prompt)    │
│  │  Agent mode:  strict (semantic + fail)      │
│  │                                             │
│  │  Trust overrides:                           │
│  │    my-company-tap → none (no scanning)      │
│  │    npm sources    → standard                │
│  └─────────────────────────────────────────────┘
│
◆  Save these settings?
│  ○ Yes  ○ No
│
└  Wrote ~/.config/skilltap/config.toml
```

**Implementation Notes**:
- Uses clack `group`, `select`, `confirm`, `note` for the wizard
- The summary uses `note()` to render a formatted box
- When configuring "Both", the same values are written to both `security.human` and `security.agent`
- Override editing shows existing overrides and allows add/remove
- Tap name autocomplete: load installed taps from config to offer as options

**Acceptance Criteria**:
- [ ] Interactive wizard walks through mode → preset/custom → overrides → summary → save
- [ ] "Both" mode writes identical settings to human and agent sections
- [ ] Preset selection sets scan + on_warn + require_scan atomically
- [ ] Custom path allows individual field selection
- [ ] Trust override loop allows adding multiple overrides
- [ ] Summary displays effective policy for both modes and all overrides
- [ ] Ctrl+C at any point cancels cleanly (exit 2)
- [ ] TTY check — exits with error if not interactive (unless flags provided)

---

### Unit 5: `config security` — Non-Interactive Mode

**File**: `packages/cli/src/commands/config/security.ts` (same file as Unit 4)

When any flag is provided, skip the wizard and apply changes directly.

```
# Apply preset to human mode
skilltap config security --preset standard --mode human

# Apply preset to both modes
skilltap config security --preset strict

# Set individual fields
skilltap config security --mode agent --scan off --on-warn allow

# Add trust override
skilltap config security --trust tap:my-company=none
skilltap config security --trust source:npm=standard

# Remove trust override
skilltap config security --remove-trust my-company

# Show current security config (no flags modifying)
skilltap config security --mode human   # (no setting flags → prints current)
```

**Implementation Notes**:
- `--preset` applies `PRESET_VALUES[preset]` to the target mode(s)
- `--mode` defaults to `"both"` when not specified
- `--trust` format: `tap:<name>=<preset>` or `source:<type>=<preset>` — parsed and appended to `security.overrides[]`
- `--remove-trust` removes the first override with matching `match` field
- If only `--mode` is provided with no setting flags, print the current security config for that mode (like `config get` but formatted)
- All validation errors exit 1 with a clear error message

**Acceptance Criteria**:
- [ ] `--preset strict` applies strict settings to both modes by default
- [ ] `--preset none --mode agent` applies none to agent mode only
- [ ] `--trust tap:my-corp=none` adds override to config
- [ ] `--trust source:npm=strict` adds source-type override
- [ ] `--remove-trust my-corp` removes matching override
- [ ] Invalid preset name, mode, or trust format exits 1 with helpful error
- [ ] Works in non-TTY (piped) environments

---

### Unit 6: Update General Config Wizard

**File**: `packages/cli/src/commands/config.ts`

Remove inline security prompts from the general wizard. Replace with a summary line and pointer to `config security`.

```typescript
// Remove: scan, agent, onWarn prompts
// Add after the scope/also prompts:

note(
  `Security: ${describeSecurityMode(existing.security.human)} (human) / ` +
  `${describeSecurityMode(existing.security.agent)} (agent)\n` +
  `Run 'skilltap config security' to change.`,
  "Security",
);
```

**Implementation Notes**:
- `describeSecurityMode(mode)` returns a human-friendly string like `"standard (static + prompt)"` by reverse-mapping mode values to preset names, or `"custom"` if no preset matches
- The general wizard no longer modifies `security.*` fields
- Keeps the same config prompts for scope, also, showDiff, registries, telemetry

**Acceptance Criteria**:
- [ ] General wizard no longer has scan/onWarn/agent prompts
- [ ] Displays current security summary with pointer to `config security`
- [ ] Security config is preserved (not overwritten) when saving from general wizard

---

### Unit 7: Update Agent Mode Wizard

**File**: `packages/cli/src/commands/config/agent-mode.ts`

Update to configure `security.agent` mode settings instead of overwriting the shared `security.*` fields.

```typescript
// Current: writes to config.security.scan (shared)
// New: writes to config.security.agent.scan, config.security.agent.on_warn, etc.

// Remove the scan mode filter that excluded "off" — agent mode is now fully configurable
// Add preset selection as the primary path (same as config security wizard)
```

**Implementation Notes**:
- Enable/disable toggle stays the same
- When enabling, offer preset selection (none/relaxed/standard/strict) for the agent security profile
- Custom option allows individual field selection
- No longer touches `config.security.scan` or `config.security.on_warn` (those are now `config.security.human.*`)
- Agent mode enable no longer forces any security minimums — it uses whatever `security.agent` is configured to

**Acceptance Criteria**:
- [ ] Enabling agent mode configures `security.agent.*` fields
- [ ] "Off" / "none" preset is available (no longer filtered out)
- [ ] Disabling agent mode does not change security settings
- [ ] Human security settings are never touched by this wizard

---

### Unit 8: Update Install Callbacks for `on_warn: "allow"`

**File**: `packages/cli/src/ui/install-callbacks.ts`

Add handling for `on_warn: "allow"` — when warnings are found, log them but continue without prompting.

```typescript
// Current: "prompt" → ask, "fail" → exit
// New:     "prompt" → ask, "fail" → exit, "allow" → log + continue

if (policy.onWarn === "allow") {
  // Log warnings for visibility but auto-continue
  // onWarnings callback returns true (proceed)
  // onSemanticWarnings callback returns true (proceed)
}
```

**Implementation Notes**:
- Warnings are still displayed (user should see what was found) but installation proceeds automatically
- In agent mode with `on_warn: "allow"`, warnings are logged as info lines (not errors)
- This is the only behavioral change in the install flow; the rest of the install pipeline is unchanged

**Acceptance Criteria**:
- [ ] `on_warn: "allow"` logs warnings but does not prompt or block
- [ ] `on_warn: "allow"` in agent mode outputs warnings as info, not errors
- [ ] `on_warn: "prompt"` and `on_warn: "fail"` behave unchanged

---

### Unit 9: Per-Source Policy in Install Flow

**File**: `packages/core/src/install.ts`

Update `installSkill` to accept source metadata and use `composePolicyForSource` for per-source security resolution.

```typescript
// Add to InstallOptions:
export type InstallOptions = {
  // ... existing fields ...
  /** Source metadata for trust-tier override resolution */
  source?: { tapName?: string; sourceType: "tap" | "git" | "npm" | "local" };
};
```

**Implementation Notes**:
- The CLI layer already knows the source type from `resolveSource()` — pass it through to `installSkill`
- If `source` is provided, call `composePolicyForSource` instead of `composePolicy`
- If `source` is omitted (backwards compat), fall back to `composePolicy`
- The resolved override preset is logged when verbose mode is on: `"Using 'none' security for tap 'my-corp'"`

**Acceptance Criteria**:
- [ ] Install from a tap with a `none` override skips all scanning
- [ ] Install from a git URL with no override uses the mode default
- [ ] Override resolution logged in verbose mode
- [ ] Omitting `source` in options falls back to base policy (backwards compat)

---

### Unit 10: Config Keys Update

**File**: `packages/core/src/config-keys.ts`

Update `SETTABLE_KEYS` and `BLOCKED_SET_KEYS` for the new schema.

```typescript
// New settable keys
export const SETTABLE_KEYS: Record<string, SettableKeyDef> = {
  // ... existing non-security keys unchanged ...
  "security.agent_cli": { type: "string" },
  "security.ollama_model": { type: "string" },
  "security.threshold": { type: "number" },
  "security.max_size": { type: "number" },
};

// Security mode fields go through the wizard
const BLOCKED_SET_KEYS: Record<string, string> = {
  "security.human.scan": "Use 'skilltap config security'",
  "security.human.on_warn": "Use 'skilltap config security'",
  "security.human.require_scan": "Use 'skilltap config security'",
  "security.agent.scan": "Use 'skilltap config security'",
  "security.agent.on_warn": "Use 'skilltap config security'",
  "security.agent.require_scan": "Use 'skilltap config security'",
  "security.overrides": "Use 'skilltap config security --trust'",
  // ... existing blocked keys ...
};
```

**Acceptance Criteria**:
- [ ] `config set security.agent_cli claude` works
- [ ] `config set security.human.scan off` is blocked with hint to use wizard
- [ ] `config set security.overrides ...` is blocked with hint to use `--trust`
- [ ] Old key `security.agent` is blocked with migration hint
- [ ] Old key `security.scan` is blocked with migration hint

---

### Unit 11: Helper — `describeSecurityMode`

**File**: `packages/core/src/security/describe.ts`

Pure function that maps security mode values to human-friendly descriptions.

```typescript
import type { SecurityMode } from "../schemas/config";
import { PRESET_VALUES, SECURITY_PRESETS } from "../schemas/config";

/**
 * Return a human-friendly label for a security mode configuration.
 * Matches against known presets first, falls back to "custom (...)" description.
 *
 * Examples:
 *   { scan: "static", on_warn: "prompt", require_scan: false } → "standard (static + prompt)"
 *   { scan: "semantic", on_warn: "fail", require_scan: true }  → "strict (semantic + fail + require scan)"
 *   { scan: "static", on_warn: "fail", require_scan: false }   → "custom (static + fail)"
 */
export function describeSecurityMode(mode: SecurityMode): string;

/**
 * Return the preset name if the mode exactly matches a preset, or null.
 */
export function matchPreset(mode: SecurityMode): (typeof SECURITY_PRESETS)[number] | null;
```

**Acceptance Criteria**:
- [ ] All 4 presets are recognized and labeled correctly
- [ ] Non-preset combinations return "custom" with details
- [ ] Used by config wizard summary, `config security` display, and `doctor` output

---

## Implementation Order

1. **Unit 1**: Config schema — everything depends on this
2. **Unit 3**: Config migration — must work before any code reads the new schema
3. **Unit 11**: `describeSecurityMode` helper — used by multiple UI units
4. **Unit 2**: Policy composition — core logic for the new per-mode + override system
5. **Unit 10**: Config keys — update settable/blocked keys
6. **Unit 8**: Install callbacks for `on_warn: "allow"`
7. **Unit 9**: Per-source policy in install flow
8. **Unit 4**: `config security` interactive wizard
9. **Unit 5**: `config security` non-interactive mode
10. **Unit 6**: Update general config wizard
11. **Unit 7**: Update agent-mode wizard

Units 4-5 can be implemented together (same file). Units 6-7 can be parallelized.

---

## TOML Config Example (after migration)

```toml
[security]
agent_cli = "claude"
threshold = 5
max_size = 51200
ollama_model = ""

[security.human]
scan = "static"
on_warn = "prompt"
require_scan = false

[security.agent]
scan = "static"
on_warn = "fail"
require_scan = true

[[security.overrides]]
match = "my-company-tap"
kind = "tap"
preset = "none"

[[security.overrides]]
match = "npm"
kind = "source"
preset = "standard"
```

---

## Testing

### Unit Tests: `packages/core/src/schemas/config.test.ts`
- Schema accepts valid v2 security config
- Schema rejects invalid `on_warn: "allow"` only if we forgot to add it (regression)
- `PRESET_VALUES` covers all 4 presets with correct values
- `SecurityModeSchema` defaults are correct

### Unit Tests: `packages/core/src/policy.test.ts`
- Human mode reads `security.human.*`
- Agent mode reads `security.agent.*` (no hardcoded overrides)
- Agent mode with `scan: "off"` is allowed (no enforced floor)
- `resolveOverride` — tap match beats source match
- `resolveOverride` — no match returns null
- `composePolicyForSource` — override replaces mode defaults
- `composePolicyForSource` — CLI flags override trust tier
- `--skip-scan` rejected when `requireScan: true`
- `on_warn: "allow"` propagates through

### Unit Tests: `packages/core/src/config.test.ts` (migration)
- v1 flat config migrates to v2 per-mode
- v2 config passes through unchanged
- `security.agent` (string) moves to `security.agent_cli`
- Missing v1 fields get defaults

### Unit Tests: `packages/core/src/security/describe.test.ts`
- All 4 presets matched and labeled
- Custom combo returns "custom" with details
- `matchPreset` returns null for non-matching combos

### Integration Tests: `packages/cli/src/commands/config/security.test.ts`
- Non-interactive: `--preset strict` writes correct config
- Non-interactive: `--trust tap:foo=none` adds override
- Non-interactive: `--remove-trust foo` removes override
- Non-interactive: invalid preset exits 1
- Subprocess: TTY required for interactive mode (non-TTY without flags exits 1)

### CLI Subprocess Tests: `packages/cli/src/commands/install.test.ts` (additions)
- Install from tap with `none` override → no scan output
- Install from git URL with `strict` override → semantic scan runs
- `on_warn: "allow"` → warnings logged, install succeeds

---

## Verification Checklist

```bash
# Schema tests
bun test packages/core/src/schemas/config.test.ts

# Policy tests
bun test packages/core/src/policy.test.ts

# Migration tests
bun test packages/core/src/config.test.ts

# Describe helper tests
bun test packages/core/src/security/describe.test.ts

# Config security command tests
bun test packages/cli/src/commands/config/security.test.ts

# Install integration tests
bun test packages/cli/src/commands/install.test.ts

# Full suite
bun test

# Manual: run the wizard
bun run dev config security

# Manual: non-interactive preset
bun run dev config security --preset strict --mode agent

# Manual: trust override
bun run dev config security --trust tap:my-corp=none

# Manual: verify migration from old config
# (copy a v1 config.toml, run any command, check config was updated)
```
