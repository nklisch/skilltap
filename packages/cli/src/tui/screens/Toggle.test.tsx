import { describe, expect, test } from "bun:test";
import { render } from "ink-testing-library";
import React from "react";
import { Toggle } from "./Toggle";
import { initialToggleState } from "../state/toggle";

describe("Toggle", () => {
  test("renders type selection on step=type", () => {
    const { lastFrame } = render(
      <Toggle state={initialToggleState} dispatch={() => {}} />,
    );
    const frame = lastFrame() ?? "";
    expect(frame).toContain("skill");
    expect(frame).toContain("plugin");
    expect(frame).toContain("mcp");
  });

  test("renders name step heading", () => {
    const state = { ...initialToggleState, step: "name" as const, type: "skill" as const };
    const { lastFrame } = render(<Toggle state={state} dispatch={() => {}} />);
    expect(lastFrame()).toContain("skill");
  });

  test("renders components step with component names", () => {
    const state = {
      ...initialToggleState,
      step: "components" as const,
      type: "plugin" as const,
      selectedName: "my-plugin",
      components: [
        { name: "skills", active: true },
        { name: "mcp", active: false },
      ],
      selectedComponentIndices: [0],
    };
    const { lastFrame } = render(<Toggle state={state} dispatch={() => {}} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("skills");
    expect(frame).toContain("mcp");
    expect(frame).toContain("my-plugin");
  });

  test("renders footer hints on each step", () => {
    const { lastFrame } = render(
      <Toggle state={initialToggleState} dispatch={() => {}} />,
    );
    expect(lastFrame()).toContain("quit");
  });
});
