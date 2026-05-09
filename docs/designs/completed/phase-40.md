# Design: Phase 40 — Drop Legacy Fallbacks + Agent-Mode (One-Shot Cleanup)

## Overview

Phase 40 demolishes the entire "agent-mode" runtime split and the v0.x state-file
fallback. After this phase:

- `state.json` is the only canonical store. `installed.json` / `plugins.json`
  are read by `migrate` only; production code paths never touch them.
- The CLI runs as a single non-branching runtime. No `--agent` flag, no
  `SKILLTAP_AGENT` env var, no `[agent-mode]` config block, no per-mode
  security split, no parallel `runAgentMode` / `runInteractiveMode` orchestrators.
- `composePolicy()` returns a single `EffectivePolicy` shape (no `agentMode`
  field).
- Every call site of `agentError` / `agentSuccess` / `agentSkip` /
  `exitWithError(agentMode, ...)` / `isAgentMode()` is rewritten to use
  `format.ts` helpers (`errorLine`, `successLine`, `infoLine`).

The full set of files to delete:
- `packages/cli/src/ui/agent-out.ts` (82 lines)
- `packages/core/src/agent-env.ts` (5 lines)
- `packages/cli/src/commands/config/agent-mode.ts` (210 lines)
- `packages/cli/src/commands/config/agent-mode.test.ts`
- `packages/cli/src/commands/update.agent-mode.test.ts`

After Phase 40 the repo grows simpler, not larger. ~600 production LOC removed; ~950 test LOC rewritten or deleted.

## Acceptance Criteria (project-wide)

- `grep -r "agentMode\|agent-mode\|SKILLTAP_AGENT\|agent-out\|agent-env\|isAgentEnv\|isAgentMode" packages/ --include="*.ts" | grep -v ".test.ts"` returns **no results in source files**. (Test files may retain transitional references for the migrate-translation tests.)
- Full test suite passes (`bun test`).
- `bun run build` produces a clean binary.
- `skilltap install --help` shows no `--agent` flag.
- `skilltap config set agent-mode.enabled true` errors with a hint to run `migrate`.
- A v1 config with `[security.human]` / `[security.agent]` / `[agent-mode]` blocks is parseable by the loader (extra keys silently ignored), and `skilltap migrate` translates it cleanly.
- Existing `--yes` and `--json` flags carry the unattended-use story.

## Architectural Options Considered

### Option A — Big-bang cutover

Delete all 3 files, refactor all 22 importers, collapse policy + schema + commands in one mega-commit. Pro: clean. Con: huge commit, hard to bisect, all tests break at once and must be fixed in the same pass.

### Option B — Schema-first → runtime → call sites → deletions (chosen)

Six implementation units in dependency order. Each unit is independently testable, leaves the codebase in a buildable state, and produces a focused commit. Slightly more ceremony per step but the test gate after each step catches regressions early.

### Option C — Output abstraction first (defer to Phase 41)

Build a unified `Output` layer (Phase 41 work) before the demolition so all CLI commands route through it from day one. Cleaner end state but blurs the Phase 40/41 boundary; pulls Phase 41 effort earlier than the roadmap's plan.

**Choice: Option B.** Each unit is small enough for a single Sonnet implementation agent. The chosen test gate after each unit (`bun test packages/...`) catches schema/policy regressions immediately. The output abstraction work stays in Phase 41 where it belongs — this phase only consolidates output through the existing `format.ts` helpers; if those helpers prove inadequate, Phase 41 will replace them anyway.

## Trickiest Unit — Designed First

**Unit 2: composePolicy collapse** is the highest-risk because it sits at the
intersection of every CLI command's flag handling, the security policy, and
the legacy-config back-compat story. Get this wrong and everything downstream
either loses a check or surprises a user.

The collapsed policy must:
- Read flat `[security]` (no per-mode lookup)
- Drop `agentMode` from `EffectivePolicy`
- Drop `agent` from `CliFlags`
- Preserve `composePolicyForSource` semantics for the `[[security.overrides]]` preset path
- Preserve all behavior that was identical between agent-mode and human-mode (scope resolution, scan mode, on_warn handling)

Designed in Unit 2 below.

## Implementation Units

### Unit 1 — Flat `SecurityConfigSchema` and updated `ConfigSchema`

**File**: `packages/core/src/schemas/config.ts`

