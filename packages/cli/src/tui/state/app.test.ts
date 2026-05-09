import { describe, expect, test } from "bun:test";
import { appReducer, initialAppState } from "./app";
import { initialDashboardState } from "./dashboard";
import { initialFindState } from "./find";
import { initialToggleState } from "./toggle";
import type { Action, AppState } from "./types";

describe("initialAppState", () => {
  test("defaults to dashboard screen", () => {
    const state = initialAppState();
    expect(state.screen).toBe("dashboard");
    if (state.screen === "dashboard") {
      expect(state.state).toEqual(initialDashboardState);
    }
  });

  test("can initialize to find screen", () => {
    const state = initialAppState("find");
    expect(state.screen).toBe("find");
    if (state.screen === "find") {
      expect(state.state).toEqual(initialFindState);
    }
  });

  test("can initialize to toggle screen", () => {
    const state = initialAppState("toggle");
    expect(state.screen).toBe("toggle");
    if (state.screen === "toggle") {
      expect(state.state).toEqual(initialToggleState);
    }
  });

  test("can initialize to adopt screen", () => {
    const state = initialAppState("adopt");
    expect(state.screen).toBe("adopt");
    if (state.screen === "adopt") {
      expect(state.state.focusIndex).toBe(0);
      expect(state.state.candidates).toEqual([]);
    }
  });
});

describe("appReducer", () => {
  test("navigate reinitializes the target screen", () => {
    const state: AppState = {
      screen: "dashboard",
      state: { tab: "updates", selectedIndex: 5, loading: false },
    };
    const next = appReducer(state, { type: "navigate", screen: "find" });
    expect(next.screen).toBe("find");
    if (next.screen === "find") {
      expect(next.state).toEqual(initialFindState);
    }
  });

  test("navigate to same screen reinitializes state", () => {
    const state: AppState = {
      screen: "dashboard",
      state: { tab: "updates", selectedIndex: 5, loading: false },
    };
    const next = appReducer(state, { type: "navigate", screen: "dashboard" });
    expect(next.screen).toBe("dashboard");
    if (next.screen === "dashboard") {
      expect(next.state).toEqual(initialDashboardState);
    }
  });

  test("exit is a no-op", () => {
    const state = initialAppState("dashboard");
    const next = appReducer(state, { type: "exit" });
    expect(next).toBe(state);
  });

  test("dashboard actions route to dashboard reducer", () => {
    const state = initialAppState("dashboard");
    const next = appReducer(state, { type: "dashboard:tab", tab: "updates" });
    expect(next.screen).toBe("dashboard");
    if (next.screen === "dashboard") {
      expect(next.state.tab).toBe("updates");
    }
  });

  test("find actions route to find reducer", () => {
    const state = initialAppState("find");
    const next = appReducer(state, { type: "find:query", query: "zod" });
    expect(next.screen).toBe("find");
    if (next.screen === "find") {
      expect(next.state.query).toBe("zod");
    }
  });

  test("toggle actions route to toggle reducer", () => {
    const state = initialAppState("toggle");
    const next = appReducer(state, { type: "toggle:set-type", value: "mcp" });
    expect(next.screen).toBe("toggle");
    if (next.screen === "toggle") {
      expect(next.state.type).toBe("mcp");
    }
  });

  test("adopt actions route to adopt reducer", () => {
    const state = initialAppState("adopt");
    const candidates = [
      { kind: "skill" as const, name: "foo", source: "/path/foo" },
    ];
    const next = appReducer(state, {
      type: "adopt:candidates-loaded",
      candidates,
    });
    expect(next.screen).toBe("adopt");
    if (next.screen === "adopt") {
      expect(next.state.candidates).toEqual(candidates);
    }
  });

  test("dashboard actions are no-ops on other screens", () => {
    const state = initialAppState("find");
    const action: Action = { type: "dashboard:tab", tab: "taps" };
    const next = appReducer(state, action);
    expect(next.screen).toBe("find");
    if (next.screen === "find") {
      expect(next.state).toEqual(initialFindState);
    }
  });

  test("navigate through all screens in sequence", () => {
    let state = initialAppState();
    for (const screen of ["find", "toggle", "adopt", "dashboard"] as const) {
      state = appReducer(state, { type: "navigate", screen });
      expect(state.screen).toBe(screen);
    }
  });
});
