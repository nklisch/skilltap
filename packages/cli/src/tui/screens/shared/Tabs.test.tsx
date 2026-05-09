import { describe, expect, test } from "bun:test";
import { render } from "ink-testing-library";
import { Tabs } from "./Tabs";

const TABS = [
  { id: "installed" as const, label: "Installed" },
  { id: "taps" as const, label: "Taps" },
  { id: "updates" as const, label: "Updates" },
];

describe("Tabs", () => {
  test("renders all tab labels", () => {
    const { lastFrame } = render(<Tabs current="installed" tabs={TABS} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("Installed");
    expect(frame).toContain("Taps");
    expect(frame).toContain("Updates");
  });

  test("renders numbered tab labels", () => {
    const { lastFrame } = render(<Tabs current="installed" tabs={TABS} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("1 Installed");
    expect(frame).toContain("2 Taps");
    expect(frame).toContain("3 Updates");
  });

  test("renders with a non-first tab current", () => {
    const { lastFrame } = render(<Tabs current="taps" tabs={TABS} />);
    const frame = lastFrame() ?? "";
    expect(frame).toContain("Taps");
  });
});
