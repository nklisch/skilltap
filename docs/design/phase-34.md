# Design: Phase 34 — Component-Ref Toggle/Enable/Disable

## Overview

Add three top-level commands — `skilltap toggle`, `skilltap enable`, `skilltap disable` — that accept a `<name>[:component]` argument:

- `:component` form → direct action on one component (toggle/enable/disable).
- Bare `<name>` form → picker (toggle) or bulk action (enable = activate all inactive; disable = deactivate all active).

Existing `skilltap plugin toggle` continues to work unchanged (with --skills/--mcps/--agents flags). Top-level commands are convenience shortcuts; the existing plugin command stays the canonical flag-driven one.

## Autonomous Decisions

### D1. Parser placement: `core/src/plugin/component-ref.ts`

Reusable across CLI commands AND potentially future commands (status drift display, etc.). Place in core; CLI imports.

### D2. Bulk action semantics for bare names

- `toggle <name>` → open multiselect picker (mirrors `plugin toggle`).
- `enable <name>` → activate all currently-inactive components (no prompt; emit per-component result lines).
- `disable <name>` → deactivate all currently-active components (no prompt; emit per-component result lines).

Rationale: enable/disable verbs are unambiguous in their intent; toggle is ambiguous so it picks. This matches the user's likely mental model.

### D3. Plugin lookup across scopes

Use the same load-from-global-then-project pattern as `cli/src/commands/plugin/toggle.ts`. Extract a small helper `loadAllPlugins()` to keep the three new commands DRY.

### D4. `--json` output supported

For agent-mode users and scripting. Each command emits an array of `{ component, action, nowActive }` objects.

## Implementation Units

### Unit 1 — `core/src/plugin/component-ref.ts`

```typescript
import type { StoredComponent, PluginRecord } from "../schemas/plugins";

export interface ParsedComponentRef {
  name: string;
  /** Component name after the colon, or undefined if no colon was present. */
  component?: string;
}

// Parse "foo:bar" → { name: "foo", component: "bar" }; "foo" → { name: "foo" }.
// First colon splits; remainder is the component (component names may contain colons).
export function parseComponentRef(input: string): ParsedComponentRef {
  const colonIdx = input.indexOf(":");
  if (colonIdx === -1) return { name: input };
  if (colonIdx === 0 || colonIdx === input.length - 1) {
    // Treat ":bar" or "foo:" as malformed → fall back to name-only.
    return { name: input };
  }
  return {
    name: input.slice(0, colonIdx),
    component: input.slice(colonIdx + 1),
  };
}

// Look up a component by name within a plugin record. Returns null if not found.
// When multiple components share a name across types, returns the first match
// in iteration order — which is by component placement in the manifest.
export function findComponentInPlugin(
  plugin: PluginRecord,
  componentName: string,
): StoredComponent | null {
  return plugin.components.find((c) => c.name === componentName) ?? null;
}
```

**Acceptance Criteria**:
- [ ] `parseComponentRef("foo:bar")` returns `{ name: "foo", component: "bar" }`.
- [ ] `parseComponentRef("foo")` returns `{ name: "foo" }`.
- [ ] `parseComponentRef("foo:bar:baz")` returns `{ name: "foo", component: "bar:baz" }`.
- [ ] `parseComponentRef(":bar")` returns `{ name: ":bar" }` (malformed → name-only).
- [ ] `parseComponentRef("foo:")` returns `{ name: "foo:" }` (malformed → name-only).
- [ ] `findComponentInPlugin(plugin, "missing")` returns `null`.
- [ ] `findComponentInPlugin(plugin, existingName)` returns the matching component.

### Unit 2 — Tests for component-ref

`packages/core/src/plugin/component-ref.test.ts` — covers all parser cases + lookup happy path / miss.

### Unit 3 — Shared CLI helper for plugin lookup

