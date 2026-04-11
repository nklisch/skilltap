import { describe, expect, test } from "bun:test";
import {
  PluginRecordSchema,
  PluginsJsonSchema,
  StoredComponentSchema,
} from "./plugins";

const VALID_RECORD = {
  name: "dev-toolkit",
  description: "Development productivity tools",
  format: "claude-code",
  repo: "https://github.com/nklisch/dev-toolkit",
  ref: "main",
  sha: "abc123def456",
  scope: "global",
  also: ["claude-code", "cursor"],
  tap: null,
  components: [
    { type: "skill", name: "code-review", active: true },
    { type: "skill", name: "commit-helper", active: true },
    { type: "mcp", name: "database", active: true, command: "npx", args: ["-y", "@corp/db-mcp"], env: {} },
    { type: "agent", name: "code-review", active: true, platform: "claude-code" },
  ],
  installedAt: "2026-04-10T12:00:00Z",
  updatedAt: "2026-04-10T12:00:00Z",
  active: true,
};

const SPEC_EXAMPLE = {
  version: 1,
  plugins: [VALID_RECORD],
};

describe("PluginsJsonSchema", () => {
  test("accepts the SPEC example JSON", () => {
    const result = PluginsJsonSchema.safeParse(SPEC_EXAMPLE);
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.version).toBe(1);
    expect(result.data.plugins).toHaveLength(1);
    expect(result.data.plugins[0]?.name).toBe("dev-toolkit");
  });

  test("accepts empty plugins array", () => {
    const result = PluginsJsonSchema.safeParse({ version: 1, plugins: [] });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.plugins).toEqual([]);
  });

  test("rejects invalid version", () => {
    const result = PluginsJsonSchema.safeParse({ version: 99, plugins: [] });
    expect(result.success).toBe(false);
  });

  test("defaults plugins to [] when omitted", () => {
    const result = PluginsJsonSchema.safeParse({ version: 1 });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.plugins).toEqual([]);
  });
});

describe("PluginRecordSchema", () => {
  test("accepts valid record with all fields", () => {
    const result = PluginRecordSchema.safeParse(VALID_RECORD);
    expect(result.success).toBe(true);
  });

  test("defaults active to true", () => {
    const { active: _, ...without } = VALID_RECORD;
    const result = PluginRecordSchema.safeParse(without);
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.active).toBe(true);
  });

  test("defaults also to []", () => {
    const { also: _, ...without } = VALID_RECORD;
    const result = PluginRecordSchema.safeParse(without);
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.also).toEqual([]);
  });

  test("defaults tap to null", () => {
    const { tap: _, ...without } = VALID_RECORD;
    const result = PluginRecordSchema.safeParse(without);
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.tap).toBeNull();
  });

  test("defaults description to empty string", () => {
    const { description: _, ...without } = VALID_RECORD;
    const result = PluginRecordSchema.safeParse(without);
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.description).toBe("");
  });

  test("rejects missing name", () => {
    const { name: _, ...without } = VALID_RECORD;
    expect(PluginRecordSchema.safeParse(without).success).toBe(false);
  });

  test("rejects missing format", () => {
    const { format: _, ...without } = VALID_RECORD;
    expect(PluginRecordSchema.safeParse(without).success).toBe(false);
  });

  test("rejects invalid scope", () => {
    expect(PluginRecordSchema.safeParse({ ...VALID_RECORD, scope: "local" }).success).toBe(false);
  });
});

describe("StoredComponentSchema", () => {
  test("discriminates on type", () => {
    expect(StoredComponentSchema.safeParse({ type: "skill", name: "foo" }).success).toBe(true);
    expect(StoredComponentSchema.safeParse({ type: "mcp", name: "db", command: "npx" }).success).toBe(true);
    expect(StoredComponentSchema.safeParse({ type: "agent", name: "bot" }).success).toBe(true);
  });

  test("accepts skill component and defaults active to true", () => {
    const result = StoredComponentSchema.safeParse({ type: "skill", name: "code-review" });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.active).toBe(true);
  });

  test("accepts mcp component with command/args/env", () => {
    const result = StoredComponentSchema.safeParse({
      type: "mcp",
      name: "database",
      active: true,
      command: "npx",
      args: ["-y", "@corp/db-mcp"],
      env: { DB_URL: "postgres://localhost" },
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    if (result.data.type !== "mcp") return;
    expect(result.data.command).toBe("npx");
    expect(result.data.args).toEqual(["-y", "@corp/db-mcp"]);
    expect(result.data.env).toEqual({ DB_URL: "postgres://localhost" });
  });

  test("accepts agent component and defaults platform to claude-code", () => {
    const result = StoredComponentSchema.safeParse({ type: "agent", name: "bot" });
    expect(result.success).toBe(true);
    if (!result.success) return;
    if (result.data.type !== "agent") return;
    expect(result.data.platform).toBe("claude-code");
  });

  test("rejects unknown type", () => {
    expect(StoredComponentSchema.safeParse({ type: "unknown", name: "foo" }).success).toBe(false);
  });

  test("rejects mcp component without command", () => {
    expect(StoredComponentSchema.safeParse({ type: "mcp", name: "db" }).success).toBe(false);
  });
});
