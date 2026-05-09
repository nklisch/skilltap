import type { Action, AppState, Screen } from "./types";
import { dashboardReducer, initialDashboardState } from "./dashboard";
import { findReducer, initialFindState } from "./find";
import { toggleReducer, initialToggleState } from "./toggle";
import { adoptReducer, initialAdoptState } from "./adopt";

export function initialAppState(initial: Screen = "dashboard"): AppState {
  switch (initial) {
    case "dashboard":
      return { screen: "dashboard", state: initialDashboardState };
    case "find":
      return { screen: "find", state: initialFindState };
    case "toggle":
      return { screen: "toggle", state: initialToggleState };
    case "adopt":
      return { screen: "adopt", state: initialAdoptState };
  }
}

export function appReducer(state: AppState, action: Action): AppState {
  if (action.type === "navigate") {
    return initialAppState(action.screen);
  }
  if (action.type === "exit") return state;

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
