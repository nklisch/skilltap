import { describe, expect, test } from "bun:test";
import {
  errorLine,
  formatDiffFileLine,
  formatDiffStatSummary,
  formatInstallCount,
  formatShaChange,
  formatUnifiedDiff,
  highlightMatches,
  successLine,
  table,
  termWidth,
  truncate,
} from "./format";

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

  test("handles rows with fewer cells than max columns", () => {
    const result = table([["a", "b", "c"], ["x"]]);
    expect(result).toContain("a");
    expect(result).toContain("x");
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

describe("errorLine", () => {
  test("writes error message to stderr", () => {
    const chunks: string[] = [];
    const original = process.stderr.write.bind(process.stderr);
    process.stderr.write = (chunk: string | Uint8Array) => {
      chunks.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
      return true;
    };
    try {
      errorLine("something went wrong");
      const output = chunks.join("");
      expect(output).toContain("error");
      expect(output).toContain("something went wrong");
    } finally {
      process.stderr.write = original;
    }
  });

  test("writes hint when provided", () => {
    const chunks: string[] = [];
    const original = process.stderr.write.bind(process.stderr);
    process.stderr.write = (chunk: string | Uint8Array) => {
      chunks.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
      return true;
    };
    try {
      errorLine("oops", "try this instead");
      const output = chunks.join("");
      expect(output).toContain("hint");
      expect(output).toContain("try this instead");
    } finally {
      process.stderr.write = original;
    }
  });

  test("does not write hint line when hint is undefined", () => {
    const chunks: string[] = [];
    const original = process.stderr.write.bind(process.stderr);
    process.stderr.write = (chunk: string | Uint8Array) => {
      chunks.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
      return true;
    };
    try {
      errorLine("oops");
      expect(chunks).toHaveLength(1);
    } finally {
      process.stderr.write = original;
    }
  });
});

describe("successLine", () => {
  test("writes success message to stdout", () => {
    const chunks: string[] = [];
    const original = process.stdout.write.bind(process.stdout);
    process.stdout.write = (chunk: string | Uint8Array) => {
      chunks.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
      return true;
    };
    try {
      successLine("installed my-skill");
      const output = chunks.join("");
      expect(output).toContain("✓");
      expect(output).toContain("installed my-skill");
    } finally {
      process.stdout.write = original;
    }
  });
});

describe("formatInstallCount", () => {
  test("formats single install", () => {
    expect(formatInstallCount(1)).toBe("1 install");
  });

  test("formats plural installs", () => {
    expect(formatInstallCount(0)).toBe("0 installs");
    expect(formatInstallCount(5)).toBe("5 installs");
    expect(formatInstallCount(999)).toBe("999 installs");
  });

  test("formats K range", () => {
    expect(formatInstallCount(1000)).toBe("1K installs");
    expect(formatInstallCount(1500)).toBe("1.5K installs");
    expect(formatInstallCount(10000)).toBe("10K installs");
  });

  test("formats M range", () => {
    expect(formatInstallCount(1_000_000)).toBe("1M installs");
    expect(formatInstallCount(1_500_000)).toBe("1.5M installs");
    expect(formatInstallCount(2_000_000)).toBe("2M installs");
  });
});

describe("formatShaChange", () => {
  test("truncates both SHAs to 7 chars with arrow", () => {
    const result = formatShaChange("abc1234567890", "def9876543210");
    expect(result).toBe("abc1234 → def9876");
  });

  test("handles short SHAs", () => {
    const result = formatShaChange("abc", "def");
    expect(result).toBe("abc → def");
  });
});

describe("formatDiffStatSummary", () => {
  test("one file changed", () => {
    expect(formatDiffStatSummary({ filesChanged: 1, insertions: 0, deletions: 0 })).toBe(
      "(1 file changed)",
    );
  });

  test("multiple files changed with insertions and deletions", () => {
    expect(formatDiffStatSummary({ filesChanged: 3, insertions: 10, deletions: 5 })).toBe(
      "(3 files changed, +10, -5)",
    );
  });

  test("insertions only", () => {
    expect(formatDiffStatSummary({ filesChanged: 2, insertions: 7, deletions: 0 })).toBe(
      "(2 files changed, +7)",
    );
  });

  test("deletions only", () => {
    expect(formatDiffStatSummary({ filesChanged: 1, insertions: 0, deletions: 3 })).toBe(
      "(1 file changed, -3)",
    );
  });
});

describe("formatDiffFileLine", () => {
  test("shows status, path, and counts", () => {
    const result = formatDiffFileLine({ status: "M", path: "SKILL.md", insertions: 5, deletions: 2 });
    expect(result).toContain("M");
    expect(result).toContain("SKILL.md");
    expect(result).toContain("+5");
    expect(result).toContain("-2");
  });

  test("omits counts when both zero", () => {
    const result = formatDiffFileLine({ status: "A", path: "new.sh", insertions: 0, deletions: 0 });
    expect(result).toContain("new.sh");
    expect(result).not.toContain("(");
  });

  test("shows only insertions when deletions zero", () => {
    const result = formatDiffFileLine({ status: "A", path: "new.sh", insertions: 18, deletions: 0 });
    expect(result).toContain("+18");
    expect(result).not.toContain("-");
  });
});

describe("formatUnifiedDiff", () => {
  test("colorises added lines green", () => {
    const result = formatUnifiedDiff("+new line");
    expect(result).toContain("\x1b[32m");
  });

  test("colorises removed lines red", () => {
    const result = formatUnifiedDiff("-old line");
    expect(result).toContain("\x1b[31m");
  });

  test("colorises hunk headers cyan", () => {
    const result = formatUnifiedDiff("@@ -1,3 +1,4 @@");
    expect(result).toContain("\x1b[36m");
  });

  test("dims --- and +++ headers", () => {
    const result = formatUnifiedDiff("--- a/file\n+++ b/file");
    expect(result).toContain("\x1b[2m");
  });

  test("dims diff/index lines", () => {
    const diffLine = "diff --git a/file b/file";
    const result = formatUnifiedDiff(diffLine);
    expect(result).toContain("\x1b[2m");
  });

  test("leaves context lines unchanged", () => {
    const result = formatUnifiedDiff(" unchanged line");
    expect(result).toBe(" unchanged line");
  });
});

describe("highlightMatches", () => {
  test("bolds characters at match positions", () => {
    const result = highlightMatches("abc", new Set([0, 2]));
    expect(result).toContain("\x1b[1m");
    expect(result).toContain("a");
    expect(result).toContain("c");
  });

  test("leaves unmatched characters plain", () => {
    const result = highlightMatches("abc", new Set([1]));
    // 'a' and 'c' should not be wrapped in bold
    expect(result).toContain("a");
    expect(result).toContain("c");
  });

  test("empty positions returns plain string", () => {
    const result = highlightMatches("hello", new Set());
    expect(result).toBe("hello");
  });
});
