import { describe, expect, test } from "bun:test";
import { findBestMatch, matchesRange, parseRange } from "./range";

describe("parseRange", () => {
  test("parses '*' as any", () => {
    expect(parseRange("*")).toEqual({ kind: "any" });
  });

  test("parses 'latest' as any", () => {
    expect(parseRange("latest")).toEqual({ kind: "any" });
  });

  test("parses '' as any", () => {
    expect(parseRange("")).toEqual({ kind: "any" });
  });

  test("parses caret semver", () => {
    expect(parseRange("^1.2.3")).toEqual({ kind: "caret", major: 1, minor: 2, patch: 3 });
    expect(parseRange("^1.0")).toEqual({ kind: "caret", major: 1, minor: 0, patch: 0 });
    expect(parseRange("^2")).toEqual({ kind: "caret", major: 2, minor: 0, patch: 0 });
  });

  test("parses tilde semver", () => {
    expect(parseRange("~1.2.3")).toEqual({ kind: "tilde", major: 1, minor: 2, patch: 3 });
    expect(parseRange("~1.2")).toEqual({ kind: "tilde", major: 1, minor: 2, patch: 0 });
  });

  test("parses exact version", () => {
    expect(parseRange("v1.2.3")).toEqual({ kind: "exact", value: "v1.2.3" });
    expect(parseRange("1.0.0")).toEqual({ kind: "exact", value: "1.0.0" });
  });

  test("parses non-semver as exact", () => {
    expect(parseRange("main")).toEqual({ kind: "exact", value: "main" });
    expect(parseRange("abc123")).toEqual({ kind: "exact", value: "abc123" });
  });

  test("falls back to exact when caret/tilde target is non-semver", () => {
    expect(parseRange("^main")).toEqual({ kind: "exact", value: "^main" });
    expect(parseRange("~develop")).toEqual({ kind: "exact", value: "~develop" });
  });

  test("trims whitespace", () => {
    expect(parseRange("  ^1.0  ")).toEqual({ kind: "caret", major: 1, minor: 0, patch: 0 });
  });
});

describe("matchesRange", () => {
  test("any matches everything", () => {
    const range = parseRange("*");
    expect(matchesRange(range, "v1.2.3")).toBe(true);
    expect(matchesRange(range, "main")).toBe(true);
    expect(matchesRange(range, "abc123")).toBe(true);
    expect(matchesRange(range, "")).toBe(true);
  });

  test("exact requires identical match", () => {
    const range = parseRange("v1.2.3");
    expect(matchesRange(range, "v1.2.3")).toBe(true);
    expect(matchesRange(range, "1.2.3")).toBe(false);
    expect(matchesRange(range, "v1.2.4")).toBe(false);
    expect(matchesRange(range, "main")).toBe(false);
  });

  test("exact with non-semver matches identical string", () => {
    const range = parseRange("main");
    expect(matchesRange(range, "main")).toBe(true);
    expect(matchesRange(range, "develop")).toBe(false);
  });

  test("caret matches same major and >= base", () => {
    const range = parseRange("^1.2.0");
    expect(matchesRange(range, "1.2.0")).toBe(true);
    expect(matchesRange(range, "v1.2.0")).toBe(true);
    expect(matchesRange(range, "1.2.5")).toBe(true);
    expect(matchesRange(range, "1.3.0")).toBe(true);
    expect(matchesRange(range, "1.10.10")).toBe(true);
    expect(matchesRange(range, "1.1.99")).toBe(false);
    expect(matchesRange(range, "2.0.0")).toBe(false);
    expect(matchesRange(range, "0.9.9")).toBe(false);
  });

  test("caret rejects non-semver candidates", () => {
    const range = parseRange("^1.0");
    expect(matchesRange(range, "main")).toBe(false);
    expect(matchesRange(range, "abc123")).toBe(false);
  });

  test("tilde matches same major.minor and >= base", () => {
    const range = parseRange("~1.2.3");
    expect(matchesRange(range, "1.2.3")).toBe(true);
    expect(matchesRange(range, "1.2.10")).toBe(true);
    expect(matchesRange(range, "1.3.0")).toBe(false);
    expect(matchesRange(range, "1.2.2")).toBe(false);
    expect(matchesRange(range, "2.2.3")).toBe(false);
  });

  test("tilde with two-part base sets patch to 0", () => {
    const range = parseRange("~1.2");
    expect(matchesRange(range, "1.2.0")).toBe(true);
    expect(matchesRange(range, "1.2.99")).toBe(true);
    expect(matchesRange(range, "1.3.0")).toBe(false);
  });
});

describe("findBestMatch", () => {
  test("returns the highest semver matching a caret range", () => {
    const range = parseRange("^1.0");
    const candidates = ["v1.0.0", "v1.2.0", "v1.5.3", "v2.0.0", "v0.9.9"];
    expect(findBestMatch(range, candidates)).toBe("v1.5.3");
  });

  test("returns null when no candidate matches", () => {
    const range = parseRange("^3.0");
    const candidates = ["v1.0.0", "v2.0.0"];
    expect(findBestMatch(range, candidates)).toBeNull();
  });

  test("returns the exact match for an exact range", () => {
    const range = parseRange("v1.2.3");
    const candidates = ["v1.2.3", "v1.3.0"];
    expect(findBestMatch(range, candidates)).toBe("v1.2.3");
  });

  test("non-semver candidates rank below semver matches", () => {
    const range = parseRange("*");
    const candidates = ["main", "v1.0.0", "v0.5.0"];
    expect(findBestMatch(range, candidates)).toBe("v1.0.0");
  });

  test("returns a non-semver candidate when only non-semver candidates exist", () => {
    const range = parseRange("*");
    const candidates = ["main", "develop"];
    const best = findBestMatch(range, candidates);
    // Order between non-semver candidates is unstable but a value must be returned.
    expect(best === "main" || best === "develop").toBe(true);
  });

  test("returns null on empty candidate list", () => {
    expect(findBestMatch(parseRange("*"), [])).toBeNull();
  });
});
