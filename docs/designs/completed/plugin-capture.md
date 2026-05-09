# Design: Plugin Capture (Take Ownership of Already-Installed Skills/MCPs)

## Overview

When `skilltap install` lands a plugin, it may declare components — skills or MCP servers — that the user has already installed standalone. Today the plugin install would silently overwrite the on-disk skill directory and namespace its own MCP keys alongside the standalone ones, leaving:

- A duplicate record in `state.json` (`skills[]` *and* `plugins[].components[]` both naming the same skill).
- An orphaned `skilltap.toml` `[skills]` entry that `skilltap sync` will report as drift forever.
- Two MCP entries in agent configs (the bare standalone key and the `skilltap:<plugin>:<server>` key) that may behave non-deterministically inside the agent.
- Disabled (`.disabled/<name>`) or linked (`scope: "linked"`) standalone copies still pointed at by the old record but no longer the live skill.

This design adds a **capture** step to the plugin install pipeline. After security scan, before placement, capture surfaces the conflicts to the user as a single decision (capture-all-or-abort), then atomically transfers ownership: the plugin record gains the components, the standalone records are dropped from `state.json` + `skilltap.toml` + `skilltap.lock`, and stale MCP keys are pruned from agent configs before the plugin's namespaced keys land.

After capture, `skilltap update <plugin>`, `skilltap remove <plugin>`, `skilltap toggle`, and `skilltap sync` all do the right thing for the captured components automatically — they're plugin components like any other.

Capture is **source-aware**. Detection canonicalizes the plugin's repo URL and every matched standalone's repo URL, then partitions the matches into:

- **Same-source** — plugin and standalone resolve to the same canonical source (e.g., the plugin `github:alice/dev-toolkit` matching a previously-installed `github:alice/dev-toolkit` skill, or a standalone `github:alice/commit-helper` matching a plugin from the same repo). The user already trusted this source for this name; capture flows through the normal confirm path.
- **Cross-source** — different canonical sources, or the standalone has no recorded source (linked skill, null repo). This is the silent-substitution risk: a plugin from one author replacing a skill the user installed from a different author under the same name. Cross-source conflicts default to **abort** in agent mode and `skilltap sync --apply`. Interactive mode can offer an explicit force-override path so a user who knows what they're doing isn't blocked.

### Goals

- **Plugin install becomes idempotent over a project's history.** Reinstalling a plugin where the user previously hand-installed its skills is no longer a corruption hazard.
- **One source of truth per name per scope.** A skill/MCP name in a given scope is either standalone or plugin-owned; never both.
- **Defense against silent substitution.** A plugin from `github:bob/...` cannot capture a standalone from `github:alice/...` without explicit human override. Auto-confirm paths (agent mode, sync) refuse cross-source captures and emit an actionable error.
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
| Match identifier — skills | Skill name + canonical-source partition | Plugin sub-skills carry no per-component source URL, but the plugin's `repo` is a stable proxy. Name match is the necessary condition; canonical-source equality between `plugin.repo` and `standalone.repo` decides whether the match is `sameSource` (clean capture) or `crossSource` (conflict). |
| Match identifier — MCPs | Server name + canonical-source partition | Standalone MCPs are stored as `skilltap:<slug>:<server>` namespaced keys (`mcp-install.ts:160`); the plugin's component carries a bare server name. We extract `serverName` via `parseNamespacedKey`, then compare canonical sources of `plugin.repo` and `standalone.source`. |
| Match scope | Same scope as the plugin install only | Cross-scope match would require deciding whether to remove the global record while installing a project plugin; user intent is unclear. |
| Cross-source defaults — agent mode + sync | Abort with actionable error | Auto-confirm paths must not silently swap one author's content for another's. Resolution is documented in the error: remove the standalone, or run interactively. |
| Cross-source defaults — interactive | Prompt with both URLs visible; offer explicit force-override | A human can recognize a legitimate case (e.g., adopting a fork they trust) and force; the default is still abort. |
| Aggregation | Per-plugin-install: one callback for same-source captures, one for cross-source conflicts | Two prompts is the minimum that preserves the safety distinction. |
| Confirmation default — same-source | Capture-all-or-abort | Mixed states are not representable on disk. |
| Agent mode default — same-source | Auto-confirm capture | Plugin install in agent mode already auto-confirms scan + plugin selection; same-source capture follows the same precedent. |
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

