import { describe, expect, test } from "bun:test";
import { isTrusted, trustMatches } from "./trust-glob";

describe("trustMatches", () => {
  test("exact literal match", () => {
    expect(trustMatches("home", "home")).toBe(true);
    expect(trustMatches("home", "homely")).toBe(false);
    expect(trustMatches("home", "myhome")).toBe(false);
  });

  test("case-sensitive", () => {
    expect(trustMatches("Home", "home")).toBe(false);
    expect(trustMatches("Home", "Home")).toBe(true);
  });

  test("trailing wildcard", () => {
    expect(trustMatches("github.com/corp/*", "github.com/corp/foo")).toBe(true);
    expect(trustMatches("github.com/corp/*", "github.com/corp/")).toBe(true);
    expect(trustMatches("github.com/corp/*", "github.com/other/foo")).toBe(
      false,
    );
  });

  test("leading wildcard", () => {
    expect(trustMatches("*.example.com", "skills.example.com")).toBe(true);
    expect(trustMatches("*.example.com", "example.com")).toBe(false);
  });

  test("middle wildcard", () => {
    expect(trustMatches("github.com/*/skills", "github.com/corp/skills")).toBe(
      true,
    );
    expect(trustMatches("github.com/*/skills", "github.com/corp/other")).toBe(
      false,
    );
  });

  test("multiple wildcards", () => {
    expect(
      trustMatches("*://github.com/corp/*", "https://github.com/corp/foo"),
    ).toBe(true);
    expect(
      trustMatches("*://github.com/corp/*", "ssh://github.com/corp/bar"),
    ).toBe(true);
    expect(trustMatches("*://github.com/corp/*", "github.com/corp/foo")).toBe(
      false,
    );
  });

  test("npm prefix matching", () => {
    expect(trustMatches("npm:@corp/*", "npm:@corp/code-review")).toBe(true);
    expect(trustMatches("npm:@corp/*", "npm:@other/code-review")).toBe(false);
  });

  test("regex specials are escaped", () => {
    // dots, parentheses, plus signs should match literally, not as regex meta.
    expect(trustMatches("a.b.c", "axbxc")).toBe(false);
    expect(trustMatches("a.b.c", "a.b.c")).toBe(true);
    expect(trustMatches("a+b", "a+b")).toBe(true);
    expect(trustMatches("a+b", "aaab")).toBe(false);
    expect(trustMatches("(group)", "(group)")).toBe(true);
  });

  test("empty pattern matches only empty string", () => {
    expect(trustMatches("", "")).toBe(true);
    expect(trustMatches("", "x")).toBe(false);
  });

  test("anchored at both ends", () => {
    expect(trustMatches("foo", "foobar")).toBe(false);
    expect(trustMatches("foo", "barfoo")).toBe(false);
    expect(trustMatches("foo", "foo")).toBe(true);
  });
});

describe("isTrusted", () => {
  test("empty trust list → never trusted", () => {
    expect(isTrusted([], { sourceUrl: "github.com/corp/foo" })).toBe(false);
    expect(
      isTrusted([], { sourceUrl: "github.com/corp/foo", tapName: "home" }),
    ).toBe(false);
  });

  test("matches against tapName when present", () => {
    expect(
      isTrusted(["home"], { sourceUrl: "https://...", tapName: "home" }),
    ).toBe(true);
    expect(isTrusted(["home"], { sourceUrl: "https://..." })).toBe(false);
  });

  test("matches against sourceUrl", () => {
    expect(
      isTrusted(["github.com/corp/*"], { sourceUrl: "github.com/corp/foo" }),
    ).toBe(true);
    expect(
      isTrusted(["github.com/corp/*"], { sourceUrl: "github.com/other/foo" }),
    ).toBe(false);
  });

  test("any one pattern in the list is sufficient", () => {
    const trust = ["never-matches", "github.com/corp/*", "another-no-match"];
    expect(isTrusted(trust, { sourceUrl: "github.com/corp/foo" })).toBe(true);
  });

  test("matches tapName OR sourceUrl, not just one", () => {
    expect(
      isTrusted(["home"], {
        sourceUrl: "https://gitlab.example.com/n/r",
        tapName: "home",
      }),
    ).toBe(true);
    expect(
      isTrusted(["https://gitlab.example.com/*"], {
        sourceUrl: "https://gitlab.example.com/n/r",
        tapName: "home",
      }),
    ).toBe(true);
  });

  test("first match wins (no need to evaluate the rest)", () => {
    // This is a behavioral check — function returns true on first match.
    expect(isTrusted(["github.com/*"], { sourceUrl: "github.com/foo" })).toBe(
      true,
    );
  });
});
