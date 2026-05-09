import type { Action, AdoptState } from "./types";

export const initialAdoptState: AdoptState = {
  candidates: [],
  focusIndex: 0,
  selectedIndices: [],
  perItemMode: new Map(),
  loading: false,
};

export function adoptReducer(state: AdoptState, action: Action): AdoptState {
  switch (action.type) {
    case "adopt:candidates-loaded":
      return {
        ...state,
        candidates: action.candidates,
        focusIndex: 0,
        selectedIndices: [],
        perItemMode: new Map(),
        loading: false,
      };
    case "adopt:cursor": {
      const next = Math.max(
        0,
        Math.min(state.candidates.length - 1, state.focusIndex + action.delta),
      );
      return { ...state, focusIndex: next };
    }
    case "adopt:select-toggle": {
      const idx = state.focusIndex;
      const already = state.selectedIndices.includes(idx);
      const selectedIndices = already
        ? state.selectedIndices.filter((i) => i !== idx)
        : [...state.selectedIndices, idx];
      return { ...state, selectedIndices };
    }
    case "adopt:mode-toggle": {
      const candidate = state.candidates[state.focusIndex];
      if (!candidate) return state;
      const current = state.perItemMode.get(candidate.name) ?? "track-in-place";
      const next = current === "track-in-place" ? "move" : "track-in-place";
      const perItemMode = new Map(state.perItemMode);
      perItemMode.set(candidate.name, next);
      return { ...state, perItemMode };
    }
    default:
      return state;
  }
}
