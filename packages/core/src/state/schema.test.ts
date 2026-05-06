import { describe, expect, test } from "bun:test";
import { StateSchema, StoredMcpStandaloneSchema } from "./schema";

const VALID_SKILL = {
  name: "commit-helper",
  repo: "https://github.com/nathan/commit-helper",
  ref: "v1.2.0",
  sha: "abc123",
  scope: "global" as const,
  path: null,
  tap: null,
  also: [],
  installedAt: "2026-05-05T00:00:00.000Z",
  updatedAt: "2026-05-05T00:00:00.000Z",
};

const VALID_PLUGIN = {
  name: "dev-toolkit",
  format: "skilltap" as const,
  repo: "https://github.com/corp/dev-toolkit",
  ref: "main",
  sha: "abc123",
  scope: "global" as const,
  components: [],
  installedAt: "2026-05-05T00:00:00.000Z",
  updatedAt: "2026-05-05T00:00:00.000Z",
};

const VALID_MCP_STDIO = {
  name: "skilltap:db",
  source: "github:corp/db-mcp",
  config: {
    type: "stdio" as const,
    command: "node",
    args: ["server.js"],
    env: { DATABASE_URL: "postgres://..." },
  },
  targets: ["claude-code", "cursor"],
  installedAt: "2026-05-05T00:00:00.000Z",
};

const VALID_MCP_HTTP = {
  name: "skilltap:search",
  source: "https://search.example.com/mcp",
  config: {
    type: "http" as const,
    url: "https://search.example.com/mcp",
    headers: { Authorization: "Bearer x" },
  },
  targets: ["claude-code"],
  installedAt: "2026-05-05T00:00:00.000Z",
};

describe("StoredMcpStandaloneSchema", () => {
  test("accepts a stdio MCP entry", () => {
    const result = StoredMcpStandaloneSchema.safeParse(VALID_MCP_STDIO);
    expect(result.success).toBe(true);
  });

  test("accepts an http MCP entry", () => {
    const result = StoredMcpStandaloneSchema.safeParse(VALID_MCP_HTTP);
    expect(result.success).toBe(true);
  });

  test("defaults targets and config defaults", () => {
    const result = StoredMcpStandaloneSchema.safeParse({
      name: "minimal",
      source: "...",
      config: { type: "stdio", command: "node" },
      installedAt: "2026-05-05T00:00:00.000Z",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.targets).toEqual([]);
      expect(result.data.config.type).toBe("stdio");
      if (result.data.config.type === "stdio") {
        expect(result.data.config.args).toEqual([]);
        expect(result.data.config.env).toEqual({});
      }
    }
  });

  test("rejects without name", () => {
    const { name: _, ...rest } = VALID_MCP_STDIO;
    expect(StoredMcpStandaloneSchema.safeParse(rest).success).toBe(false);
  });

  test("rejects with invalid installedAt", () => {
    expect(
      StoredMcpStandaloneSchema.safeParse({
        ...VALID_MCP_STDIO,
        installedAt: "not-a-date",
      }).success,
    ).toBe(false);
  });
});

describe("StateSchema", () => {
  test("accepts an empty state at version 2", () => {
    const result = StateSchema.safeParse({ version: 2 });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.skills).toEqual([]);
      expect(result.data.plugins).toEqual([]);
      expect(result.data.mcpServers).toEqual([]);
    }
  });

  test("rejects wrong version", () => {
    expect(StateSchema.safeParse({ version: 1 }).success).toBe(false);
    expect(StateSchema.safeParse({ version: 3 }).success).toBe(false);
    expect(StateSchema.safeParse({}).success).toBe(false);
  });

  test("accepts a populated state", () => {
    const result = StateSchema.safeParse({
      version: 2,
      skills: [VALID_SKILL],
      plugins: [VALID_PLUGIN],
      mcpServers: [VALID_MCP_STDIO, VALID_MCP_HTTP],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.skills).toHaveLength(1);
      expect(result.data.plugins).toHaveLength(1);
      expect(result.data.mcpServers).toHaveLength(2);
    }
  });

  test("rejects an invalid skill in the array", () => {
    const result = StateSchema.safeParse({
      version: 2,
      skills: [{ ...VALID_SKILL, scope: "bad" }],
    });
    expect(result.success).toBe(false);
  });
});
