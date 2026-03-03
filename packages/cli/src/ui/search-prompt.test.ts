import { describe, expect, test } from "bun:test";
import { Fzf, byLengthAsc } from "fzf";
import { highlightMatches } from "./format";

// ---------------------------------------------------------------------------
// fzf integration — verify the config we use in searchPrompt
// ---------------------------------------------------------------------------

type Entry = { name: string; description: string };

function makeFzf(items: Entry[]) {
  return new Fzf(items, {
    selector: (item: Entry) => `${item.name} ${item.description}`,
    limit: 50,
    tiebreakers: [byLengthAsc],
    casing: "case-insensitive",
  });
}

describe("fzf integration", () => {
  const items: Entry[] = [
    { name: "react-testing", description: "Test utilities for React" },
    { name: "react-skill", description: "Component helpers" },
    { name: "preact-compat", description: "React compatibility layer" },
    { name: "commit-helper", description: "Generates commit messages" },
    { name: "code-review", description: "Code review assistant" },
  ];

  test("exact prefix match ranks highest", () => {
    const fzf = makeFzf(items);
    const results = fzf.find("react");
    expect(results.length).toBeGreaterThanOrEqual(3);
    // Items starting with "react" should rank above "preact"
    const topTwo = results.slice(0, 2).map((r) => r.item.name);
    expect(topTwo).toContain("react-testing");
    expect(topTwo).toContain("react-skill");
  });

  test("fuzzy matching finds partial matches", () => {
    const fzf = makeFzf(items);
    const results = fzf.find("cmit");
    const names = results.map((r) => r.item.name);
    expect(names).toContain("commit-helper");
  });

  test("case-insensitive matching works", () => {
    const fzf = makeFzf(items);
    const results = fzf.find("REACT");
    expect(results.length).toBeGreaterThanOrEqual(1);
    const names = results.map((r) => r.item.name);
    expect(names).toContain("react-skill");
  });

  test("empty query returns all items", () => {
    const fzf = makeFzf(items);
    const results = fzf.find("");
    expect(results).toHaveLength(items.length);
  });

  test("match positions are returned as a Set", () => {
    const fzf = makeFzf(items);
    const results = fzf.find("react");
    const first = results[0];
    expect(first.positions).toBeInstanceOf(Set);
    expect(first.positions.size).toBeGreaterThan(0);
    // "react" is 5 chars — all should be matched in the name portion
    for (let i = 0; i < 5; i++) {
      expect(first.positions.has(i)).toBe(true);
    }
  });

  test("description is searchable via selector", () => {
    const fzf = makeFzf(items);
    const results = fzf.find("utilities");
    const names = results.map((r) => r.item.name);
    expect(names).toContain("react-testing");
  });

  test("tiebreaker prefers shorter names", () => {
    const shortAndLong: Entry[] = [
      { name: "a-very-long-react-tool-name", description: "Something" },
      { name: "react", description: "Something" },
    ];
    const fzf = makeFzf(shortAndLong);
    const results = fzf.find("react");
    expect(results[0].item.name).toBe("react");
  });

  test("limit caps results", () => {
    const many = Array.from({ length: 100 }, (_, i) => ({
      name: `skill-${i}`,
      description: `Description ${i}`,
    }));
    const fzf = new Fzf(many, {
      selector: (item: Entry) => `${item.name} ${item.description}`,
      limit: 10,
      casing: "case-insensitive",
    });
    const results = fzf.find("skill");
    expect(results.length).toBeLessThanOrEqual(10);
  });
});

// ---------------------------------------------------------------------------
// highlightMatches
// ---------------------------------------------------------------------------

describe("highlightMatches", () => {
  test("highlights characters at specified positions", () => {
    const result = highlightMatches("hello", new Set([0, 2, 4]));
    // h, l, o should be bold+underline; e, l should be plain
    expect(result).toContain("h");
    expect(result).toContain("e");
    // Bold+underline ANSI codes should be present
    expect(result).toMatch(/\x1b\[1m/); // bold
    expect(result).toMatch(/\x1b\[4m/); // underline
  });

  test("returns plain text when positions is empty", () => {
    const result = highlightMatches("hello", new Set());
    expect(result).toBe("hello");
  });

  test("handles all positions highlighted", () => {
    const result = highlightMatches("ab", new Set([0, 1]));
    // Both chars should have ANSI codes
    const stripped = result.replace(/\x1b\[[0-9;]*m/g, "");
    expect(stripped).toBe("ab");
    // Two bold sequences
    const boldCount = (result.match(/\x1b\[1m/g) ?? []).length;
    expect(boldCount).toBe(2);
  });

  test("handles empty string", () => {
    expect(highlightMatches("", new Set())).toBe("");
  });

  test("positions beyond text length are ignored", () => {
    const result = highlightMatches("hi", new Set([0, 5, 10]));
    const stripped = result.replace(/\x1b\[[0-9;]*m/g, "");
    expect(stripped).toBe("hi");
    // Only position 0 matched
    const boldCount = (result.match(/\x1b\[1m/g) ?? []).length;
    expect(boldCount).toBe(1);
  });
});
