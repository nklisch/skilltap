# Design: Phase 44 — TUI Dashboard (Ink)

## Overview

Phase 44 adds a multi-screen terminal UI as the rich-mode fork of the `Output` abstraction
shipped in Phase 41. Bare `skilltap` (TTY only) opens an Ink-rendered dashboard with four
screens: Dashboard, Find, Toggle, Adopt. Plain mode (piped stdin/stdout) and JSON mode are
unchanged — they remain headless.

The TUI is **dispatch-only** for state-mutating actions: every action a user takes in the
TUI calls the same core functions that Phase 42's flat commands call. The TUI never
duplicates business logic; it's a richer surface for the same operations.

A **Spike Unit (Unit 0)** validates Ink-on-Bun stability before the rest of the phase
commits. Phase 41's pre-mortem flagged this as the riskiest assumption. The Spike's
verification gate must pass before Units 1+ proceed.

## Acceptance criteria (project-wide)

- `bun run dev` (no args, TTY) opens the TUI dashboard.
- `bun run dev` (no args, non-TTY) errors with hint: "skilltap requires a TTY for the
  dashboard. Run `skilltap status` for headless output."
- `bun run dev` opens the dashboard cleanly; `q` or Ctrl+C exits without leaving the
  terminal in raw mode.
- `bun run dev | cat` (piped) renders no TUI; falls through to `status` text output.
- All four screens render: Dashboard tabs, Find type-ahead, Toggle picker, Adopt list.
- Each screen has a flat-command equivalent reachable without the TUI (existing Phase
  42 commands all keep working).
- `bun test` passes; PTY smoke tests cover dashboard mount + clean exit + screen
  navigation; reducer unit tests cover state transitions.
- Existing tests continue to pass — no breakage in Phase 39/40/41/42/43 work.

## Out of scope (deferred follow-ups)

- Mouse support (keyboard-only).
- Themable colors (use existing `format.ts` ANSI palette).
- Resize handling beyond what Ink provides natively.
- Persistent UI state across invocations (each `skilltap` invocation starts fresh).
- Editing skill/plugin files in-TUI ($EDITOR launch is fine; in-line edit is not).
- Phase 45 migrate flow integration (a later phase).

## Architectural Options

### Option A — Pure Ink + hooks
Each screen is a React component with `useState`/`useEffect`/`useApp`. State and rendering
intermingled. Idiomatic Ink. Hard to unit-test without mounting components.

### Option B — Reducer + Ink renderer split (chosen)
Pure reducer functions return next-state from `(state, action)`. Ink components are dumb
renderers that take state + dispatch via props. Reducers test with `bun:test` (no Ink
mount); components test via `ink-testing-library` snapshot pattern. Mirrors Phase 41's
port/adapter split (output interface in core, adapters in cli). More boilerplate but the
state machine is the source of truth.

### Option C — State machine library (xstate)
External library for state. Visual diagrams, time-travel debugging. Heavyweight. Project
doesn't use it. Adds a dependency for limited gain over Option B.

**Choice: Option B.** Aligns with existing port/adapter pattern. Reducer tests are the
primary test surface — they catch regressions without flaky rendering. Ink components
become simple enough that snapshot tests are sufficient. PTY smoke tests provide an
end-to-end safety net.

## Trickiest Unit — Spike (Unit 0)

The Phase 41 pre-mortem identified Ink-on-Bun as the riskiest assumption. Bun's terminal
raw-mode handling and signal cleanup have historically had quirks. **Unit 0 validates
this BEFORE any other unit lands.** If the Spike fails, the design pivots to a richer
@clack/prompts-based screen orchestrator (no React, no raw-mode hooks) — but we don't
write that fallback unless we need it.

### Unit 0 — Spike: Ink-on-Bun stability

**File**: `packages/cli/src/tui/spike.ts` (deleted after the Spike succeeds)

```typescript
import { render, Box, Text, useApp, useInput } from "ink";
import React from "react";

const SpikeApp: React.FC = () => {
  const { exit } = useApp();
  useInput((input, key) => {
    if (input === "q" || (key.ctrl && input === "c")) {
      exit();
    }
  });
  return (
    <Box flexDirection="column">
      <Text color="green">✓ Ink renders under Bun</Text>
      <Text dimColor>Press q to exit cleanly</Text>
    </Box>
  );
};

export function runSpike(): void {
  const { unmount, waitUntilExit } = render(<SpikeApp />);
  waitUntilExit().then(() => unmount());
}
```