/**
 * A flat bucket of capture candidates — used both as the input to
 * `applyCapture` and as the value passed into the confirm/conflict callbacks.
 */
export interface CaptureBucket {
  skills: Extract<CaptureCandidate, { kind: "skill" }>[];
  mcpServers: Extract<CaptureCandidate, { kind: "mcp" }>[];
}

/**
 * The full match set for a plugin install, partitioned by source provenance.
 *
 * `sameSource`: standalone's canonicalized source equals the plugin's
 *   canonicalized repo. Safe to capture under the normal confirm flow.
 *
 * `crossSource`: different canonical sources, or the standalone has no
 *   recorded source (linked skill, null `repo`, null `source`). Treated as
 *   a conflict — default-aborted in auto-confirm modes; force-overridable
 *   in interactive modes.
 */
export interface CaptureMatches {
  sameSource: CaptureBucket;
  crossSource: CaptureBucket;
  /** sameSource.skills.length + sameSource.mcpServers.length. */
  sameSourceTotal: number;
  /** crossSource.skills.length + crossSource.mcpServers.length. */
  crossSourceTotal: number;
  /** sameSourceTotal + crossSourceTotal. */
  total: number;
}

/**
 * Pure function — finds every standalone in `state` whose name (skill)
 * or server-name (mcp) collides with a component declared by `manifest`,
 * and partitions the matches by source provenance.
 *
 * `pluginRepo` is the plugin's source URL (the value that lands in
 * `PluginRecord.repo`). When non-null, each candidate's standalone source
 * is canonicalized and compared against `canonicalizeSourceKey(pluginRepo)`.
 * When `pluginRepo` is null (e.g., local plugin install with no remote),
 * every match is `crossSource`.
 *
 * A standalone with `repo: null` (linked skill, adopt-without-git) — or,
 * for MCP, a standalone with no parseable source — is always `crossSource`.
 *
 * Does NOT mutate state. Filtering by scope is the caller's responsibility
 * — `state` is expected to already be the scope-correct state.json.
 */
export function detectCaptureMatches(
  state: State,
  manifest: PluginManifest,
  pluginRepo: string | null,
): CaptureMatches;

/** Trivial helper — concatenates two buckets. */
export function mergeBuckets(a: CaptureBucket, b: CaptureBucket): CaptureBucket;
```

**Implementation notes**:

- For each `PluginSkillComponent` in `manifest.components`, search `state.skills` for an entry whose `.name === component.name`. There is at most one match per name per scope (state load deduplicates by name + scope already; cf. `discover.ts:130`).
- For each `PluginMcpComponent`, parse every `state.mcpServers[i].name` with `parseNamespacedKey` (`plugin/mcp-inject.ts:55`). Compare the resulting `serverName` to `component.server.name`. A plugin component may match multiple standalones if more than one source installed the same server name under different slugs — record all of them.
- **Source classification**: compute `canonicalPlugin = pluginRepo ? canonicalizeSourceKey(pluginRepo) : null`. For each match:
  - Skill: `canonicalStandalone = standalone.repo ? canonicalizeSourceKey(standalone.repo) : null`.
  - MCP: `canonicalStandalone = standalone.source ? canonicalizeSourceKey(standalone.source) : null`. The MCP `source` field is the *original* user-passed string (e.g., `mcp:user/repo`); strip the `mcp:` prefix before canonicalizing so that `mcp:github:alice/x` and `github:alice/x` compare equal. Define a small private helper `canonicalMcpSource(s)` for this — it lives next to `parseMcpRef` semantics.
  - Same-source iff both canonicals are non-null and equal. Otherwise cross-source. (Both null → cross-source — a no-provenance plugin can't claim ownership of a no-provenance standalone implicitly.)
- An entry whose namespaced key fails to parse (legacy / malformed) is skipped silently. The function does not error.
- `mergeBuckets({skills:[a], mcps:[]}, {skills:[b], mcps:[c]}) === {skills:[a,b], mcps:[c]}`. Used by `installPlugin` after a force decision to fold cross-source conflicts back into the main capture flow.

**Acceptance criteria**:
- [ ] Returns empty `sameSource` and `crossSource` buckets when the plugin's component names don't appear in `state`.
- [ ] A skill match where `canonicalizeSourceKey(plugin.repo) === canonicalizeSourceKey(standalone.repo)` lands in `sameSource.skills`.
- [ ] A skill match where `plugin.repo` and `standalone.repo` canonicalize differently lands in `crossSource.skills`.
- [ ] A skill match where `standalone.repo` is null (linked) lands in `crossSource.skills` regardless of `pluginRepo`.
- [ ] A skill match where `pluginRepo` is null lands in `crossSource.skills` regardless of standalone source.
- [ ] An MCP match where standalone's `source` (`mcp:` prefix stripped, canonicalized) equals `canonicalizeSourceKey(plugin.repo)` lands in `sameSource.mcpServers`.
- [ ] An MCP match with mismatched canonical sources lands in `crossSource.mcpServers`.
- [ ] Returns *every* `StoredMcpStandalone` whose parsed `serverName` matches a plugin MCP component's `server.name`, partitioned correctly even if some are sameSource and others crossSource.
- [ ] Skips standalones with unparseable namespaced keys without throwing.
- [ ] `total === sameSourceTotal + crossSourceTotal` always.
- [ ] Pure — does not call I/O, does not import callbacks.

---

### Unit 2: Capture module — atomic apply

**File**: `packages/core/src/plugin/capture.ts` (same file, separate export)

```typescript
import type { Result, UserError } from "../types";
import type { CaptureBucket } from "./capture";

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
 * Atomically transfer ownership of the candidates in `bucket` to the plugin.
 *
 * NOTE: this function is provenance-agnostic. The same-source / cross-source
 * partitioning is the detector's responsibility. Callers that want to capture
 * cross-source matches (e.g., after a user "force" override in interactive
 * mode) merge them into the bucket via `mergeBuckets` before calling here.
 *
 * Steps:
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
 * Idempotent: calling with an empty bucket is a no-op.
 */
