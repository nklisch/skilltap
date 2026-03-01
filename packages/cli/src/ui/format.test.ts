import { describe, expect, test } from "bun:test";
import { table, termWidth, truncate } from "./format";

describe("termWidth", () => {
  test("returns a number", () => {
    expect(typeof termWidth()).toBe("number");
    expect(termWidth()).toBeGreaterThan(0);
  });
});

describe("truncate", () => {
  test("returns string unchanged when shorter than max", () => {
    expect(truncate("hello", 10)).toBe("hello");
  });

  test("returns string unchanged at exact max length", () => {
    expect(truncate("hello", 5)).toBe("hello");
  });

  test("truncates with ellipsis when over max", () => {
    const result = truncate("hello world", 8);
    expect(result.length).toBe(8);
    expect(result).toEndWith("…");
  });

  test("truncates to exactly max characters including ellipsis", () => {
    const result = truncate("abcdef", 4);
    expect(result).toBe("abc…");
  });
});

describe("table", () => {
  test("returns empty string for empty rows array", () => {
    expect(table([])).toBe("");
  });

  test("renders rows with indent", () => {
    const result = table([["name", "val"]]);
    expect(result).toContain("name");
    expect(result).toContain("val");
    expect(result.startsWith("  ")).toBe(true);
  });

  test("renders header as first row", () => {
    const result = table([["row1", "row2"]], { header: ["Col1", "Col2"] });
    expect(result).toContain("Col1");
    expect(result).toContain("Col2");
    expect(result).toContain("row1");
  });

  test("adds separator line after header", () => {
    const result = table([["a", "b"]], { header: ["H1", "H2"] });
    expect(result).toContain("─");
  });

  test("pads all rows to consistent column widths", () => {
    const result = table(
      [
        ["short", "x"],
        ["longer-value", "y"],
      ],
      { header: ["H1", "H2"] },
    );
    const lines = result
      .split("\n")
      .filter((l) => l.trim() && !l.includes("─"));
    // Strip ANSI codes for comparison
    const stripped = lines.map((l) => l.replace(/\x1b\[[0-9;]*m/g, ""));
    // All content lines should have the same length (padded to max column widths)
    const lengths = stripped.map((l) => l.length);
    expect(Math.max(...lengths) - Math.min(...lengths)).toBe(0);
  });
});
