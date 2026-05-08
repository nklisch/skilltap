# Design: Plugin Capture (Take Ownership of Already-Installed Skills/MCPs)

## Overview

When `skilltap install` lands a plugin, it may declare components — skills or MCP servers — that the user has already installed standalone. Today the plugin install would silently overwrite the on-disk skill directory and namespace its own MCP keys alongside the standalone ones, leaving:

- A duplicate record in `state.json` (`skills[]` *and* `plugins[].components[]` both naming the same skill).
- An orphaned `skilltap.toml` `[skills]` entry that `skilltap sync` will report as drift forever.
- Two MCP entries in agent configs (the bare standalone key and the `skilltap:<plugin>:<server>` key) that may behave non-deterministically inside the agent.
- Disabled (`.disabled/<name>`) or linked (`scope: "linked"`) standalone copies still pointed at by the old record but no longer the live skill.

This design adds a **capture** step to the plugin install pipeline. After security scan, before placement, capture surfaces the conflicts to the user as a single decision (capture-all-or-abort), then atomically transfers ownership: the plugin record gains the components, the standalone records are dropped from `state.json` + `skilltap.toml` + `skilltap.lock`, and stale MCP keys are pruned from agent configs before the plugin's namespaced keys land.

After capture, `skilltap update <plugin>`, `skilltap remove <plugin>`, `skilltap toggle`, and `skilltap sync` all do the right thing for the captured components automatically — they're plugin components like any other.

### Goals

- **Plugin install becomes idempotent over a project's history.** Reinstalling a plugin where the user previously hand-installed its skills is no longer a corruption hazard.
- **One source of truth per name per scope.** A skill/MCP name in a given scope is either standalone or plugin-owned; never both.
- **Sync stays clean.** No orphan manifest entries after a plugin captures their corresponding standalones.
- **Reversible-ish via the user, not magic.** Capture is permanent: removing the plugin removes the captured component. Rebuilding a standalone is a re-`install` away. We don't auto-restore.

### Non-goals

- Capturing across scopes. A project-scoped plugin will not capture a globally-installed standalone (different install dirs, different state files; merging would surprise the user).
- Capturing agent definitions. There is no standalone equivalent in skilltap state today; nothing to merge.
- Per-component "keep standalone, skip plugin's copy" mode. That creates an on-disk collision (`.agents/skills/<name>/` is a single path) and a half-installed plugin record. Capture is all-or-abort.
- Renaming. If a plugin's component name conflicts with a standalone the user wants to preserve under that name, the user must remove or rename the standalone before installing.

### Key design decisions

