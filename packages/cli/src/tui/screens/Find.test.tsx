import { describe, expect, test } from "bun:test";
import { render } from "ink-testing-library";
import React from "react";
import { Find } from "./Find";
import { initialFindState } from "../state/find";

describe("Find", () => {
  test("renders search prompt when query is empty", () => {
    const { lastFrame } = render(
      <Find state={initialFindState} dispatch={() => {}} />,
    );
    const frame = lastFrame() ?? "";
    expect(frame).toContain("Search");
  });

  test("renders query text when query is set", () => {
    const state = { ...initialFindState, query: "zod" };
    const { lastFrame } = render(<Find state={state} dispatch={() => {}} />);
    expect(lastFrame()).toContain("zod");
  });

  test("renders results list", () => {
    const state = {
      ...initialFindState,
      query: "foo",
      results: [
        { name: "foo-skill", description: "a skill", source: "github.com", type: "skill" as const },
      ],
    };
    const { lastFrame } = render(<Find state={state} dispatch={() => {}} />);
    expect(lastFrame()).toContain("foo-skill");
  });

  test("renders loading state", () => {
    const state = { ...initialFindState, query: "zod", loading: true };
    const { lastFrame } = render(<Find state={state} dispatch={() => {}} />);
    expect(lastFrame()).toContain("Searching");
  });

  test("renders no-results message", () => {
    const state = { ...initialFindState, query: "zzz", results: [] };
    const { lastFrame } = render(<Find state={state} dispatch={() => {}} />);
    expect(lastFrame()).toContain("no results");
  });

  test("renders detail pane for selected result", () => {
    const state = {
      ...initialFindState,
      query: "foo",
      results: [
        { name: "foo-skill", description: "my desc", source: "tap:core", type: "skill" as const },
      ],
      selectedIndex: 0,
    };
    const { lastFrame } = render(<Find state={state} dispatch={() => {}} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("foo-skill");
    expect(frame).toContain("tap:core");
  });

  test("renders footer hints", () => {
    const { lastFrame } = render(
      <Find state={initialFindState} dispatch={() => {}} />,
    );
    expect(lastFrame()).toContain("quit");
  });
});
