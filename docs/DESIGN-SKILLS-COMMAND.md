# Design: `skilltap skills` Command Group

## Overview

A new `skills` command group that consolidates skill lifecycle management and adds two new capabilities: **discovery of unmanaged skills** and **migration between scopes/locations**. This replaces the existing top-level `list`, `remove`, `info`, `link`, and `unlink` commands (which become silent aliases).

### Problem

skilltap currently has a blind spot: it only knows about skills it installed. Skills placed manually in agent-specific directories (e.g. `~/.claude/skills/seo`) or project directories (e.g. `.agents/skills/patterns`) are invisible to `skilltap list`, can't be updated, and can't be moved between scopes. Users managing skills across multiple agents and projects need a unified view and migration tools.

### Command Tree (after)

```
skilltap
├── install <source>           # stays top-level
├── update [name]              # stays top-level
├── find [query]               # stays top-level
├── skills                     # NEW: unified skill view (replaces list)
│   ├── info <name>            # moved from top-level
│   ├── remove [name...]       # moved from top-level (handles managed + unmanaged)
│   ├── link <path>            # moved from top-level
│   ├── unlink <name>          # moved from top-level
│   ├── adopt [name...]        # NEW: bring unmanaged skills under management
│   └── move <name>            # NEW: move skills between scopes/locations
├── tap ...                    # unchanged
├── config ...                 # unchanged
└── ...

# Silent aliases (not shown in --help):
skilltap list       → skilltap skills
skilltap remove     → skilltap skills remove
skilltap info       → skilltap skills info
skilltap link       → skilltap skills link
skilltap unlink     → skilltap skills unlink
```

---

## Implementation Units

### Unit 1: `discoverSkills()` — Core Discovery Engine

**File**: `packages/core/src/discover.ts`

The foundation for everything else. Scans all known skill locations on disk and correlates with installed.json records to produce a unified inventory.

```typescript
import type { InstalledSkill } from "./schemas/installed";
import type { Result } from "./types";

/** Where a discovered skill physically lives */
export type SkillLocation = {
  /** Absolute path to the skill directory (or symlink target) */
  path: string;
  /** Which directory tree it was found in */
  source:
    | { type: "agents"; scope: "global" | "project" }
    | { type: "agent-specific"; agent: string; scope: "global" | "project" };
  /** True if the entry at `path` is a symlink */
  isSymlink: boolean;
  /** If symlink, where it points */
  symlinkTarget: string | null;
};

export type DiscoveredSkill = {
  /** Skill name (directory name) */
  name: string;
  /** Whether skilltap tracks this skill in installed.json */
  managed: boolean;
  /** The installed.json record, if managed */
  record: InstalledSkill | null;
  /** All locations where this skill appears (may be in multiple dirs) */
  locations: SkillLocation[];
  /** Git remote URL if the skill dir is a git repo */
  gitRemote: string | null;
  /** Description from SKILL.md frontmatter (parsed on demand) */
  description: string;
};

export type DiscoverOptions = {
  /** Only scan global dirs (default: scan both) */
  global?: boolean;
  /** Only scan project dirs (default: scan both) */
  project?: boolean;
  /** Project root for project-scoped scanning */
  projectRoot?: string;
  /** Only return unmanaged skills */
  unmanagedOnly?: boolean;
};

export type DiscoverResult = {
  skills: DiscoveredSkill[];
  /** Count by management status */
  managed: number;
  unmanaged: number;
};

export async function discoverSkills(
  options?: DiscoverOptions,
): Promise<Result<DiscoverResult, UserError>>;
```

**Implementation Notes**:

1. **Scan order**: For each scope (global, project):
   - Scan `.agents/skills/` — the canonical location
   - Scan each agent-specific dir from `AGENT_PATHS` (`.claude/skills/`, `.cursor/skills/`, etc.)
