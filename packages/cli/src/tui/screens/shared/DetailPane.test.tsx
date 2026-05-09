import { describe, expect, test } from "bun:test";
import { render } from "ink-testing-library";
import React from "react";
import { DetailPane } from "./DetailPane";

describe("DetailPane", () => {
  test("renders title", () => {
    const { lastFrame } = render(<DetailPane title="My Title" body="Some body text" />);
    expect(lastFrame()).toContain("My Title");
  });

  test("renders string body", () => {
    const { lastFrame } = render(<DetailPane title="T" body="Hello world" />);
    expect(lastFrame()).toContain("Hello world");
  });

  test("renders array body lines", () => {
    const { lastFrame } = render(<DetailPane title="T" body={["Line one", "Line two"]} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("Line one");
    expect(frame).toContain("Line two");
  });
});