```typescript
// Delete:
//   export const SecurityModeSchema = ...
//   export const AgentModeSchema = ...
//   AGENT_MODE_SCOPES const

// New flat shape — fields previously nested under [security.human|agent]
// are promoted to top-level [security].
export const SecurityConfigSchema = z.object({
  scan: z.enum(SCAN_MODES).default("static"),
  on_warn: z.enum(ON_WARN_MODES).default("prompt"),
  require_scan: z.boolean().default(false),
  agent_cli: z.string().default(""),
  threshold: z.number().int().min(0).max(10).default(5),
  max_size: z.number().int().default(51200),
  ollama_model: z.string().default(""),
  overrides: z.array(TrustOverrideSchema).default([]),
});

// ConfigSchema: drop "agent-mode" key entirely.
export const ConfigSchema = z.object({
  defaults: DefaultsSchema.prefault({}),
  security: SecurityConfigSchema.prefault({}),
  // (no "agent-mode" key)
  registry: RegistryConfigSchema.prefault({}),
  builtin_tap: z.boolean().default(true),
  verbose: z.boolean().default(true),
  taps: z.array(TapEntrySchema).default([]),
  updates: UpdatesConfigSchema.prefault({}),
  telemetry: TelemetryConfigSchema.prefault({}),
  default_git_host: z.string().default("https://github.com"),
});

export type Config = z.infer<typeof ConfigSchema>;
export type SecurityConfig = z.infer<typeof SecurityConfigSchema>;
// Delete: SecurityMode, AgentMode types.
```

Zod's default object-mode is "strip extras." A v1 config file with `[security.human]`, `[security.agent]`, `[agent-mode]` parses without error — those keys are silently dropped on load. The `migrate` command (Unit 6) is responsible for translating them into the new flat shape before they're lost.

**Implementation Notes**:
- `PRESET_VALUES` + `SECURITY_PRESETS` stay unchanged (still maps to `{ scan, on_warn, require_scan }`).
- `TrustOverrideSchema` stays unchanged.
- `SCAN_MODES`, `ON_WARN_MODES` arrays stay unchanged.
- `DefaultsSchema` keeps `also`, `yes`, `scope`. No changes.

**Acceptance Criteria**:
- [ ] `SecurityModeSchema` and `AgentModeSchema` no longer exported.
- [ ] `Config` type's `security` field is `SecurityConfig` (flat).
- [ ] `Config` type has no `"agent-mode"` key.
- [ ] `ConfigSchema.parse(<v1 config with deprecated keys>)` succeeds (extras stripped).
- [ ] All `core/src/schemas/config.test.ts` cases for the new shape pass; old per-mode parsing cases deleted.
- [ ] `bun test packages/core/src/schemas/` passes.

---

### Unit 2 — Collapsed `composePolicy`

**File**: `packages/core/src/policy.ts`

```typescript
// Delete imports:
//   import { isAgentEnv } from "./agent-env";
// Delete buildAgentScope().
// Delete agentMode field from EffectivePolicy.
// Delete agent field from CliFlags.

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
};

export function composePolicy(
  config: Config,
  flags: CliFlags,
): Result<EffectivePolicy, UserError> {
  const sec = config.security;

  if (flags.skipScan && sec.require_scan) {
    return err(new UserError(
      "Security scanning is required by config (security.require_scan = true). Cannot use --skip-scan.",
    ));
  }

  let onWarn: "prompt" | "fail" | "allow";
  if (flags.strict) onWarn = "fail";
  else if (flags.noStrict) onWarn = "prompt";
  else onWarn = sec.on_warn;

  const scope = buildScope(
    flags,
    config.defaults.scope as "global" | "project" | "",
  );

  const scanMode =
    flags.semantic && sec.scan !== "semantic" ? "semantic" : sec.scan;

  return ok({
    yes: flags.yes || config.defaults.yes,
    onWarn,
    requireScan: sec.require_scan,
    skipScan: flags.skipScan ?? false,
    scanMode,
    scope,
    also: config.defaults.also,
  });
}

export function composePolicyForSource(
  config: Config,
  flags: CliFlags,
  source: { tapName?: string; sourceType: "tap" | "git" | "npm" | "local" },
): Result<EffectivePolicy, UserError> {
  const preset = resolveOverride(config.security.overrides, source);
  if (preset === null) return composePolicy(config, flags);

  const presetValues = PRESET_VALUES[preset];
  const patchedConfig: Config = {
    ...config,
    security: {
      ...config.security,
      scan: presetValues.scan,
      on_warn: presetValues.on_warn,
      require_scan: presetValues.require_scan,
    },
  };
  return composePolicy(patchedConfig, flags);
}
```