| Decision | Choice | Rationale |
|---|---|---|
| Match identifier — skills | Skill name only | Plugin sub-skills carry no source URL (they're paths inside the plugin repo); name is the only stable comparator. |
| Match identifier — MCPs | Server name (parsed from `skilltap:<slug>:<server>` namespaced key) | Standalone MCPs are stored as namespaced keys (`mcp-install.ts:160`); the plugin's component carries a bare server name. We extract `serverName` via `parseNamespacedKey` and compare. |
| Match scope | Same scope as the plugin install only | Cross-scope match would require deciding whether to remove the global record while installing a project plugin; user intent is unclear. |
| Aggregation | Per-plugin-install: one callback presenting all candidates at once | Matches the existing `onPluginConfirm` UX shape; one prompt is cheaper than per-component round-trips. |
| Confirmation default | Capture-all-or-abort | Mixed states are not representable on disk. |
| Agent mode default | Auto-confirm capture | Plugin install in agent mode already auto-confirms scan + plugin selection; capture follows the same precedent. |
| Manifest semantics | Remove the captured standalones from `[skills]` + `skill[]` lock array | The plugin's manifest entry now declaratively covers them. |
| Filesystem semantics | Plugin's bundled content always wins | "Takes over ownership" — same semantics as plugin install today, just made explicit. |
| Disabled standalones | Capturable; we prune `.disabled/<name>` too | Otherwise stale dirs accumulate. |
| Linked standalones | Capturable, but we surface the `linked` flag in the candidate so the user knows they're losing the in-place dev workflow | The `linked` record's `path` outside the install dir is left untouched on disk; only the record + symlinks are removed. |

---

## Implementation Units

### Unit 1: Capture module — match detection

**File**: `packages/core/src/plugin/capture.ts` (new)

```typescript
import type { State } from "../state/schema";
import type { InstalledSkill } from "../schemas/installed";
import type { StoredMcpStandalone } from "../state/schema";
import type { PluginManifest, PluginSkillComponent, PluginMcpComponent } from "../schemas/plugin";

/**
 * A single standalone record that a plugin's component will take ownership of.
 *
 * `kind` distinguishes which state array it lives in today; the surrounding
 * code reads the discriminator before touching the union.
 */
export type CaptureCandidate =
  | {
      kind: "skill";
      /** The plugin component that triggered the match. */
      component: PluginSkillComponent;
      /** The standalone record that will be released. */
      standalone: InstalledSkill;
    }
  | {
      kind: "mcp";
      component: PluginMcpComponent;
      standalone: StoredMcpStandalone;
      /**
       * Server name parsed out of the namespaced standalone key
       * (i.e., `parseNamespacedKey(standalone.name).serverName`).
       * Cached here because callers and the UI need it without re-parsing.
       */
      serverName: string;
    };

export interface CaptureMatches {
  skills: Extract<CaptureCandidate, { kind: "skill" }>[];
  mcpServers: Extract<CaptureCandidate, { kind: "mcp" }>[];
  /** Convenience: skills.length + mcpServers.length. */
  total: number;
}

/**
 * Pure function — finds, for a given plugin manifest in a given scope,
 * every standalone record in `state` whose name (skill) or server-name (mcp)
 * collides with a plugin component.
 *
 * Does NOT mutate state. Filtering by scope is the caller's responsibility
 * — `state` is expected to already be the scope-correct state.json.
 */
export function detectCaptureMatches(
  state: State,
  manifest: PluginManifest,
): CaptureMatches;
```

**Implementation notes**:

- For each `PluginSkillComponent` in `manifest.components`, search `state.skills` for an entry whose `.name === component.name`. There is at most one match per name per scope (state load deduplicates by name + scope already; cf. `discover.ts:130`).
- For each `PluginMcpComponent`, parse every `state.mcpServers[i].name` with `parseNamespacedKey` (`plugin/mcp-inject.ts:55`). Compare the resulting `serverName` to `component.server.name`. A plugin component may match multiple standalones if more than one source installed the same server name under different slugs — record all of them.
- An entry whose namespaced key fails to parse (legacy / malformed) is skipped. The function does not error.

**Acceptance criteria**:
- [ ] Returns empty `skills` and `mcpServers` arrays when the plugin's component names don't appear in `state`.
- [ ] Returns the matched `InstalledSkill` for each plugin skill component whose name appears in `state.skills`.
- [ ] Returns *every* `StoredMcpStandalone` whose parsed `serverName` matches a plugin MCP component's `server.name`, even if multiple standalones match.
- [ ] Skips standalones with unparseable `name` keys without throwing.
- [ ] Pure — does not call I/O, does not import callbacks.

---

### Unit 2: Capture module — atomic apply

**File**: `packages/core/src/plugin/capture.ts` (same file, separate export)

```typescript
import type { Result, UserError } from "../types";
import type { CaptureMatches } from "./capture";

export interface ApplyCaptureOptions {
  scope: "global" | "project";
  projectRoot?: string;
  /** Plugin name — used to namespace replacement MCP keys and target manifest. */
  pluginName: string;
}

export interface ApplyCaptureResult {
  /** Names of skills released from state.skills[]. */
  capturedSkills: string[];
  /** Server names released from state.mcpServers[] (keyed by parsed serverName, not full key). */
  capturedMcpServers: string[];
  /** Agent ids whose MCP config files had standalone keys removed. */
  prunedAgents: string[];
}

/**
 * Atomically transfer ownership of the matched standalones to the plugin:
 *
 * 1. Load current state for the given scope.
 * 2. Remove captured skills from `state.skills[]`.
 * 3. Remove captured MCPs from `state.mcpServers[]`.
 * 4. Save state.
 * 5. Remove the captured skills' agent symlinks (the plugin install will
 *    recreate symlinks for the plugin's `also` set on its own).
 * 6. Delete `.agents/skills/.disabled/<name>` for any captured skill that was
 *    disabled (active=false). Active dirs are left as-is; the plugin install
 *    overwrites them next.
 * 7. Remove standalone MCP keys from each captured MCP's `targets[]` agent
 *    config files (one `removeMcpServers` call per distinct slug).
 * 8. Remove captured skills' entries from `skilltap.toml` `[skills]` + lockfile
 *    (project scope only, no-op without manifest).
 *
 * The plugin record + plugin's MCP injection happen AFTER this function in
 * the install flow — by the time control returns, the standalone slots are
 * empty and the plugin can claim them cleanly.
 *
 * Idempotent: calling with empty matches is a no-op.
 */
export async function applyCapture(
  matches: CaptureMatches,
  options: ApplyCaptureOptions,
): Promise<Result<ApplyCaptureResult, UserError>>;
```

**Implementation notes**:

- **State mutation order matters.** Save state once after both arrays are filtered (not twice). Use `loadState` / `saveState` directly — `saveInstalled` and `savePlugins` would do partial overwrites we don't want.
- **MCP standalone-key removal**: group captured standalones by their parsed `pluginName` (slug). For each distinct slug, call `removeMcpServers({ pluginName: slug, agents: union(targets), scope, projectRoot })`. This reuses the existing prefix-prune logic in `mcp-inject.ts:271`. Collect agent ids into the result.
- **Symlink removal for skills**: for each captured `InstalledSkill`, call `removeAgentSymlinks(skill.name, skill.also, skill.scope === "linked" ? "global" : skill.scope, projectRoot)`. The `linked` scope normalization: linked records still create symlinks under either global or project trees depending on where the `link` command was run; for capture we treat it like the plugin's scope. (Practically: if a `linked` skill is being captured into a project plugin, we pass `"project"`. If global, `"global"`.)
- **Disabled-skill cleanup**: when capturing a skill with `active === false`, also `rm -rf` the `skillDisabledDir(name, scope, projectRoot)` directory. Use `force: true` so a missing dir is fine.
- **Manifest cleanup**: only project scope. For each captured skill whose `repo` is non-null, call `removeSkillFromManifest(projectRoot, skill.repo)`. The helper canonicalizes the source key and is a no-op when no manifest exists. We don't remove standalone MCP entries from the manifest because there's no skill-side manifest entry for `mcp:<source>` installs in v2.1 (see `manifest/update.ts` — only skills + plugins go into `skilltap.toml`).
- **Linked skills' on-disk content** at `skill.path` is intentionally NOT touched. The user's source dir keeps existing; only the skilltap record + symlinks are released.
- **Failure semantics**: if state save fails, return error before any agent-config or manifest mutation — those are best-effort downstream effects. If a downstream effect fails (manifest, MCP key prune, symlink removal), log via `debug` and continue. State is the source of truth and has already been updated; partial cleanup is acceptable for now (and `skilltap doctor` can flag any leftover MCP key drift).

**Acceptance criteria**:
- [ ] Removes every `CaptureCandidate.standalone` (skills) from `state.skills[]`.
- [ ] Removes every `CaptureCandidate.standalone` (mcps) from `state.mcpServers[]`.
- [ ] Saves state exactly once.
- [ ] Removes agent symlinks for each captured skill's `also` list.
- [ ] Deletes `.agents/skills/.disabled/<name>` only when the captured skill had `active === false`.
- [ ] Calls `removeMcpServers` with the parsed plugin slug, not the plugin's `pluginName` — because the existing keys are namespaced under the slug, not the new plugin.
- [ ] On project scope, calls `removeSkillFromManifest` for each captured skill that has a non-null `repo`.
- [ ] On global scope, does not touch any manifest.
- [ ] Empty `matches` returns `ok` immediately (no reads, no writes); `capturedSkills`/`capturedMcpServers`/`prunedAgents` are empty arrays.
- [ ] Returns `prunedAgents` as the union of all agents whose configs were touched.
- [ ] Returns `Result.err` only on state save failure.

---

### Unit 3: Wire capture into `installPlugin`

**File**: `packages/core/src/plugin/install.ts` (modify existing)

Add to `PluginInstallOptions`:

```typescript
/**
 * Called when the plugin's components collide with already-installed
 * standalones in the same scope. Return true to capture (plugin takes
 * ownership), false to abort the entire install.
 *
 * If omitted and matches are non-empty, install proceeds with capture
 * automatically — this matches the existing pattern where a missing
 * `onConfirm` auto-proceeds.
 *
 * The CLI layer is responsible for converting this into the right UX:
 * a single confirmation prompt in interactive mode, auto-true in agent mode.
 */
onCaptureConfirm?: (matches: CaptureMatches) => Promise<boolean>;
```

Add to `PluginInstallResult`:

```typescript
/** Components transferred from standalone state to this plugin. */
captured: {
  skills: string[];
  mcpServers: string[];
};
```

**Insertion point**: between the security-scan block (current line ~99) and skill placement (current line ~101). Pseudocode:

```typescript
// 1.5. Capture detection
const stateForCapture = await loadState(
  scope === "project" ? projectRoot : undefined,
);
if (!stateForCapture.ok) return stateForCapture;
const matches = detectCaptureMatches(stateForCapture.value, manifest);

if (matches.total > 0) {
  if (options.onCaptureConfirm) {
    const proceed = await options.onCaptureConfirm(matches);
    if (!proceed) {
      return err(
        new UserError(
          `Install of plugin "${manifest.name}" cancelled — would capture ${matches.total} standalone component(s).`,
        ),
      );
    }
  }
  const applied = await applyCapture(matches, {
    scope,
    projectRoot,
    pluginName: manifest.name,
  });
  if (!applied.ok) return applied;
  capturedSkills = applied.value.capturedSkills;
  capturedMcpServers = applied.value.capturedMcpServers;
}
```

`capturedSkills` / `capturedMcpServers` are declared at the function top with `[]` defaults, then included in the final `ok({ ..., captured: { skills: capturedSkills, mcpServers: capturedMcpServers } })` return.

**Implementation notes**:
- Do NOT pass `loadInstalled` / `loadPlugins` — capture works against `state.json` directly. This avoids the read-fallback path that's already deprecated and keeps capture's mental model simple.
- Capture happens BEFORE skill placement so that the standalone's `also` symlinks are removed before the plugin's are created. That avoids a window where the plugin's symlink targets and the standalone's overlap.
- Capture happens BEFORE MCP injection so standalone MCP keys are pruned before plugin keys land. Otherwise an agent restarting in that window could see the standalone's command on the bare key alongside the plugin's namespaced key.
- Capture happens AFTER security scan — if the plugin's content fails the scan, we never touch the user's existing setup.

**Acceptance criteria**:
- [ ] No matches → `installPlugin` behaves exactly as today; `result.captured.skills` and `result.captured.mcpServers` are empty arrays.
- [ ] Matches present, no callback → captures automatically and reports captured names in result.
- [ ] Matches present, callback returns `true` → captures and proceeds.
- [ ] Matches present, callback returns `false` → returns `UserError`, does not mutate state, does not place skills, does not inject MCP.
- [ ] After successful capture install, `state.skills[]` has zero entries with the captured names; `state.mcpServers[]` has zero entries with the captured server names.
- [ ] After successful capture install, the plugin record's `components[]` includes the corresponding `StoredSkillComponent` / `StoredMcpComponent` entries (the existing `manifestToRecord` produces these — capture doesn't need to add anything extra).
- [ ] The captured skills' physical directories at `.agents/skills/<name>` contain the plugin's content, not the standalone's, after install.
- [ ] On project scope with a `skilltap.toml` present, captured skills are removed from `[skills]` and `skill[]` lock array.