Inline this in each of the three new commands (don't extract to a separate file unless duplication grows). The pattern:

```typescript
async function findPlugin(name: string): Promise<PluginRecord | null> {
  const projectRoot = await tryFindProjectRoot();
  const globalResult = await loadPlugins();
  if (!globalResult.ok) return null;
  const projectResult = projectRoot ? await loadPlugins(projectRoot) : null;
  const all = [
    ...globalResult.value.plugins,
    ...(projectResult?.ok ? projectResult.value.plugins : []),
  ];
  return all.find((p) => p.name === name) ?? null;
}
```

### Unit 4 — `cli/src/commands/toggle.ts` (top-level)

```typescript
import { multiselect } from "@clack/prompts";
import {
  loadPlugins,
  parseComponentRef,
  findComponentInPlugin,
  toggleInstalledComponent,
  type PluginRecord,
  type StoredComponent,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { agentError, exitWithError } from "../ui/agent-out";
import { ansi, errorLine, successLine } from "../ui/format";
import { isAgentMode } from "../ui/policy";
import { tryFindProjectRoot } from "../ui/resolve";

export default defineCommand({
  meta: {
    name: "toggle",
    description: "Toggle a plugin component (name:component) or open a picker",
  },
  args: {
    target: { type: "positional", description: "plugin or plugin:component", required: true },
    json: { type: "boolean", default: false },
  },
  async run({ args }) {
    const agentMode = await isAgentMode();
    const ref = parseComponentRef(args.target as string);
    const projectRoot = await tryFindProjectRoot();

    const plugin = await loadPluginByName(ref.name, projectRoot);
    if (!plugin) {
      exitWithError(agentMode, `Plugin '${ref.name}' is not installed`,
        "Run 'skilltap plugin' to see installed plugins.");
    }

    if (ref.component) {
      const component = findComponentInPlugin(plugin, ref.component);
      if (!component) {
        exitWithError(agentMode, `Component '${ref.component}' not found in plugin '${ref.name}'`,
          `Available: ${plugin.components.map((c) => c.name).join(", ") || "(none)"}`);
      }
      const result = await toggleInstalledComponent(plugin.name, component.type, component.name, { projectRoot });
      if (!result.ok) {
        errorLine(result.error.message);
        process.exit(1);
      }
      const action = result.value.nowActive ? "Enabled" : "Disabled";
      if (args.json) {
        process.stdout.write(`${JSON.stringify({ component: result.value.component, nowActive: result.value.nowActive }, null, 2)}\n`);
      } else {
        successLine(`${action} ${component.type}: ${component.name}`);
      }
      return;
    }

    // Bare name — picker (interactive only)
    if (agentMode) {
      agentError("toggle requires a component name in agent mode. Use plugin:component syntax.");
      process.exit(1);
    }
    await runPicker(plugin, projectRoot, args.json as boolean);
  },
});
```

`runPicker` mirrors the multiselect logic in `cli/src/commands/plugin/toggle.ts:81–117` — reused as-is (extract to a shared helper if duplication starts mattering; for now inline).

`loadPluginByName` is the helper from Unit 3.

### Unit 5 — `cli/src/commands/enable.ts` (top-level)

```typescript
async run({ args }) {
  // ... same plugin lookup as toggle ...

  if (ref.component) {
    const component = findComponentInPlugin(plugin, ref.component);
    if (!component) { /* error */ }
    if (component.active) {
      successLine(`${component.type}: ${component.name} is already enabled`);
      return;
    }
    const result = await toggleInstalledComponent(plugin.name, component.type, component.name, { projectRoot });
    /* render */
  }

  // Bare name — bulk enable all currently-inactive components
  const inactive = plugin.components.filter((c) => !c.active);
  if (inactive.length === 0) {
    process.stdout.write(`No inactive components in plugin '${plugin.name}'.\n`);
    return;
  }
  const results = [];
  for (const c of inactive) {
    const r = await toggleInstalledComponent(plugin.name, c.type, c.name, { projectRoot });
    results.push({ component: c, ok: r.ok, error: r.ok ? undefined : r.error.message });
  }
  /* render summary */
}
```

### Unit 6 — `cli/src/commands/disable.ts` (top-level)

Mirror of `enable.ts` but filters `c.active` and toggles active components.

### Unit 7 — Register top-level commands in `cli/src/index.ts`

Add to `subCommands`:

```typescript
toggle:  () => import("./commands/toggle").then((m) => m.default),
enable:  () => import("./commands/enable").then((m) => m.default),
disable: () => import("./commands/disable").then((m) => m.default),
```

### Unit 8 — Smoke tests

A small test verifying `parseComponentRef` is the priority. CLI-level smoke testing requires a fully-installed plugin which is heavy. Skip CLI-level for this phase; the underlying `toggleInstalledComponent` is already tested.

## Implementation Order

1. Unit 1 (component-ref core).
2. Unit 2 (parser tests).
3. Unit 4 (toggle.ts) — references Unit 1.
4. Unit 5 (enable.ts).
5. Unit 6 (disable.ts).
6. Unit 7 (index.ts registration).

## Verification

```bash
bun test packages/core/src/plugin/component-ref.test.ts

# Smoke check: toggle help renders
SKILLTAP_NO_STARTUP=1 bun packages/cli/src/index.ts toggle --help
SKILLTAP_NO_STARTUP=1 bun packages/cli/src/index.ts enable --help
SKILLTAP_NO_STARTUP=1 bun packages/cli/src/index.ts disable --help

# Full v2 baseline
bun test packages/core/src/manifest/ packages/core/src/state/ packages/core/src/migrate/ packages/core/src/sync/ packages/core/src/plugin-v2/ packages/core/src/plugin/detect.test.ts packages/core/src/plugin/component-ref.test.ts packages/core/src/plugin/mcp-inject.claude-desktop.test.ts packages/core/src/schemas/config-v2.test.ts packages/core/src/policy-v2/ packages/core/src/status/ packages/core/src/try.test.ts
```

## Out of Scope

- `plugin enable` / `plugin disable` subcommands — top-level only suffices; symmetric `plugin enable` can land later if a user asks.
- Updating shell completions for `:component` dynamic completions — Phase 37 surface promotion.
- Updating existing `plugin toggle` to also accept `:component` for parity — small follow-up; defer if not needed.
