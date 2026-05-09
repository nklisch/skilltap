# Design: Phase 42 — Required-Subcommand Install / Remove / Update / Toggle

## Overview

Phase 42 reshapes the four highest-traffic CLI verbs into a uniform typed surface:

- `install <type> <source>` — type required (skill | plugin | mcp). Subcommand groups.
- `remove <type> <name>` — type required. Subcommand groups.
- `update [type] [name]` — type optional. Bare = all; type alone = all of type; type + name = one.
- `toggle [type] [name[:component]]` — type optional. Bare opens a picker.

Drops:
- `mcp:` URL prefix from user input (the prefix lives on internally as a `state.mcpServers[].source` marker, not as a CLI input convention).
- `tap install` interactive picker (redundant — `install skill <name>` already resolves through configured taps).
- `skills/` subcommand group (8 files).
- `plugin/` subcommand group (4 files).
- `enable` / `disable` top-level commands (folded into `toggle` per VISION's "toggle is enough").
- 5 silent aliases in `index.ts` (`list`, `remove`, `info`, `link`, `unlink` without explicit type).

Reduces:
- `InstallOptions` callbacks from 17 → ~10 by merging similar shapes (skill+plugin warnings into one callback, skill+plugin confirm into one callback).

Adds:
- Top-level `info <name>` (auto-detects type from state).
- Top-level `move <name>` (was `skills/move`).
- Top-level `adopt <name>` (was `skills/adopt`; Phase 43 extends with external path support).

CLI command count: 24 entries in `index.ts` today (19 unique + 5 aliases) → 14 entries after Phase 42 (no aliases, install/remove are subcommand groups counted as one each, but each has 3 subcommands underneath).

## Acceptance Criteria

- `bun run dev install` errors with "specify a subcommand: skill, plugin, or mcp" (or citty's native subcommand listing).
- `bun run dev install skill owner/repo` works.
- `bun run dev install plugin owner/repo` works.
- `bun run dev install mcp owner/repo` works.
- `bun run dev remove skill <name>` works; same for plugin and mcp.
- `bun run dev update` updates everything; `update skill` updates all skills; `update skill <name>` updates one.
- `bun run dev toggle` opens a clack picker (Phase 44 will replace with Ink); `toggle plugin <name>:<component>` toggles directly.
- `bun run dev install mcp:foo` errors (or proceeds silently treating `mcp:foo` as a literal source — design choice below).
- `bun run dev tap install` no longer registered (errors as unknown command).
- `bun run dev list` errors as unknown.
- `bun run dev info <name>` works (auto-detects type).
- `grep -rn "mcp:" packages/cli/src/commands/ --include="*.ts" | grep -v ".test.ts"` returns nothing user-facing — internal state markers only.
- `bun test` passes.

## Architectural Options Considered

### Option A — Citty subcommand groups everywhere
`install`, `remove`, `update`, `toggle` all use `subCommands: { skill, plugin, mcp }`. Pro: uniform. Con: doesn't fit `update` and `toggle` whose type is optional.

### Option B — Positional args everywhere
All four use a `type` positional. Pro: uniform. Con: --help less discoverable for required-type commands; users have to read docs to learn valid types upfront.

### Option C — Hybrid (chosen)
- `install` and `remove` → subcommand groups (type required, naturally enforced by citty).
- `update` and `toggle` → optional positional `[type] [name]`.

Pro: each command gets the best UX for its semantics. Citty's --help for subcommand groups lists subcommands cleanly; positional commands show `[type] [name]` placeholders. Con: two patterns to teach. Acceptable.

**Choice: Option C.** The `install` / `remove` commands are the ones a brand-new user types first; subcommand discoverability matters most there. `update` / `toggle` are repeat-use commands where bare invocation is common.

## Trickiest Unit — Designed First

### Unit 1: `install` subcommand restructure

**Files**:
- `packages/cli/src/commands/install/index.ts` (new — citty group)
- `packages/cli/src/commands/install/skill.ts` (new — current install.ts logic, scoped to skill)
- `packages/cli/src/commands/install/plugin.ts` (new — when source resolves to a plugin)
- `packages/cli/src/commands/install/mcp.ts` (new — current mcp-install logic)
- `packages/cli/src/commands/install/shared.ts` (new — extracted helpers: `createInstallCallbacks`, `parseAlsoFlag`, `resolveScope`)
- `packages/cli/src/commands/install.ts` — DELETE

```typescript
// install/index.ts
import { defineCommand } from "citty";
import { skillCommand } from "./skill";
import { pluginCommand } from "./plugin";
import { mcpCommand } from "./mcp";

export const installCommand = defineCommand({
  meta: {
    name: "install",
    description:
      "Install a skill, plugin, or MCP server. Type is required.",
  },
  subCommands: {
    skill: skillCommand,
    plugin: pluginCommand,
    mcp: mcpCommand,
  },
});
```

```typescript
// install/skill.ts
export const skillCommand = defineCommand({
  meta: { name: "skill", description: "Install a skill" },
  args: {
    source: {
      type: "positional",
      required: true,
      description: "Source: tap name, github shorthand, git URL, or local path",
    },
    project: { type: "boolean", description: "Install to project scope", default: false },
    global: { type: "boolean", description: "Install to global scope", default: false },
    also: { type: "string", description: "Comma-separated agent dirs to symlink into" },
    ref: { type: "string", description: "Branch or tag" },
    yes: { type: "boolean", default: false, alias: "y" },
    strict: { type: "boolean", default: false },
    "no-strict": { type: "boolean", default: false },
    semantic: { type: "boolean", default: false },
    "skip-scan": { type: "boolean", default: false },
    quiet: { type: "boolean", default: false },
    json: { type: "boolean", default: false },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: args.quiet });
    return runInstallSkill(args, out);
  },
});

async function runInstallSkill(args, out): Promise<void> {
  // Body extracted from current commands/install.ts runInstall(),
  // scoped to skill flow only:
  //   - No mcp: prefix detection (deleted in Unit 5).
  //   - If source resolves to a plugin manifest, error with hint to use
  //     `install plugin <source>` instead. (Unless --auto-detect flag —
  //     see Implementation Notes.)
  //   - Existing onPluginDetected callback still runs; if user picks "plugin",
  //     suggest using `install plugin` and exit.
  // ...
}
```

```typescript
// install/plugin.ts
export const pluginCommand = defineCommand({
  meta: { name: "plugin", description: "Install a plugin" },
  args: {
    source: { type: "positional", required: true },
    // ... same flag set as skill
  },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: args.quiet });
    return runInstallPlugin(args, out);
  },
});

async function runInstallPlugin(args, out): Promise<void> {
  // Calls installSkill() under the hood (the plugin path is already gated
  // through installSkill via onPluginDetected), but with explicit
  // expectation that source has a plugin manifest. If no manifest detected
  // after clone, error: "expected a plugin manifest in <source>; use
  // `install skill` for skill-only repos".
  // ...
}
```

```typescript
// install/mcp.ts — current commands/install.ts runMcpInstall body, lifted.
export const mcpCommand = defineCommand({
  meta: { name: "mcp", description: "Install a standalone MCP server" },
  args: {
    source: { type: "positional", required: true },
    project: { type: "boolean", default: false },
    global: { type: "boolean", default: false },
    also: { type: "string" },
    yes: { type: "boolean", default: false, alias: "y" },
    quiet: { type: "boolean", default: false },
    json: { type: "boolean", default: false },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: args.quiet });
    return runInstallMcp(args, out);
  },
});
```

**Implementation Notes**:
- `install/shared.ts` exports `createInstallCallbacks(out: Output, args: SharedArgs)` — used by skill and plugin handlers.
- `install/skill.ts` and `install/plugin.ts` both call into `installSkill()` from core, but each scopes the `onPluginDetected` callback differently:
  - skill handler: if a plugin manifest is detected, return "skills-only" silently (or error with hint, per --strict-type discussion below).
  - plugin handler: if no plugin manifest is detected, error.
  - Decision: **strict-type by default**. `install skill` on a plugin repo errors; `install plugin` on a skill-only repo errors. This matches the user's stated direction ("as a human I do like explicitly saying what I am trying to install"). No silent dispatch.
- The `mcp:` prefix detection (5 sites) is replaced by:
  - `install mcp <source>` calls `installMcpServer(source, ...)` directly from core.
  - State storage of MCP servers continues to use the `mcp:` prefix (in `state.mcpServers[].source`) — internal marker, never user-typed.
- The current `install.ts` file is deleted entirely. `index.ts` imports from `./commands/install` (auto-resolves to `install/index.ts`).

**Acceptance Criteria**:
- [ ] `bun run dev install` (no subcommand) → citty error or hint listing skill | plugin | mcp.
- [ ] `bun run dev install skill owner/repo` runs the skill flow.
- [ ] `bun run dev install skill owner/plugin-repo` (a repo with a plugin manifest) errors with hint to use `install plugin`.
- [ ] `bun run dev install plugin owner/skill-repo` (a repo with only SKILL.md) errors with hint.
- [ ] `bun run dev install mcp npm:@scope/pkg` works.
- [ ] `bun run dev install mcp mcp:legacy` (legacy prefix typed by user) errors: "the `mcp:` prefix is removed; just pass the source".
- [ ] `commands/install.ts` no longer exists.
- [ ] `commands/install/` directory contains 5 files (index, skill, plugin, mcp, shared).
- [ ] All install-flow tests pass.

---

## Implementation Units

### Unit 2: `remove` subcommand restructure

**Files**:
- `packages/cli/src/commands/remove/index.ts` (new — citty group)
- `packages/cli/src/commands/remove/skill.ts` (extracted from `skills/remove.ts`)
- `packages/cli/src/commands/remove/plugin.ts` (extracted from `plugin/remove.ts`)
- `packages/cli/src/commands/remove/mcp.ts` (extracted from `skills/remove.ts` runMcpRemove path)

```typescript
export const removeCommand = defineCommand({
  meta: { name: "remove", description: "Remove a skill, plugin, or MCP server" },
  subCommands: {
    skill: skillRemoveCommand,
    plugin: pluginRemoveCommand,
    mcp: mcpRemoveCommand,
  },
});
```

Each subcommand mirrors install: positional `name` + scope flags + `--yes` + `--json`.

The current `commands/skills/remove.ts` had a "list of names; first one starting with `mcp:` triggers MCP path" pattern. Phase 42 splits this:
- `remove skill <name>` → calls `removeSkill(name, scope)` from core.
- `remove plugin <name>` → calls `removeInstalledPlugin(name, scope)` from core.
- `remove mcp <name>` → calls `removeMcpServer(name, scope)` from core.
- `remove <name>` (no type) → not allowed; type required.

Multiple names (current behavior) is preserved per type: `remove skill foo bar` removes both. Same for plugin/mcp.

**Acceptance Criteria**:
- [ ] `remove skill foo` removes skill foo.
- [ ] `remove plugin foo` removes plugin foo and all components (existing plugin remove logic).
- [ ] `remove mcp foo` removes the MCP server foo.
- [ ] `remove` (no subcommand) errors.
- [ ] `commands/skills/remove.ts` and `commands/plugin/remove.ts` no longer exist.

---

### Unit 3: `update` typed positional

**File**: `packages/cli/src/commands/update.ts` (rewrite)

```typescript
const VALID_UPDATE_TYPES = ["skill", "plugin", "mcp"] as const;
type UpdateType = (typeof VALID_UPDATE_TYPES)[number];

export const updateCommand = defineCommand({
  meta: {
    name: "update",
    description:
      "Update installed skills, plugins, and MCP servers. Bare = all.",
  },
  args: {
    type: {
      type: "positional",
      required: false,
      description: "skill | plugin | mcp. Omit to update everything.",
    },
    name: {
      type: "positional",
      required: false,
      description: "Specific name. Omit to update all of the chosen type.",
    },
    yes: { type: "boolean", default: false, alias: "y" },
    strict: { type: "boolean", default: false },
    semantic: { type: "boolean", default: false },
    "skip-scan": { type: "boolean", default: false },
    quiet: { type: "boolean", default: false },
    json: { type: "boolean", default: false },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: args.quiet });

    // Validation: if `type` is provided but invalid, error.
    if (args.type && !VALID_UPDATE_TYPES.includes(args.type as UpdateType)) {
      out.error(
        `Invalid type: "${args.type}".`,
        `Valid types: ${VALID_UPDATE_TYPES.join(", ")}. Or omit type to update everything.`,
      );
      process.exit(1);
    }

    return runUpdate(
      args.type as UpdateType | undefined,
      args.name,
      args,
      out,
    );
  },
});
```

`runUpdate(type | undefined, name | undefined, args, out)` body:
- `(undefined, undefined)` → load all skills + plugins + mcp; update each.
- `("skill", undefined)` → load all skills; update each.
- `("skill", name)` → update one skill.
- Same for "plugin" and "mcp".
- The existing single-skill update logic in `core/update.ts` is reused per item.

**Acceptance Criteria**:
- [ ] `bun run dev update` updates all installed.
- [ ] `bun run dev update skill` updates all skills only.
- [ ] `bun run dev update skill foo` updates one skill.
- [ ] `bun run dev update bogus` errors with valid-types hint.
- [ ] `bun run dev update plugin foo` updates one plugin.
- [ ] `bun run dev update mcp` updates all MCP servers.

---

### Unit 4: `toggle` with placeholder picker

**File**: `packages/cli/src/commands/toggle.ts` (rewrite)

```typescript
const VALID_TOGGLE_TYPES = ["skill", "plugin", "mcp"] as const;

export const toggleCommand = defineCommand({
  meta: {
    name: "toggle",
    description:
      "Toggle a skill, plugin, or component active state. Bare opens a picker.",
  },
  args: {
    type: { type: "positional", required: false },
    target: {
      type: "positional",
      required: false,
      description: "name (or plugin name:component for plugins)",
    },
    json: { type: "boolean", default: false },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json });

    if (!args.type && !args.target) {
      // Phase 42 placeholder. Phase 44 replaces with Ink TUI.
      return runTogglePicker(out);
    }

    if (!args.type || !args.target) {
      out.error(
        "Toggle requires both type and target.",
        "Usage: skilltap toggle <type> <target> | toggle (no args opens picker)",
      );
      process.exit(1);
    }

    if (!VALID_TOGGLE_TYPES.includes(args.type)) {
      out.error(
        `Invalid type: "${args.type}".`,
        `Valid types: ${VALID_TOGGLE_TYPES.join(", ")}`,
      );
      process.exit(1);
    }

    return runToggle(args.type, args.target, out);
  },
});

async function runTogglePicker(out: Output): Promise<void> {
  // 1. clack select: "What do you want to toggle?" → skill | plugin | mcp
  // 2. Load relevant items from state.
  // 3. clack select: "Which one?" → list of names.
  // 4. If plugin and has components: clack multiselect components to toggle.
  // 5. Apply toggle, emit out.success.
}

async function runToggle(type: string, target: string, out: Output): Promise<void> {
  // Existing logic from current toggle.ts / plugin/toggle.ts:
  //   - skill: toggle skill active
  //   - plugin: toggle whole plugin active OR `target` may be `name:component`
  //   - mcp: toggle mcp active
}
```

**Implementation Notes**:
- The picker uses `@clack/prompts` `select` and `multiselect`. Pause `out.progress()` via Phase 41's pause/resume if any progress is in flight (none expected for toggle).
- The picker is a placeholder for Phase 44's Ink TUI. Tests for the picker assert on output text structure but not pixel-perfect rendering.
- `enable.ts` and `disable.ts` (top-level) are deleted (Unit 11). The "enable component" and "disable component" intents are now expressed as: `toggle plugin foo:bar` (which flips current state) or via the picker.

**Acceptance Criteria**:
- [ ] `bun run dev toggle plugin foo` toggles whole plugin.
- [ ] `bun run dev toggle plugin foo:component` toggles one component.
- [ ] `bun run dev toggle skill foo` toggles skill.
- [ ] `bun run dev toggle` (no args, TTY) opens the picker.
- [ ] `bun run dev toggle` (no args, non-TTY) errors with usage hint.
- [ ] `bun run dev toggle bogus foo` errors with valid-types hint.

---

### Unit 5: Drop `mcp:` URL prefix from user input

**Files modified**:
- `packages/cli/src/commands/install/skill.ts` — no `mcp:` detection.
- `packages/cli/src/commands/install/mcp.ts` — accepts bare source; calls `installMcpServer(source, ...)` directly. If user passes `mcp:foo`, error with hint: "the `mcp:` prefix is no longer accepted; just pass the source".
- `packages/cli/src/commands/remove/{skill,mcp}.ts` — same: `remove mcp foo` works without prefix.
- `packages/core/src/mcp-install.ts:51` — `parseMcpRef()` is now an internal helper for state.json reads only. Update its name to `parseStoredMcpKey` to avoid confusion. The function still strips the prefix when reading stored entries.
- `packages/core/src/plugin/capture.ts:102` — already strips the prefix on read. Leave unchanged; the prefix is a state.json convention.

**Implementation Notes**:
- The `mcp:` prefix lives on as a **state-store convention**: `state.mcpServers[].source` may begin with `mcp:` for entries created from the legacy `install mcp:foo` invocation. New entries (created by Phase 42's `install mcp foo`) store the source without the prefix.
- Phase 45's migrate command can normalize old entries (strip the prefix at migrate time) — that's a follow-up. For Phase 42, both shapes are read-tolerant.

**Acceptance Criteria**:
- [ ] `install skill mcp:foo` errors (mcp: not a valid skill source).
- [ ] `install mcp mcp:foo` errors with hint about prefix removal.
- [ ] `install mcp foo` works.
- [ ] State entries created by Phase 42 invocations have `source: "foo"` (no prefix).
- [ ] State entries from prior versions with `source: "mcp:foo"` continue to read correctly.

---

### Unit 6: Drop `tap install`

**Files**:
- `packages/cli/src/commands/tap/install.ts` — DELETE.
- `packages/cli/src/commands/tap/index.ts` — remove the `install` entry from subCommands.

The interactive picker functionality (browse-tap-skills + select) is not preserved in Phase 42. Users wanting tap-skill discovery use `skilltap find <query>` (existing) or wait for Phase 44's TUI.

**Acceptance Criteria**:
- [ ] `bun run dev tap install` errors as unknown subcommand.
- [ ] `bun run dev install skill <tap-resolvable-name>` continues to work (tap resolution happens inside `installSkill()`).
- [ ] `commands/tap/install.ts` no longer exists.

---

### Unit 7: Reduce `InstallOptions` callbacks

**File**: `packages/core/src/install.ts`

Current count: 17. Target: ~10 (consolidated, not 6 — that target was optimistic given how many distinct decision points genuinely exist).

**Callbacks merged**:
- `onWarnings(warnings, kind)` — single callback. `kind: "skill-static" | "plugin-static" | "skill-semantic"`. Replaces:
  - `onWarnings` (skill static)
  - `onPluginWarnings` (plugin static)
  - `onSemanticWarnings` (skill semantic)
- `onConfirmInstall(kind, manifest?)` — single confirm callback. `kind: "skill" | "plugin"`. Replaces:
  - `onConfirmInstall` (skill)
  - `onPluginConfirm` (plugin)

**Callbacks dropped (replaced by Output progress)**:
- `onStaticScanStart` — replace with `out.progress("Static scan").update(...)`.
- `onSemanticScanStart` — same.
- `onSemanticProgress` — same.
- `onOfferSemantic` — implied by `--semantic` flag or config.

**Callbacks kept**:
1. `onWarnings(warnings, kind)` — merged.
2. `onSelectSkills(skills)` — multi-skill picker.
3. `onSelectTap(matches)` — tap-disambiguation picker.
4. `onAlreadyInstalled(name)` — update vs abort decision.
5. `onConfirmInstall(kind, manifest?)` — merged.
6. `onDeepScan(count)` — deep scan prompt.
7. `onPluginDetected(manifest)` — install-as-plugin choice (kept; this is the cross-type decision).
8. `onPluginCaptureConfirm(matches)` — Phase 39.
9. `onPluginCaptureConflict(matches)` — Phase 39.
10. `onOrphansFound(orphans)` — purge prompt.

**Net: 17 → 10.**

**Implementation Notes**:
- The merged callbacks need the kind discriminator at call sites. CLI's `createInstallCallbacks(out)` produces a single callback that switches on kind for prompt text differences.
- `onStaticScanStart` etc. were thin pass-throughs to `out.progress()`. Removing them means `core/install.ts` and `core/update.ts` import from `@skilltap/core` output (which they can — output types live in core; the output instance is passed in via options). Actually, simpler: the CLI command's `out` instance is passed via a new `options.out: Output` field, and core uses `options.out?.progress(...)` directly without callbacks.

**Schema change**: add `out?: Output` to `InstallOptions` for progress reporting.

**Acceptance Criteria**:
- [ ] `InstallOptions` callback fields count = 10 (down from 17).
- [ ] `core/install.ts` calls `options.out?.progress(...)` for scan progress (replacing `onStaticScanStart`).
- [ ] `commands/install/*.ts` updated to construct callbacks with the merged signatures.
- [ ] All install/update/sync/plugin tests pass.

---

### Unit 8: Remove silent aliases

**File**: `packages/cli/src/index.ts`

Delete subCommands entries:
- `list` (alias for skills)
- bare `remove` (alias for skills/remove)
- bare `info` (alias for skills/info)
- bare `link` (alias for skills/link)
- bare `unlink` (alias for skills/unlink)
- `plugins` (dual entry; Phase 42 keeps only `plugin` — but Unit 10 deletes both anyway)

A new top-level `info <name>` (Unit 11) replaces the alias.

**Acceptance Criteria**:
- [ ] `bun run dev list` errors as unknown command.
- [ ] `bun run dev unlink foo` errors as unknown command (with hint to `adopt`/`remove skill`).
- [ ] `bun run dev info foo` works (NEW top-level info, Unit 11).
- [ ] `index.ts` subCommands has no aliases.

---

### Unit 9: Drop `commands/skills/` group

**Files DELETED**:
- `packages/cli/src/commands/skills/index.ts`
- `packages/cli/src/commands/skills/info.ts`
- `packages/cli/src/commands/skills/remove.ts`
- `packages/cli/src/commands/skills/link.ts`
- `packages/cli/src/commands/skills/unlink.ts`
- `packages/cli/src/commands/skills/adopt.ts`
- `packages/cli/src/commands/skills/move.ts`
- `packages/cli/src/commands/skills/toggle.ts`
- (and corresponding `.test.ts` files)

**Files MOVED**:
- `skills/info.ts` → folded into top-level `info.ts` (Unit 11).
- `skills/adopt.ts` → top-level `adopt.ts`. Phase 43 extends it with external-path support; Phase 42 keeps current single-skill-name behavior.
- `skills/move.ts` → top-level `move.ts` (or keep as `adopt --scope X` flag — design choice; for Phase 42 we keep `move <name>` as a simple top-level command).
- `skills/remove.ts` body → split into `commands/remove/skill.ts` (managed-skill removal) and `commands/remove/mcp.ts` (managed-mcp removal).
- `skills/toggle.ts` (factory for enable/disable) → DELETED. Phase 42's `toggle.ts` covers skills.

**Files SHOWING DEPRECATION ERROR**:
- `link` / `unlink` are not registered as standalone commands. Users running `bun run dev link <path>` get citty's "unknown command" error. Phase 43 adds `adopt <path>` for this workflow.

**Listing functionality** (`skills` command's primary purpose was a unified list view): replaced by `status`. `status` already shows skills + plugins + mcp; the `--global`/`--project`/`--unmanaged`/`--disabled`/`--active` filters move there.

**`skills` listing flags MOVED to `status`** (Unit 11 covers this):
- `--global`, `--project` — already in `status`.
- `--unmanaged`, `--disabled`, `--active`, `--json` — add to `status`.

**Acceptance Criteria**:
- [ ] `commands/skills/` directory does not exist.
- [ ] `bun run dev skills` errors as unknown command.
- [ ] `bun run dev skills info foo` errors.
- [ ] `bun run dev info foo` works (top-level).
- [ ] `bun run dev adopt <name>` works (top-level, Phase-42 single-name behavior).
- [ ] `bun run dev move <name> --to global` works (top-level).
- [ ] `bun run dev status --unmanaged` shows unmanaged skills.

---

### Unit 10: Drop `commands/plugin/` group

**Files DELETED**:
- `packages/cli/src/commands/plugin/index.ts`
- `packages/cli/src/commands/plugin/info.ts`
- `packages/cli/src/commands/plugin/remove.ts`
- `packages/cli/src/commands/plugin/toggle.ts`
- (and tests)

**Files MOVED**:
- `plugin/info.ts` → folded into top-level `info.ts`.
- `plugin/remove.ts` body → folded into `commands/remove/plugin.ts`.
- `plugin/toggle.ts` body → folded into top-level `toggle.ts`.

**Listing functionality** (was `plugin` index): folded into `status`.

**Top-level `enable.ts` and `disable.ts` DELETED** (per VISION's "toggle is enough"):
- The `--component` selection logic from those files moves into `toggle plugin foo:bar`.
- Picker logic from those files merges into `runTogglePicker()` (Unit 4).

**Acceptance Criteria**:
- [ ] `commands/plugin/` directory does not exist.
- [ ] `bun run dev plugin foo` errors as unknown command.
- [ ] `bun run dev plugin toggle foo` errors.
- [ ] `bun run dev toggle plugin foo` works (Unit 4).
- [ ] `bun run dev enable foo` errors as unknown.
- [ ] `bun run dev disable foo` errors as unknown.
- [ ] `bun run dev info foo` shows plugin info if foo is a plugin (Unit 11).

---

### Unit 11: Top-level `info`, `adopt`, `move`

**File**: `packages/cli/src/commands/info.ts` (new)

```typescript
export const infoCommand = defineCommand({
  meta: { name: "info", description: "Show details for a skill, plugin, or MCP server" },
  args: {
    name: { type: "positional", required: true },
    json: { type: "boolean", default: false },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json });
    // 1. Load state (global + project).
    // 2. Find name in state.skills, state.plugins, state.mcpServers.
    // 3. If found in multiple, prefer project scope; emit warning if ambiguous.
    // 4. Render the appropriate info renderer (existing per-type renderers from
    //    skills/info.ts and plugin/info.ts, lifted into shared functions).
    // 5. If not found anywhere, error with "skill/plugin/mcp 'foo' not installed".
  },
});
```

**File**: `packages/cli/src/commands/adopt.ts` (new — moved from `skills/adopt.ts`)

For Phase 42, simply lift `skills/adopt.ts` to top-level. Phase 43 extends with `--from <path>` for external skills.

**File**: `packages/cli/src/commands/move.ts` (new — moved from `skills/move.ts`)

Same: lift to top level.

**File**: `packages/cli/src/commands/status.ts` (modified)

Add the `--unmanaged`, `--disabled`, `--active`, `--json` flags from the deleted `skills` listing. Reuse existing rendering code.

**Acceptance Criteria**:
- [ ] `info <name>` resolves the type via state lookup.
- [ ] `info` with no name errors.
- [ ] `adopt <name>` works for a previously-unmanaged skill.
- [ ] `move foo --to global` moves foo from project to global.
- [ ] `status --unmanaged` shows unmanaged.
- [ ] `status --disabled` shows disabled.

---

### Unit 12: Tests

- Update existing CLI subprocess tests:
  - Tests calling `runSkilltap(["install", "foo"], ...)` → `runSkilltap(["install", "skill", "foo"], ...)`.
  - Tests calling `runSkilltap(["install", "mcp:foo"], ...)` → `runSkilltap(["install", "mcp", "foo"], ...)`.
  - Tests calling `runSkilltap(["remove", "foo"], ...)` → `runSkilltap(["remove", "skill", "foo"], ...)`.
  - Tests calling `runSkilltap(["tap", "install", ...], ...)` → equivalent `install skill <name>` or delete.
  - Tests using deleted commands (`list`, `link`, `unlink`, `enable`, `disable`, `skills/*`, `plugin/*`) → update to new shape or delete.
- Add new tests:
  - `install` (no subcommand) errors.
  - `install bogus foo` (unknown subcommand) errors.
  - `update` updates everything; `update skill` updates all skills; `update skill foo` updates one.
  - `toggle` opens picker (PTY test via `runInteractive`).
  - `info <name>` auto-detects type.
- Update `installSkill` callback tests for the merged callback signatures.

**Acceptance Criteria**:
- [ ] Full `bun test` passes.
- [ ] No test references deleted commands.
- [ ] Net test count change documented (subtract deletions, add new shape tests).

---

## Implementation Order

Sequential dependency:

1. **Unit 7** — Reduce `installSkill` callbacks first. This changes the core API; downstream CLI code adapts.
2. **Unit 1** — Restructure install. Big lift; depends on Unit 7's new callback signatures.
3. **Unit 2** — Restructure remove. Mirrors install pattern.
4. **Unit 3** — Update typed positional. Independent.
5. **Unit 4** — Toggle with placeholder picker.
6. **Unit 5** — Drop `mcp:` URL prefix. Depends on Unit 1 (mcp subcommand exists).
7. **Unit 6** — Drop tap install.
8. **Units 8, 9, 10, 11** — Aliases removal, skills/plugin group deletion, top-level info/adopt/move. Largely mechanical.
9. **Unit 12** — Test updates. Run after each major unit lands.

Parallelizable agents:
- Agent A: Units 7, 1, 2 (core API + install + remove restructure). Foundation.
- Agent B: Units 3, 4 (update + toggle).
- Agent C: Units 5, 6, 8, 9, 10, 11 (deletions + moves).
- Each agent runs Unit 12 tests for its scope.

Sequential gate: Agent A must complete before B and C can rely on the new callback signatures.

## Pre-Mortem

**Riskiest assumption**: That citty's subCommand groups handle positional args + flags consistently across `install skill <source>`, `install plugin <source>`, `install mcp <source>`. Specifically: when a user runs `install skill foo --yes`, citty parses `skill` as the subcommand, `foo` as the skill subcommand's positional, `--yes` as the skill subcommand's flag.

**Mitigation**: Verify with a small spike before Unit 1 — write a minimal citty subcommand group with a positional + flag and run it. If citty has quirks, adapt the design before Unit 1 proceeds. (Citty docs and existing `tap` subcommand group in this codebase confirm this pattern works.)

**What would have to be true to fail**:
- Citty rejects positional args inside subCommands. (Verified: it doesn't; `tap add <name> <url>` already works.)
- The `--strict-type` per-type detection (skill repo vs plugin repo) errors confusingly when user mistypes. (Mitigation: clear error messages with the right hint.)
- Tests grep for old command paths (`install foo`, `remove foo`). (Mitigation: Unit 12 addresses this; budget effort.)

**Fallback**: If Phase 42's full scope is too large for a single round, the install subcommand can land first (most user-facing), then remove/update/toggle in follow-up. Each unit is independently shippable.

**Where I'm least sure**:
- Whether `info` should auto-detect type or also require a type subcommand. Decision: auto-detect — `info` is read-only and the user knows the name; matching state lookup is fast and unambiguous when names are unique. If a name appears as both a skill and a plugin (rare), warn and prefer plugin (which usually owns the components).
- Whether `move` should be a flag on `adopt` or a top-level. Decision: keep as top-level for Phase 42 — simpler migration. Can fold into adopt in a later pass.

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Tests grep for old command paths | High | Unit 12 explicitly rewrites them. Budget effort. |
| Users with scripts call `install foo` (no type) | Low (no users yet) | Hard-error per VISION's clean-break stance. |
| Plugin auto-detect was a useful "I don't know what this is" UX | Low | `install skill <repo-with-plugin>` errors with the hint to use `install plugin`; the user gets the type discoverability they need. |
| Citty subcommand `--help` not great for newcomers | Low | --help shows "skilltap install <type> <source>" with the three types listed. Good enough. |
| `toggle` picker needs full TUI but Phase 44 hasn't landed | Low | clack-based placeholder is functional; tests use `runInteractive` PTY pattern. Phase 44 swaps in Ink without breaking the interface. |

## Verification Checklist

```bash
# 1. Build
bun run build

# 2. Source-side checks
test -f packages/cli/src/commands/install.ts && echo FAIL: install.ts should be deleted
test -d packages/cli/src/commands/install/ || echo FAIL: install/ directory missing
test -d packages/cli/src/commands/skills/ && echo FAIL: skills/ should be deleted
test -d packages/cli/src/commands/plugin/ && echo FAIL: plugin/ should be deleted
test -f packages/cli/src/commands/enable.ts && echo FAIL: enable.ts should be deleted
test -f packages/cli/src/commands/disable.ts && echo FAIL: disable.ts should be deleted
test -f packages/cli/src/commands/tap/install.ts && echo FAIL: tap/install.ts should be deleted

grep -rn "mcp:" packages/cli/src/commands/ --include="*.ts" | grep -v ".test.ts" | head -3
# Expect: no user-facing mcp: prefix references.

# 3. CLI exposed surface
bun run dev --help | head -50
bun run dev install --help
bun run dev remove --help
bun run dev update --help
bun run dev toggle --help

# 4. Tests
bun test packages/cli/src/commands/
bun test
```