`buildScope`, `resolveOverride`, `mapAdapterToSourceType` keep their existing
implementations.

**Implementation Notes**:
- The "human-mode" branch is now the only branch — replaces both old branches.
- `flags.yes || config.defaults.yes` is the only `yes` resolution (old agent-mode branch hardcoded `yes: true`; the new pattern is "use `--yes` explicitly when invoking unattended").
- Unattended-use behavioral difference: agent-mode used to **auto-accept warnings** unless `on_warn = "fail"`. Now: warnings prompt (`on_warn = "prompt"` default) unless `--yes` is passed or config sets `on_warn = "allow"`. For CI/scripted use, the recipe is `skilltap install --yes ...` or set `[security] on_warn = "allow"`. This is documented in the migration guide (Unit 6).

**Acceptance Criteria**:
- [ ] `EffectivePolicy.agentMode` no longer exists.
- [ ] `CliFlags.agent` no longer exists.
- [ ] `composePolicy(config, { agent: true })` is a TS error.
- [ ] `composePolicy()` reads `config.security.scan` (not `config.security.human.scan`).
- [ ] `composePolicyForSource()` patches `config.security.*` directly (not `config.security.human.*` / `config.security.agent.*`).
- [ ] Tests in `policy.test.ts` rewritten: agent-mode-specific tests deleted; remaining tests verify flat-policy resolution. Aim for ~40 tests (down from ~73).
- [ ] `bun test packages/core/src/policy.test.ts` passes.

---

### Unit 3 — Drop `--agent` flag and `isAgentMode()` callers

**Files**:
- `packages/cli/src/commands/install.ts` — remove `agent` arg, collapse `runAgentMode` + `runInteractiveMode` into `runInstall`.
- `packages/cli/src/commands/update.ts` — same shape: collapse `runAgentModeUpdate` + `runInteractiveUpdate` into `runUpdate`.
- `packages/cli/src/commands/skills/remove.ts` — remove `agent` arg, drop `await isAgentMode()`, replace `exitWithError(agentMode, ...)` with `errorLine + exit`.
- `packages/cli/src/commands/skills/toggle.ts` — same.
- `packages/cli/src/commands/tap/install.ts` — same.
- `packages/cli/src/commands/disable.ts` — drop `await isAgentMode()`, rewrite `exitWithError`.
- `packages/cli/src/commands/enable.ts` — same.
- `packages/cli/src/commands/toggle.ts` — same.
- `packages/cli/src/commands/verify.ts` — same.
- `packages/cli/src/commands/tap/info.ts` — same.
- `packages/cli/src/commands/skills/info.ts` — same.
- `packages/cli/src/commands/tap/list.ts` — same.
- `packages/cli/src/commands/plugin/index.ts` — drop `isAgentMode` import.
- `packages/cli/src/commands/plugin/remove.ts`, `plugin/info.ts`, `plugin/toggle.ts` — same.
- `packages/cli/src/commands/skills/link.ts`, `skills/adopt.ts`, `skills/move.ts` — same.
- `packages/cli/src/ui/policy.ts` — `loadPolicyOrExit()` no longer takes/forwards `agent`. `isAgentMode` helper deleted.
- `packages/cli/src/ui/resolve.ts` — drop `agentError` import; use `errorLine`.

**Replacement patterns**:

```typescript
// BEFORE
import { exitWithError } from "../ui/agent-out";
import { isAgentMode } from "../ui/policy";
// ...
const agentMode = await isAgentMode();
exitWithError(agentMode, "thing failed", "try X");

// AFTER
import { errorLine } from "../ui/format";
// ...
errorLine("thing failed", "try X");
process.exit(1);
```

```typescript
// BEFORE — install.ts dispatcher
if (policy.agentMode) {
  return runAgentMode(sources, args, config, policy);
}
return runInteractiveMode(sources, args, config, policy, verbose);

// AFTER
return runInstall(sources, args, config, policy, verbose);
```

