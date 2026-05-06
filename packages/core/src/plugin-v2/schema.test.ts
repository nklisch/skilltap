import { describe, expect, test } from "bun:test";
import {
  PluginManifestV2Schema,
  PluginV2HttpServerSchema,
  PluginV2ServerSchema,
  PluginV2StdioServerSchema,
} from "./schema";

const VALID_PLUGIN = {
  name: "team-toolkit",
  version: "1.0.0",
  description: "Internal dev tools",
  publish: true,
  skills: [{ name: "code-review", path: "./skills/code-review" }],
  servers: [
    {
      name: "db",
      type: "stdio" as const,
      command: "node",
      args: ["./mcp/db.js"],
    },
  ],
  agents: [{ name: "reviewer", path: "./agents/reviewer.md" }],
};

describe("PluginV2StdioServerSchema", () => {
  test("accepts a minimal stdio server", () => {
    const result = PluginV2StdioServerSchema.safeParse({
      name: "db",
      command: "node",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("stdio");
      expect(result.data.args).toEqual([]);
      expect(result.data.env).toEqual({});
    }
  });

  test("accepts a fully-populated stdio server", () => {
    const result = PluginV2StdioServerSchema.safeParse({
      type: "stdio",
      name: "db",
      command: "node",
      args: ["./mcp/db.js", "--port", "3000"],
      env: { DATABASE_URL: "postgres://..." },
    });
    expect(result.success).toBe(true);
  });

  test("rejects without command", () => {
    const result = PluginV2StdioServerSchema.safeParse({ name: "db" });
    expect(result.success).toBe(false);
  });
});

describe("PluginV2HttpServerSchema", () => {
  test("accepts a minimal http server", () => {
    const result = PluginV2HttpServerSchema.safeParse({
      type: "http",
      name: "search",
      url: "https://search.example.com/mcp",
    });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.headers).toEqual({});
  });

  test("accepts an http server with headers", () => {
    const result = PluginV2HttpServerSchema.safeParse({
      type: "http",
      name: "search",
      url: "https://search.example.com/mcp",
      headers: { Authorization: "Bearer xyz" },
    });
    expect(result.success).toBe(true);
  });

  test("rejects without type", () => {
    const result = PluginV2HttpServerSchema.safeParse({
      name: "search",
      url: "https://...",
    });
    expect(result.success).toBe(false);
  });
});

describe("PluginV2ServerSchema (union)", () => {
  test("accepts stdio entries", () => {
    const result = PluginV2ServerSchema.safeParse({
      type: "stdio",
      name: "db",
      command: "node",
    });
    expect(result.success).toBe(true);
  });

  test("accepts http entries", () => {
    const result = PluginV2ServerSchema.safeParse({
      type: "http",
      name: "search",
      url: "https://...",
    });
    expect(result.success).toBe(true);
  });
});

describe("PluginManifestV2Schema", () => {
  test("accepts a fully populated plugin manifest", () => {
    const result = PluginManifestV2Schema.safeParse(VALID_PLUGIN);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.name).toBe("team-toolkit");
      expect(result.data.publish).toBe(true);
      expect(result.data.skills).toHaveLength(1);
      expect(result.data.servers).toHaveLength(1);
      expect(result.data.agents).toHaveLength(1);
    }
  });

  test("publish defaults to false", () => {
    const { publish: _, ...rest } = VALID_PLUGIN;
    const result = PluginManifestV2Schema.safeParse(rest);
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.publish).toBe(false);
  });

  test("accepts a minimal plugin manifest", () => {
    const result = PluginManifestV2Schema.safeParse({
      name: "minimal",
      version: "0.1.0",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.skills).toEqual([]);
      expect(result.data.servers).toEqual([]);
      expect(result.data.agents).toEqual([]);
      expect(result.data.publish).toBe(false);
    }
  });

  test("rejects invalid plugin name", () => {
    expect(
      PluginManifestV2Schema.safeParse({ ...VALID_PLUGIN, name: "MyPlugin" })
        .success,
    ).toBe(false);
    expect(
      PluginManifestV2Schema.safeParse({ ...VALID_PLUGIN, name: "my_plugin" })
        .success,
    ).toBe(false);
    expect(
      PluginManifestV2Schema.safeParse({ ...VALID_PLUGIN, name: "-leading" })
        .success,
    ).toBe(false);
  });

  test("rejects without version", () => {
    const { version: _, ...rest } = VALID_PLUGIN;
    expect(PluginManifestV2Schema.safeParse(rest).success).toBe(false);
  });

  test("rejects invalid skill name pattern", () => {
    const result = PluginManifestV2Schema.safeParse({
      ...VALID_PLUGIN,
      skills: [{ name: "Bad_Skill", path: "./x" }],
    });
    expect(result.success).toBe(false);
  });

  test("accepts mixed stdio + http servers", () => {
    const result = PluginManifestV2Schema.safeParse({
      ...VALID_PLUGIN,
      servers: [
        { name: "db", type: "stdio", command: "node" },
        { name: "search", type: "http", url: "https://..." },
      ],
    });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.servers).toHaveLength(2);
  });
});
