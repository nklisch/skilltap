import { describe, expect, test } from "bun:test";
import { adoptReducer, initialAdoptState } from "./adopt";
import type { Action, AdoptCandidate, AdoptState } from "./types";

const CANDIDATES: AdoptCandidate[] = [
  { kind: "skill", name: "alpha", source: "/home/user/.claude/skills/alpha" },
  { kind: "plugin", name: "beta", source: "marketplace", description: "Beta plugin" },
  { kind: "skill", name: "gamma", source: "/home/user/.claude/skills/gamma" },
];

describe("adoptReducer", () => {
  test("initial state has correct defaults", () => {
    expect(initialAdoptState.candidates).toEqual([]);
    expect(initialAdoptState.focusIndex).toBe(0);
    expect(initialAdoptState.selectedIndices).toEqual([]);
    expect(initialAdoptState.perItemMode.size).toBe(0);
    expect(initialAdoptState.loading).toBe(false);
  });

  test("adopt:candidates-loaded populates candidates and resets selection", () => {
    const state: AdoptState = {
      ...initialAdoptState,
      focusIndex: 2,
      selectedIndices: [0, 1],
      perItemMode: new Map([["alpha", "move"]]),
      loading: true,
    };
    const next = adoptReducer(state, { type: "adopt:candidates-loaded", candidates: CANDIDATES });
    expect(next.candidates).toEqual(CANDIDATES);
    expect(next.focusIndex).toBe(0);
    expect(next.selectedIndices).toEqual([]);
    expect(next.perItemMode.size).toBe(0);
    expect(next.loading).toBe(false);
  });

  test("adopt:cursor moves focus forward", () => {
    const state: AdoptState = { ...initialAdoptState, candidates: CANDIDATES, focusIndex: 0 };
    const next = adoptReducer(state, { type: "adopt:cursor", delta: 1 });
    expect(next.focusIndex).toBe(1);
  });

  test("adopt:cursor moves focus backward", () => {
    const state: AdoptState = { ...initialAdoptState, candidates: CANDIDATES, focusIndex: 2 };
    const next = adoptReducer(state, { type: "adopt:cursor", delta: -1 });
    expect(next.focusIndex).toBe(1);
  });

  test("adopt:cursor doesn't go below 0", () => {
    const state: AdoptState = { ...initialAdoptState, candidates: CANDIDATES, focusIndex: 0 };
    const next = adoptReducer(state, { type: "adopt:cursor", delta: -1 });
    expect(next.focusIndex).toBe(0);
  });

  test("adopt:cursor doesn't exceed last candidate index", () => {
    const state: AdoptState = {
      ...initialAdoptState,
      candidates: CANDIDATES,
      focusIndex: CANDIDATES.length - 1,
    };
    const next = adoptReducer(state, { type: "adopt:cursor", delta: 1 });
    expect(next.focusIndex).toBe(CANDIDATES.length - 1);
  });

  test("adopt:select-toggle selects focused candidate", () => {
    const state: AdoptState = { ...initialAdoptState, candidates: CANDIDATES, focusIndex: 1 };
    const next = adoptReducer(state, { type: "adopt:select-toggle" });
    expect(next.selectedIndices).toContain(1);
  });

  test("adopt:select-toggle deselects already-selected candidate", () => {
    const state: AdoptState = {
      ...initialAdoptState,
      candidates: CANDIDATES,
      focusIndex: 1,
      selectedIndices: [0, 1, 2],
    };
    const next = adoptReducer(state, { type: "adopt:select-toggle" });
    expect(next.selectedIndices).not.toContain(1);
    expect(next.selectedIndices).toContain(0);
    expect(next.selectedIndices).toContain(2);
  });

  test("adopt:mode-toggle switches track-in-place to move", () => {
    const state: AdoptState = { ...initialAdoptState, candidates: CANDIDATES, focusIndex: 0 };
    const next = adoptReducer(state, { type: "adopt:mode-toggle" });
    expect(next.perItemMode.get("alpha")).toBe("move");
  });

  test("adopt:mode-toggle switches move back to track-in-place", () => {
    const state: AdoptState = {
      ...initialAdoptState,
      candidates: CANDIDATES,
      focusIndex: 0,
      perItemMode: new Map([["alpha", "move"]]),
    };
    const next = adoptReducer(state, { type: "adopt:mode-toggle" });
    expect(next.perItemMode.get("alpha")).toBe("track-in-place");
  });

  test("adopt:mode-toggle is a no-op when no candidates", () => {
    const next = adoptReducer(initialAdoptState, { type: "adopt:mode-toggle" });
    expect(next).toBe(initialAdoptState);
  });

  test("adopt:mode-toggle uses immutable Map update", () => {
    const state: AdoptState = { ...initialAdoptState, candidates: CANDIDATES, focusIndex: 0 };
    const next = adoptReducer(state, { type: "adopt:mode-toggle" });
    expect(next.perItemMode).not.toBe(state.perItemMode);
  });

  test("unknown action is a no-op", () => {
    const action = { type: "dashboard:tab", tab: "taps" } as Action;
    const state: AdoptState = { ...initialAdoptState, candidates: CANDIDATES };
    const next = adoptReducer(state, action);
    expect(next).toBe(state);
  });
});
