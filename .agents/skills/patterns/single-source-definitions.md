# Pattern: Single-Source Definitions

Every enumerable concept (agent list, config enum values, path mappings) has **one authoritative definition**. All other files import or derive from it — never copy literals.

## Rule

If you need to add a new value to a set (new agent, new scan mode, etc.), you should touch **exactly one file**. Everything else — wizards, validators, completion scripts, scanners — derives its data automatically.

## Agent entity — `packages/core/src/symlink.ts`

The agent entity has three properties: ID, filesystem path, and display label. All three live together in `symlink.ts`:

```typescript
// The canonical definition — one place for all agent metadata
export const AGENT_PATHS: Record<string, string> = {
  "claude-code": ".claude/skills",
  cursor: ".cursor/skills",
  // add a new agent here only
};

export const AGENT_LABELS: Record<string, string> = {
  "claude-code": "Claude Code",
  cursor: "Cursor",
  // label goes here alongside the path
};

export const VALID_AGENT_IDS: string[] = Object.keys(AGENT_PATHS);
```

**What derives from this:**
- Scanner patterns: `Object.values(AGENT_PATHS).map(p => \`${p}/*/SKILL.md\`)`
- Wizard select options: `VALID_AGENT_IDS.map(id => ({ value: id, label: AGENT_LABELS[id] ?? id }))`
- Completion scripts: `VALID_AGENT_IDS.join(" ")` injected into template strings
- Validation errors: `VALID_AGENT_IDS.join(", ")` in error hints

## Config enum values — `packages/core/src/schemas/config.ts`

Extract enum value arrays as named `as const` constants before the schema. The schema uses these constants; consumers import them instead of duplicating the arrays.

```typescript
// Named constants — import these wherever you need to list valid values
export const SCAN_MODES = ["static", "semantic", "off"] as const;
export const ON_WARN_MODES = ["prompt", "fail"] as const;
export const SCOPE_VALUES = ["", "global", "project"] as const;
export const AUTO_UPDATE_MODES = ["off", "patch", "minor"] as const;
export const SHOW_DIFF_MODES = ["full", "stat", "none"] as const;

// Schema references the constants — never embeds the array literals directly
export const SecurityConfigSchema = z.object({
  scan: z.enum(SCAN_MODES).default("static"),
  on_warn: z.enum(ON_WARN_MODES).default("prompt"),
  // ...
});
```

**What derives from this:**
- `config-keys.ts` SETTABLE_KEYS: imports `SCOPE_VALUES`, `AUTO_UPDATE_MODES`, `SHOW_DIFF_MODES`
- Config wizard select options: could import and use as value arrays (labels are still defined in UI layer)

## Where each definition type belongs

| Concept | Where it lives | Why |
|---|---|---|
| Agent IDs + paths + labels | `core/src/symlink.ts` | Functional data used across both packages |
| Config enum valid values | `core/src/schemas/config.ts` | Zod is source of truth for config shapes |
| Template names | `core/src/templates/index.ts` | Validated and used in both core and CLI |
| Semantic scan agent names | `core/src/agents/detect.ts` (`KNOWN_AGENT_NAMES`) | Derived from `ADAPTER_MAP` + ollama |
| Shared wizard select options (value+label+hint) | `cli/src/ui/prompts.ts` | UI concern; shared across commands that show the same prompt |

## What NOT to do

```typescript
// BAD — duplicating values in three files
// config-keys.ts:
"updates.show_diff": { type: "enum", enum: ["full", "stat", "none"] }
// schemas/config.ts:
show_diff: z.enum(["full", "stat", "none"])
// agent-mode.ts:
options: [{ value: "claude-code" }, { value: "cursor" }, ...]

// GOOD — import from single source
import { SHOW_DIFF_MODES } from "./schemas/config";
import { AGENT_LABELS, VALID_AGENT_IDS } from "@skilltap/core";
"updates.show_diff": { type: "enum", enum: SHOW_DIFF_MODES }
options: VALID_AGENT_IDS.map(id => ({ value: id, label: AGENT_LABELS[id] ?? id }))
```

## Acceptable exceptions

`DEFAULT_CONFIG_TEMPLATE` in `config.ts` is a static string of TOML comments shown to users. Its valid-values comments are documentation, not logic — they're acceptable as manual sync. When changing enum values, remember to update the template comments. A code comment marks the dependency:
```typescript
// Keep valid-values comments below in sync with SCAN_MODES / AGENT_PATHS in symlink.ts
const DEFAULT_CONFIG_TEMPLATE = `...`;
```

## Completion scripts

Completion scripts are TypeScript functions returning shell strings. They CAN import from core:

```typescript
import { VALID_AGENT_IDS } from "@skilltap/core";

export function generateBashCompletions(): string {
  const agents = VALID_AGENT_IDS.join(" ");
  return `...local agents="${agents}"...`;
}
```