2. **Deduplication**: A skill name may appear in multiple locations (e.g. `.agents/skills/foo` and `.claude/skills/foo → .agents/skills/foo`). Group by name, collect all locations.
3. **Symlink detection**: Use `lstat()` to detect symlinks. If a symlink points to another scanned location, don't count it as a separate skill — just add it to the existing skill's `locations[]`.
4. **Management status**: Cross-reference discovered names against both global and project `installed.json`. If a record exists, `managed = true`.
5. **Git remote detection**: For unmanaged skills that are actual directories (not symlinks), try `git -C <path> remote get-url origin` to detect if it's a cloned repo. Wrap in try/catch — many won't be git repos.
6. **Description parsing**: Read `SKILL.md` frontmatter from each skill dir to extract description. Fall back to empty string.
7. **Performance**: Use `readdir` (not glob) for each known directory. Don't recurse. This is a shallow scan of ~5-6 directories per scope.

**Acceptance Criteria**:
- [ ] Discovers skills in `.agents/skills/` (global and project)
- [ ] Discovers skills in all `AGENT_PATHS` directories (global and project)
- [ ] Correctly identifies symlinks vs real directories
- [ ] Deduplicates skills that appear in multiple locations via symlinks
- [ ] Correlates with installed.json to set `managed` flag
- [ ] Detects git remotes for unmanaged skills
- [ ] Parses SKILL.md description from frontmatter
- [ ] Respects `global`/`project`/`unmanagedOnly` filters
- [ ] Returns `Result<DiscoverResult, UserError>`

---

### Unit 2: `adoptSkill()` — Core Adopt Logic

**File**: `packages/core/src/adopt.ts`

Takes an unmanaged skill and brings it under skilltap management. Default behavior: move to `.agents/skills/` and create symlinks back to original locations.

```typescript
import type { InstalledSkill } from "./schemas/installed";
import type { DiscoveredSkill } from "./discover";
import type { Result } from "./types";

export type AdoptMode = "move" | "track-in-place";

export type AdoptOptions = {
  /** How to adopt: move to .agents/ + symlink, or track in current location */
  mode?: AdoptMode;
  /** Target scope for the adopted skill (default: "global") */
  scope?: "global" | "project";
  /** Project root (required if scope is "project") */
  projectRoot?: string;
  /** Agent IDs to create symlinks for (in addition to existing locations) */
  also?: string[];
  /** Skip security scan */
  skipScan?: boolean;
  /** Callback: security warnings found */
  onWarnings?: (warnings: StaticWarning[], skillName: string) => Promise<boolean>;
};

export type AdoptResult = {
  record: InstalledSkill;
  /** Paths that were symlinked back to the new location */
  symlinksCreated: string[];
};

/**
 * Adopt a single discovered skill into skilltap management.
 *
 * In "move" mode (default):
 * 1. Move skill dir to canonical .agents/skills/<name>
 * 2. Create symlinks from original location(s) back to new location
 * 3. Create agent symlinks for `also` agents
 * 4. Run static security scan (unless skipScan)
 * 5. Add record to installed.json
 *
 * In "track-in-place" mode:
 * 1. Create a "linked" record in installed.json pointing to current path
 * 2. Create agent symlinks for `also` agents
 * 3. Run static security scan (unless skipScan)
 */
export async function adoptSkill(
  skill: DiscoveredSkill,
  options?: AdoptOptions,
): Promise<Result<AdoptResult, UserError>>;
```

**Implementation Notes**:

