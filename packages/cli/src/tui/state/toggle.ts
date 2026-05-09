import type { Action, ToggleState } from "./types";

export const initialToggleState: ToggleState = {
  step: "type",
  type: null,
  selectedName: null,
  components: [],
  selectedComponentIndices: [],
};

export function toggleReducer(state: ToggleState, action: Action): ToggleState {
  switch (action.type) {
    case "toggle:set-type":
      return { ...state, type: action.value, step: "name", selectedName: null };
    case "toggle:set-name":
      return { ...state, selectedName: action.value, step: "components", components: [] };
    case "toggle:components-loaded":
      return { ...state, components: action.components, selectedComponentIndices: [] };
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
        return { ...state, step: "name", components: [], selectedComponentIndices: [] };
      }
      if (state.step === "name") {
        return { ...state, step: "type", type: null, selectedName: null };
      }
      return state;
    }
    default:
      return state;
  }
}