---

### Unit 4: CLI install command — capture confirmation prompt

**File**: `packages/cli/src/commands/install.ts` (modify existing)

#### Interactive mode

In `runInteractiveMode` (the path that constructs the `installSkill` call), add `onCaptureConfirm` to the option object. The callback:

1. Stops the spinner if running.
2. Renders one summary block listing what will be captured. Skills and MCP servers grouped under headers. Each row shows the standalone's source (`record.repo` or "linked" path) so the user can identify what they'll lose ownership of.
3. Calls `confirm({ message: "Capture these components into <plugin>?", initialValue: true })`.
4. Handles `isCancel` (treat as abort, return `false`).
5. Restarts the spinner.
6. Returns the boolean.

Plain-text rendering is in a new helper `printCaptureSummary(matches: CaptureMatches): void` placed in `cli/src/ui/install-callbacks.ts` (where the other plugin callback rendering lives) — uses `log.info` from clack. Keep formatting consistent with `printWarnings` style:

```
Plugin "dev-toolkit" wants to take ownership of:

  Skills (2):
    • commit-helper       (was: github:nathan/commit-helper, project)
    • code-reviewer       (was: linked at /home/nathan/dev/code-reviewer)

  MCP servers (1):
    • postgres            (was: skilltap:my-stuff:postgres → claude-code, cursor)

These standalones will be removed from skilltap.toml and skilltap.lock.
The plugin's bundled versions will replace them on disk.
```

