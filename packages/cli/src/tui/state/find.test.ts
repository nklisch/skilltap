import { describe, expect, test } from "bun:test";
import { findReducer, initialFindState } from "./find";
import type { Action, FindResult, FindState } from "./types";

const RESULT: FindResult = { name: "foo", description: "A skill", source: "tap1", type: "skill" };
const RESULTS: FindResult[] = [
  { name: "foo", description: "A skill", source: "tap1", type: "skill" },
  { name: "bar", description: "A plugin", source: "tap1", type: "plugin" },
  { name: "baz", description: "Another skill", source: "tap2", type: "skill" },
];

describe("findReducer", () => {
  test("initial state has correct defaults", () => {
    expect(initialFindState.query).toBe("");
    expect(initialFindState.results).toEqual([]);
    expect(initialFindState.selectedIndex).toBe(0);
    expect(initialFindState.loading).toBe(false);
  });

  test("find:query updates query and sets loading when non-empty", () => {
    const next = findReducer(initialFindState, { type: "find:query", query: "react" });
    expect(next.query).toBe("react");
    expect(next.loading).toBe(true);
    expect(next.selectedIndex).toBe(0);
  });

  test("find:query with empty string clears loading", () => {
    const state: FindState = { ...initialFindState, query: "react", loading: true };
    const next = findReducer(state, { type: "find:query", query: "" });
    expect(next.query).toBe("");
    expect(next.loading).toBe(false);
  });

  test("find:query resets selectedIndex", () => {
    const state: FindState = { ...initialFindState, selectedIndex: 4 };
    const next = findReducer(state, { type: "find:query", query: "new" });
    expect(next.selectedIndex).toBe(0);
  });

  test("find:results populates results and clears loading", () => {
    const state: FindState = { ...initialFindState, loading: true };
    const next = findReducer(state, { type: "find:results", results: RESULTS });
    expect(next.results).toEqual(RESULTS);
    expect(next.loading).toBe(false);
    expect(next.selectedIndex).toBe(0);
  });

  test("find:cursor moves forward within bounds", () => {
    const state: FindState = { ...initialFindState, results: RESULTS, selectedIndex: 0 };
    const next = findReducer(state, { type: "find:cursor", delta: 1 });
    expect(next.selectedIndex).toBe(1);
  });

  test("find:cursor doesn't exceed last result index", () => {
    const state: FindState = { ...initialFindState, results: RESULTS, selectedIndex: 2 };
    const next = findReducer(state, { type: "find:cursor", delta: 1 });
    expect(next.selectedIndex).toBe(2);
  });

  test("find:cursor doesn't go below 0", () => {
    const state: FindState = { ...initialFindState, results: RESULTS, selectedIndex: 0 };
    const next = findReducer(state, { type: "find:cursor", delta: -1 });
    expect(next.selectedIndex).toBe(0);
  });

  test("find:cursor with empty results stays at 0 going up", () => {
    const next = findReducer(initialFindState, { type: "find:cursor", delta: 1 });
    expect(next.selectedIndex).toBe(0);
  });

  test("unknown action is a no-op", () => {
    const action = { type: "dashboard:tab", tab: "taps" } as Action;
    const next = findReducer(initialFindState, action);
    expect(next).toBe(initialFindState);
  });
});
