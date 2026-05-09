import { describe, expect, test } from "bun:test";
import { render } from "ink-testing-library";
import React from "react";
import { List } from "./List";

const ITEMS = [
  { key: "a", label: "Alpha" },
  { key: "b", label: "Beta", hint: "v1.0" },
  { key: "c", label: "Gamma" },
];

describe("List", () => {
  test("renders all item labels", () => {
    const { lastFrame } = render(<List items={ITEMS} focusIndex={0} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("Alpha");
    expect(frame).toContain("Beta");
    expect(frame).toContain("Gamma");
  });

  test("renders empty message when no items", () => {
    const { lastFrame } = render(<List items={[]} focusIndex={0} emptyMessage="nothing here" />);
    expect(lastFrame()).toContain("nothing here");
  });

  test("renders default empty message", () => {
    const { lastFrame } = render(<List items={[]} focusIndex={0} />);
    expect(lastFrame()).toContain("(empty)");
  });

  test("renders focus cursor on focused item", () => {
    const { lastFrame } = render(<List items={ITEMS} focusIndex={1} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("▶");
    expect(frame).toContain("Beta");
  });

  test("renders hint text when provided", () => {
    const { lastFrame } = render(<List items={ITEMS} focusIndex={0} />);
    expect(lastFrame()).toContain("v1.0");
  });

  test("renders checkboxes for items with selected property", () => {
    const checkItems = [
      { key: "x", label: "X", selected: true },
      { key: "y", label: "Y", selected: false },
    ];
    const { lastFrame } = render(<List items={checkItems} focusIndex={0} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("[x]");
    expect(frame).toContain("[ ]");
  });
});