#### Agent mode

In `runAgentMode`, add `onCaptureConfirm: async () => true`. To match the existing agent-mode auto-accept callbacks (e.g., `onPluginDetected: async () => "plugin"`), do not gate on `--strict`.

`composePolicy.onWarn === "fail"` is for security warnings, not for capture — capture is a structural transfer, not a security finding. We keep capture auto-confirming in agent mode regardless of strict.

#### Output after success

After plugin install, augment `componentSummary(pr)` rendering: when `result.value.captured.skills.length > 0` or `captured.mcpServers.length > 0`, print a one-line `log.info` like:

```
Captured 2 standalone skill(s), 1 MCP server into "dev-toolkit".
```

In agent mode, the plain-text install-summary block gets a `Captured: ...` line under the existing `Plugin: ...` line.

**Acceptance criteria**:
- [ ] Interactive install with no captures prints no capture text.
- [ ] Interactive install with captures shows the summary block + a confirm prompt.
- [ ] Cancelling the prompt aborts the install with a clear "cancelled" message.
- [ ] Agent-mode install captures without prompting.
- [ ] The post-install summary mentions the capture count when non-zero.
- [ ] Plain-text mode (agent / non-TTY) renders the summary via `process.stdout.write`, not via `log.info`.

---

### Unit 5: Barrel exports