**Verification gate** (the Spike must pass these before Units 1+ proceed):

```bash
# 1. Install Ink and run the spike
bun add ink react
bun add -d @types/react ink-testing-library

# 2. Run the spike interactively
bun run packages/cli/src/tui/spike.ts
# Expected: "✓ Ink renders under Bun" + "Press q to exit cleanly"
# Press q → clean exit, terminal restored.
# Press Ctrl+C → clean exit, terminal restored.

# 3. PTY-driven smoke test
# Use packages/test-utils/src/pty.ts to spawn the spike, send "q", assert clean exit.

# 4. Build verification
bun run build
```

**Acceptance criteria** (Unit 0):
- [ ] `ink`, `react` installed as runtime deps; `@types/react`, `ink-testing-library` as dev deps.
- [ ] `bun run packages/cli/src/tui/spike.ts` mounts and exits cleanly on `q`.
- [ ] Terminal is not left in raw mode after exit (try typing in shell — if echo is broken, raw mode wasn't restored).
- [ ] Ctrl+C exits cleanly without dumping a stack trace.
- [ ] PTY smoke test passes: `runInteractive` mounts spike, sends `q`, asserts exit code 0 and stripped output contains "Ink renders under Bun".
- [ ] `bun run build` produces a working binary that includes the spike (binary size delta < 5MB).

**If the Spike fails:**
- Document the specific failure in `docs/design/phase-44-spike-result.md`.
- Pivot to a fallback design (clack-based multi-screen orchestrator) — written as a
  separate addendum, not in this doc.
- Do not proceed with Units 1+ until either the spike works or the fallback is approved.

After verification: delete `spike.ts`. The infrastructure (deps, PTY pattern) carries
forward to Unit 1+.

---

## Implementation Units

### Unit 1 — TUI module skeleton + state types

**Files**:
- `packages/cli/src/tui/state/types.ts` — pure types.
- `packages/cli/src/tui/state/app.ts` — root reducer combining screen reducers.
- `packages/cli/src/tui/keys.ts` — key-binding registry.

```typescript
// state/types.ts

export type Screen = "dashboard" | "find" | "toggle" | "adopt";

export type DashboardTab = "installed" | "taps" | "updates" | "drift";

export interface DashboardState {
  tab: DashboardTab;
  selectedIndex: number;     // index into the current tab's list
  loading: boolean;
}

export interface FindState {
  query: string;
  results: FindResult[];
  selectedIndex: number;
  loading: boolean;
}

export interface FindResult {
  name: string;
  description: string;
  source: string;        // tap name or registry name
  type: "skill" | "plugin";
}

export interface ToggleState {
  step: "type" | "name" | "components";
  type: "skill" | "plugin" | "mcp" | null;
  selectedName: string | null;
  components: { name: string; active: boolean }[];   // for plugin step
  selectedComponentIndices: number[];
}

export interface AdoptState {
  candidates: AdoptCandidate[];
  selectedIndices: number[];
  perItemMode: Map<string, "track-in-place" | "move">;   // candidate name → mode
  loading: boolean;
}

export interface AdoptCandidate {
  kind: "skill" | "plugin";
  name: string;
  source: string;          // location for skills, marketplace for plugins
  description?: string;
}

export type AppState =
  | { screen: "dashboard"; state: DashboardState }
  | { screen: "find"; state: FindState }
  | { screen: "toggle"; state: ToggleState }
  | { screen: "adopt"; state: AdoptState };

export type Action =
  | { type: "navigate"; screen: Screen }
  | { type: "exit" }
  | { type: "dashboard:tab"; tab: DashboardTab }
  | { type: "dashboard:cursor"; delta: -1 | 1 }
  | { type: "find:query"; query: string }
  | { type: "find:results"; results: FindResult[] }
  | { type: "find:cursor"; delta: -1 | 1 }
  | { type: "toggle:step-back" }
  | { type: "toggle:set-type"; value: ToggleState["type"] }
  | { type: "toggle:set-name"; value: string }
  | { type: "toggle:components-loaded"; components: ToggleState["components"] }
  | { type: "toggle:component-toggle"; index: number }
  | { type: "adopt:candidates-loaded"; candidates: AdoptCandidate[] }
  | { type: "adopt:cursor"; delta: -1 | 1 }
  | { type: "adopt:select-toggle" }       // toggle current item's selection
  | { type: "adopt:mode-toggle" };        // toggle current item's mode

export interface AppContext {
  // Injected dispatchers — TUI calls these to apply Phase 42 commands.
  // Each returns Result<void, UserError>; the TUI displays errors via the footer.
  dispatchInstall: (type: "skill" | "plugin" | "mcp", source: string) => Promise<{ ok: boolean; error?: string }>;
  dispatchToggle: (type: "skill" | "plugin" | "mcp", name: string, component?: string) => Promise<{ ok: boolean; error?: string }>;
  dispatchAdopt: (kind: "skill" | "plugin", name: string, mode: "track-in-place" | "move") => Promise<{ ok: boolean; error?: string }>;
  dispatchSync: () => Promise<{ ok: boolean; error?: string }>;
  // Read-only loaders for screen content
  loadDashboardData: (tab: DashboardTab) => Promise<unknown>;
  loadFindResults: (query: string) => Promise<FindResult[]>;
  loadToggleComponents: (type: "skill" | "plugin" | "mcp", name: string) => Promise<ToggleState["components"]>;
  loadAdoptCandidates: () => Promise<AdoptCandidate[]>;
}
```

```typescript
// state/app.ts
import type { Action, AppState } from "./types";
import { dashboardReducer, initialDashboardState } from "./dashboard";
import { findReducer, initialFindState } from "./find";
import { toggleReducer, initialToggleState } from "./toggle";
import { adoptReducer, initialAdoptState } from "./adopt";

export function initialAppState(initial: AppState["screen"] = "dashboard"): AppState {
  switch (initial) {
    case "dashboard": return { screen: "dashboard", state: initialDashboardState };
    case "find":      return { screen: "find",      state: initialFindState };
    case "toggle":    return { screen: "toggle",    state: initialToggleState };
    case "adopt":     return { screen: "adopt",     state: initialAdoptState };
  }
}

export function appReducer(state: AppState, action: Action): AppState {
  if (action.type === "navigate") {
    return initialAppState(action.screen);
  }
  if (action.type === "exit") return state;   // exit is handled by Ink useApp; reducer doesn't change state

  switch (state.screen) {
    case "dashboard":
      return { screen: "dashboard", state: dashboardReducer(state.state, action) };
    case "find":
      return { screen: "find", state: findReducer(state.state, action) };
    case "toggle":
      return { screen: "toggle", state: toggleReducer(state.state, action) };
    case "adopt":
      return { screen: "adopt", state: adoptReducer(state.state, action) };
  }
}
```

```typescript
// keys.ts
export interface KeyBinding {
  key: string;        // raw input from Ink useInput
  modifier?: "ctrl" | "shift";
  description: string;
  action: { type: string; [k: string]: unknown };
}

// Global keys that work on any screen
export const GLOBAL_KEYS: KeyBinding[] = [
  { key: "q",     description: "Quit",         action: { type: "exit" } },
  { key: "1",     description: "Dashboard",    action: { type: "navigate", screen: "dashboard" } },
  { key: "2",     description: "Find",         action: { type: "navigate", screen: "find" } },
  { key: "3",     description: "Toggle",       action: { type: "navigate", screen: "toggle" } },
  { key: "4",     description: "Adopt",        action: { type: "navigate", screen: "adopt" } },
];
```

**Acceptance criteria**:
- [ ] `state/types.ts` exports all types listed; no runtime code.
- [ ] `state/app.ts` exports `appReducer` and `initialAppState`; pure functions.
- [ ] `keys.ts` exports `GLOBAL_KEYS`.
- [ ] No imports from `react` or `ink` in `state/` (state is pure; rendering is separate).

---

### Unit 2 — Per-screen reducers (4 files)

Each reducer is in `packages/cli/src/tui/state/<screen>.ts`. Pure `(state, action) → state`
functions. Initial state exported alongside.

Example for dashboard (the simplest):

```typescript
// state/dashboard.ts
import type { Action, DashboardState, DashboardTab } from "./types";

export const initialDashboardState: DashboardState = {
  tab: "installed",
  selectedIndex: 0,
  loading: false,
};

export function dashboardReducer(state: DashboardState, action: Action): DashboardState {
  switch (action.type) {
    case "dashboard:tab":
      return { ...state, tab: action.tab, selectedIndex: 0 };
    case "dashboard:cursor":
      return { ...state, selectedIndex: Math.max(0, state.selectedIndex + action.delta) };
    default:
      return state;
  }
}
```

Each reducer has its own test file with tests for every action it handles, including
boundary cases (cursor goes negative, tab change resets cursor, etc.).

**Acceptance criteria**:
- [ ] Four reducer files exist: `dashboard.ts`, `find.ts`, `toggle.ts`, `adopt.ts`.
- [ ] Each exports `initial<Screen>State` and `<screen>Reducer`.
- [ ] Reducers are pure: same input → same output, no side effects.
- [ ] `bun test packages/cli/src/tui/state/` passes (Unit 7's tests).

---

### Unit 3 — Ink components (dumb renderers)

**Files**: `packages/cli/src/tui/screens/<Screen>.tsx` (4 files) + shared components.

Each component takes `state` + `dispatch` as props and renders. No business logic, no
state mutation, no async work — just rendering.

```typescript
// screens/Dashboard.tsx
import { Box, Text } from "ink";
import React from "react";
import type { Action, DashboardState } from "../state/types";
import { Tabs } from "./shared/Tabs";
import { Footer } from "./shared/Footer";

interface Props {
  state: DashboardState;
  dispatch: (action: Action) => void;
  data: unknown;   // tab-specific data loaded by the App
}

export const Dashboard: React.FC<Props> = ({ state, dispatch, data }) => {
  return (
    <Box flexDirection="column" height="100%">
      <Tabs
        current={state.tab}
        tabs={[
          { id: "installed", label: "Installed" },
          { id: "taps",      label: "Taps" },
          { id: "updates",   label: "Updates" },
          { id: "drift",     label: "Drift" },
        ]}
        onChange={(tab) => dispatch({ type: "dashboard:tab", tab })}
      />
      <Box flexGrow={1}>
        {/* render `data` per tab */}
      </Box>
      <Footer hints={[
        { key: "1-4", description: "switch tabs" },
        { key: "↑↓",   description: "navigate" },
        { key: "i",    description: "install" },
        { key: "r",    description: "remove" },
        { key: "t",    description: "toggle" },
        { key: "f",    description: "find" },
        { key: "a",    description: "adopt" },
        { key: "q",    description: "quit" },
      ]} />
    </Box>
  );
};
```

Shared components:
- `screens/shared/Tabs.tsx` — horizontal tab bar.
- `screens/shared/List.tsx` — selectable scrolling list with cursor.
- `screens/shared/DetailPane.tsx` — right-side detail pane.
- `screens/shared/Footer.tsx` — key-binding hints.

**Acceptance criteria**:
- [ ] Four screen components + four shared components exist.
- [ ] Components are pure: `(props) → JSX`, no internal state beyond what props provide.
- [ ] Each component has a snapshot test via `ink-testing-library` (Unit 7).

---

### Unit 4 — App root + router

**File**: `packages/cli/src/tui/App.tsx`

```typescript
import { Box, useApp, useInput, useReducer as useReducerInk } from "ink";
import React, { useCallback, useEffect, useReducer, useState } from "react";
import { initialAppState, appReducer } from "./state/app";
import { GLOBAL_KEYS } from "./keys";
import type { AppContext, AppState, Action, Screen } from "./state/types";
import { Dashboard } from "./screens/Dashboard";
import { Find } from "./screens/Find";
import { Toggle } from "./screens/Toggle";
import { Adopt } from "./screens/Adopt";

interface AppProps {
  initialScreen: Screen;
  context: AppContext;
}

export const App: React.FC<AppProps> = ({ initialScreen, context }) => {
  const { exit } = useApp();
  const [state, dispatch] = useReducer(appReducer, initialAppState(initialScreen));
  const [data, setData] = useState<unknown>(null);

  // Global key handler
  useInput((input, key) => {
    for (const binding of GLOBAL_KEYS) {
      if (binding.key === input) {
        if (binding.action.type === "exit") {
          exit();
          return;
        }
        dispatch(binding.action as Action);
        return;
      }
    }
    // Per-screen keys handled inside each screen component.
  });

  // Load data when screen changes
  useEffect(() => {
    let cancelled = false;
    (async () => {
      switch (state.screen) {
        case "dashboard":
          setData(await context.loadDashboardData(state.state.tab));
          break;
        case "find":
          if (state.state.query.length > 0) {
            const results = await context.loadFindResults(state.state.query);
            if (!cancelled) dispatch({ type: "find:results", results });
          }
          break;
        // ... other screens
      }
    })();
    return () => { cancelled = true; };
  }, [state.screen, /* tab change */, /* query change */]);

  switch (state.screen) {
    case "dashboard":
      return <Dashboard state={state.state} dispatch={dispatch} data={data} />;
    case "find":
      return <Find state={state.state} dispatch={dispatch} context={context} />;
    case "toggle":
      return <Toggle state={state.state} dispatch={dispatch} context={context} />;
    case "adopt":
      return <Adopt state={state.state} dispatch={dispatch} context={context} />;
  }
};
```

**Acceptance criteria**:
- [ ] `App.tsx` mounts via `render(<App {...props} />)`.
- [ ] Pressing `q` exits cleanly via `useApp().exit()`.
- [ ] Pressing `1`–`4` navigates to the corresponding screen.
- [ ] Effects are cancellable on unmount/rerender.

---

### Unit 5 — `mountTui()` entry + AppContext factory

**File**: `packages/cli/src/tui/index.ts`

```typescript
import { render } from "ink";
import React from "react";
import { App } from "./App";
import type { AppContext, Screen } from "./state/types";
import { createAppContext } from "./context";

export async function mountTui(initialScreen: Screen = "dashboard"): Promise<void> {
  const context = await createAppContext();
  const { waitUntilExit } = render(<App initialScreen={initialScreen} context={context} />);
  await waitUntilExit();
}
```

**File**: `packages/cli/src/tui/context.ts`

```typescript
import { findCommand } from "../commands/find";
import { ... } from "@skilltap/core";   // installSkill, installPlugin, installMcp, etc.
import type { AppContext } from "./state/types";

/**
 * Build an AppContext that dispatches TUI actions to the same core functions
 * Phase 42's flat commands use. The TUI is dispatch-only — no business logic
 * reimplementation.
 */
export async function createAppContext(): Promise<AppContext> {
  return {
    dispatchInstall: async (type, source) => {
      // Call the relevant core install function with TUI-friendly callbacks
      // (auto-confirm scan warnings; auto-pick on multi-skill — TUI handles
      // user picks before this dispatch). Return Result.
    },
    // ... other dispatchers
    loadDashboardData: async (tab) => { /* status data per tab */ },
    loadFindResults: async (query) => { /* search */ },
    loadToggleComponents: async (type, name) => { /* read state */ },
    loadAdoptCandidates: async () => { /* discoverAllAdoptable */ },
  };
}
```

**Acceptance criteria**:
- [ ] `mountTui("dashboard")` works.
- [ ] `mountTui("find")` works.
- [ ] AppContext methods all return Result-shaped objects (`{ ok, error? }`).
- [ ] Loaders never throw; errors surface via Result.

---

### Unit 6 — Bare `skilltap` integration

**File**: `packages/cli/src/index.ts` (modify existing)

Replace the current bare-command-routes-to-status logic:

```typescript
// Current (around line 348):
if (process.argv.length === 2) {
  const statusCmd = await import("./commands/status").then((m) => m.default);
  await statusCmd.run({ ... });
  process.exit(0);
}

// New:
if (process.argv.length === 2) {
  if (process.stdout.isTTY) {
    const { mountTui } = await import("./tui");
    await mountTui("dashboard");
    process.exit(0);
  } else {
    process.stderr.write(
      "skilltap requires a TTY for the dashboard.\n" +
      "  hint: Run `skilltap status` for headless output.\n",
    );
    process.exit(1);
  }
}
```

**Acceptance criteria**:
- [ ] Bare `bun run dev` (TTY) opens the dashboard.
- [ ] Bare `bun run dev` (piped, non-TTY) errors with the hint.
- [ ] `bun run dev status` (explicit) still works in any mode.

---

### Unit 7 — Tests

#### Reducer tests (state/<screen>.test.ts)

Pure unit tests — no Ink, no rendering. ~5–10 tests per reducer.

```typescript
// state/dashboard.test.ts
import { describe, expect, test } from "bun:test";
import { dashboardReducer, initialDashboardState } from "./dashboard";

describe("dashboardReducer", () => {
  test("dashboard:tab switches tab and resets cursor", () => {
    const state = { ...initialDashboardState, tab: "installed", selectedIndex: 5 };
    const next = dashboardReducer(state, { type: "dashboard:tab", tab: "taps" });
    expect(next.tab).toBe("taps");
    expect(next.selectedIndex).toBe(0);
  });
  test("dashboard:cursor doesn't go below 0", () => { ... });
  test("unknown action is a no-op", () => { ... });
});
```

Cover for all four reducers; test count: ~30 reducer tests.

#### Component snapshot tests (screens/<Screen>.test.tsx)

Using `ink-testing-library`:

```typescript
import { render } from "ink-testing-library";
import { describe, expect, test } from "bun:test";
import React from "react";
import { Dashboard } from "./Dashboard";
import { initialDashboardState } from "../state/dashboard";

describe("Dashboard component", () => {
  test("renders Installed tab by default", () => {
    const { lastFrame } = render(
      <Dashboard
        state={initialDashboardState}
        dispatch={() => {}}
        data={null}
      />
    );
    expect(lastFrame()).toContain("Installed");
  });
});
```

#### PTY smoke tests (tui.smoke.test.ts)

End-to-end via `runInteractive`:

```typescript
test("bare skilltap (TTY) opens dashboard and exits on q", async () => {
  const { homeDir, configDir, cleanup } = await createTestEnv();
  try {
    const session = await runInteractive(["bun", "run", "--bun", CLI_ENTRY], {
      cwd: process.cwd(),
      env: { SKILLTAP_HOME: homeDir, XDG_CONFIG_HOME: configDir },
    });
    await session.waitForText("Installed", 5000);
    session.send("q");
    const { exitCode } = await session.finish();
    expect(exitCode).toBe(0);
  } finally {
    await cleanup();
  }
});
```

Cover at minimum:
- Bare skilltap opens dashboard.
- `q` exits cleanly (exit code 0).
- Number-key navigation switches screens.
- Ctrl+C exits cleanly (exit code 130 or 0 — verify what Ink does).

#### Headless equivalents test

Verify each TUI screen has a flat-command path:
- Dashboard → `skilltap status` (existing test).
- Find → `skilltap find <query> --json` (existing test).
- Toggle → `skilltap toggle plugin foo:bar` (existing test).
- Adopt → `skilltap adopt <name>` (existing test).

These tests already exist; no new test work — just confirm Phase 44 doesn't break them.

**Acceptance criteria**:
- [ ] Reducer tests pass; ~30 tests total.
- [ ] Component snapshot tests pass; ~12 tests total.
- [ ] PTY smoke tests pass; at least 4 tests covering the cases above.
- [ ] Existing flat-command tests still pass.

---

## Implementation Order

1. **Unit 0 — Spike**. Verification gate must pass before anything else.
2. **Unit 1 — types + root reducer + keys**. Pure types and pure functions; no rendering yet. Test via reducer tests.
3. **Unit 2 — per-screen reducers**. Implement dashboard reducer first (simplest), then find, toggle, adopt. Each with its test file.
4. **Unit 3 — Ink components**. Dashboard component first (uses simplest reducer). Then Find, Toggle, Adopt. Shared components (`Tabs`, `List`, `Footer`) emerge as needed.
5. **Unit 4 — App root**. Wires reducer to components. Handles global keys.
6. **Unit 5 — mountTui + AppContext**. Wires App to core dispatch functions.
7. **Unit 6 — index.ts integration**. Bare skilltap → mountTui.
8. **Unit 7 — tests**. Reducer + component + PTY smoke tests.

Spike (Unit 0) blocks everything. Units 1+2 can land in one commit (pure code). Units 3+4
should land together (rendering + wiring). Units 5+6 land together (entry + integration).
Unit 7 tests are interleaved with implementation — write the reducer test alongside the
reducer file, the component snapshot alongside the component file.

Suggested agent split (after Spike passes):
- **Agent A**: Units 1+2 (state types + reducers) + their tests.
- **Agent B**: Units 3+4 (components + App root) + their snapshot tests.
- **Agent C**: Units 5+6+7 (mountTui + AppContext + index.ts wiring + PTY smoke tests).

Sequential: A → B → C. State must compile before components consume it; components must
exist before App root mounts them; App root must work before index.ts routes to it.

## Pre-Mortem

**Riskiest assumption**: That Ink-on-Bun is stable enough for production use, particularly:
- Raw mode handling (signal cleanup, terminal restoration).
- React 18+'s concurrent features under Bun's runtime.
- `ink-testing-library` actually testing renders correctly.

**Mitigation**: Unit 0 Spike validates ALL of this before any Unit 1+ work commits. If
the Spike reveals issues, we have a clean off-ramp (either fix specific issues or pivot
to a clack-based fallback).

**What would have to be true to fail in production**:
- A user's terminal is left in raw mode after exit. Mitigation: `useApp().exit()` triggers Ink's cleanup; PTY tests verify.
- An async error inside a screen's effect crashes the whole React tree. Mitigation: Result-typed dispatchers; error surfaces via footer text, not exception.
- Bun-specific signal handling differs from Node — Ctrl+C doesn't restore terminal. Mitigation: PTY tests assert terminal echo works after exit (if echo is broken, the test catches it).

**Fallback if the Spike fails**:
A clack-based multi-screen orchestrator. Phase 41 already has `pickOne()` (refactored
from earlier picker code). Extend with `multiSelect()` and `freeFormInput()`. Build a
linear flow per "screen" via clack's existing primitives. Less rich UX (no persistent
side-by-side panes), but reliable on Bun.

**Where I'm least sure**:
- Whether the dashboard's "tabs with right-side detail pane" layout works smoothly in
  Ink's flexbox layout. Mitigation: snapshot tests catch layout regressions; if the
  layout is too brittle, fall back to a single-pane scrolling list per tab.
- Whether PTY tests can be made stable on CI (Ink's render timing + node-pty's input
  buffering can be flaky). Mitigation: use `waitForText` with generous timeouts;
  the existing `runInteractive` helper is already battle-tested for clack output.

## Risks (post pre-mortem)

| Risk | Likelihood | Mitigation |
|---|---|---|
| Bun + Ink raw mode quirks | Medium | Unit 0 Spike + PTY smoke tests on every signal path |
| React peer-dep version conflicts | Low | Pin `react` to a known-working version (18.x); ink declares 18.x as peer |
| Async load races (user changes screen mid-load) | Medium | `cancelled` flag in useEffect; cancellation via cleanup function |
| ink-testing-library + bun:test integration | Medium | Spike includes one ink-testing-library snapshot to validate the integration |
| TUI bundle bloats the binary | Low | `bun build --compile` already optimizes; budget < 5MB increase |
| Effect timing — data loads after navigation | Medium | useEffect cleanup; `loading: boolean` per screen state |
| `useApp().exit()` doesn't restore terminal in some shells | Low | Manual cleanup via `process.stdout.write("\x1b[?25h\x1b[?1049l")` if needed; PTY test verifies |

## Verification Checklist

```bash
# 1. Spike (Unit 0) — manual
bun run packages/cli/src/tui/spike.ts
# Press q. Expect: clean exit, terminal echoes work afterward.

# 2. Build
bun run build

# 3. Reducer tests
bun test packages/cli/src/tui/state/

# 4. Component snapshot tests
bun test packages/cli/src/tui/screens/

# 5. PTY smoke tests
bun test packages/cli/src/tui/*.smoke.test.ts

# 6. Full suite
bun test

# 7. Manual smoke
bun run dev                # opens TUI
bun run dev | cat          # falls through to status
bun run dev status         # explicit, works in any mode
bun run dev install skill foo   # flat command still works
bun run dev --help              # shows command tree
```

Expected outcomes:
- Spike passes.
- Build clean.
- Tests green.
- Bare skilltap opens TUI in TTY; errors with hint when piped.
- Flat commands all still work (Phase 42's surface preserved).
- Ctrl+C in TUI exits cleanly without leaving raw mode.
