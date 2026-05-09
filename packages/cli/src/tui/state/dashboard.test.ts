import { describe, expect, test } from "bun:test";
import { dashboardReducer, initialDashboardState } from "./dashboard";
import type { Action, DashboardState } from "./types";

describe("dashboardReducer", () => {
  test("initial state has correct defaults", () => {
    expect(initialDashboardState.tab).toBe("installed");
    expect(initialDashboardState.selectedIndex).toBe(0);
    expect(initialDashboardState.loading).toBe(false);
  });

  test("dashboard:tab switches tab and resets cursor", () => {
    const state: DashboardState = { ...initialDashboardState, tab: "installed", selectedIndex: 5 };
    const next = dashboardReducer(state, { type: "dashboard:tab", tab: "taps" });
    expect(next.tab).toBe("taps");
    expect(next.selectedIndex).toBe(0);
  });

  test("dashboard:tab switches to each valid tab", () => {
    for (const tab of ["installed", "taps", "updates", "drift"] as const) {
      const next = dashboardReducer(initialDashboardState, { type: "dashboard:tab", tab });
      expect(next.tab).toBe(tab);
    }
  });

  test("dashboard:cursor moves forward", () => {
    const state: DashboardState = { ...initialDashboardState, selectedIndex: 2 };
    const next = dashboardReducer(state, { type: "dashboard:cursor", delta: 1 });
    expect(next.selectedIndex).toBe(3);
  });

  test("dashboard:cursor moves backward", () => {
    const state: DashboardState = { ...initialDashboardState, selectedIndex: 3 };
    const next = dashboardReducer(state, { type: "dashboard:cursor", delta: -1 });
    expect(next.selectedIndex).toBe(2);
  });

  test("dashboard:cursor doesn't go below 0", () => {
    const state: DashboardState = { ...initialDashboardState, selectedIndex: 0 };
    const next = dashboardReducer(state, { type: "dashboard:cursor", delta: -1 });
    expect(next.selectedIndex).toBe(0);
  });

  test("dashboard:tab change resets cursor regardless of current position", () => {
    const state: DashboardState = { ...initialDashboardState, tab: "taps", selectedIndex: 99 };
    const next = dashboardReducer(state, { type: "dashboard:tab", tab: "updates" });
    expect(next.selectedIndex).toBe(0);
    expect(next.tab).toBe("updates");
  });

  test("unknown action is a no-op", () => {
    const action = { type: "find:query", query: "foo" } as Action;
    const next = dashboardReducer(initialDashboardState, action);
    expect(next).toBe(initialDashboardState);
  });
});