**File**: `packages/core/src/plugin/index.ts`

Add:

```typescript
export {
  detectCaptureMatches,
  applyCapture,
  type CaptureCandidate,
  type CaptureMatches,
  type ApplyCaptureOptions,
  type ApplyCaptureResult,
} from "./capture";
```

These are also used by tests directly.

---

### Unit 6: Sync command — pass capture-confirm callback through

**File**: `packages/core/src/sync/apply.ts` (modify existing)

`applyAddSkill` is the single call site that delegates to `installSkill` for both skill and plugin add operations (`apply.ts:103`). Add `onCaptureConfirm: async () => true` to the option object next to the other auto-accept callbacks. Sync is a non-interactive reconciliation; capturing during sync is the same intent as capturing during agent-mode install.

**Acceptance criteria**:
- [ ] `skilltap sync --apply` of a plugin add with capture-eligible standalones in state succeeds without prompting and records the capture.
- [ ] After sync, the captured standalones are gone from state.

---

### Unit 7: Doctor — defensive check

**File**: `packages/core/src/doctor/checks/installed.ts` (modify existing) or add new check `packages/core/src/doctor/checks/plugin-capture-collision.ts`.

Add a check that flags any name appearing both as a `state.skills[].name` and as a `state.plugins[].components[]` entry of `type: "skill"` (in the same scope). After this design, that situation should be impossible during normal operation — this is the canary.