The unified `runInstall` takes the existing `runInteractiveMode` body. The
`runAgentMode` body is deleted entirely; behaviors the user relied on
(auto-accept warnings, stop on first error) are still reachable via
`--yes`, `--strict`, and config (`[security] on_warn = "fail" | "allow"`).

**Implementation Notes**:
- `loadPolicyOrExit` in `cli/src/ui/policy.ts`: drop the `agent` flag forwarding. The function still loads config and composes policy — just doesn't read `--agent`.
- The `args.agent` field in `install.ts` / `update.ts` / `remove.ts` / `tap/install.ts` is removed from the citty `args:` block AND the destructuring of `args`.
- Where `composePolicy` was called via `loadPolicyOrExit({...flags, agent: args.agent})`, drop the `agent` key.
- Where command code branched on `policy.agentMode` for output formatting, remove the branch (always use the human-style output).
- `agentSuccess(name, path, ref, trust)` callers → `successLine(\`Installed ${name} → ${path}${ref ? \` (${ref})\` : ""}${trust ? \` [${agentTrustLabel(trust)}]\` : ""}\`)`.
- `agentUpdated(name, fromRef, toRef, trust)` callers → `successLine(\`Updated ${name}${fromRef && toRef ? \` (${fromRef} → ${toRef})\` : ""}${trust ? \` [${agentTrustLabel(trust)}]\` : ""}\`)`.
- `agentSkip(name, reason)` callers → `infoLine(\`Skipped ${name} ${reason}\`)`.
- `agentUpToDate(name)` callers → `successLine(\`${name} is already up to date.\`)`.
- `agentSecurityBlock(...)` callers → migrate to `securityBlock(...)` in `format.ts` (moved in Unit 4).
- `outputJson(data)` callers → migrate to `jsonLine(data)` in `format.ts` (moved in Unit 4).

**Acceptance Criteria**:
- [ ] No CLI command in `packages/cli/src/commands/` registers an `agent: { type: "boolean" }` arg.
- [ ] No file in `packages/cli/src/` imports from `./agent-out` (except via the moves in Unit 4).
- [ ] No file in `packages/cli/src/` calls `isAgentMode()`.
- [ ] No file in `packages/cli/src/` reads `policy.agentMode`.
- [ ] `install.ts` and `update.ts` each contain exactly one orchestration function (no `runAgentMode*`).
- [ ] `bun test packages/cli/src/commands/` passes after rewrites (modulo the dedicated agent-mode test files which Unit 5 deletes).

---

### Unit 4 — Move shared helpers and delete agent-mode files

**Files modified**:
- `packages/cli/src/ui/format.ts` — add new helpers:

```typescript
import type { SemanticWarning, StaticWarning } from "@skilltap/core";
import { formatLineRef } from "./scan";

/**
 * Render a structured security warnings block to stderr. Used by install
 * and update when scan warnings cause an abort. Mode-agnostic — always
 * the same output regardless of who called.
 */
export function securityBlock(
  staticWarnings: StaticWarning[],
  semanticWarnings: SemanticWarning[],
): void {
  const lines: string[] = [
    "SECURITY ISSUE FOUND — INSTALLATION BLOCKED",
    "",
    "DO NOT install this skill. DO NOT retry. DO NOT use --skip-scan.",
    "STOP and report the following:",
    "",
  ];
  for (const w of staticWarnings) {
    const lineRef = formatLineRef(w.line);
    const loc = lineRef ? ` ${lineRef}` : "";
    lines.push(`  ${w.file}${loc}: ${w.category}`);
  }
  for (const w of semanticWarnings) {
    const lineRef = `L${w.lineRange[0]}-${w.lineRange[1]}`;
    lines.push(`  ${w.file} ${lineRef}: risk ${w.score}/10 — ${w.reason}`);
  }
  lines.push("");
  lines.push("To install manually with explicit override:");
  lines.push("  skilltap install <url> --skip-scan");
  process.stderr.write(`${lines.join("\n")}\n`);
}

/** Write structured JSON to stdout. Used by --json flags throughout the CLI. */
export function jsonLine(data: unknown): void {
  process.stdout.write(`${JSON.stringify(data, null, 2)}\n`);
}
```