1. **Pre-check**: Verify the skill is not already managed (has no installed.json record). Return error if it is.
2. **Move mode** (default):
   - Determine target path: `skillInstallDir(name, scope, projectRoot)`
   - If skill already lives at the target path (e.g. it's in `.agents/skills/` but untracked), skip the move — just create the record.
   - Otherwise, `mv` the skill directory to the target path.
   - For each original location in `skill.locations` where the skill was a real directory (not already a symlink), create a symlink from the old path to the new path. This preserves agent-specific directory references.
   - Create additional agent symlinks via `createAgentSymlinks()` for the `also` list.
3. **Track-in-place mode**:
   - Record the skill as `scope: "linked"` with `path` pointing to current location.
   - Create agent symlinks pointing to the current location.
4. **Security scan**: Run `scanStatic()` on the skill directory before finalizing. Call `onWarnings` if findings. If user rejects, abort.
5. **Git remote detection**: If `skill.gitRemote` is set, store it as `repo` in the installed record.
6. **Record creation**: Build an `InstalledSkill` record with:
   - `name`, `description` from the discovered skill
   - `repo`: git remote or `null`
   - `ref`: current branch name (from `git` if available) or `null`
   - `sha`: current HEAD (from `git` if available) or `null`
   - `scope`: target scope (or "linked" for track-in-place)
   - `also`: merged list of existing agent locations + requested `also`
   - `installedAt`: now

**Acceptance Criteria**:
- [ ] Move mode: moves skill dir to `.agents/skills/<name>`
- [ ] Move mode: creates symlinks from all original locations to new path
- [ ] Move mode: handles skill already at target path (no-move, just track)
- [ ] Track-in-place mode: creates "linked" record without moving
- [ ] Runs static security scan (unless `skipScan`)
- [ ] Respects `onWarnings` callback
- [ ] Detects and records git remote/ref/sha when available
- [ ] Writes record to installed.json
- [ ] Returns `AdoptResult` with created symlinks list

---

### Unit 3: `moveSkill()` — Core Move Logic

**File**: `packages/core/src/move.ts`

Moves a managed skill between scopes (global ↔ project) or from an agent-specific location to the canonical `.agents/skills/` path.

```typescript
import type { InstalledSkill } from "./schemas/installed";
import type { Result } from "./types";

export type MoveTarget =
  | { scope: "global" }
  | { scope: "project"; projectRoot: string };

export type MoveOptions = {
  /** Where to move the skill */
  to: MoveTarget;
  /** Additional agent symlinks to create at the destination */
  also?: string[];
};

export type MoveResult = {
  record: InstalledSkill;
  from: string;
  to: string;
};

/**
 * Move an installed skill to a different scope.
 *
 * 1. Look up skill in installed.json
 * 2. Compute source path and destination path
 * 3. Remove old agent symlinks
 * 4. Move the skill directory
 * 5. Create new agent symlinks (preserving existing `also` list + new `also`)
 * 6. Update installed.json record with new scope/path
 */
export async function moveSkill(
  name: string,
  options: MoveOptions,
): Promise<Result<MoveResult, UserError>>;
```

**Implementation Notes**:

1. **Lookup**: Find the skill in installed.json (check both global and project). Error if not found or if already in the target scope.
2. **Path computation**: Use `skillInstallDir()` for both source and destination.
3. **Symlink cleanup**: Call `removeAgentSymlinks()` with the current scope's agents.
4. **Move**: Use `mv` (or `rename` if same filesystem, `cp -r` + `rm -rf` if cross-filesystem).
5. **Symlink recreation**: Call `createAgentSymlinks()` with merged `also` list at the new scope.
6. **Record update**: Update the `scope`, `also`, and `updatedAt` fields in the record. If moving from project to global, the record moves from project `installed.json` to global `installed.json` (remove from source, add to destination).
7. **Linked skills**: If the skill is `scope: "linked"`, moving it converts it to a fully managed skill (copy from linked path to target, remove symlink at old location, update scope).

**Acceptance Criteria**:
- [ ] Moves skill directory from global to project and vice versa
- [ ] Updates installed.json in both source and destination when crossing scope boundaries
- [ ] Removes old agent symlinks and creates new ones at the target scope
- [ ] Merges existing `also` agents with new `also` option
- [ ] Handles linked skills (converts to managed)
- [ ] Errors if skill is already in the target scope
- [ ] Returns `MoveResult` with from/to paths

---

### Unit 4: `removeUnmanagedSkill()` — Core Unmanaged Removal

**File**: `packages/core/src/remove.ts` (extend existing file)

Extends the existing `removeSkill()` to also handle skills not in installed.json.

```typescript
/** Options for removing any skill (managed or unmanaged) */
export type RemoveAnyOptions = {
  /** The discovered skill to remove (from discoverSkills) */
  skill: DiscoveredSkill;
  /** If the skill appears in multiple locations, remove from all? */
  removeAll?: boolean;
  /** Specific locations to remove (subset of skill.locations) */
  locations?: SkillLocation[];
};

/**
 * Remove a skill from disk, whether managed or not.
 *
 * For managed skills: delegates to existing removeSkill().
 * For unmanaged skills: removes the directory/symlink at each specified location.
 */
export async function removeAnySkill(
  options: RemoveAnyOptions,
): Promise<Result<void, UserError>>;
```

**Implementation Notes**:
1. If `skill.managed` and `skill.record`, delegate to existing `removeSkill()`.
2. If unmanaged, iterate `locations` (or all locations if `removeAll`), `rm -rf` each.
3. When removing from an agent-specific dir, check if a symlink — if so, just `unlink()` instead of `rm -rf`.

**Acceptance Criteria**:
- [ ] Delegates to `removeSkill()` for managed skills
- [ ] Removes directories for unmanaged skills
- [ ] Only unlinks symlinks (doesn't follow and delete target)
- [ ] Supports removing from specific locations vs all

---

### Unit 5: `skilltap skills` — CLI Unified View Command

**File**: `packages/cli/src/commands/skills/index.ts`

The default subcommand (runs when `skilltap skills` is invoked with no subcommand). Shows a unified view of all skills across all locations.

```typescript
import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "skills",
    description: "Manage installed skills",
  },
  args: {
    global: {
      type: "boolean",
      description: "Show only global skills",
      default: false,
    },
    project: {
      type: "boolean",
      description: "Show only project skills",
      default: false,
    },
    unmanaged: {
      type: "boolean",
      description: "Show only unmanaged skills",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  subCommands: {
    info: () => import("./info").then((m) => m.default),
    remove: () => import("./remove").then((m) => m.default),
    link: () => import("./link").then((m) => m.default),
    unlink: () => import("./unlink").then((m) => m.default),
    adopt: () => import("./adopt").then((m) => m.default),
    move: () => import("./move").then((m) => m.default),
  },
  async run({ args }) {
    // Call discoverSkills() with filters from args
    // Display unified table grouped by location/scope
  },
});
```

**Output format (interactive)**:

```
Global (.agents/skills/) — 23 skills
  Name                  Status     Agents         Source
  design                managed    claude-code    nklisch/skills
  implement             managed    claude-code    nklisch/skills
  spectator             linked     —              ~/dev/spectator

Claude Code (~/.claude/skills/) — 13 unmanaged
  Name                  Status     Source
  seo                   unmanaged  (local)
  seo-audit             unmanaged  (local)

Project (.agents/skills/) — 9 skills
  Name                  Status     Source
  bun                   managed    nklisch/skills
  patterns              unmanaged  (local)
  update-completions    unmanaged  (local)
```

**Output format (agent mode)**:
```
GLOBAL managed design agents=claude-code source=nklisch/skills
GLOBAL managed implement agents=claude-code source=nklisch/skills
GLOBAL linked spectator path=~/dev/spectator
CLAUDE_CODE unmanaged seo
CLAUDE_CODE unmanaged seo-audit
PROJECT managed bun source=nklisch/skills
PROJECT unmanaged patterns
```

**Output format (JSON)**:
```json
[
  {
    "name": "design",
    "managed": true,
    "locations": [
      {"source": {"type": "agents", "scope": "global"}, "path": "..."},
      {"source": {"type": "agent-specific", "agent": "claude-code", "scope": "global"}, "path": "...", "isSymlink": true}
    ],
    "record": { ... },
    "gitRemote": null,
    "description": "..."
  }
]
```

**Implementation Notes**:
- Uses `discoverSkills()` from core
- Groups output by location section: Global (.agents), each agent-specific dir with unmanaged skills, Project (.agents), each project agent-specific dir
- Sections with only symlinks to already-displayed skills are hidden (avoid noise)
- Only shows agent-specific sections if they contain unmanaged skills (managed skills show their agents in the "Agents" column instead)
- Status column: `managed` (green), `linked` (blue), `unmanaged` (yellow/dim)

**Acceptance Criteria**:
- [ ] Shows all skills across all locations
- [ ] Groups by scope and location type
- [ ] Shows management status (managed/linked/unmanaged)
- [ ] Shows which agents have symlinks for managed skills
- [ ] `--global`, `--project`, `--unmanaged` filters work
- [ ] `--json` outputs full DiscoveredSkill array
- [ ] Agent mode outputs parseable plain text
- [ ] Unmanaged agent-specific skills shown in separate sections
- [ ] Symlink-only entries don't create duplicate rows

---

### Unit 6: `skilltap skills adopt` — CLI Adopt Command

**File**: `packages/cli/src/commands/skills/adopt.ts`

```typescript
import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "adopt",
    description: "Adopt unmanaged skills into skilltap management",
  },
  args: {
    name: {
      type: "positional",
      description: "Skill name(s) to adopt (interactive picker if omitted)",
      required: false,
    },
    global: {
      type: "boolean",
      description: "Adopt into global scope",
      default: false,
    },
    project: {
      type: "boolean",
      description: "Adopt into project scope",
      default: false,
    },
    "track-in-place": {
      type: "boolean",
      description: "Track skill at current location instead of moving to .agents/",
      default: false,
    },
    also: {
      type: "string",
      description: "Also symlink to agent-specific directory",
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip security scan",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-accept all prompts",
      default: false,
    },
  },
  async run({ args }) {
    // 1. Run discoverSkills({ unmanagedOnly: true })
    // 2. If no name given (and interactive): show multiselect of unmanaged skills
    // 3. For each selected skill: call adoptSkill()
    // 4. Display results
  },
});
```

**Interactive flow**:
```
$ skilltap skills adopt

  Found 15 unmanaged skills:

  ◻ seo           ~/.claude/skills/seo          (local)
  ◻ seo-audit     ~/.claude/skills/seo-audit    (local)
  ◻ seo-content   ~/.claude/skills/seo-content  (local)
  ◻ patterns      .agents/skills/patterns       (local)
  ◻ update-completions  .agents/skills/update-completions  (local)

  Select skills to adopt (space to toggle, enter to confirm):

  ✓ Adopted 5 skills into global scope
    seo → ~/.agents/skills/seo (symlink: ~/.claude/skills/seo)
    seo-audit → ~/.agents/skills/seo-audit (symlink: ~/.claude/skills/seo-audit)
    ...
```

**Agent mode**:
```
$ skilltap skills adopt seo seo-audit --yes
OK: Adopted seo → ~/.agents/skills/seo
OK: Adopted seo-audit → ~/.agents/skills/seo-audit
```

**Acceptance Criteria**:
- [ ] Interactive multiselect when no names given
- [ ] Accepts positional name(s) for non-interactive use
- [ ] Default behavior: move to `.agents/skills/` + symlink back
- [ ] `--track-in-place` creates linked record without moving
- [ ] `--also` creates additional agent symlinks
- [ ] `--skip-scan` skips security scan
- [ ] `--yes` auto-accepts all prompts
- [ ] Agent mode: plain text output, requires name arg
- [ ] Errors if skill is already managed

---

### Unit 7: `skilltap skills move` — CLI Move Command

**File**: `packages/cli/src/commands/skills/move.ts`

```typescript
import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "move",
    description: "Move a skill between scopes",
  },
  args: {
    name: {
      type: "positional",
      description: "Skill name to move",
      required: true,
    },
    global: {
      type: "boolean",
      description: "Move to global scope",
      default: false,
    },
    project: {
      type: "boolean",
      description: "Move to project scope",
      default: false,
    },
    also: {
      type: "string",
      description: "Also symlink to agent-specific directory",
    },
  },
  async run({ args }) {
    // 1. Determine target scope from --global/--project flags
    // 2. Call moveSkill(args.name, { to: target, also })
    // 3. Display result
  },
});
```

**Interactive output**:
```
$ skilltap skills move patterns --global
  ✓ Moved patterns: .agents/skills/patterns (project) → ~/.agents/skills/patterns (global)
```

**Agent mode**:
```
$ skilltap skills move patterns --global
OK: Moved patterns from project to global
```

**Acceptance Criteria**:
- [ ] Requires exactly one of `--global` or `--project`
- [ ] Errors if skill not found
- [ ] Errors if already in target scope
- [ ] Displays from/to paths
- [ ] Agent mode: plain text output

---

### Unit 8: Move Existing Commands to skills/ Directory

**Files**:
- `packages/cli/src/commands/skills/info.ts` — move from `commands/info.ts`
- `packages/cli/src/commands/skills/remove.ts` — move from `commands/remove.ts`, extend to handle unmanaged
- `packages/cli/src/commands/skills/link.ts` — move from `commands/link.ts`
- `packages/cli/src/commands/skills/unlink.ts` — move from `commands/unlink.ts`

**Implementation Notes**:
- Move the existing command files into `commands/skills/` with no functional changes (except `remove` — see below).
- **`remove` enhancement**: Before looking up in installed.json, call `discoverSkills()` to check if the skill exists on disk. If it's unmanaged, call `removeAnySkill()`. If managed, delegate to existing `removeSkill()`. This means `skilltap skills remove seo` works even though `seo` is unmanaged.
- Update imports but keep the same citty `defineCommand` structure.

**Acceptance Criteria**:
- [ ] All four commands work identically from `skilltap skills <cmd>`
- [ ] `remove` handles both managed and unmanaged skills
- [ ] No regressions in existing behavior

---

### Unit 9: Wire Up Command Group and Aliases

**File**: `packages/cli/src/index.ts`

```typescript
// In the main command definition:
const main = defineCommand({
  meta: { name: "skilltap", version: VERSION, description: "Install agent skills from any git host" },
  subCommands: {
    install: () => import("./commands/install").then((m) => m.default),
    update: () => import("./commands/update").then((m) => m.default),
    find: () => import("./commands/find").then((m) => m.default),
    skills: () => import("./commands/skills/index").then((m) => m.default),

    // Silent aliases — not shown in --help (citty meta.hidden or just undocumented)
    list: () => import("./commands/skills/index").then((m) => m.default),
    remove: () => import("./commands/skills/remove").then((m) => m.default),
    info: () => import("./commands/skills/info").then((m) => m.default),
    link: () => import("./commands/skills/link").then((m) => m.default),
    unlink: () => import("./commands/skills/unlink").then((m) => m.default),

    // Unchanged
    create: () => import("./commands/create").then((m) => m.default),
    verify: () => import("./commands/verify").then((m) => m.default),
    doctor: () => import("./commands/doctor").then((m) => m.default),
    config: () => import("./commands/config").then((m) => m.default),
    "self-update": () => import("./commands/self-update").then((m) => m.default),
    completions: () => import("./commands/completions").then((m) => m.default),
    status: () => import("./commands/status").then((m) => m.default),
    tap: defineCommand({ /* unchanged */ }),
  },
});
```

**Implementation Notes**:
- citty doesn't have a built-in `hidden` flag for subcommands, but the aliases are simply additional entries in the `subCommands` map — they point to the same module. The help text for `skilltap` shows whatever citty auto-generates from the subCommands keys. To hide aliases: check if citty supports `meta.hidden`, or filter them from the help output. Worst case, the aliases appear in help text — this is acceptable for v1.
- Delete the old files at `commands/list.ts`, `commands/remove.ts`, `commands/info.ts`, `commands/link.ts`, `commands/unlink.ts` after moving.
- Update shell completions in `completions/generate.ts` to include the new `skills` subcommand and its children.

**Acceptance Criteria**:
- [ ] `skilltap skills` shows unified view
- [ ] `skilltap skills adopt` works
- [ ] `skilltap skills move` works
- [ ] `skilltap list` routes to `skilltap skills`
- [ ] `skilltap remove` routes to `skilltap skills remove`
- [ ] `skilltap info` routes to `skilltap skills info`
- [ ] `skilltap link` routes to `skilltap skills link`
- [ ] `skilltap unlink` routes to `skilltap skills unlink`
- [ ] Old command files deleted
- [ ] Shell completions updated

---

### Unit 10: Core Barrel Export Updates

**File**: `packages/core/src/index.ts`

```typescript
// Add new exports:
export * from "./discover";
export * from "./adopt";
export * from "./move";
// removeAnySkill is already exported via ./remove
```

**Acceptance Criteria**:
- [ ] `discoverSkills`, `adoptSkill`, `moveSkill`, `removeAnySkill` all importable from `@skilltap/core`
- [ ] All new types exported

---

## Implementation Order

1. **Unit 1: `discoverSkills()`** — Foundation. Everything depends on this.
2. **Unit 2: `adoptSkill()`** — Depends on Unit 1 for `DiscoveredSkill` type.
3. **Unit 3: `moveSkill()`** — Independent of Unit 2, depends on existing core modules.
4. **Unit 4: `removeAnySkill()`** — Extends existing `remove.ts`, depends on Unit 1.
5. **Unit 10: Barrel exports** — Quick, unblocks CLI work.
6. **Unit 8: Move existing commands** — Move files to `commands/skills/`, update remove.
7. **Unit 5: `skilltap skills` view** — Depends on Unit 1, replaces list.
8. **Unit 6: `skilltap skills adopt`** — Depends on Units 1, 2.
9. **Unit 7: `skilltap skills move`** — Depends on Unit 3.
10. **Unit 9: Wire up index.ts + aliases** — Final wiring, depends on all CLI units.

## Testing

### Unit Tests: `packages/core/src/discover.test.ts`

```
describe("discoverSkills")
  test("discovers skills in .agents/skills/")
  test("discovers skills in agent-specific dirs")
  test("deduplicates symlinked skills")
  test("marks managed skills from installed.json")
  test("marks unmanaged skills without records")
  test("detects git remotes on unmanaged skills")
  test("respects global/project/unmanagedOnly filters")
  test("parses SKILL.md description")
```

### Unit Tests: `packages/core/src/adopt.test.ts`

```
describe("adoptSkill")
  test("move mode: moves dir to .agents/skills/")
  test("move mode: creates symlink from original location")
  test("move mode: skill already at target path just creates record")
  test("track-in-place mode: creates linked record")
  test("records git remote when available")
  test("runs security scan and respects onWarnings")
  test("errors on already-managed skill")
```

### Unit Tests: `packages/core/src/move.test.ts`

```
describe("moveSkill")
  test("moves from global to project")
  test("moves from project to global")
  test("updates installed.json in both scopes")
  test("recreates agent symlinks at new scope")
  test("converts linked skill to managed")
  test("errors if already in target scope")
  test("errors if skill not found")
```

### CLI Tests: `packages/cli/src/commands/skills/skills.test.ts`

```
describe("skilltap skills")
  test("shows unified view with managed and unmanaged skills")
  test("--global filters to global only")
  test("--project filters to project only")
  test("--unmanaged filters to unmanaged only")
  test("--json outputs full discovery result")
  test("agent mode outputs plain text")

describe("skilltap skills adopt")
  test("adopts an unmanaged skill by name")
  test("interactive multiselect when no name given")
  test("--track-in-place creates linked record")
  test("--yes auto-accepts")

describe("skilltap skills move")
  test("moves skill from project to global")
  test("errors when --global and --project both set")

describe("aliases")
  test("skilltap list routes to skills")
  test("skilltap remove routes to skills remove")
  test("skilltap info routes to skills info")
```

### Test Fixtures Needed

- Unmanaged skill directory (in `.claude/skills/` without installed.json entry)
- Unmanaged skill with git remote
- Mixed state: some managed, some unmanaged in same scope

Use `@skilltap/test-utils` `makeTmpDir` + fixture helpers with `SKILLTAP_HOME` env var override.

## Verification Checklist

```bash
# Unit tests
bun test packages/core/src/discover.test.ts
bun test packages/core/src/adopt.test.ts
bun test packages/core/src/move.test.ts

# CLI tests
bun test packages/cli/src/commands/skills/

# Full test suite (no regressions)
bun test

# Manual verification
bun run dev -- skills                    # unified view
bun run dev -- skills --unmanaged        # just unmanaged
bun run dev -- skills adopt seo --yes    # adopt a skill
bun run dev -- skills move patterns --global  # move scope
bun run dev -- list                      # alias works
bun run dev -- remove <name> --yes       # alias works
```