```typescript
export async function checkPluginCaptureCollisions(
  projectRoot?: string,
): Promise<DoctorCheckResult>;
```

Same shape as the surrounding doctor checks. The fix hint:

> "A skill name appears in both standalone state and a plugin's components. Run `skilltap remove <skill>` (the standalone) or `skilltap remove <plugin>` (the plugin) to resolve."

**Acceptance criteria**:
- [ ] Returns `ok` for a clean state.
- [ ] Returns `warning` with the colliding name(s) when the same name is in both `state.skills[]` and `state.plugins[].components[]` (skill).
- [ ] Wired into the `doctor` command's check list.

---

## Implementation Order

1. **Unit 1** — `detectCaptureMatches` (pure function, no I/O dependencies, easiest to test).
2. **Unit 2** — `applyCapture` (depends only on existing state I/O; covered by integration tests with temp dirs).
3. **Unit 5** — Barrel export so tests can import.
4. **Unit 3** — Wire into `installPlugin`. After this lands, capture is functionally complete in core.
5. **Unit 6** — Pass `onCaptureConfirm: async () => true` in `applySync`.
6. **Unit 4** — CLI install command UX (interactive prompt + agent-mode auto-confirm + summary line).
7. **Unit 7** — Doctor canary check.

Units 1-3 unblock end-to-end testing through `installSkill` → `installPlugin` with a callback that always returns `true` (no UI involvement). The CLI changes (Unit 4) sit on top.

---

## Testing

### Unit Tests

#### `packages/core/src/plugin/capture.test.ts` (new — covers Units 1 & 2)

Pure-function tests for `detectCaptureMatches`:

```
describe("detectCaptureMatches")
  - returns empty matches when state has no skills or mcpServers
  - returns empty matches when no plugin component name overlaps state
  - matches a single skill by name within scope
  - matches multiple skills in one manifest
  - matches an MCP standalone whose parsed serverName equals the plugin component's server.name
  - matches multiple MCP standalones from different slugs sharing a server name
  - skips state.mcpServers entries with unparseable namespaced keys
  - ignores plugin agent components (no standalone analog)
  - is pure: returns same result on repeated calls with same inputs
```

Integration-style tests for `applyCapture` (uses `createTestEnv` from `@skilltap/test-utils`):

```
describe("applyCapture")
  - removes skill records from state.skills[]
  - removes mcp records from state.mcpServers[]
  - saves state exactly once (verify by hooking saveState or counting writes)
  - removes agent symlinks for captured skills' also lists
  - deletes .agents/skills/.disabled/<name> only for captured skills with active=false
  - leaves linked skill's source path on disk untouched
  - groups MCP key removal by parsed slug (one removeMcpServers call per slug)
  - removes manifest [skills] entries on project scope when skilltap.toml exists
  - is no-op without skilltap.toml on project scope
  - does not touch any manifest on global scope
  - empty matches: returns ok immediately, no I/O side-effects
  - propagates state save errors as Result.err
```

#### `packages/core/src/plugin/install.test.ts` (extend existing)

```
describe("installPlugin with capture")
  - no overlap: result.captured arrays are empty, behavior unchanged
  - overlap, no callback: auto-captures and reports captured names
  - overlap, callback returns true: captures, install completes
  - overlap, callback returns false: returns UserError, state.skills retained, no plugin record written
  - after capture install, plugin's components include the captured names as StoredSkillComponent
  - after capture install, .agents/skills/<name>/ contains the plugin's content
  - capture occurs before security scan failure (test: scan fails after callback returns true → state mutations should NOT have happened)
    [Implementation guarantees: capture only runs after scan passes, so this test verifies that order]
  - captured MCP standalone's targets agents have the bare/standalone-namespaced keys removed before plugin keys are injected
```

### Integration / E2E Tests

#### `packages/core/src/plugin/install-integration.test.ts` (extend existing)

