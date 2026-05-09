# Design: Current-State Cleanup

## Overview

Foundation docs and the codebase still narrate skilltap's evolution — "in v2.2 we removed X", "previously Y", "legacy fallback for v0.x". The user's mandate: docs and code describe **only the current state**. Git holds history. An agent reading the repo should learn what skilltap *is*, never what it *used to be*. A new contributor should not have to subtract historical residue to assemble the present.

This design also folds in the residual user-facing bugs and API naming residue that the v2.2 cutover left behind, plus the long-tail polish (version-stamped strings, phase-numbered comments, dead code).

### What changes

**Docs** rewritten to current-state framing:
- `docs/SPEC.md` — drop the "Removed in v2.2" appendix, drop the version-stamped Migration tables (the migrate command's own help can carry that), drop "V2"/"v2.0"/"v2.2" markers, drop "legacy" framing.
- `docs/ARCH.md` — drop "from v0.x and pre-V2" markers, rename "Removed-Command Hints" → "Removed-Command Errors" (current-state framing of the error class).
- `docs/VISION.md` — drop "Considered and removed" section entirely; rename forward-looking section.
- `docs/UX.md` — drop "Legacy Commands" section heading + content (replaced by "Removed-command errors" with current-state framing); drop "v2.0 redesign" lead-in; drop "Migrating from v2.1 or earlier" subsection (move to migrate page).
- `README.md` — drop the "upgrading from any pre-v2.2" paragraph + gotcha.
- `.claude/CLAUDE.md` + `AGENTS.md` — rename "v2.2 conventions" → "Conventions"; drop version stamps in lead-in; drop reference to deleted `docs/PROGRESS.md` and `docs/designs/completed/`.
- Website pages (`getting-started.md`, `what-is-skilltap.md`, `doctor.md`, `taps.md`, `reference/cli.md`) — same sweep.
- `website/public/llms-full.txt` — regenerate.
- `.claude/rules/patterns.md` — rename `loadInstalled`/`saveInstalled` references to current names.

**ROADMAP** trimmed to the present:
- Drop Phases 0–46 + dependency graphs + v2.2 cleanup wave (lines 5–941).
- Keep `# Roadmap` header, a brief "Current state" snapshot, and the `## What's Deferred` list.
- The deferred list keeps its forward-state framing — it's the only thing roadmaps currently need to say.

**Design archive** deleted:
- `docs/designs/completed/` — entire directory removed (47 files).
- `docs/designs/v2.2-cleanup.md` + `docs/designs/v2.2-cleanup/` — removed.
- After this design lands and is implemented, `docs/designs/cleanup-current-state.md` itself is the only surviving design doc until the next one.

**Code residue removed**:
- `InstalledJson` / `PluginsJson` legacy file-wrapper types — moved out of public API; functions return raw arrays.
- `plugin-v2/` directory — renamed to `skilltap-plugin/`; `*V2` schema names dropped.
- `resolveAgentForAgentMode` dead function — deleted.
- `info.ts` / `status.ts` `--global`/`--project` boolean pair — switched to `--scope` (locks in the v2.2 carve-out).
- Phase-numbered test comments — stripped.
- `e2e-v2.test.ts` — renamed to `e2e.test.ts` with neutral header.
- Benchmark fixture writes wrong file — fixed.

**User-facing bug fixes**:
- `update.ts:380` prompt that mentions `installed.json` — fixed.
- 5 hint strings pointing at `skilltap list` (a command that doesn't exist) — rewritten to `skilltap status`.
- 2 hint strings pointing at `skilltap skills` (removed) — rewritten to `skilltap status`.
- Stale `--global`/`--project` flags in bash + zsh `move` completions — replaced with `--scope`.
- SPEC's `### list` section + `skilltap list [flags]` tree line — deleted (the command doesn't exist; status is canonical).

**Version-label residue**:
- `cli/src/index.ts` v1.0/v2.0 soft-hint text → version-neutral.
- `state/schema.ts:29` "for v2.0" comment → neutral.
- `migrate.ts` help strings `v1.0 → v2.0` → "legacy → state.json".
- `completions/zsh.ts:18` and `completions/fish.ts:20` migrate descriptions → neutral.

### What does NOT change

- `website/changelog.md` — full release-note history kept; changelogs are intrinsically additive.
- The `migrate` command — it exists for users coming from older state. Its own description and help text may reference legacy formats (that's its purpose), but the rest of the codebase stops doing so.
- `state.json` `version: 2` field — that's the on-disk schema version, not historical narrative.
- `version: 1` literal in `migrate/legacy-schemas.ts` (relocated) — required to parse legacy files. Lives strictly inside migrate's read path.

### Architectural options considered

**Option A — One-shot rewrite**: single mega-PR touching everything. Maximum throughput, but high blast radius if a verification gate fails.

**Option B — Sequential phases (Code → Docs → ROADMAP → Mass-delete → Verify)**: ordered waves, each verifiable independently. Lower per-wave risk; doc edits land after code so they accurately describe code state.

**Option C — Delete-first**: nuke completed/ + ROADMAP first for visible progress, then code, then docs. Front-loads psychological win but reorders dependencies (docs would describe code before code is updated).

**Choice: Option B.** Rationale:
- Code-level changes (rename `plugin-v2/`, drop `InstalledJson` wrapper, switch `info`/`status` to `--scope`) must land before docs that describe them.
- Docs trim must precede `llms-full.txt` regen.
- Mass-deletes (completed/, ROADMAP body) are the lowest-risk last step.
- Each phase has its own verification gate — failures are localized.

### The trickiest unit

**Unit 5 (drop `InstalledJson`/`PluginsJson` wrappers)** crosses 8 production files and changes a function signature. The single non-migrate consumer that legitimately reads the legacy file (`doctor/checks/installed.ts`) must keep access to the legacy `version: 1` schema for diagnostic purposes. Resolution: relocate the legacy schemas into `migrate/legacy-schemas.ts` (private to migrate), have `doctor/checks/installed.ts` import from there explicitly, and rewrite all other consumers to operate on raw `InstalledSkill[]` / `PluginRecord[]` arrays. Designed first; rest of the design depends on knowing this is feasible.

---

## Implementation Phases

| Phase | Units | Depends on | Verification |
|---|---|---|---|
| 1 | High-severity user-facing bug fixes (Units 1–4) | — | `bun test` for affected files; manual `bun run dev <removed-cmd>` |
| 2 | API residue (Units 5–8) | Phase 1 (no overlap, parallel-safe) | `bun test`; `bun run build`; `bun run verify:binary` |
| 3 | Lock in info/status `--scope` (Unit 9) | Phase 2 | `bun test packages/cli/src/commands/{info,status}.test.ts` |
| 4 | Polish + version-label scrub (Units 10–14) | Phase 1–3 (touches some same files) | `bun test`; lint clean |
| 5 | Foundation docs rewrite (Units 15–22) | Phases 1–4 (docs describe new code) | manual review; anchor sweep |
| 6 | ROADMAP trim (Unit 23) | Phase 5 (style consistency) | manual review |
| 7 | Design archive deletion (Unit 24) | All previous (no consumer refs left) | grep for inbound references |
| 8 | `llms-full.txt` regen + final verification (Units 25–26) | All previous | full 29-gate verification |

---

## Implementation Units

### Unit 1: Fix `update.ts:380` `installed.json` prompt

**File**: `packages/cli/src/commands/update.ts`

**Current** (line 380):
```typescript
message: `Remove "${skillName}" from installed.json?`,
```

**Change to**:
```typescript
message: `Remove "${skillName}" from skilltap?`,
```

**Acceptance Criteria**:
- [ ] No occurrence of `installed.json` remains in any user-facing string in `packages/cli/src/commands/`.
- [ ] `bun test packages/cli/src/commands/update.test.ts` passes.

---

### Unit 2: Rewrite all `skilltap list` / `skilltap skills` hints to `skilltap status`

**Decision rationale**: `skilltap list` was specced but never shipped. `status` is feature-complete and accepts the same filter flags (`--global`, `--project`, `--json`, `--unmanaged`, `--disabled`, `--active`). Adding a `list` alias would be net new code; rewriting hints is a strictly subtractive cleanup.

**Files to edit** (string replacements):

| File:line | Current | Replace with |
|---|---|---|
| `packages/core/src/disable.ts:60` | `Run 'skilltap skills' to see installed skills.` | `Run 'skilltap status' to see installed skills.` |
| `packages/core/src/disable.ts:120` | `Run 'skilltap skills' to see installed skills.` | `Run 'skilltap status' to see installed skills.` |
| `packages/core/src/remove.ts:40` | `Run 'skilltap list' to see installed skills.` | `Run 'skilltap status' to see installed skills.` |
| `packages/core/src/update.ts:760` | `Run 'skilltap list' to see installed skills.` | `Run 'skilltap status' to see installed skills.` |
| `packages/core/src/move.ts:89` | `Run 'skilltap list' to see installed skills.` | `Run 'skilltap status' to see installed skills.` |
| `packages/core/src/adopt.ts:103` | `Run 'skilltap list' to see managed skills.` | `Run 'skilltap status' to see managed skills.` |
| `packages/cli/src/index.ts:378` | `Use \`skilltap list\` (and the typed \`install\`/\`remove\`/\`update\`/\`toggle\` subcommands).` | `Use \`skilltap status\` (and the typed \`install\`/\`remove\`/\`update\`/\`toggle\` subcommands).` |

**Acceptance Criteria**:
- [ ] `grep -rnE "'skilltap (list\|skills)'" packages/ --include="*.ts" \| grep -v test` returns empty.
- [ ] `grep -rn "skilltap list" packages/ --include="*.ts" \| grep -v test` returns empty.
- [ ] `bun test packages/core/src/disable.test.ts packages/core/src/remove.test.ts` passes.

---

### Unit 3: Delete the `### list` SPEC section

**File**: `docs/SPEC.md`

**Changes**:
1. Delete the line `skilltap list    [flags]` from the command tree at line 81 (one line).
2. Delete the entire `### list` subsection at lines 368–374 (about 7 lines).

**Implementation Notes**: Run `grep -n "^### list" docs/SPEC.md` to confirm the target line; delete that section through the next `###` heading or until the section ends.

**Acceptance Criteria**:
- [ ] `grep -n "^### list" docs/SPEC.md` returns empty.
- [ ] `grep -n "skilltap list" docs/SPEC.md` returns empty (or only matches inside the migrate/Removed-in-v22 sections, which Unit 15 also deletes).

---

### Unit 4: Fix bash + zsh `move` completion stale flags

**File 1**: `packages/cli/src/completions/bash.ts:249–260`

**Current**:
```bash
move)
  case "$prev" in
    --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
  esac
  if [[ "$cur" == -* ]]; then
    COMPREPLY=($(compgen -W "--global --project --also" -- "$cur"))
  else
    ...
```

**Change to**:
```bash
move)
  case "$prev" in
    --scope) COMPREPLY=($(compgen -W "project global" -- "$cur")); return ;;
    --also) COMPREPLY=($(compgen -W "$agents" -- "$cur")); return ;;
  esac
  if [[ "$cur" == -* ]]; then
    COMPREPLY=($(compgen -W "--scope --also" -- "$cur"))
  else
    ...
```

**File 2**: `packages/cli/src/completions/zsh.ts:257–265`

**Current**:
```zsh
move)
  ...
  _arguments \
    '--global[Move to global scope]' \
    '--project[Move to project scope]' \
    '--also[Symlink to agent dir]:agent:(${agentSpec})' \
    "1:skill:($skills)"
  ;;
```

**Change to**:
```zsh
move)
  ...
  _arguments \
    '--scope[Target scope]:scope:(project global)' \
    '--also[Symlink to agent dir]:agent:(${agentSpec})' \
    "1:skill:($skills)"
  ;;
```

**Acceptance Criteria**:
- [ ] `grep -n "global.*project\|--global.*move\|move.*--global" packages/cli/src/completions/` returns empty for the `move` command block.
- [ ] `bun test packages/cli/src/commands/completions.test.ts` passes.
- [ ] Manual: invoke `skilltap completions bash | grep -A 12 "^[[:space:]]*move)"` shows `--scope --also` only.

---

### Unit 5: Drop `InstalledJson` / `PluginsJson` wrappers from public API (TRICKIEST)

**Goal**: `loadSkillState` / `saveSkillState` operate on raw `InstalledSkill[]`. `loadPlugins` / `savePlugins` operate on raw `PluginRecord[]`. The `version: 1` legacy file-wrapper schemas live exclusively inside `migrate/`.

**Step 5.1**: Move legacy schemas into migrate.

**New file**: `packages/core/src/migrate/legacy-schemas.ts`

```typescript
import { z } from "zod/v4";
import { InstalledSkillSchema } from "../schemas/installed";
import { PluginRecordSchema } from "../schemas/plugins";

// Legacy file-wrapper schemas. Used only when reading pre-v2 installed.json /
// plugins.json files. Production state lives in state.json.

export const LegacyInstalledJsonSchema = z.object({
  version: z.literal(1),
  skills: z.array(InstalledSkillSchema),
});
export type LegacyInstalledJson = z.infer<typeof LegacyInstalledJsonSchema>;

export const LegacyPluginsJsonSchema = z.object({
  version: z.literal(1),
  plugins: z.array(PluginRecordSchema).default([]),
});
export type LegacyPluginsJson = z.infer<typeof LegacyPluginsJsonSchema>;
```

**Step 5.2**: Strip wrapper schemas from `packages/core/src/schemas/installed.ts`.

**Current** (lines 32–38):
```typescript
export const InstalledJsonSchema = z.object({
  version: z.literal(1),
  skills: z.array(InstalledSkillSchema),
});
export type InstalledSkill = z.infer<typeof InstalledSkillSchema>;
export type InstalledJson = z.infer<typeof InstalledJsonSchema>;
```

**Change to**:
```typescript
export type InstalledSkill = z.infer<typeof InstalledSkillSchema>;
```

(Drop `InstalledJsonSchema` and `InstalledJson` — they are no longer the current shape.)

**Step 5.3**: Same for `packages/core/src/schemas/plugins.ts:78–94`. Drop `PluginsJsonSchema` and `PluginsJson` exports; keep `PluginRecord` and `PluginRecordSchema`.

**Step 5.4**: Rewrite `loadSkillState` / `saveSkillState` (`packages/core/src/config.ts:178–201`).

**Change to**:
```typescript
import type { InstalledSkill } from "./schemas/installed";

/** state.json is the only canonical store. Skill-slice accessor: read just skills[]. */
export async function loadSkillState(
  projectRoot?: string,
): Promise<Result<InstalledSkill[]>> {
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) return stateResult;
  return ok([...stateResult.value.skills]);
}

export async function saveSkillState(
  skills: InstalledSkill[],
  projectRoot?: string,
): Promise<Result<void>> {
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) return stateResult;
  const newState = {
    version: 2 as const,
    skills,
    plugins: stateResult.value.plugins,
    mcpServers: stateResult.value.mcpServers,
  };
  return saveState(newState, projectRoot);
}
```

**Step 5.5**: Rewrite `loadPlugins` / `savePlugins` (`packages/core/src/plugin/state.ts:14–35`) analogously, taking/returning `PluginRecord[]`.

**Step 5.6**: Update all consumers. For each, change `result.value.skills` → `result.value` (or `result.value.plugins` → `result.value`); change parameter types from `InstalledJson` → `InstalledSkill[]` and `PluginsJson` → `PluginRecord[]`.

| File | Lines | Change |
|---|---|---|
| `packages/core/src/orphan.ts` | 6, 40, 121 | Param type + `.skills` access |
| `packages/core/src/update.ts` | 20, 640, 709 | Param type + local var |
| `packages/core/src/doctor/checks/skills.ts` | 6, 10 | Param type + `.skills` access |
| `packages/core/src/doctor/checks/npm.ts` | 2, 6 | Param type |
| `packages/core/src/doctor/checks/symlinks.ts` | 5, 10 | Param type |
| `packages/core/src/doctor/index.ts` | 79 | `installed ?? { version: 1 as const, skills: [] }` → `installed ?? []` |
| `packages/core/src/plugin/lifecycle.ts` | 18, 49 | Type annotation on plugin state field |

**Step 5.7**: `packages/core/src/doctor/checks/installed.ts` is the one legitimate legacy-file consumer. Update its imports:

**Current**: `import { InstalledJsonSchema, type InstalledJson } from "../../schemas/installed";`

**Change to**: `import { LegacyInstalledJsonSchema, type LegacyInstalledJson } from "../../migrate/legacy-schemas";` — and update all uses of `InstalledJsonSchema` / `InstalledJson` to the legacy- prefix.

**Step 5.8**: Update barrel re-exports (`packages/core/src/index.ts`). Drop any `export { InstalledJsonSchema, InstalledJson, PluginsJsonSchema, PluginsJson }`. Confirm only `InstalledSkill`, `InstalledSkillSchema`, `PluginRecord`, `PluginRecordSchema` are re-exported.

**Step 5.9**: Update `.claude/rules/patterns.md` `json-state I/O` line to reflect the new function signatures (drop the `loadInstalled`/`saveInstalled` references that are stale anyway):

**Current**:
> json-state I/O: `loadJsonState<T>(path, schema, label, default)` / `saveJsonState(path, data, label, projectRoot?, ensureDirs)` generic helpers in `json-state.ts`; both `loadInstalled`/`saveInstalled` and `loadPlugins`/`savePlugins` delegate to these — no ad-hoc JSON file I/O in state modules

**Change to**:
> json-state I/O: `loadJsonState<T>(path, schema, label, default)` / `saveJsonState(path, data, label, projectRoot?, ensureDirs)` generic helpers in `json-state.ts`; both `loadSkillState`/`saveSkillState` and `loadPlugins`/`savePlugins` delegate to these — no ad-hoc JSON file I/O in state modules

**Acceptance Criteria**:
- [ ] `grep -rn "InstalledJson\|PluginsJson" packages/core/src/schemas/ --include="*.ts"` returns empty (the wrappers are no longer schema exports).
- [ ] `grep -rn "InstalledJson\|PluginsJson" packages/core/src/index.ts` returns empty.
- [ ] `grep -rn "type InstalledJson\|type PluginsJson" packages/ --include="*.ts" | grep -v migrate` returns empty.
- [ ] `LegacyInstalledJson` / `LegacyPluginsJson` types appear ONLY in `migrate/legacy-schemas.ts` and `doctor/checks/installed.ts`.
- [ ] `bun test` passes (full suite).
- [ ] `bun run build` succeeds.
- [ ] `bun run verify:binary` passes.

---

### Unit 6: Rename `plugin-v2/` → `skilltap-plugin/`; drop `*V2` suffixes

**Goal**: native skilltap plugin manifest format has no other version it differentiates from. The `V2` suffix is naming residue.

**Step 6.1**: Rename directory.

```bash
git mv packages/core/src/plugin-v2 packages/core/src/skilltap-plugin
```

**Step 6.2**: Rename schema symbols in `packages/core/src/skilltap-plugin/schema.ts`:

| Current | Renamed |
|---|---|
| `PluginV2SkillSchema` | `SkilltapPluginSkillSchema` |
| `PluginV2Skill` | `SkilltapPluginSkill` |
| `PluginV2StdioServerSchema` | `SkilltapPluginStdioServerSchema` |
| `PluginV2StdioServer` | `SkilltapPluginStdioServer` |
| `PluginV2HttpServerSchema` | `SkilltapPluginHttpServerSchema` |
| `PluginV2HttpServer` | `SkilltapPluginHttpServer` |
| `PluginV2ServerSchema` | `SkilltapPluginServerSchema` |
| `PluginV2Server` | `SkilltapPluginServer` |
| `PluginV2AgentSchema` | `SkilltapPluginAgentSchema` |
| `PluginV2Agent` | `SkilltapPluginAgent` |
| `PluginManifestV2Schema` | `SkilltapPluginManifestSchema` |
| `PluginManifestV2` | `SkilltapPluginManifest` |

Also rename function `pluginV2ToManifest` → `skilltapPluginToManifest` in `normalize.ts`.

**Step 6.3**: Update header comment in `schema.ts`:

**Current**: `// Native v2.0 plugin manifest format. Lives at .skilltap/<plugin-name>.toml`

**Change to**: `// Native skilltap plugin manifest format. Lives at .skilltap/<plugin-name>.toml`

**Step 6.4**: Update importers. All four files:
- `packages/core/src/index.ts:38` — barrel re-export path.
- `packages/core/src/manifest/publish.ts:5,6,11,23,49` — type + function imports.
- `packages/core/src/plugin/detect.ts:39,89,164` — `discoverSkilltapPlugins` import (path already says `skilltap`; just update directory).
- `packages/core/src/skilltap-plugin/index.ts` — barrel itself uses new symbol names.

**Step 6.5**: Update test files inside the renamed directory (`schema.test.ts`, `normalize.test.ts`) to use the new symbol names.

**Step 6.6**: `docs/ARCH.md` line 163 + 435 mention `PluginManifestV2` and `PluginV2Server` in tree comments — update to new names. (Covered also in Unit 16 doc rewrite, but flag explicitly here so the rename is mechanical.)

**Acceptance Criteria**:
- [ ] `find packages -type d -name "plugin-v2"` returns empty.
- [ ] `grep -rn "PluginV2\|pluginV2\|PluginManifestV2" packages/ --include="*.ts"` returns empty.
- [ ] `grep -rn "from.*plugin-v2" packages/ --include="*.ts"` returns empty.
- [ ] `bun test packages/core/src/skilltap-plugin/` passes.
- [ ] `bun run build` succeeds.

---

### Unit 7: Delete dead `resolveAgentForAgentMode`

**File 1**: `packages/cli/src/ui/resolve.ts`

Delete the entire function at lines 177–188 (12 lines including the JSDoc comment).

**File 2**: `packages/cli/src/ui/install-callbacks.test.ts:154–158`

**Current**:
```typescript
const hasAgentResolution =
  content.includes("resolveSemanticInteractive") ||
  content.includes("resolveAgentForAgentMode");
```

**Change to**:
```typescript
const hasAgentResolution = content.includes("resolveSemanticInteractive");
```

**Acceptance Criteria**:
- [ ] `grep -rn "resolveAgentForAgentMode" packages/` returns empty.
- [ ] `bun test packages/cli/src/ui/install-callbacks.test.ts` passes.

---

### Unit 8: Fix `scan.bench.ts` writing wrong file

**File**: `packages/core/src/benchmarks/scan.bench.ts:100–128`

**Change**: rewrite the fixture-construction block to write `state.json` (v2 schema) instead of `installed.json` (v1 schema). Replace the `installedJson` const with `stateJson` shaped as `{ version: 2, skills: [...], plugins: [], mcpServers: [] }`. Update the comment to remove the `"v0.x fallback"` clause — there is no fallback.

**Acceptance Criteria**:
- [ ] The bench fixture writes `state.json`, not `installed.json`.
- [ ] Comment block at lines 100–102 contains no reference to `installed.json` or `v0.x`.
- [ ] `bun run packages/core/src/benchmarks/scan.bench.ts` (or equivalent invocation) shows the bench actually loading 100 skills (vs the empty default that it produces today).

---

### Unit 9: Switch `info` and `status` from `--global`/`--project` to `--scope`

**Goal**: lock in the v2.2 carve-out. All commands consistent with `--scope project|global`.

**Step 9.1**: Update `packages/cli/src/commands/info.ts:35–46`. Replace the `project` and `global` boolean args with:

```typescript
scope: {
  type: "string",
  description: "Filter to scope (project|global)",
  valueHint: "project|global",
},
```

Update internal logic at line 83: replace `args.global ? "global" : args.project ? "project" : null` with validated derivation from `args.scope`. Reject `--scope foo` with a clear error.

**Step 9.2**: Same for `packages/cli/src/commands/status.ts:13–44` and the consumers at lines 50–93. Update `runUnmanagedMode(args)` signature at line 229 to accept `{ scope?: "project" | "global"; json: boolean }`.

**Step 9.3**: Update `packages/cli/src/commands/status.test.ts:138`:

**Current**: `["status", "--json", "--global"]`

**Change to**: `["status", "--json", "--scope", "global"]`

**Step 9.4**: Update completions to drop the `info`/`status` `--global`/`--project` advertisements:
- `packages/cli/src/completions/bash.ts:120` (info block) — flag list update.
- `packages/cli/src/completions/zsh.ts:143–144` (info), `:178–179` if status has its own block — flag list update.
- `packages/cli/src/completions/fish.ts` — verify and update if present.

**Step 9.5**: Update SPEC + UX docs to drop the `info`/`status` carve-out language. Specific lines:
- `docs/SPEC.md:332,346,362,366` — wherever the boolean carve-out is documented, replace with `--scope`.
- `website/reference/cli.md:34,366,397` — three carve-out paragraphs to delete.
- `website/public/llms-full.txt:2518,2850,2881` — covered by the regen in Unit 25.

**Acceptance Criteria**:
- [ ] `grep -rnE 'project:\s*\{\s*type:\s*"boolean"|global:\s*\{\s*type:\s*"boolean"' packages/cli/src/commands/ --include="*.ts"` returns empty (no command uses the boolean pair).
- [ ] `bun test packages/cli/src/commands/status.test.ts packages/cli/src/commands/info.test.ts` passes.
- [ ] `grep -rn "carve-out\|legacy boolean" docs/ website/ --include="*.md"` returns empty.

---

### Unit 10: Strip version labels from CLI runtime strings

**File**: `packages/cli/src/index.ts:105–127`

**Current**:
```typescript
// v2.0 soft hint: if v1.0 markers exist (installed.json/plugins.json/v1 config keys)
// and no state.json exists yet, suggest the user run `skilltap migrate`.
async function runV1DetectionNotice(): Promise<void> {
  ...
  out.block(
    [
      "↑  v1.0 state detected. Run 'skilltap migrate' to upgrade to v2.0.",
      "",
    ],
    { stream: "stderr" },
  );
  ...
}
```

**Change to**:
```typescript
// Legacy-state soft hint: if pre-state.json markers exist and no state.json
// is present yet, suggest `skilltap migrate`.
async function runLegacyStateDetectionNotice(): Promise<void> {
  ...
  out.block(
    [
      "↑  Legacy state detected. Run 'skilltap migrate' to upgrade.",
      "",
    ],
    { stream: "stderr" },
  );
  ...
}
```

Update the caller at line ~133 from `runV1DetectionNotice()` to `runLegacyStateDetectionNotice()`.

**File**: `packages/core/src/state/schema.ts:29`

**Current**: `// Unified state file for v2.0 — replaces installed.json + plugins.json.`

**Change to**: `// Unified state file. Stored at ~/.config/skilltap/state.json (global) and <projectRoot>/.agents/state.json (project).`

**File**: `packages/cli/src/commands/migrate.ts`

| Line | Current | Change to |
|---|---|---|
| 10 | `description: "Migrate v1.0 setup to v2.0 (one-shot)."` | `description: "Migrate legacy config and state to current format (one-shot)."` |
| 54 | `out.raw(\`${ansi.green("✓")} Already on v2.0. Nothing to do.\n\`);` | `out.raw(\`${ansi.green("✓")} Already migrated. Nothing to do.\n\`);` |
| 58 | `out.raw(\`\n${ansi.bold("skilltap migrate")} — v1.0 → v2.0\n\n\`);` | `out.raw(\`\n${ansi.bold("skilltap migrate")} — legacy → state.json\n\n\`);` |

**File**: `packages/cli/src/completions/zsh.ts:18`

**Current**: `'migrate:Migrate v0.x setup to v2.x'`

**Change to**: `'migrate:Migrate legacy setup to current state'`

**File**: `packages/cli/src/completions/fish.ts:20`

**Current**: `complete -c skilltap -n '__fish_use_subcommand' -a migrate -d 'Migrate v0.x setup to v2.x'`

**Change to**: `complete -c skilltap -n '__fish_use_subcommand' -a migrate -d 'Migrate legacy setup to current state'`

**Acceptance Criteria**:
- [ ] `grep -rnE "v[12]\.[0x]\b" packages/ --include="*.ts" \| grep -v test \| grep -v "// .*"` returns empty (or only matches schema versions like `version: 2`).
- [ ] `bun test packages/cli/src/commands/migrate.test.ts` passes.
- [ ] No string in user-visible output mentions specific version numbers like `v1.0` or `v2.0`.

---

### Unit 11: Strip phase-numbered comments from tests

**Files** (replace the `Phase 31c-c-2d-1` prefix in each):

| File:line | Current | Change to |
|---|---|---|
| `packages/core/src/doctor.test.ts:389` | `test("--fix removes orphan records from canonical state (Phase 31c-c-2d-1: state.json)", async () => {` | `test("--fix removes orphan records from state.json", async () => {` |
| `packages/core/src/doctor.test.ts:392` | `// Phase 31c-c-2d-1: state.json is canonical. Orphan record is now\n  // tracked here; saveSkillState (via --fix) writes here too.` | `// state.json is canonical; saveSkillState (via --fix) writes here.` |
| `packages/core/src/plugin/install.test.ts:222` | `// Phase 31c-c-2d-1: state.json is the canonical store.` | `// state.json is the canonical store.` |
| `packages/core/src/plugin/state.test.ts:113` | `test("creates .agents/ dir for project scope (writes state.json post-cutover)", async () => {` | `test("creates .agents/ dir for project scope", async () => {` |
| `packages/core/src/plugin/state.test.ts:117` | `// Phase 31c-c-2d-1: savePlugins writes to state.json, not plugins.json.` | `// savePlugins writes to state.json.` |
| `packages/cli/src/e2e-v2.test.ts:112–115` | `// Phase 31c-c-2d-1: install writes ONLY to state.json. installed.json\n// is no longer maintained (it's read-fallback only for unmigrated\n// v0.x users). CLAUDE.md "v2.1 conventions": "Don't re-introduce\n// installed.json writes; the dual-write layer was deleted in Refactor 2."` | `// install writes only to state.json.` |

**Acceptance Criteria**:
- [ ] `grep -rn "Phase 31c\|Phase [0-9]\+\b" packages/ --include="*.ts"` returns empty.
- [ ] `bun test packages/core/src/doctor.test.ts packages/core/src/plugin/` passes.

---

### Unit 12: Rename `e2e-v2.test.ts` → `e2e.test.ts`

**Operation**: `git mv packages/cli/src/e2e-v2.test.ts packages/cli/src/e2e.test.ts`

**Then update header** (lines 1–12):

**Current**:
```typescript
/**
 * v2.0 end-to-end test (Phase 38.5).
 *
 * Walks the canonical v2 journey as a real CLI subprocess:
 *   clean init  →  install (writes manifest + lockfile + state)
 *               →  status dashboard
 *               →  doctor (must run cleanly)
 *               →  fresh-clone sync (manifest+lockfile only → reinstalls)
 *               →  migrate (v1 installed.json → v2 state.json)
 *
 * Tests run sequentially and share homeDir/configDir/projectRoot.
 */
```

**Change to**:
```typescript
/**
 * End-to-end test: canonical CLI journey (install → status → doctor → sync → migrate).
 * Tests run sequentially and share homeDir/configDir/projectRoot.
 */
```

**Acceptance Criteria**:
- [ ] `find packages -name "e2e-v2.test.ts"` returns empty.
- [ ] `find packages -name "e2e.test.ts"` returns the renamed file.
- [ ] `bun test packages/cli/src/e2e.test.ts` passes.

---

### Unit 13: Drop reference to `e2e-v2-redesign.test.ts` semantic if applicable

If `packages/cli/src/e2e-v2-redesign.test.ts` exists (it ran in gate 29), examine its contents. If it tests the same canonical journey, fold into `e2e.test.ts`; if distinct (e.g., specifically tests migrate behavior end-to-end), rename to a content-descriptive name like `e2e-migrate.test.ts`.

**Implementation Notes**: The previous validation noted gate 29 ran `e2e-v2-redesign.test.ts` with 13/13 pass. Don't blindly merge — read the file first.

**Acceptance Criteria**:
- [ ] No test filename contains `v2` or `redesign` as a versioning suffix (only literal feature/behavior names).
- [ ] `bun test` passes.

---

### Unit 14: Clean up CLAUDE.md / AGENTS.md key-docs index

**File**: `.claude/CLAUDE.md` (and identical updates to `AGENTS.md`)

**Step 14.1**: Lines 9–17 currently read:
```markdown
## Key Docs

Read these before making architectural decisions:
- docs/SPEC.md — exact behavior, CLI commands, file formats, algorithms, edge cases
- docs/ARCH.md — module boundaries, tech decisions, data flow
- docs/UX.md — CLI reference, flag combos, prompt flows
- docs/ROADMAP.md — phase plan with dependency graph (v0.1–v2.2 done)
- docs/VISION.md — motivation, design principles, V2 direction
- docs/SECURITY.md — security model
- docs/PROGRESS.md — autopilot tracking: phase status, decision log, deviations
- docs/designs/completed/phase-{N}.md — per-phase design docs produced before implementation
```

**Change to**:
```markdown
## Key Docs

Read these before making architectural decisions:
- docs/SPEC.md — exact behavior, CLI commands, file formats, algorithms, edge cases
- docs/ARCH.md — module boundaries, tech decisions, data flow
- docs/UX.md — CLI reference, flag combos, prompt flows
- docs/ROADMAP.md — current state and deferred work
- docs/VISION.md — motivation and design principles
- docs/SECURITY.md — security model
```

(Drop the "PROGRESS.md" line and the "designs/completed/" line — both refer to artifacts being deleted in Unit 24. Drop the `(v0.1–v2.2 done)` and `V2 direction` parentheticals.)

**Step 14.2**: Lines 19–34 (the `## v2.2 conventions` block):

**Current**:
```markdown
## v2.2 conventions

skilltap shipped a full V2 cutover in v2.2.0. Key conventions:

- **CLI surface**: `install <type> <source>` where type is `skill | plugin | mcp`. ...
- **Non-interactive**: TTY detection + `--yes` + `--json`. No `--agent` flag, no `SKILLTAP_AGENT` env var, no `[agent]` or `[agent-mode]` config.
- **Flat `[security]` block**: `scan` (`semantic|static|none`), `on_warn` (`prompt|fail|install`), `trust` (glob array matched against tap name or source URL).
- **`[scanner]` block** (operational, separate from policy): `agent_cli`, `ollama_model`, `threshold`, `max_size`.
- **`composePolicy`** in `core/src/policy/` is the canonical resolver. No per-mode branching, no preset resolution, no override array.
- **state.json** is the only state store. Pre-v2.2 configs/state files trigger a hard error pointing at `skilltap migrate`. No silent fallback anywhere.
- **skilltap.toml + skilltap.lock** project manifest gained `[[mcps]]` + `[[mcps.lock]]` tables in v2.2. Sync reconciles skills, plugins, and mcps.
- **Smart scope default**: inside a git repo, `install` defaults to `project`; outside, `global`. The inferred scope is reported in the install output.
- **`Output` interface**: all output goes through `setupOutput(args)` in CLI commands.
- **HTTP registry adapter removed** — taps are git-only.

When adding new code, write against `state.json` directly. Don't re-introduce `installed.json` writes or any per-mode agent branching.
```

**Change to**:
```markdown
## Conventions

- **CLI surface**: `install <type> <source>` where type is `skill | plugin | mcp`. `remove <type> <name>`, `update [type] [name]`, `toggle [type] [name[:component]]`. `adopt`, `doctor`, `toggle` for adoption / verification / state changes.
- **Non-interactive**: TTY detection + `--yes` + `--json`. No `--agent` flag, no `SKILLTAP_AGENT` env var.
- **Flat `[security]` block**: `scan` (`semantic|static|none`), `on_warn` (`prompt|fail|install`), `trust` (glob array matched against tap name or source URL).
- **`[scanner]` block** (operational, separate from policy): `agent_cli`, `ollama_model`, `threshold`, `max_size`.
- **`composePolicy`** in `core/src/policy/` is the canonical resolver.
- **state.json** is the only state store. `loadConfig` hard-fails on legacy shapes pointing at `skilltap migrate`.
- **skilltap.toml + skilltap.lock** carry `[[mcps]]` + `[[mcps.lock]]` tables. Sync reconciles skills, plugins, and mcps.
- **Smart scope default**: inside a git repo, `install` defaults to `project`; outside, `global`. The inferred scope is reported in the install output.
- **`Output` interface**: all output goes through `setupOutput(args)` in CLI commands.
- **Taps are git-only.**

When adding new code, write against `state.json` directly. Do not introduce `installed.json` writes or any per-mode agent branching.
```

**Acceptance Criteria**:
- [ ] `grep -n "v2\.[0-9]\|V2 direction\|v0\.x\|cutover\|previously" .claude/CLAUDE.md AGENTS.md` returns empty.
- [ ] `## Conventions` heading present (no "v2.2 conventions").
- [ ] `docs/PROGRESS.md` and `designs/completed/` references deleted from key-docs list.

---

### Unit 15: Rewrite `docs/SPEC.md` to current-state framing

**Step 15.1**: Drop version-stamped lead-in.

| Line | Current | Change to |
|---|---|---|
| 3 | `> Canonical behavioral specification for skilltap v2.2.` | `> Canonical behavioral specification for skilltap.` |
| 32 | `22. [Removed in v2.2](#removed-in-v22)` | (delete TOC entry) |
| 387 | `HTTP taps were removed in v2.0; tap add always treats the URL as git.` | `tap add treats the URL as git. There is no HTTP tap support.` |
| 467 | `# ~/.config/skilltap/config.toml — V2` | `# ~/.config/skilltap/config.toml` |
| 521 | `### Hard-fail on legacy shapes` | `### Schema enforcement` |
| 537 | `accepts only V2 keys` | `accepts only the keys defined above` |
| 568 | `# Standalone MCP servers — first-class manifest entries (added in v2.2).` | `# Standalone MCP servers — first-class manifest entries.` |
| 646 | `The native v2.x publish format is **TOML**.` | `The native publish format is **TOML**.` |
| 646–648 | `Existing .claude-plugin/plugin.json and .codex-plugin/plugin.json remain readable inputs (skilltap normalizes them internally).` | `.claude-plugin/plugin.json and .codex-plugin/plugin.json are readable inputs (skilltap normalizes them internally).` |
| 696–697 | `state.json is the only canonical state store. There is no fallback path — pre-v2.2 installed.json and plugins.json are read **only** by migrate.` | `state.json is the only canonical state store. The migrate command reads legacy files; nothing else does.` |

**Step 15.2**: Move Migration content to dedicated location.

The current `## Migration` section (lines 821–905) belongs on the migrate command's documentation page, not in SPEC. Options:
- (a) Move to `docs/MIGRATION.md` (new file).
- (b) Move into the `## migrate` command-section content (lines 200ish in the CLI Commands section — adjust). 
- (c) Move into `website/guide/migrate.md` (new website page).

**Choice**: (b) — fold the translation tables and back-up filename rules into the `migrate` command's own SPEC entry under `## CLI Commands`. The standalone `## Migration` section is then deleted. Cross-references to `#migration` need updating to `#migrate`.

**Step 15.3**: Drop `## Removed in v2.2` appendix (lines 2037–2070). 34 lines deleted plus the L32 TOC entry.

**Step 15.4**: Sweep remaining "v2"/"V2"/"v0.x"/"pre-v2"/"legacy" markers. Use `grep -nE 'v[0-9]+\.[0-9x]+|\bV2\b|legacy|pre-v|previously|formerly' docs/SPEC.md` and update each hit to neutral current-state phrasing.

**Acceptance Criteria**:
- [ ] `grep -nE 'in v[0-9]\.|V2\b|pre-v|previously|formerly' docs/SPEC.md` returns empty (zero matches outside the migrate section).
- [ ] `grep -n "## Removed in" docs/SPEC.md` returns empty.
- [ ] All anchors `#migration`/`#removed-in-v22` are deleted from the doc; no inbound link inside SPEC dangles.
- [ ] SPEC line count drops by ~80 lines (Removed-in-v22 + Migration consolidation).

---

### Unit 16: Rewrite `docs/ARCH.md` to current-state framing

**Targets** (per the audit):

| Line | Current | Change to |
|---|---|---|
| 96 | tree comment `# Migration from v0.x and pre-V2 setups` | `# Migration` |
| 98 | `# v0.x + pre-V2 [security.*]/[agent-mode] → V2 [security] + [scanner]` | `# Translates legacy [security.*]/[agent-mode] keys` |
| 99 | `# legacy installed.json + plugins.json → state.json (preserves mcpServers)` | `# Translates legacy installed.json + plugins.json → state.json` |
| 106 | `# ConfigSchema (V2: flat [security], [scanner], etc.)` | `# ConfigSchema (flat [security], [scanner], etc.)` |
| 107 | `installed.ts          # legacy schemas, migrate-only (v0.x InstalledJsonSchema)` | (delete after Unit 5; the schemas move to migrate/legacy-schemas.ts) |
| 108 | `plugins.ts            # legacy schemas, migrate-only (v0.x PluginsJsonSchema)` | (delete) |
| 163 | `# PluginManifestV2 → existing PluginManifest` | `# SkilltapPluginManifest → PluginManifest` |
| 290 | `Production code reads/writes state.json; pre-V2 installed.json and plugins.json are not read at runtime — loadConfig hard-fails on legacy shapes with a hint pointing at skilltap migrate.` | `Production code reads and writes state.json. loadConfig validates against the current schema and rejects unknown shapes.` |
| 360 / 573 | `### Migrate Module` content describing v0.x and pre-V2 | Keep the section as the canonical migrate-module reference (it intrinsically describes legacy translation). Drop only the explicit "v0.x AND pre-V2" framings — say "legacy" once. |
| 391 | `agent-plugins/codex.ts — Stub. Codex doesn't have a published marketplace yet; the file holds the slot for future support.` | `agent-plugins/codex.ts — Stub.` (drop the future-support narrative) |
| 435 | `**.skilltap/<name>.toml** — \`PluginManifestV2Schema\`` | `**.skilltap/<name>.toml** — \`SkilltapPluginManifestSchema\`` (Unit 6) |
| 442 | `The legacy installed.json and plugins.json schemas (schemas/installed.ts, schemas/plugins.ts) are kept for migrate-only use. Production code does not read or write them.` | (delete; after Unit 5 these schemas don't live in `schemas/` anymore) |
| 668 | `## Removed-Command Hints` | `## Removed-Command Errors` |
| 670 | `... each exit non-zero with an explicit replacement hint instead of falling through to citty's generic "unknown command":` | `Six retired command names exit non-zero with an explicit replacement hint:` |
| 681 | `The mcp: URL prefix was removed; type is explicit via install mcp <source>.` | `Type is explicit via install mcp <source>.` |
| 718 | Decision-log row alternative-considered column | Keep — it's a decision log, the historical comparison is the column's purpose. |
| 725 | `Removing the parallel agent-mode runtime cuts duplicated orchestration; ...` | `A single runtime cuts duplicated orchestration; ...` |

**Acceptance Criteria**:
- [ ] `grep -nE "from v0\.x|pre-V2|V2\b\|previously|formerly|was replaced" docs/ARCH.md` returns empty (or only inside the `### Migrate Module` section which intrinsically describes legacy translation).
- [ ] `## Removed-Command Errors` heading present; `## Removed-Command Hints` not.

---

### Unit 17: Rewrite `docs/VISION.md` to current-state framing

| Line | Current | Action |
|---|---|---|
| 143 | `migrate — one-shot upgrade for pre-V2 setups.` | Replace with `migrate — translate legacy config and state to current format.` |
| 231 | `### Future — community trust signals` | Rename to `### Community trust signals` |
| 295 | `## Considered and removed` (heading) | DELETE entire section through line 311 (the bulleted "removed" rationale) |

**Acceptance Criteria**:
- [ ] `grep -nE "Considered and removed|HTTP registry adapter.*shipped|was removed|was retired|previously" docs/VISION.md` returns empty.
- [ ] VISION.md drops by roughly 20 lines.

---

### Unit 18: Rewrite `docs/UX.md` to current-state framing

**Step 18.1**: Lead-in (line 3).

**Current**: `Dense CLI reference for the v2.0 redesign — command tree, flag combinations, prompt flows, and common workflows. This is the canonical CLI reference; there is no legacy section.`

**Change to**: `Dense CLI reference for skilltap — command tree, flag combinations, prompt flows, and common workflows.`

**Step 18.2**: Migration callouts.

| Line | Current | Action |
|---|---|---|
| 1000 | `... all four were removed in v2.2 (see [SPEC.md → Removed in v2.2](./SPEC.md#removed-in-v22)).` | Strip the parenthetical and the "all four were removed in v2.2" clause. |
| 1162 | `### Migrating from v2.1 or earlier` (heading) | Rename to `### Migration` (and prune content to the current command's behavior) |
| 1174 | `## Legacy Commands` (heading) | Rename to `## Removed-command errors` |
| 1176 | `Five commands were retired in v2.2. ...` | `These commands exit non-zero with explicit replacement hints:` |
| 1186 | `The v0.x skilltap skills subgroup ... is also gone — every operation lifted to the top level (info, adopt, move, remove).` | `The skilltap skills subgroup is not present — operations live at the top level (info, adopt, move, remove).` |
| 1196 | error-table row `Config schema is pre-v2.2 — run skilltap migrate` | Keep — this is a literal error message string the binary emits. (Verify it's still emitted with this exact text after Unit 10's neutralization. If Unit 10 changed the user-facing error string, sync this row to match.) |
| 1200 | error-table row `Error: HTTP tap not supported \| v0.x config has type = "http" tap \| Remove HTTP tap or run skilltap migrate` | Same: keep iff it matches the binary's actual error output post-cleanup. |

**Step 18.3**: Sweep self-update example versions (lines 1076–1082). Replace `v2.1.1` / `v2.2.0` literals with placeholder `v<old>` / `v<new>` to avoid baking specific releases into the doc.

**Acceptance Criteria**:
- [ ] `grep -nE "v[0-9]\.[0-9x]\b|legacy commands|retired in|v0\.x skilltap" docs/UX.md` returns empty (or only inside the migrate-command's own subsection).
- [ ] `## Removed-command errors` heading present; `## Legacy Commands` not.

---

### Unit 19: Rewrite `docs/SECURITY.md` if needed

The audit reported SECURITY.md is clean. Confirm with `grep -nE 'in v[0-9]|previously|legacy|pre-v|was removed|was replaced' docs/SECURITY.md` — expect empty. If hits surface during implementation, neutralize them with the same rules as Unit 15.

**Acceptance Criteria**:
- [ ] `grep -nE 'in v[0-9]|previously|legacy|pre-v|was removed|was replaced' docs/SECURITY.md` returns empty.

---

### Unit 20: Rewrite `README.md`

| Line | Current | Action |
|---|---|---|
| 122–124 | `If you're upgrading from any pre-v2.2 release, run skilltap migrate once...` | DELETE entire paragraph (3 lines) |
| 309 | `- **Coming from v2.1 or earlier?** Run skilltap migrate to translate config and state files.` | DELETE bullet |

**Acceptance Criteria**:
- [ ] `grep -nE "pre-v[0-9]|coming from v|upgrading from|v[0-9]\.[0-9x]\b" README.md` returns empty.
- [ ] README drops by ~5 lines.

---

### Unit 21: Rewrite website pages

**File**: `website/guide/getting-started.md:89–91`

DELETE the `::: info Coming from v0.x or v2.1?` block (3 lines).

**File**: `website/guide/what-is-skilltap.md:58`

DELETE the `**One-shot legacy migration.** Coming from a pre-v2.2 install? ...` feature bullet.

**File**: `website/guide/doctor.md`

| Line | Current | Action |
|---|---|---|
| 39 | `If the loader rejects the file because of a legacy schema marker ([security.human], [[security.overrides]], [agent-mode], etc.), the failure message points at skilltap migrate — loadConfig hard-fails on legacy keys instead of silently translating.` | Replace with `If the loader rejects the file because the schema doesn't match, the failure message points at skilltap migrate.` |
| 104 | `**14. legacy file orphans** — Detects leftover installed.json / plugins.json / pre-v2.2 config blocks. ...` | Replace with `**14. legacy file orphans** — Detects leftover installed.json / plugins.json files.` |

**File**: `website/guide/taps.md`

| Line | Action |
|---|---|
| 9 | DELETE the `> HTTP registry taps were removed in v2.0...` blockquote intro line |
| 49 | DELETE the parenthetical `(Pre-v2.0, the type column also distinguished git from http taps; ...)` |
| 215–221 | DELETE entire `## HTTP registry taps (removed in v2.0)` section (7 lines) |

**File**: `website/reference/cli.md`

| Line | Action |
|---|---|
| 34 | After Unit 9, this `info and status deliberately retain the legacy boolean pair...` paragraph is irrelevant. DELETE. |
| 341 | `- Converts legacy HTTP taps into errors with a list for manual handling` → `- Errors on HTTP taps with a list for manual handling` |
| 343 | `After migration, loadConfig hard-fails on any remaining legacy markers — there is no silent translation at runtime, the migration is the explicit upgrade path.` | DELETE sentence. |
| 366 | After Unit 9, this `Note: status retains the legacy boolean...` line is irrelevant. DELETE. |
| 397 | After Unit 9, this `Note: info retains the legacy boolean...` line is irrelevant. DELETE. |
| 462 | `config set is restricted to settable keys (the V2 surface).` → `config set is restricted to settable keys.` |
| 501–521 | DELETE entire `## Removed in v2.2` section (21 lines) |

**Acceptance Criteria**:
- [ ] `grep -rnE "(Coming from v|legacy migration|deliberately retain|removed in v|was removed in)" website/ --include="*.md"` returns empty (excluding `website/changelog.md` which is exempt).

---

### Unit 22: Update website theme components and sidebar config

**Files** (`website/.vitepress/theme/`):

Audit components for any `v2.2`/`V2`/`legacy`/`previously` strings used in landing copy. The previous audit already swept these for the v2.2 cutover; verify with:

```bash
grep -rnE "(v[0-9]\.[0-9x]\b|V2\b|legacy|previously|was removed)" website/.vitepress/ --include="*.vue" --include="*.ts" --include="*.json"
```

Any hits → update to current-state framing.

**Sidebar/nav config** (`website/.vitepress/config.ts` or similar): if a "Removed in v2.2" or "Migration" link points at the deleted SPEC anchor, remove or retarget.

**Acceptance Criteria**:
- [ ] Above grep returns empty.
- [ ] `bun run --filter @skilltap/website docs:build` (or equivalent VitePress build) succeeds; no broken anchor warnings.

---

### Unit 23: Trim ROADMAP to current state

**File**: `docs/ROADMAP.md`

**Goal**: replace 961 lines with ~40 lines: header, current-state snapshot, deferred-work list. Phases 0–46 + dependency graphs are deleted (git holds them).

**New file content**:

```markdown
# Roadmap

The current state of skilltap, plus what hasn't been scheduled.

## Current state

skilltap is at v2.2.x — V2 surface canonical: typed `install <type> <source>` family, flat `[security]` + `[scanner]` config, `state.json` as the single state store, `skilltap.toml` + `skilltap.lock` with skills/plugins/mcps tables, smart-scope default, TUI dashboard, plugin capture, Claude Code plugin adoption.

There is no in-flight phase. The next release line will start a new entry in this file.

## What's Deferred (no scheduled version)

Real future-work items, not blocked on technical issues — they're either large efforts, platform-specific features, or design problems that haven't been prioritized.

- Windows support
- Linux distro packages (.deb, .rpm, AUR, Nix)
- `security.require_provenance` config option (block unverified skills)
- Direct LLM API integrations for semantic scan (Anthropic API, OpenAI API — bypassing CLI)
- Plugin for popular editors (VS Code extension)
- Skill dependency system
- SBOM generation for installed skills
- Plugin hooks support (Claude Code hooks.json)
- Plugin LSP server support (Claude Code .lsp.json)
- Plugin commands support (Claude Code commands/*.md)
- Agent definitions for non-Claude-Code platforms (when other agents adopt the format)
- Plugin user config / secrets management (Claude Code userConfig with keychain)
```

**Implementation Notes**:
- The "Removed entries (no longer planned)" list at the bottom of the current ROADMAP (line 961) is also deleted — that's history.
- Avoid mentioning version numbers like "v2.2.x" if it ages poorly; consider the placeholder "the current released minor version." Decide at write-time based on how close to release we are.

**Acceptance Criteria**:
- [ ] `wc -l docs/ROADMAP.md` reports ≤ 50 lines.
- [ ] `grep -n "Phase\|## v[0-9]" docs/ROADMAP.md` returns empty (no phase headings, no version-block headings).
- [ ] `grep -n "## What's Deferred" docs/ROADMAP.md` returns one match.

---

### Unit 24: Mass-delete design archive

**Goal**: `docs/designs/completed/` is gone. `docs/designs/v2.2-cleanup.md` is gone. `docs/designs/v2.2-cleanup/` is gone. After implementation, only `docs/designs/cleanup-current-state.md` (this file) remains until it too gets archived/deleted post-implementation.

**Pre-flight check**: confirm no inbound references.

```bash
grep -rn "designs/completed\|v2\.2-cleanup\|phase-[0-9]\+\.md" docs/ website/ packages/ README.md AGENTS.md .claude/ 2>/dev/null
```

Expected (after Units 14, 15, 16): empty. If hits remain, fix them in their respective Units before this delete.

**Operations**:

```bash
rm -rf docs/designs/completed
rm -f docs/designs/v2.2-cleanup.md
rm -rf docs/designs/v2.2-cleanup
```

**Acceptance Criteria**:
- [ ] `ls docs/designs/` lists only `cleanup-current-state.md`.
- [ ] `find docs/designs -name "phase-*.md"` returns empty.
- [ ] `find docs/designs -name "v2.2-cleanup*"` returns empty.
- [ ] No grep hit anywhere in repo for `designs/completed/` or `v2.2-cleanup`.
- [ ] `bun test` still passes (no test should reference design docs).

---

### Unit 25: Regenerate `llms-full.txt`

**File**: `website/public/llms-full.txt`

**Operation**: rerun the website's `llms-full.txt` generator (per the v2.2 cutover wave 4e workflow) so the file reflects the rewritten guide/reference pages.

**Implementation Notes**: locate the generator script (likely a `bun run` task in `website/package.json` or root `package.json`). Per Wave 4e of the prior cleanup, the file is ~4000 lines and is a concatenation of website source pages.

**Acceptance Criteria**:
- [ ] `grep -nE "(Coming from v|legacy migration|deliberately retain|removed in v|pre-v[0-9]|## Removed in v|HTTP registry taps \(removed)" website/public/llms-full.txt` returns empty.
- [ ] File regenerates deterministically (re-running produces the same output).

---

### Unit 26: Final verification — extended gate set

In addition to the 29-gate verification from the v2.2 cleanup design (still applicable), run these additional gates:

```bash
# G30: No version-stamped framing in foundation docs (excluding changelog and migrate command help)
grep -rnE 'in v[0-9]\.|V2\b|pre-v|previously|formerly|was removed|was replaced' \
  docs/SPEC.md docs/ARCH.md docs/SECURITY.md docs/VISION.md docs/UX.md docs/ROADMAP.md \
  README.md AGENTS.md .claude/CLAUDE.md \
  website/index.md website/guide/ website/reference/
# Expected: empty (or only inside the migrate command's own subsection)

# G31: ROADMAP is trimmed
test "$(wc -l < docs/ROADMAP.md)" -le 50 && echo OK

# G32: Design archive is gone
test ! -d docs/designs/completed && echo OK
test ! -f docs/designs/v2.2-cleanup.md && echo OK
test ! -d docs/designs/v2.2-cleanup && echo OK

# G33: No `skilltap list` or `skilltap skills` references in CLI source
grep -rnE "'skilltap (list|skills)'|\"skilltap (list|skills)\"|\`skilltap (list|skills)\`" \
  packages/ --include="*.ts" | grep -v test
# Expected: empty

# G34: No InstalledJson/PluginsJson public-API consumers
grep -rn "InstalledJson\|PluginsJson" packages/core/src/index.ts
grep -rn "type InstalledJson\|type PluginsJson" packages/ --include="*.ts" | grep -v migrate
# Expected: empty

# G35: plugin-v2/ directory is gone
test ! -d packages/core/src/plugin-v2 && echo OK
grep -rn "PluginV2\|pluginV2\|PluginManifestV2\|from.*plugin-v2" packages/ --include="*.ts"
# Expected: empty

# G36: Phase-numbered comments stripped
grep -rn "Phase 31c\|// Phase [0-9]\+\b" packages/ --include="*.ts"
# Expected: empty

# G37: --global / --project boolean pair not in any command
grep -rnE 'project:\s*\{\s*type:\s*"boolean"|global:\s*\{\s*type:\s*"boolean"' \
  packages/cli/src/commands/ --include="*.ts"
# Expected: empty

# G38: Test suite + binary verification (carry forward from v2.2 gates)
bun test
bun run build
bun run verify:binary
bun run verify:binary:tests
bun run lint
# Expected: all green
```

**Acceptance Criteria**:
- [ ] All gates G1–G29 from the v2.2 cleanup design still pass.
- [ ] All gates G30–G38 pass.
- [ ] Working tree is clean modulo the design doc itself + `install` build artifact (gitignored).

---

## Implementation Order

```
Phase 1: User-facing bug fixes (parallel-safe within phase)
  Unit 1 ─┐
  Unit 2 ─┼─→ Unit 3 (depends on Unit 2's hint rewrites being done)
  Unit 4 ─┘

Phase 2: API residue (parallel-safe within phase, runs after Phase 1)
  Unit 5 (TRICKIEST — schema relocation + signature change)
  Unit 6 (independent of Unit 5)
  Unit 7 (trivial — independent)
  Unit 8 (trivial — independent)

Phase 3: Lock-in (after Phase 2)
  Unit 9 (independent of Unit 5/6/7/8 by file but should land after API residue settles)

Phase 4: Polish (after Phase 3)
  Unit 10
  Unit 11
  Unit 12
  Unit 13
  Unit 14

Phase 5: Foundation docs (after Phase 4 — docs describe code state)
  Unit 15 (SPEC) ─┐
  Unit 16 (ARCH) ─┼─ parallel
  Unit 17 (VISION) ─┤
  Unit 18 (UX) ─┤
  Unit 19 (SECURITY — verify-only) ─┤
  Unit 20 (README) ─┤
  Unit 21 (website pages) ─┤
  Unit 22 (theme components) ─┘

Phase 6: ROADMAP trim (after Phase 5)
  Unit 23

Phase 7: Mass-delete (after Phase 6)
  Unit 24

Phase 8: Final
  Unit 25 (llms-full.txt regen)
  Unit 26 (verification)
```

---

## Testing

### Test infrastructure assumptions
- `runSkilltap` (pipe) for testing CLI exit codes and stderr text.
- `runInteractive` (PTY) for clack-rendered prompts (the `update.ts` prompt in Unit 1 lands in this category — but no test asserts its exact text today).
- `bun test` synchronously, foreground only.

### Per-unit test approach

- **Unit 1**: existing `update.test.ts` covers the orphan-removal flow. Verify it still passes; the prompt text isn't asserted on.
- **Unit 2**: existing tests for `disable.ts`, `remove.ts`, `move.ts`, `adopt.ts`, `update.ts` cover the error paths. Add ONE assertion in each test that the hint contains `"skilltap status"` (not `list`/`skills`) — or rely on the grep-based acceptance criteria to catch regressions.
- **Unit 3**: pure doc edit; no test impact. Manually verify SPEC renders cleanly (`bun run --filter website docs:build` if VitePress consumes SPEC; otherwise just `grep`).
- **Unit 4**: existing `completions.test.ts` covers the bash/zsh completions. Add an assertion that the `move` block contains `--scope` and not `--global`/`--project`. ~2 line test addition.
- **Unit 5**: full test suite is the gate. Specifically:
  - `packages/core/src/orphan.test.ts`
  - `packages/core/src/update.test.ts`
  - `packages/core/src/doctor.test.ts` (especially `--fix` paths reading state)
  - `packages/core/src/plugin/state.test.ts`
  - `packages/cli/src/e2e.test.ts` (the renamed e2e)
  - `packages/cli/src/e2e-v2-redesign.test.ts` (migrate path — must still pass; legacy schemas now in migrate/legacy-schemas.ts)
- **Unit 6**: `packages/core/src/skilltap-plugin/{schema,normalize}.test.ts` (renamed from `plugin-v2`); plus `manifest/publish.test.ts` and `plugin/detect.test.ts`.
- **Unit 7**: `install-callbacks.test.ts` is the only consumer of the deleted symbol.
- **Unit 8**: bench is not under bun:test; verify by running it manually and confirming it loads 100 records.
- **Unit 9**: `info.test.ts` and `status.test.ts`. The single existing test that uses `--global` (status.test.ts:138) updates to `--scope global`.
- **Unit 10–14**: existing CLI subprocess tests for `migrate`, `index.ts` startup hint, completions; no new tests, just verify nothing asserts on the old strings.
- **Unit 15–22**: pure doc edits. Manually review rendered website. Run VitePress build to catch broken anchors.
- **Unit 23**: pure doc edit.
- **Unit 24**: confirm `bun test` still passes (no test imports a design doc).
- **Unit 25**: regenerate; confirm idempotent.
- **Unit 26**: full 38-gate run.

### New tests
- None required. Existing coverage is sufficient; the cleanup is subtractive.

### Tests to delete
- None. (`packages/cli/src/e2e-v2.test.ts` is renamed in Unit 12, not deleted.)

### Tests to update
- `packages/cli/src/commands/status.test.ts:138` — `--global` → `--scope global` (Unit 9).
- `packages/cli/src/ui/install-callbacks.test.ts:154–158` — drop `resolveAgentForAgentMode` clause (Unit 7).
- `packages/core/src/doctor.test.ts:389,392` — drop `Phase 31c-c-2d-1:` prefix (Unit 11).
- `packages/core/src/plugin/install.test.ts:222` — same (Unit 11).
- `packages/core/src/plugin/state.test.ts:113,117` — same (Unit 11).
- `packages/cli/src/e2e.test.ts:112–115` — neutralize the comment block (Unit 11/12).
- `packages/core/src/skilltap-plugin/schema.test.ts` + `normalize.test.ts` — symbol-name updates (Unit 6).

---

## Verification Checklist

(Run in order; each gate must pass before the next phase starts.)

After Phase 1 (Units 1–4):
```bash
bun test packages/core/src/disable.test.ts packages/core/src/remove.test.ts \
         packages/core/src/move.test.ts packages/core/src/adopt.test.ts \
         packages/core/src/update.test.ts packages/cli/src/commands/completions.test.ts
grep -rnE "'skilltap (list|skills)'" packages/ --include="*.ts" | grep -v test
# Expected: empty
```

After Phase 2 (Units 5–8):
```bash
bun test
bun run build
bun run verify:binary
grep -rn "InstalledJson\|PluginsJson" packages/core/src/schemas/ --include="*.ts"
grep -rn "PluginV2\|pluginV2" packages/ --include="*.ts"
# Expected: all empty (except legacy-schemas.ts inside migrate/)
```

After Phase 3 (Unit 9):
```bash
bun test packages/cli/src/commands/info.test.ts packages/cli/src/commands/status.test.ts
grep -rnE 'project:\s*\{\s*type:\s*"boolean"|global:\s*\{\s*type:\s*"boolean"' \
  packages/cli/src/commands/ --include="*.ts"
# Expected: empty
```

After Phase 4 (Units 10–14):
```bash
bun test
grep -rn "Phase 31c\|// Phase [0-9]\+\b" packages/ --include="*.ts"
grep -nE "v1\.0|v2\.0\b" packages/cli/src/index.ts packages/cli/src/commands/migrate.ts \
  packages/core/src/state/schema.ts packages/cli/src/completions/zsh.ts \
  packages/cli/src/completions/fish.ts | grep -v "// "
# Expected: empty
```

After Phase 5 (Units 15–22):
```bash
grep -rnE 'in v[0-9]\.|V2\b|pre-v|previously|formerly|was removed|was replaced' \
  docs/SPEC.md docs/ARCH.md docs/SECURITY.md docs/VISION.md docs/UX.md \
  README.md AGENTS.md .claude/CLAUDE.md \
  website/index.md website/guide/ website/reference/
# Expected: empty (or only inside migrate-command sections)

bun run --filter website docs:build  # if applicable
# Expected: 0 broken-link warnings
```

After Phase 6 (Unit 23):
```bash
test "$(wc -l < docs/ROADMAP.md)" -le 50
grep -n "Phase\|## v[0-9]" docs/ROADMAP.md
# Expected: empty
```

After Phase 7 (Unit 24):
```bash
test ! -d docs/designs/completed
test ! -f docs/designs/v2.2-cleanup.md
test ! -d docs/designs/v2.2-cleanup
ls docs/designs/
# Expected output: cleanup-current-state.md
bun test
# Expected: still green
```

After Phase 8 (Units 25–26):
- All gates G1–G29 from the v2.2 cleanup design still pass.
- All gates G30–G38 in Unit 26 pass.
- `bun test` / `bun run build` / `bun run verify:binary` / `bun run verify:binary:tests` / `bun run lint` all green.

---

## Risks

### R1: `InstalledJson`/`PluginsJson` removal breaks an undiscovered consumer
The audit found 8 production-file consumers; tests cover most. Risk: a code path not exercised by tests breaks at runtime. Mitigation: full-suite `bun test` + `bun run verify:binary:tests` after Unit 5. If a consumer surfaces, add it to the unit's edit list before declaring Phase 2 complete.

### R2: `plugin-v2/` rename leaks past barrel re-exports
Risk: an external user of `@skilltap/core` imports `PluginV2*` symbols. Mitigation: this is a major internal rename; the package is monorepo-internal. No external consumers exist outside `packages/cli/`. Search via `grep -rn "PluginV2\|pluginV2" .` (entire repo, not just packages/) before Unit 6 to confirm.

### R3: Doc rewrites break inbound anchors
Risk: deleting `## Removed in v2.2` and `## Migration` sections breaks anchor links from elsewhere (e.g., website nav). Mitigation: pre-flight grep before Unit 24:
```bash
grep -rn "#removed-in-v22\|#migration\|#legacy-commands" docs/ website/ README.md AGENTS.md .claude/
```
Update or delete every match before deleting the targets.

### R4: `info` / `status` `--scope` switch breaks scripts
Risk: external scripts using `--global` or `--project` against `info`/`status`. Mitigation: this design treats both as user surface (per the cleanup mandate). The carve-out was noted as a v2.2 deviation; closing it is intentional. Document in the next changelog entry.

### R5: Test suite has 1 baseline failure (`taps.http-removal.test.ts`)
This is pre-existing per the v2.2 progress doc and is OUT OF SCOPE for this design. Verification gates compare against the same baseline (`fail=1` is acceptable iff the failing test is unchanged).

### R6: `llms-full.txt` regeneration doesn't exist or is non-deterministic
Risk: the regen script is missing or produces drift on each run. Mitigation: locate the script before Unit 25; if non-deterministic, fix it as part of the unit. Per the v2.2 cleanup, Wave 4e regenerated this file successfully — assume the path still works.

### R7: `migrate` command's own SPEC content gets accidentally version-stamped during sweep
Risk: aggressive grep+replace strips legitimate language from the migrate command's own description (which intrinsically describes legacy formats). Mitigation: each Unit's edit list is itemized line-by-line. The migrate-command sections in SPEC + UX are explicitly carved out as legitimate.

### R8: Deleting `docs/designs/completed/` removes content other agents reference
Risk: an agent (this project's own auto-loaded skills, or a contributing dev's tooling) references a phase design doc by path. Mitigation: pre-flight grep across the entire repo for `designs/completed/` (including `.claude/`, `AGENTS.md`, all source). Already covered in Unit 24 pre-flight.

### R9: This design itself is residue once implemented
After implementation, `docs/designs/cleanup-current-state.md` becomes the kind of doc the user wants nuked. Decision deferred — the next design pass should either delete this doc or move it forward into a "current design queue" pattern. Not in scope here.

---

## Notes for the implementer

- This is a strictly subtractive design. No new features, no new APIs (except the `migrate/legacy-schemas.ts` extraction, which is a relocation not an addition).
- Trust the audit data in this design. The line numbers were captured fresh; cross-check with `grep -n` only if a line number doesn't match (file may have shifted since audit).
- Follow `.claude/rules/patterns.md`: Result type, single-source definitions, json-state I/O patterns.
- When in doubt about whether to delete vs neutralize content: **delete**. Per the user's mandate, "current state only" — uncertain content fails toward removal.
- Run `bun test` between Units 5 and 6 (both touch core schemas). Don't batch them.
- Commit at each phase boundary, not between every unit. Phase boundaries are natural commit points.