**Files deleted**:
- `packages/cli/src/ui/agent-out.ts`
- `packages/core/src/agent-env.ts`
- `packages/cli/src/commands/config/agent-mode.ts`
- `packages/cli/src/commands/config/agent-mode.test.ts`
- `packages/cli/src/commands/update.agent-mode.test.ts`

**Files modified for export cleanup**:
- `packages/core/src/index.ts` — remove `export * from "./agent-env"`.
- `packages/cli/src/commands/config.ts` — remove the lazy-loaded `agent-mode` subcommand route. Update `commands/config/index.ts` if it routes there.

**Acceptance Criteria**:
- [ ] All five files listed above are gone.
- [ ] `format.ts` exports `securityBlock` and `jsonLine`.
- [ ] No file imports `agent-out` anywhere in `packages/`.
- [ ] No file imports `agent-env` anywhere in `packages/`.
- [ ] `skilltap config agent-mode` exits with "Unknown command" (citty default behavior for unregistered subcommand).
- [ ] `bun run build` succeeds.

---

### Unit 5 — Test rewrites

**Files modified**:

- `packages/core/src/policy.test.ts` — delete agent-mode-specific tests (~40 of 73). Keep tests covering: `buildScope`, `resolveOverride`, `mapAdapterToSourceType`, scope resolution from flags vs config, scan-mode resolution from flags vs config, `on_warn` resolution from flags (`--strict`, `--no-strict`) vs config, `composePolicyForSource` preset application, `--skip-scan` + `require_scan` error path. Rewrite per-mode test cases as flat-policy cases.
- `packages/cli/src/ui/policy.test.ts` — delete `isAgentMode` tests; keep `loadPolicyOrExit` happy-path coverage.
- `packages/cli/src/commands/install.test.ts` — find tests that pass `--agent`; remove the flag, add equivalent tests with `--yes` + piped stdin.
- `packages/cli/src/commands/update.test.ts` — same.
- `packages/cli/src/commands/install.capture.test.ts` — review: this file already uses `--agent` to drive non-interactive install of plugins. After Phase 40 these tests need to pipe stdin OR pass `--yes`. The capture flow itself (same-source auto-confirm, cross-source hard-abort) stays exactly the same — it never depended on agent-mode, only on the absence of `onCaptureConflict` callback (which CLI installs via clack only when stdout is TTY post-Phase 40, see Implementation Notes).
- Any test file under `packages/cli/src/commands/skills/`, `tap/`, `plugin/` that currently passes `--agent` — drop the flag.

**Files deleted**:
- `packages/cli/src/commands/config/agent-mode.test.ts` (deleted in Unit 4 alongside the source).
- `packages/cli/src/commands/update.agent-mode.test.ts` — its 16 test cases either fold into `update.test.ts` as plain pipe-mode tests or are deleted if they only verified agent-mode-specific output.

**Test-rewrite mapping for capture flow** (since it intersects with our recent Phase 39 work):
- Existing test "agent-mode + cross-source conflict exits 1" → new test "non-TTY stdin + cross-source conflict (no `onCaptureConflict` callback) exits 1". This works because: post-Phase 40, `runInstall` (the unified orchestrator) wires `onPluginCaptureConflict` only when stdout is TTY (or always — implementation choice). When piped, the missing callback causes `installPlugin` to default-abort cross-source conflicts, matching old agent-mode behavior.

**Implementation Notes**:
- The aim is to get the suite green again, not to add coverage. Net test count should drop noticeably (estimate: ~80 fewer tests in this phase).
- Use `bun test packages/core/src/ -t "agent"` to find any remaining agent-related test names that need attention.
- A few tests will need explicit `process.stdout.isTTY = false` setup if they assert non-TTY behavior; use `runSkilltap` (which already pipes) over `runInteractive` for those.

**Acceptance Criteria**:
- [ ] `bun test` reports zero failures.
- [ ] No test file imports `agent-out` or `agent-env`.
- [ ] `grep -rn "SKILLTAP_AGENT" packages/` returns at most a single reference inside `migrate/` translating old configs.
- [ ] Net test count change documented in PROGRESS.md (subtract the deletions, add any new TTY-driven tests).

---

### Unit 6 — Migrate command + state-fallback deletion

**Files modified**:

- `packages/core/src/migrate/run.ts` (or wherever migrate's translation logic lives) — extend the config translator:
  - `[security.human].*` + `[security.agent].*` → `[security].*` — pick the **stricter** of the two: `on_warn = "fail" | "prompt" | "allow"` ordered strictest-first; `scan = "semantic" | "static" | "off"` ordered strictest-first; `require_scan = a || b`. Warn to stderr if mismatch ("Note: human and agent security settings differed; using stricter values").
  - `[agent-mode].enabled = true` → warn that the block is removed and the new pattern is `--yes` + piped stdin or `[security] on_warn = "allow"`.
  - `[agent-mode].scope` → if non-empty, write to `defaults.scope`.
  - `[[security.overrides]]` — preserved unchanged.

- `packages/core/src/config.ts` — `loadInstalled()`:
  - Drop the v0.x `installed.json` read fallback.
  - Always read `state.json` directly. Empty state for an unmigrated user; `migrate` is the user-facing recovery path.

- `packages/core/src/plugin/state.ts` — `loadPlugins()`:
  - Same: drop v0.x `plugins.json` fallback. Read `state.json` only.

- `packages/core/src/schemas/installed.ts` — keep `InstalledSkillSchema` (still used by `state.skills[]`). Move `InstalledJsonSchema` and `InstalledJson` type into `core/src/schemas/v1/installed.ts` (or similar) so it's clearly migrate-only. Update migrate's import.

- `packages/core/src/schemas/plugins.ts` — same: keep `PluginRecordSchema`. Move `PluginsJsonSchema` and `PluginsJson` to `core/src/schemas/v1/plugins.ts`.

- Existing migrate marker detection (`detectMarkers` or similar) — already detects v0.x state files. Verify it still works after the schema move.

**Implementation Notes**:
- The `loadInstalled` / `loadPlugins` simplification removes the dynamic-import dance from `state/save.ts` ↔ `config.ts`. Result: fewer module-load-time surprises, simpler stack traces.
- After this unit, `state.json` is the **only** runtime store. The migrate command is the **only** code path that reads `installed.json` / `plugins.json`. Users who haven't run migrate will see empty state until they do.
- The migrate translator's "stricter wins" rule is an opinionated choice; alternative would be to prompt. For autopilot's autonomy mandate, the stricter default is documented and consistent.

**Acceptance Criteria**:
- [ ] `loadInstalled()` no longer reads `installed.json`. Verified by grep.
- [ ] `loadPlugins()` no longer reads `plugins.json`. Verified by grep.
- [ ] `migrate` translates `[security.human|agent]` to flat `[security]`.
- [ ] `migrate` warns on `[agent-mode]` removal.
- [ ] An unmigrated v0.x setup reports zero installed skills until migrate runs (instead of silently fallback-reading them).
- [ ] `migrate` test suite (in `packages/core/src/migrate/run.test.ts`) covers all four translation rules.
- [ ] `bun test packages/core/src/migrate/` passes.
- [ ] `bun test packages/core/src/config.test.ts` passes.

---

## Pre-Mortem

**Riskiest assumption**: That every `agent-out.ts` call site can be cleanly rewritten with `format.ts` helpers without losing user-facing behavior. Some commands today emit `OK:` / `SKIP:` / `ERROR:` prefixes that scripts may parse.

**What would have to be true to fail in production**:
- A user has a script that greps for `^OK: Installed` or `^SKIP:` from skilltap's output. These prefixes disappear in the rewrite (replaced by `successLine` / `infoLine` which write plain "Installed X → Y" with a green checkmark prefix). The user's script breaks silently.
- A teammate's CI pipeline relies on `--agent` flag for non-interactive installs. The flag stops being recognized; their pipeline fails with "unknown flag."

**Mitigation**:
- Phase 40's user-facing changelog (in PROGRESS.md and a CHANGELOG entry deferred to Phase 46) explicitly calls out the prefix change and the flag removal.
- The migrate command emits a clear summary of changes when run.
- For the `--agent` flag specifically: citty by default rejects unknown flags. We could opt to soft-deprecate it for one release (still parses, prints warning, treated as no-op). **Decision: hard-remove.** Per user's "no users yet" framing, soft-deprecation is overhead without payoff.

**Fallback if the riskiest unit doesn't work**:
- If Unit 3 (call-site rewrites) reveals semantically-distinct agent-mode behavior we hadn't accounted for, the fallback is to keep `agent-out.ts`'s helpers but rename them mode-agnostic (`successLine`, `errorLine`, `skipLine`, `securityBlock`) and delete the `agentMode` parameter from `exitWithError`. The 22 importers still get rewritten; the file just doesn't get deleted. We can revisit deletion in a follow-up.

**Where I'm least sure**:
- Unit 5 (test rewrites). The agent-mode test files (`update.agent-mode.test.ts`, `policy.test.ts`'s agent branches) carry implicit assumptions about CLI output format that cross test files. Plan: spawn the implementation agent for Unit 5 with a dedicated "run the suite, fix the failures, report what didn't translate" mandate.

## Implementation Order

1. **Unit 1** — Schema (atomic; no consumer breakage at this point because the human/agent fields are still readable as extras).
2. **Unit 2** — Policy (consumes the new flat schema). After this, EffectivePolicy has no `agentMode`; downstream code stops compiling. Land Units 3 and 4 in the same commit window so the tree stays buildable.
3. **Unit 3** — CLI command rewrites. ~22 files. Spawn one agent per command-group: install + update (fork collapse), config + tap (subcommand cleanup), skills + plugin + verify + others.
4. **Unit 4** — Move helpers + delete files.
5. **Unit 5** — Test rewrites. Run the full suite; fix per-file failures.
6. **Unit 6** — Migrate command + state-fallback deletion.

Units 2, 3, 4 must land together (or close together with `bun test` at the end) because each uncouples downstream consumers. Unit 6 is independent of 2–5 and can land first or last.

## Testing

The per-unit acceptance criteria above name the test files affected. Cross-unit test gates:

- After Unit 1: `bun test packages/core/src/schemas/` (Zod schema correctness).
- After Unit 2: `bun test packages/core/src/policy.test.ts` (rewritten policy tests).
- After Unit 3: `bun test packages/cli/src/commands/` (subprocess tests for the rewritten CLI surface).
- After Unit 4: `bun run build` (production build).
- After Unit 5: `bun test` (full suite).
- After Unit 6: `bun test packages/core/src/migrate/` and a manual smoke: `skilltap migrate` on a fixture v0.x config-and-state directory.

**New tests to add** (small, focused):
- `policy.test.ts` — at least one test verifying that omitted CLI flags use config defaults across all four resolution paths (scope, scan, on_warn, yes).
- `migrate/run.test.ts` — at least three new translation tests:
  1. `[security.human]` strict + `[security.agent]` relaxed → flat picks strict.
  2. `[agent-mode] enabled = true` → warning emitted, block dropped.
  3. `[agent-mode].scope = "global"` + `defaults.scope = ""` → `defaults.scope` set to "global".

## Verification Checklist

```bash
# 1. Source files have no agent-mode references
grep -r "agentMode\|agent-mode\|SKILLTAP_AGENT\|agent-out\|agent-env\|isAgentEnv\|isAgentMode" packages/ --include="*.ts" | grep -v ".test.ts" | grep -v "src/migrate"

# 2. Build succeeds
bun run build

# 3. Full test suite
bun test

# 4. CLI help no longer lists --agent
bun run dev install --help | grep -i agent

# 5. Old config paths produce a clear error
bun run dev config set agent-mode.enabled true

# 6. Migrate translates v0.x → v3 cleanly
# (manual smoke against a fixture directory)
```

## Risks (post-pre-mortem)

| Risk | Likelihood | Mitigation |
|---|---|---|
| Lost behavior: agent-mode auto-accept-warnings without `--yes` | Low | Documented in migrate output and Phase 40's PROGRESS entry; users opt into `[security] on_warn = "allow"` or pass `--yes`. |
| Lost behavior: `OK:` / `SKIP:` prefix scripts | Low (no users yet) | Documented in changelog; users adapt scripts. |
| `migrate` command edge cases: malformed v0.x configs | Medium | Existing migrate tests cover happy paths; add three new translation tests (above) for the v3-specific work. |
| Test count drops by ~80 — coverage regression? | Low | The deleted tests asserted **agent-mode-specific** behavior that no longer exists. Coverage of underlying logic (policy resolution, scope, scan modes) is preserved by the rewritten tests in `policy.test.ts`. |
| `loadInstalled` no longer falls back — silent skills disappearance for unmigrated users | Low | An unmigrated user now sees zero installed skills until they run `migrate`. The startup banner (already in place) directs them to run it. Acceptable tradeoff for the cleanup. |
