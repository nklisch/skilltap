import type { Action, FindState } from "./types";

export const initialFindState: FindState = {
  query: "",
  results: [],
  selectedIndex: 0,
  loading: false,
};

export function findReducer(state: FindState, action: Action): FindState {
  switch (action.type) {
    case "find:query":
      return { ...state, query: action.query, selectedIndex: 0, loading: action.query.length > 0 };
    case "find:results":
      return { ...state, results: action.results, selectedIndex: 0, loading: false };
    case "find:cursor":
      return {
        ...state,
        selectedIndex: Math.max(
          0,
          Math.min(state.results.length - 1, state.selectedIndex + action.delta),
        ),
      };
    default:
      return state;
  }
}
