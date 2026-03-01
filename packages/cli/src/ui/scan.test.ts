import { describe, expect, test } from "bun:test";
import type { StaticWarning } from "@skilltap/core";
import { formatWarnings } from "./scan";

describe("formatWarnings", () => {
  test("includes skill name in header", () => {
    const warnings: StaticWarning[] = [
      { file: "SKILL.md", line: 1, category: "HTML comment", raw: "<!-- hi -->" },
    ];
    const result = formatWarnings(warnings, "my-skill");
    expect(result).toContain("⚠ Static warnings in my-skill:");
  });

  test("shows line number prefix for positive line", () => {
    const warnings: StaticWarning[] = [
      { file: "SKILL.md", line: 14, category: "HTML comment", raw: "<!-- evil -->" },
    ];
    const result = formatWarnings(warnings, "skill");
    expect(result).toContain("L14: HTML comment");
  });

  test("shows line range for array line numbers", () => {
    const warnings: StaticWarning[] = [
      {
        file: "SKILL.md",
        line: [5, 10] as [number, number],
        category: "Multi-line block",
        raw: "content",
      },
    ];
    const result = formatWarnings(warnings, "skill");
    expect(result).toContain("L5-10: Multi-line block");
  });

  test("uses file path when line is 0", () => {
    const warnings: StaticWarning[] = [
      { file: "binary.bin", line: 0, category: "Binary file", raw: "ELF binary" },
    ];
    const result = formatWarnings(warnings, "skill");
    expect(result).toContain("binary.bin: Binary file");
  });

  test("shows raw and visible when they differ", () => {
    const warnings: StaticWarning[] = [
      {
        file: "SKILL.md",
        line: 14,
        category: "Invisible Unicode",
        raw: "Before \u200btext",
        visible: "Before text",
      },
    ];
    const result = formatWarnings(warnings, "skill");
    expect(result).toContain('│ Raw: "Before \u200btext"');
    expect(result).toContain('│ Visible: "Before text"');
  });

  test("shows only raw when visible is absent", () => {
    const warnings: StaticWarning[] = [
      { file: "SKILL.md", line: 8, category: "HTML comment", raw: "<!-- evil -->" },
    ];
    const result = formatWarnings(warnings, "skill");
    expect(result).toContain("│ <!-- evil -->");
    expect(result).not.toContain("Visible:");
  });

  test("shows only raw when visible matches raw", () => {
    const warnings: StaticWarning[] = [
      {
        file: "SKILL.md",
        line: 8,
        category: "Dangerous pattern",
        raw: "rm -rf /",
        visible: "rm -rf /",
      },
    ];
    const result = formatWarnings(warnings, "skill");
    expect(result).not.toContain("Visible:");
  });

  test("handles multiple warnings", () => {
    const warnings: StaticWarning[] = [
      { file: "SKILL.md", line: 1, category: "Cat A", raw: "raw1" },
      { file: "SKILL.md", line: 2, category: "Cat B", raw: "raw2" },
    ];
    const result = formatWarnings(warnings, "skill");
    expect(result).toContain("Cat A");
    expect(result).toContain("Cat B");
  });
});
