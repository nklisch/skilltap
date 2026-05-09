import type { Action, ToggleState } from "./types";

export const initialToggleState: ToggleState = {
  step: "type",
  type: null,
  selectedName: null,
  names: [],
  namesLoading: false,
  focusIndex: 0,
  components: [],
  selectedComponentIndices: [],
};

function focusListLength(state: ToggleState): number {
  if (state.step === "type") return 3;
  if (state.step === "name") return state.names.length;
  return state.components.length;
}

export function toggleReducer(state: ToggleState, action: Action): ToggleState {
  switch (action.type) {
    case "toggle:set-type":
      return {
        ...state,
        type: action.value,
        step: "name",
        selectedName: null,
        names: [],
        namesLoading: false,
        focusIndex: 0,
      };
    case "toggle:set-name":
      return {
        ...state,
        selectedName: action.value,
        step: "components",
        components: [],
        focusIndex: 0,
      };
    case "toggle:names-loading":
      return { ...state, namesLoading: true };
    case "toggle:names-loaded":
      return {
        ...state,
        names: action.names,
        namesLoading: false,
        focusIndex: 0,
      };
    case "toggle:focus": {
      const len = focusListLength(state);
      if (len === 0) return state;
      const next = Math.max(0, Math.min(len - 1, state.focusIndex + action.delta));
      return { ...state, focusIndex: next };
    }
    case "toggle:components-loaded":
      return {
        ...state,
        components: action.components,
        selectedComponentIndices: [],
        focusIndex: 0,
      };
    case "toggle:component-toggle": {
      const idx = action.index;
      const already = state.selectedComponentIndices.includes(idx);
      const selectedComponentIndices = already
        ? state.selectedComponentIndices.filter((i) => i !== idx)
        : [...state.selectedComponentIndices, idx];
      return { ...state, selectedComponentIndices };
    }
    case "toggle:step-back": {
      if (state.step === "components") {
        return {
          ...state,
          step: "name",
          components: [],
          selectedComponentIndices: [],
          focusIndex: 0,
        };
      }
      if (state.step === "name") {
        return {
          ...state,
          step: "type",
          type: null,
          selectedName: null,
          names: [],
          namesLoading: false,
          focusIndex: 0,
        };
      }
      return state;
    }
    default:
      return state;
  }
}
