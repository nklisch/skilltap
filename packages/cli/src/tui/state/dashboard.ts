import type { Action, DashboardState } from "./types";

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