Use existing fixtures `createClaudePluginRepo` / `createCodexPluginRepo`. Pre-seed state with standalone skills/MCPs whose names match the plugin's components, then drive `installSkill` end-to-end:

```
describe("installSkill plugin path with capture")
  - pre-installed standalone skill with same name as plugin component → captured
  - pre-installed standalone MCP (via installMcpOnly) → captured under plugin namespace
  - pre-installed standalone skill that's currently disabled → captured + .disabled dir cleaned up
  - pre-installed standalone skill in linked mode → captured, linked path untouched
  - pre-installed standalone in skilltap.toml → manifest [skills] entry removed, plugin entry added
  - cancellation via onCaptureConfirm=false leaves all original state intact
```

#### `packages/cli/src/commands/install.test.ts` (or new `install.capture.test.ts`)

Subprocess tests via `runSkilltap` (pipe) and `runInteractive` (PTY):

```
- Agent mode (--agent) with capture: completes silently, exits 0, summary mentions Captured count
- Interactive (PTY) with capture: shows the summary block, prompt confirms, install succeeds
- Interactive (PTY) with capture: cancelling prompt aborts with "Install ... cancelled" message and non-zero exit
- Plain pipe install with capture: no spinner artifacts, capture summary lines present
```

#### `packages/core/src/sync/apply.test.ts` (extend existing)

```
- sync apply of a plugin add when state has a matching standalone → captures, applied count = 1
```

### Doctor Test

#### `packages/core/src/doctor/checks/installed.test.ts` (or new file for the new check)

```
- clean state: no warning
- skill name in both state.skills[] and state.plugins[].components[] (skill) → warning surfaces both names
```

---

## Verification Checklist

```bash
# Unit + integration suites
bun test packages/core/src/plugin/capture.test.ts
bun test packages/core/src/plugin/install.test.ts
bun test packages/core/src/plugin/install-integration.test.ts
bun test packages/core/src/sync/apply.test.ts
bun test packages/core/src/doctor

# CLI subprocess tests
bun test packages/cli/src/commands/install.test.ts

# Full suite — no regressions
bun test
```

Manual smoke test (project scope with manifest):

```bash
# Setup
cd /tmp && mkdir capture-demo && cd capture-demo && git init
echo '[targets]\nagents = ["claude-code"]' > skilltap.toml

# Pre-install a standalone that the plugin will overlap with
skilltap install github:user/commit-helper --project

# Install a plugin that bundles commit-helper
skilltap install github:user/dev-toolkit --project

# Verify
cat .agents/state.json     # commit-helper appears under plugins[0].components, not skills
cat skilltap.toml          # commit-helper entry gone from [skills], dev-toolkit in [plugins]
skilltap doctor            # no collision warning
skilltap sync              # in sync — no drift
skilltap remove dev-toolkit
skilltap doctor            # commit-helper not auto-restored; clean state
```

---

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| User loses custom edits to a captured skill's local files | The plugin install always overwrites `.agents/skills/<name>` today; capture doesn't change that. The capture summary makes the loss explicit before confirmation. |
| Standalone MCP server has a config the plugin's version doesn't replicate (e.g., env tokens) | Capture's confirmation surfaces source. User who relies on env vars will recognize the conflict and abort. Future enhancement: show a diff of stored configs. Out of scope for v1. |
| Partial failure: state saved, MCP key prune fails | State is the source of truth; doctor will surface dangling agent-config keys. Acceptable for v1. |
| Plugin install with capture during `skilltap sync --apply --strict` | `applyAddSkill` already passes `onWarnings: async () => !options.strict`. Capture is not a warning, so strict doesn't gate it. Confirmed acceptable — the manifest declared the plugin, so capture is the user's pre-stated intent. |
| Cross-scope skill with the same name (global standalone, project plugin install) | Capture only matches within the install's scope. The global standalone is left alone. The project install may overwrite the symlink in `.claude/skills/` if the agent symlinks resolve there — but that's an existing scope-collision issue, not introduced by capture. Doctor's existing checks cover that. |
