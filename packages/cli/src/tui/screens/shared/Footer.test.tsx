import { describe, expect, test } from "bun:test";
import { render } from "ink-testing-library";
import React from "react";
import { Footer } from "./Footer";

describe("Footer", () => {
  test("renders key hints", () => {
    const hints = [
      { key: "q", description: "quit" },
      { key: "↑↓", description: "navigate" },
    ];
    const { lastFrame } = render(<Footer hints={hints} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("q");
    expect(frame).toContain("quit");
    expect(frame).toContain("↑↓");
    expect(frame).toContain("navigate");
  });

  test("renders separator between hints", () => {
    const hints = [
      { key: "a", description: "alpha" },
      { key: "b", description: "beta" },
    ];
    const { lastFrame } = render(<Footer hints={hints} />);
    expect(lastFrame()).toContain("·");
  });

  test("renders single hint without separator", () => {
    const { lastFrame } = render(<Footer hints={[{ key: "q", description: "quit" }]} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("q");
    expect(frame).not.toContain("·");
  });
});
