import { describe, expect, test } from "bun:test";
import { toggleReducer, initialToggleState } from "./toggle";
import type { Action, ToggleState } from "./types";

const COMPONENTS: ToggleState["components"] = [
  { name: "mcp-server", active: true },
  { name: "agent-def", active: false },
  { name: "skill-files", active: true },
];

describe("toggleReducer", () => {
  test("initial state has correct defaults", () => {
    expect(initialToggleState.step).toBe("type");
    expect(initialToggleState.type).toBeNull();
    expect(initialToggleState.selectedName).toBeNull();
    expect(initialToggleState.components).toEqual([]);
    expect(initialToggleState.selectedComponentIndices).toEqual([]);
  });

  test("toggle:set-type advances step to name and stores type", () => {
    const next = toggleReducer(initialToggleState, { type: "toggle:set-type", value: "plugin" });
    expect(next.step).toBe("name");
    expect(next.type).toBe("plugin");
    expect(next.selectedName).toBeNull();
  });

  test("toggle:set-type works for all valid types", () => {
    for (const value of ["skill", "plugin", "mcp", null] as const) {
      const next = toggleReducer(initialToggleState, { type: "toggle:set-type", value });
      expect(next.type).toBe(value);
    }
  });

  test("toggle:set-name advances step to components and clears components", () => {
    const state: ToggleState = { ...initialToggleState, step: "name", type: "plugin" };
    const next = toggleReducer(state, { type: "toggle:set-name", value: "foo-plugin" });
    expect(next.step).toBe("components");
    expect(next.selectedName).toBe("foo-plugin");
    expect(next.components).toEqual([]);
  });

  test("toggle:components-loaded populates components and resets selection", () => {
    const state: ToggleState = {
      ...initialToggleState,
      step: "components",
      selectedComponentIndices: [0, 2],
    };
    const next = toggleReducer(state, { type: "toggle:components-loaded", components: COMPONENTS });
    expect(next.components).toEqual(COMPONENTS);
    expect(next.selectedComponentIndices).toEqual([]);
  });

  test("toggle:component-toggle adds index when not selected", () => {
    const state: ToggleState = {
      ...initialToggleState,
      step: "components",
      components: COMPONENTS,
      selectedComponentIndices: [0],
    };
    const next = toggleReducer(state, { type: "toggle:component-toggle", index: 2 });
    expect(next.selectedComponentIndices).toContain(0);
    expect(next.selectedComponentIndices).toContain(2);
  });

  test("toggle:component-toggle removes index when already selected", () => {
    const state: ToggleState = {
      ...initialToggleState,
      step: "components",
      components: COMPONENTS,
      selectedComponentIndices: [0, 1, 2],
    };
    const next = toggleReducer(state, { type: "toggle:component-toggle", index: 1 });
    expect(next.selectedComponentIndices).not.toContain(1);
    expect(next.selectedComponentIndices).toContain(0);
    expect(next.selectedComponentIndices).toContain(2);
  });

  test("toggle:step-back from components returns to name step", () => {
    const state: ToggleState = {
      ...initialToggleState,
      step: "components",
      type: "plugin",
      selectedName: "foo",
      components: COMPONENTS,
      selectedComponentIndices: [0],
    };
    const next = toggleReducer(state, { type: "toggle:step-back" });
    expect(next.step).toBe("name");
    expect(next.components).toEqual([]);
    expect(next.selectedComponentIndices).toEqual([]);
  });

  test("toggle:step-back from name returns to type step", () => {
    const state: ToggleState = {
      ...initialToggleState,
      step: "name",
      type: "skill",
      selectedName: null,
    };
    const next = toggleReducer(state, { type: "toggle:step-back" });
    expect(next.step).toBe("type");
    expect(next.type).toBeNull();
    expect(next.selectedName).toBeNull();
  });

  test("toggle:step-back from type step is a no-op", () => {
    const next = toggleReducer(initialToggleState, { type: "toggle:step-back" });
    expect(next).toBe(initialToggleState);
  });

  test("unknown action is a no-op", () => {
    const action = { type: "find:query", query: "foo" } as Action;
    const next = toggleReducer(initialToggleState, action);
    expect(next).toBe(initialToggleState);
  });
});
