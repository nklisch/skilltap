import { describe, expect, test } from "bun:test";
import {
  SkilltapHttpServerSchema,
  SkilltapPluginManifestSchema,
  SkilltapServerSchema,
  SkilltapStdioServerSchema,
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

describe("SkilltapStdioServerSchema", () => {
  test("accepts a minimal stdio server", () => {
    const result = SkilltapStdioServerSchema.safeParse({
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
    const result = SkilltapStdioServerSchema.safeParse({
      type: "stdio",
      name: "db",
      command: "node",
      args: ["./mcp/db.js", "--port", "3000"],
      env: { DATABASE_URL: "postgres://..." },
    });
    expect(result.success).toBe(true);
  });

  test("rejects without command", () => {
    const result = SkilltapStdioServerSchema.safeParse({ name: "db" });
    expect(result.success).toBe(false);
  });
});

describe("SkilltapHttpServerSchema", () => {
  test("accepts a minimal http server", () => {
    const result = SkilltapHttpServerSchema.safeParse({
      type: "http",
      name: "search",
      url: "https://search.example.com/mcp",
    });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.headers).toEqual({});
  });

  test("accepts an http server with headers", () => {
    const result = SkilltapHttpServerSchema.safeParse({
      type: "http",
      name: "search",
      url: "https://search.example.com/mcp",
      headers: { Authorization: "Bearer xyz" },
    });
    expect(result.success).toBe(true);
  });

  test("rejects without type", () => {
    const result = SkilltapHttpServerSchema.safeParse({
      name: "search",
      url: "https://...",
    });
    expect(result.success).toBe(false);
  });
});

describe("SkilltapServerSchema (union)", () => {
  test("accepts stdio entries", () => {
    const result = SkilltapServerSchema.safeParse({
      type: "stdio",
      name: "db",
      command: "node",
    });
    expect(result.success).toBe(true);
  });

  test("accepts http entries", () => {
    const result = SkilltapServerSchema.safeParse({
      type: "http",
      name: "search",
      url: "https://...",
    });
    expect(result.success).toBe(true);
  });
});

describe("SkilltapPluginManifestSchema", () => {
  test("accepts a fully populated plugin manifest", () => {
    const result = SkilltapPluginManifestSchema.safeParse(VALID_PLUGIN);
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
    const result = SkilltapPluginManifestSchema.safeParse(rest);
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.publish).toBe(false);
  });

  test("accepts a minimal plugin manifest", () => {
    const result = SkilltapPluginManifestSchema.safeParse({
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
      SkilltapPluginManifestSchema.safeParse({ ...VALID_PLUGIN, name: "MyPlugin" })
        .success,
    ).toBe(false);
    expect(
      SkilltapPluginManifestSchema.safeParse({ ...VALID_PLUGIN, name: "my_plugin" })
        .success,
    ).toBe(false);
    expect(
      SkilltapPluginManifestSchema.safeParse({ ...VALID_PLUGIN, name: "-leading" })
        .success,
    ).toBe(false);
  });

  test("rejects without version", () => {
    const { version: _, ...rest } = VALID_PLUGIN;
    expect(SkilltapPluginManifestSchema.safeParse(rest).success).toBe(false);
  });

  test("rejects invalid skill name pattern", () => {
    const result = SkilltapPluginManifestSchema.safeParse({
      ...VALID_PLUGIN,
      skills: [{ name: "Bad_Skill", path: "./x" }],
    });
    expect(result.success).toBe(false);
  });

  test("accepts mixed stdio + http servers", () => {
    const result = SkilltapPluginManifestSchema.safeParse({
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