export async function applyCapture(
  bucket: CaptureBucket,
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
 * standalones from the SAME canonical source (plugin.repo and
 * standalone.repo canonicalize-equal). Return true to capture, false
 * to abort. Omitted with non-empty same-source matches → auto-capture
 * (matches the existing pattern where a missing confirm callback
 * auto-proceeds).
 */
onCaptureConfirm?: (sameSource: CaptureBucket) => Promise<boolean>;

/**
 * Called when the plugin's components collide with already-installed
 * standalones from a DIFFERENT canonical source — or with no recorded
 * source at all (linked, null repo, etc.).
 *
 * Returns:
 *   "abort" — fail the install with a UserError.
 *   "force" — treat the conflicts as captures (user override). The
 *             conflicts are merged into the same-source bucket and flow
 *             through the normal apply path; if `onCaptureConfirm` is
 *             also provided it sees the merged set.
 *
 * Omitted with non-empty cross-source conflicts → install fails with
 * a UserError. (Auto-confirm modes opt in by passing `() => "abort"`
 * explicitly; interactive modes that want to offer force-override pass
 * a real callback.)
 */
onCaptureConflict?: (crossSource: CaptureBucket) => Promise<"abort" | "force">;
```

Add to `PluginInstallResult`:

```typescript
/** Components transferred from standalone state to this plugin. */
captured: {
  skills: string[];
  mcpServers: string[];
  /**
   * Subset of the above whose ownership transferred via cross-source force
   * override. Empty unless the user invoked `onCaptureConflict → "force"`.
   * Tracked separately so the CLI summary can call out the override.
   */
  forcedCrossSource: { skills: string[]; mcpServers: string[] };
};
```

**Insertion point**: between the security-scan block (current line ~99) and skill placement (current line ~101). Pseudocode:

```typescript
// 1.5. Capture detection
const stateForCapture = await loadState(
  scope === "project" ? projectRoot : undefined,
);
if (!stateForCapture.ok) return stateForCapture;
const matches = detectCaptureMatches(
  stateForCapture.value,
  manifest,
  options.repo,
);

let toCapture: CaptureBucket = matches.sameSource;
let forcedBucket: CaptureBucket = { skills: [], mcpServers: [] };

if (matches.crossSourceTotal > 0) {
  if (!options.onCaptureConflict) {
    return err(
      new UserError(
        `Plugin "${manifest.name}" would replace ${matches.crossSourceTotal} standalone component(s) installed from a different source.`,
        buildCrossSourceHint(matches.crossSource, options.repo),
      ),
    );
  }
  const decision = await options.onCaptureConflict(matches.crossSource);
  if (decision === "abort") {
    return err(
      new UserError(
        `Install of plugin "${manifest.name}" cancelled — cross-source capture conflict.`,
      ),
    );
  }
  // decision === "force"
  forcedBucket = matches.crossSource;
  toCapture = mergeBuckets(matches.sameSource, forcedBucket);
}

if (toCapture.skills.length + toCapture.mcpServers.length > 0) {
  if (options.onCaptureConfirm) {
    const proceed = await options.onCaptureConfirm(toCapture);
    if (!proceed) {
      return err(
        new UserError(
          `Install of plugin "${manifest.name}" cancelled.`,
        ),
      );
    }
  }
  const applied = await applyCapture(toCapture, {
    scope,
    projectRoot,
    pluginName: manifest.name,
  });
  if (!applied.ok) return applied;
  capturedSkills = applied.value.capturedSkills;
  capturedMcpServers = applied.value.capturedMcpServers;
}
```

`capturedSkills` / `capturedMcpServers` / `forcedBucket` are declared at the function top with empty defaults. The final return becomes:

```typescript
ok({
  record, warnings, mcpAgents, agentDefsPlaced,
  captured: {
    skills: capturedSkills,
    mcpServers: capturedMcpServers,
    forcedCrossSource: {
      skills: forcedBucket.skills.map(c => c.standalone.name),
      mcpServers: forcedBucket.mcpServers.map(c => c.serverName),
    },
  },
});
```

`buildCrossSourceHint` is a small helper colocated with `detectCaptureMatches` in `capture.ts`. It produces a hint like:
> "Standalone 'commit-helper' (github:alice/commit-helper) ≠ plugin source (github:bob/dev-toolkit). Run `skilltap remove commit-helper` to release the standalone, or run `skilltap install <plugin>` interactively to force-capture."

**Implementation notes**:
- Do NOT pass `loadInstalled` / `loadPlugins` — capture works against `state.json` directly. This avoids the read-fallback path that's already deprecated and keeps capture's mental model simple.
- Capture happens BEFORE skill placement so that the standalone's `also` symlinks are removed before the plugin's are created. That avoids a window where the plugin's symlink targets and the standalone's overlap.
- Capture happens BEFORE MCP injection so standalone MCP keys are pruned before plugin keys land. Otherwise an agent restarting in that window could see the standalone's command on the bare key alongside the plugin's namespaced key.
- Capture happens AFTER security scan — if the plugin's content fails the scan, we never touch the user's existing setup.
- Cross-source conflicts are evaluated FIRST. A force decision merges them into the capture set and a same-source confirm (if any) sees the union. This means the interactive UX renders two prompts for the dual case (conflict prompt → confirm prompt) — acceptable; the second prompt is the existing same-source UX.

**Acceptance criteria**:
- [ ] No matches → `installPlugin` behaves exactly as today; `result.captured.skills`, `mcpServers`, and `forcedCrossSource.*` are all empty arrays.
- [ ] Same-source matches only, no `onCaptureConfirm` → auto-captures; `result.captured.skills`/`mcpServers` populated; `forcedCrossSource` empty.
- [ ] Same-source matches only, `onCaptureConfirm` returns `true` → captures and proceeds.
- [ ] Same-source matches only, `onCaptureConfirm` returns `false` → returns `UserError`, no state mutation, no placement, no injection.
- [ ] Cross-source conflicts present, no `onCaptureConflict` → returns `UserError` with the cross-source hint; no state mutation.
- [ ] Cross-source conflicts present, `onCaptureConflict` returns `"abort"` → returns `UserError`, no mutation.
- [ ] Cross-source conflicts present, `onCaptureConflict` returns `"force"` → captures the cross-source set; `forcedCrossSource` lists those names.
- [ ] Cross-source `force` + same-source matches: `onCaptureConfirm` (if provided) is called with the **merged** bucket; both sets land in `captured` and only the cross-source ones land in `forcedCrossSource`.
- [ ] After any successful capture, `state.skills[]` has zero entries with the captured names; `state.mcpServers[]` has zero entries with the captured server names.
- [ ] After successful capture install, the plugin record's `components[]` includes the corresponding `StoredSkillComponent` / `StoredMcpComponent` entries (the existing `manifestToRecord` produces these — capture doesn't need to add anything extra).
- [ ] The captured skills' physical directories at `.agents/skills/<name>` contain the plugin's content, not the standalone's, after install.
- [ ] On project scope with a `skilltap.toml` present, captured skills are removed from `[skills]` and `skill[]` lock array.

---

### Unit 4: CLI install command — capture confirmation prompt

**File**: `packages/cli/src/commands/install.ts` (modify existing)

#### Interactive mode

In `runInteractiveMode` (the path that constructs the `installSkill` call), add **two** callbacks to the option object: `onCaptureConflict` (cross-source, runs first) and `onCaptureConfirm` (same-source or force-merged, runs second).

##### `onCaptureConflict` — cross-source conflict prompt

1. Stop the spinner if running.
2. Render the conflict block. Each row shows the standalone's URL and the plugin's URL side-by-side so the user can see the substitution they'd be authorizing.
3. Call `select({ message: "Cross-source capture conflict — what do you want to do?", initialValue: "abort", options: [ { value: "abort", label: "Abort the install (recommended)" }, { value: "force", label: "Force capture (override and replace standalones from a different source)" } ] })`.
4. `isCancel` → `"abort"`.
5. Restart the spinner.
6. Return `"abort" | "force"`.

##### `onCaptureConfirm` — same-source (or force-merged) confirmation

1. Stop the spinner.
2. Render the capture summary. Same-source rows render normally; if the bucket includes force-merged cross-source rows (because the user said "force" upstream), they render with a `[FORCED]` prefix and the differing source URL still visible — the user gets a last chance to back out.
3. `confirm({ message: "Capture these components into <plugin>?", initialValue: true })`.
4. `isCancel` → `false`.
5. Restart the spinner.
6. Return the boolean.

##### Rendering helpers

Two helpers in `cli/src/ui/install-callbacks.ts` (where the other plugin callback rendering lives), both using `log.info`/`log.warn` from clack:

```
printCaptureConflict(matches: CaptureBucket, pluginRepo: string | null): void
printCaptureSummary(matches: CaptureBucket, pluginName: string, forced: Set<string>): void
```

Output style for the conflict block:

```
⚠ Plugin "dev-toolkit" wants to replace standalone components from a DIFFERENT source.

  Skills (1):
    • commit-helper
        standalone: github:alice/commit-helper
        plugin:     github:bob/dev-toolkit
        ⚠ Different authors. The plugin's bundled version would overwrite Alice's content.

  MCP servers (1):
    • postgres
        standalone: mcp:foo/repo  (slug=repo → skilltap:repo:postgres)
        plugin:     github:bob/dev-toolkit

This is silent substitution. Choose carefully.
```

Output style for the same-source summary (existing format, preserved, with optional `[FORCED]` rows):

```
Plugin "dev-toolkit" wants to take ownership of:

  Skills (2):
    • commit-helper       (was: github:bob/dev-toolkit, project)
    • code-reviewer       [FORCED] (was: linked at /home/nathan/dev/code-reviewer)

  MCP servers (1):
    • postgres            (was: skilltap:dev-toolkit:postgres → claude-code, cursor)

These standalones will be removed from skilltap.toml and skilltap.lock.
The plugin's bundled versions will replace them on disk.
```

#### Agent mode

In `runAgentMode`, install both callbacks with the safe defaults:

```typescript
onCaptureConflict: async () => "abort",
onCaptureConfirm: async () => true,
```

The `"abort"` decision propagates as a `UserError`; the agent-mode CLI prints a structured error including the cross-source hint produced by `buildCrossSourceHint`, then exits non-zero. Resolution copy:

> "Plugin would replace 2 standalone(s) from a different source. Remove them with `skilltap remove <name>` first, or run `skilltap install <plugin>` without `--agent` to interactively force the capture."

`composePolicy.onWarn === "fail"` is for security warnings, not for capture. Same-source captures auto-confirm regardless of strict; cross-source conflicts hard-error regardless of strict — these are structural decisions, not warning gates.

#### Output after success

After plugin install, augment `componentSummary(pr)` rendering. When `result.value.captured.skills.length + captured.mcpServers.length > 0`, print:

```
Captured 2 standalone skill(s), 1 MCP server into "dev-toolkit".
```

If `captured.forcedCrossSource.skills.length + forcedCrossSource.mcpServers.length > 0`, append a second line listing the forced names so they're visible in shell history / CI logs:

```
  ⚠ Force-captured (cross-source override): commit-helper, postgres
```

In agent mode plain-text output, the install-summary block gets a `Captured: ...` line under the `Plugin: ...` line; the forced-override line appears only when relevant. (Agent mode never reaches this code path with a non-empty `forcedCrossSource` under the default policy, but the plumbing supports it for future direct-API callers that pass their own `onCaptureConflict`.)

**Acceptance criteria**:
- [ ] Interactive install with no matches prints no capture text.
- [ ] Interactive install with same-source matches only shows the summary block + a confirm prompt.
- [ ] Interactive install with cross-source conflicts shows the conflict block + a select prompt; default is "abort"; "abort" cancels with a clear message.
- [ ] Interactive install with cross-source conflicts where the user selects "force" then sees the same-source confirm prompt with the conflicts marked `[FORCED]` and proceeds on confirm.
- [ ] Agent-mode install with same-source matches captures without prompting.
- [ ] Agent-mode install with cross-source conflicts exits non-zero with the resolution hint visible to the agent.
- [ ] The post-install summary mentions the capture count when non-zero, and adds the force line when `forcedCrossSource` is non-empty.
- [ ] Plain-text mode (agent / non-TTY) renders the summary via `process.stdout.write`, not via `log.info`.

---

### Unit 5: Barrel exports

**File**: `packages/core/src/plugin/index.ts`

Add:

```typescript
export {
  detectCaptureMatches,
  applyCapture,
  mergeBuckets,
  buildCrossSourceHint,
  type CaptureCandidate,
  type CaptureBucket,
  type CaptureMatches,
  type ApplyCaptureOptions,
  type ApplyCaptureResult,
} from "./capture";
```

These are also used by tests directly.

---

### Unit 6: Sync command — pass capture-confirm callback through

**File**: `packages/core/src/sync/apply.ts` (modify existing)

`applyAddSkill` is the single call site that delegates to `installSkill` for both skill and plugin add operations (`apply.ts:103`). Add **both** capture callbacks alongside the existing auto-accept callbacks:

```typescript
onCaptureConfirm: async () => true,
onCaptureConflict: async () => "abort",
```

Sync is a non-interactive reconciliation: same-source captures are the same intent as agent-mode install (auto-confirm), but cross-source conflicts must hard-fail. Sync is exactly the path where silent substitution is most dangerous — a teammate cloning the project and running `skilltap sync` shouldn't have skills they trust replaced because the manifest declared a plugin from a different author. The drift item bubbles up through the existing `ApplyItemResult` shape with `status: "fail"` and the `error` field carrying the cross-source resolution hint.

**Acceptance criteria**:
- [ ] `skilltap sync --apply` of a plugin add whose components match same-source standalones captures them silently and records the capture.
- [ ] `skilltap sync --apply` of a plugin add whose components match cross-source standalones returns `status: "fail"` for that item with the resolution hint in `error`.
- [ ] After a successful same-source sync, the captured standalones are gone from state.
- [ ] After a failed cross-source sync, no state mutation has occurred for that item.

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
describe("detectCaptureMatches — basic matching")
  - returns empty buckets when state has no skills or mcpServers
  - returns empty buckets when no plugin component name overlaps state
  - matches a single skill by name within scope
  - matches multiple skills in one manifest
  - matches an MCP standalone whose parsed serverName equals the plugin component's server.name
  - matches multiple MCP standalones from different slugs sharing a server name
  - skips state.mcpServers entries with unparseable namespaced keys
  - ignores plugin agent components (no standalone analog)
  - is pure: returns same result on repeated calls with same inputs
  - total === sameSourceTotal + crossSourceTotal across every test

describe("detectCaptureMatches — source partitioning")
  - skill match where canonicalize(plugin.repo) == canonicalize(standalone.repo) → sameSource
  - skill match where plugin.repo and standalone.repo canonicalize differently → crossSource
  - skill match where plugin.repo is "https://github.com/x/y.git" and standalone.repo is "git@github.com:x/y" → sameSource (canonicalization equates them)
  - skill match where standalone.repo is null (linked) → crossSource regardless of plugin.repo
  - skill match where pluginRepo arg is null → crossSource regardless of standalone.repo
  - mcp match where standalone.source is "mcp:user/repo" and plugin.repo is "github:user/repo" → sameSource (mcp: prefix stripped before canonicalization)
  - mcp match where standalone.source canonicalizes differently from plugin.repo → crossSource
  - mcp match where standalone has no parseable source → crossSource
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
describe("installPlugin with capture — same-source")
  - no overlap: result.captured arrays empty, behavior unchanged
  - same-source overlap, no onCaptureConfirm: auto-captures and reports names
  - same-source overlap, onCaptureConfirm returns true: captures, install completes
  - same-source overlap, onCaptureConfirm returns false: UserError, no state mutation, no record
  - after same-source capture, plugin.components[] includes captured names as StoredSkillComponent
  - after same-source capture, .agents/skills/<name>/ contains plugin content
  - capture only runs after scan passes (verify ordering)
  - captured MCP standalone's targets have the standalone-namespaced keys removed before plugin keys land

describe("installPlugin with capture — cross-source")
  - cross-source conflict, no onCaptureConflict: returns UserError with hint, no state mutation
  - cross-source conflict, onCaptureConflict returns "abort": UserError, no mutation
  - cross-source conflict, onCaptureConflict returns "force": captures, forcedCrossSource populated
  - cross-source force + same-source: onCaptureConfirm sees the merged bucket
  - cross-source force decision still respects onCaptureConfirm returning false (final abort)
  - mixed conflicts (some same-source, some cross-source) without onCaptureConflict: same-source captures don't proceed because cross-source check happens first
  - linked standalone always treated as crossSource even when name matches plugin component
```

### Integration / E2E Tests

#### `packages/core/src/plugin/install-integration.test.ts` (extend existing)

Use existing fixtures `createClaudePluginRepo` / `createCodexPluginRepo`. Pre-seed state with standalone skills/MCPs whose names match the plugin's components, then drive `installSkill` end-to-end:

```
describe("installSkill plugin path with capture — same-source")
  - pre-installed standalone skill from same repo as the plugin → captured
  - pre-installed standalone MCP (via installMcpOnly from same source) → captured under plugin namespace
  - pre-installed disabled standalone → captured + .disabled dir cleaned up
  - pre-installed in skilltap.toml → manifest [skills] entry removed, plugin entry added
  - cancellation via onCaptureConfirm=false leaves all original state intact

describe("installSkill plugin path with capture — cross-source")
  - pre-installed standalone from a different repo, no onCaptureConflict callback → install fails, state intact
  - pre-installed standalone from a different repo, onCaptureConflict="abort" → install fails, state intact
  - pre-installed standalone from a different repo, onCaptureConflict="force" → captured, forcedCrossSource populated
  - pre-installed linked skill (no repo) with matching name → cross-source conflict
  - pre-installed standalone in skilltap.toml from a different repo → manifest entry preserved on conflict-abort
```

#### `packages/cli/src/commands/install.test.ts` (or new `install.capture.test.ts`)

Subprocess tests via `runSkilltap` (pipe) and `runInteractive` (PTY):

```
- Agent mode (--agent) with same-source capture: completes silently, exits 0, summary mentions Captured count
- Agent mode (--agent) with cross-source conflict: exits non-zero with the resolution hint in stderr, no state mutation
- Interactive (PTY) with same-source capture: shows summary block, confirm prompt, install succeeds
- Interactive (PTY) with same-source capture, prompt cancelled: aborts with "Install ... cancelled" and non-zero exit
- Interactive (PTY) with cross-source conflict, default (abort) selected: aborts cleanly, state intact
- Interactive (PTY) with cross-source conflict, "force" selected: shows the [FORCED] line in the same-source confirm, completes on confirm
- Plain pipe install with same-source capture: no spinner artifacts, capture summary lines present
```

#### `packages/core/src/sync/apply.test.ts` (extend existing)

```
- sync apply of a plugin add when state has a same-source matching standalone → captures, applied count = 1
- sync apply of a plugin add when state has a cross-source matching standalone → fails that item with the resolution hint; other items still apply
- after a cross-source sync failure, state is unchanged for the failed item (standalone still present, plugin not recorded)
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

Manual smoke tests (project scope with manifest):

```bash
# Setup
cd /tmp && mkdir capture-demo && cd capture-demo && git init
printf '[targets]\nagents = ["claude-code"]\n' > skilltap.toml

# --- Same-source capture path ---
# Pre-install a standalone from a repo that ALSO publishes a plugin bundling it
skilltap install github:user/commit-helper --project
# Install the plugin from the same repo (bundles commit-helper)
skilltap install github:user/dev-toolkit --project

cat .agents/state.json     # commit-helper appears under plugins[0].components, not skills
cat skilltap.toml          # commit-helper entry gone from [skills], dev-toolkit in [plugins]
skilltap doctor            # no collision warning
skilltap sync              # in sync — no drift
skilltap remove dev-toolkit
skilltap doctor            # commit-helper not auto-restored; clean state

# --- Cross-source conflict path ---
# Pre-install a standalone from author A
skilltap install github:alice/commit-helper --project
# Try to install a plugin from author B that bundles a skill named commit-helper
skilltap install github:bob/dev-toolkit --project --agent
# → exits non-zero with cross-source resolution hint; state intact

# Resolve manually:
skilltap remove commit-helper --project
skilltap install github:bob/dev-toolkit --project --agent  # now succeeds

# --- Force-override path (interactive) ---
skilltap install github:alice/commit-helper --project
skilltap install github:bob/dev-toolkit --project
# → conflict prompt: select "Force capture", confirm same-source prompt
cat .agents/state.json     # commit-helper now in plugins[0].components
# CLI summary printed: "⚠ Force-captured (cross-source override): commit-helper"
```

---

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Silent name-squatting: attacker publishes a plugin whose component names collide with popular standalones, hoping users install it and let capture swap content | Cross-source partitioning. An attacker would have to publish from the *same canonical source* as the standalone the user trusted (e.g., Alice's `github:alice/commit-helper`). They can't, because canonicalization keys on host + owner + repo and they don't control Alice's namespace. Cross-source conflicts hard-error in agent mode and sync; interactive force is a deliberate user action. |
| Accidental name collision (two unrelated authors picking the same kebab-case name) | Same as above — cross-source conflict surfaces both URLs side-by-side. Even in interactive mode the default is abort, not force. |
| Plugin author legitimately bundles their own previously-standalone skill from a sibling repo (e.g., `github:alice/dev-toolkit` plugin includes a skill the user installed standalone from `github:alice/commit-helper`) | Cross-source check fires; user must `skilltap remove commit-helper` (or pick force in interactive mode) before the plugin install proceeds. Friction by design — the alternative is silent substitution. The error hint names both URLs and the resolution path. |
| Plugin author legitimately vendors someone else's skill into their bundle | Same as above — the user is forced to consciously authorize the substitution in interactive mode, or to remove the standalone first. We accept this friction as the cost of ruling out silent substitution. |
| User loses custom edits to a captured skill's local files | Plugin install always overwrites `.agents/skills/<name>`; capture doesn't change that. Same-source capture surfaces the source URL before confirmation; cross-source capture additionally requires explicit force. |
| Standalone MCP server has env vars / tokens the plugin's version doesn't replicate | Capture confirmation surfaces source. A user who relies on env vars will recognize and abort. Future enhancement: show a config diff. Out of scope for v1. |
| Partial failure: state saved, MCP key prune fails | State is the source of truth; doctor will surface dangling agent-config keys. Acceptable for v1. |
| Capture during `skilltap sync --apply --strict` | `applyAddSkill` passes `onCaptureConflict: () => "abort"`, `onCaptureConfirm: () => true`. Strict doesn't gate same-source captures (manifest already declared the plugin → user's pre-stated intent), and cross-source conflicts hard-fail with or without strict. |
| Cross-scope skill with the same name (global standalone, project plugin install) | Capture only matches within the install's scope. The global standalone is left alone. The project install may overwrite the symlink in `.claude/skills/` if the agent symlinks resolve there — but that's an existing scope-collision issue, not introduced by capture. Doctor's existing checks cover that. |
