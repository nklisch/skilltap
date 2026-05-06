import { describe, expect, test } from "bun:test";
import type { PluginRecord } from "../schemas/plugins";
import { findComponentInPlugin, parseComponentRef } from "./component-ref";

describe("parseComponentRef", () => {
  test("plain name with no colon", () => {
    expect(parseComponentRef("foo")).toEqual({ name: "foo" });
  });

  test("name and component", () => {
    expect(parseComponentRef("foo:bar")).toEqual({
      name: "foo",
      component: "bar",
    });
  });

  test("multiple colons — first splits, rest is component", () => {
    expect(parseComponentRef("foo:bar:baz")).toEqual({
      name: "foo",
      component: "bar:baz",
    });
  });

  test("malformed leading colon falls back to name-only", () => {
    expect(parseComponentRef(":bar")).toEqual({ name: ":bar" });
  });

  test("malformed trailing colon falls back to name-only", () => {
    expect(parseComponentRef("foo:")).toEqual({ name: "foo:" });
  });

  test("namespaced MCP-style component", () => {
    expect(
      parseComponentRef("dev-toolkit:skilltap:dev-toolkit:database"),
    ).toEqual({
      name: "dev-toolkit",
      component: "skilltap:dev-toolkit:database",
    });
  });

  test("empty input", () => {
    expect(parseComponentRef("")).toEqual({ name: "" });
  });
});

describe("findComponentInPlugin", () => {
  const plugin: PluginRecord = {
    name: "dev-toolkit",
    description: "",
    format: "skilltap",
    repo: null,
    ref: null,
    sha: null,
    scope: "global",
    also: [],
    tap: null,
    components: [
      { type: "skill", name: "code-review", active: true },
      {
        type: "mcp",
        serverType: "stdio",
        name: "database",
        active: true,
        command: "node",
        args: [],
        env: {},
      },
      {
        type: "agent",
        name: "reviewer",
        active: true,
        platform: "claude-code",
      },
    ],
    installedAt: "2026-05-06T00:00:00.000Z",
    updatedAt: "2026-05-06T00:00:00.000Z",
    active: true,
  };

  test("returns the matching component by name", () => {
    const result = findComponentInPlugin(plugin, "code-review");
    expect(result).not.toBeNull();
    expect(result?.type).toBe("skill");
    expect(result?.name).toBe("code-review");
  });

  test("matches across component types", () => {
    expect(findComponentInPlugin(plugin, "database")?.type).toBe("mcp");
    expect(findComponentInPlugin(plugin, "reviewer")?.type).toBe("agent");
  });

  test("returns null when component is not found", () => {
    expect(findComponentInPlugin(plugin, "missing")).toBeNull();
  });
});
