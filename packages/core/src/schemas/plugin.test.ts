import { describe, expect, test } from "bun:test";
import {
  ClaudePluginJsonSchema,
  CodexPluginJsonSchema,
  McpServerEntrySchema,
  McpStdioServerSchema,
  PluginComponentSchema,
  PluginManifestSchema,
} from "./plugin";

const VALID_SKILL_COMPONENT = {
  type: "skill" as const,
  name: "helper",
  path: "skills/helper",
  description: "A helper skill",
};

const VALID_MCP_COMPONENT = {
  type: "mcp" as const,
  server: { type: "stdio" as const, name: "db", command: "npx", args: ["-y", "db-mcp"], env: {} },
};

const VALID_AGENT_COMPONENT = {
  type: "agent" as const,
  name: "reviewer",
  path: "agents/reviewer.md",
  frontmatter: { model: "sonnet" },
};

const VALID_MANIFEST = {
  name: "test-plugin",
  format: "claude-code" as const,
  pluginRoot: "/tmp/test-plugin",
  components: [],
};

describe("PluginManifestSchema", () => {
  test("accepts valid manifest with all component types", () => {
    const result = PluginManifestSchema.safeParse({
      ...VALID_MANIFEST,
      components: [VALID_SKILL_COMPONENT, VALID_MCP_COMPONENT, VALID_AGENT_COMPONENT],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.components).toHaveLength(3);
    }
  });

  test("accepts manifest with no components", () => {
    const result = PluginManifestSchema.safeParse(VALID_MANIFEST);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.components).toEqual([]);
    }
  });

  test("rejects missing name", () => {
    const { name: _, ...without } = VALID_MANIFEST;
    expect(PluginManifestSchema.safeParse(without).success).toBe(false);
  });

  test("defaults description to empty string", () => {
    const result = PluginManifestSchema.safeParse(VALID_MANIFEST);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.description).toBe("");
    }
  });

  test("accepts optional version", () => {
    const result = PluginManifestSchema.safeParse({ ...VALID_MANIFEST, version: "1.2.3" });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.version).toBe("1.2.3");
    }
  });
});

describe("ClaudePluginJsonSchema", () => {
  test("accepts minimal { name }", () => {
    const result = ClaudePluginJsonSchema.safeParse({ name: "test" });
    expect(result.success).toBe(true);
  });

  test("accepts full manifest with all fields", () => {
    const result = ClaudePluginJsonSchema.safeParse({
      name: "full-plugin",
      description: "A full plugin",
      version: "1.0.0",
      author: { name: "Alice", email: "alice@example.com" },
      homepage: "https://example.com",
      repository: "owner/repo",
      license: "MIT",
      keywords: ["ai", "tools"],
      skills: ["skills/"],
      agents: "agents/",
      mcpServers: ".mcp.json",
    });
    expect(result.success).toBe(true);
  });

  test("tolerates unknown fields (passthrough)", () => {
    const result = ClaudePluginJsonSchema.safeParse({
      name: "test",
      unknownField: "value",
      anotherField: 42,
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect((result.data as Record<string, unknown>).unknownField).toBe("value");
    }
  });

  test("accepts skills as string", () => {
    const result = ClaudePluginJsonSchema.safeParse({ name: "test", skills: "skills/" });
    expect(result.success).toBe(true);
  });

  test("accepts skills as string[]", () => {
    const result = ClaudePluginJsonSchema.safeParse({ name: "test", skills: ["skills/a", "skills/b"] });
    expect(result.success).toBe(true);
  });

  test("rejects missing name", () => {
    expect(ClaudePluginJsonSchema.safeParse({ description: "no name" }).success).toBe(false);
  });
});

describe("CodexPluginJsonSchema", () => {
  test("accepts valid { name, version, description }", () => {
    const result = CodexPluginJsonSchema.safeParse({
      name: "test-codex",
      version: "1.0.0",
      description: "A codex plugin",
    });
    expect(result.success).toBe(true);
  });

  test("rejects missing version", () => {
    expect(CodexPluginJsonSchema.safeParse({ name: "test", description: "desc" }).success).toBe(false);
  });

  test("rejects missing description", () => {
    expect(CodexPluginJsonSchema.safeParse({ name: "test", version: "1.0.0" }).success).toBe(false);
  });

  test("tolerates unknown fields", () => {
    const result = CodexPluginJsonSchema.safeParse({
      name: "test",
      version: "1.0.0",
      description: "desc",
      extra: "ignored",
    });
    expect(result.success).toBe(true);
  });
});

describe("McpServerEntrySchema", () => {
  test("accepts stdio server with command only", () => {
    const result = McpServerEntrySchema.safeParse({ type: "stdio", name: "db", command: "npx" });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("stdio");
    }
  });

  test("accepts stdio server with command, args, env", () => {
    const result = McpServerEntrySchema.safeParse({
      type: "stdio",
      name: "db",
      command: "npx",
      args: ["-y", "db-mcp"],
      env: { TOKEN: "abc" },
    });
    expect(result.success).toBe(true);
    if (result.success && result.data.type === "stdio") {
      expect(result.data.args).toEqual(["-y", "db-mcp"]);
      expect(result.data.env).toEqual({ TOKEN: "abc" });
    }
  });

  test("defaults type to 'stdio' when omitted", () => {
    // Verify the literal default actually works when type is NOT passed
    const result = McpStdioServerSchema.safeParse({ name: "db", command: "npx" });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("stdio");
    }
  });

  test("rejects stdio server with no command", () => {
    const result = McpServerEntrySchema.safeParse({
      type: "stdio",
      name: "broken",
      env: { TOKEN: "x" },
    });
    expect(result.success).toBe(false);
  });

  test("accepts empty name string (no length constraint)", () => {
    // Plugin schemas don't enforce name length — validation is lenient
    const result = McpServerEntrySchema.safeParse({ type: "stdio", name: "", command: "npx" });
    expect(result.success).toBe(true);
  });

  test("accepts http server with type and url", () => {
    const result = McpServerEntrySchema.safeParse({
      type: "http",
      name: "api",
      url: "https://api.example.com/mcp",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("http");
    }
  });

  test("rejects entry with neither command nor url", () => {
    expect(McpServerEntrySchema.safeParse({ name: "broken" }).success).toBe(false);
  });
});

describe("PluginComponentSchema", () => {
  test("accepts skill component", () => {
    const result = PluginComponentSchema.safeParse(VALID_SKILL_COMPONENT);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("skill");
    }
  });

  test("accepts mcp component wrapping stdio server", () => {
    const result = PluginComponentSchema.safeParse(VALID_MCP_COMPONENT);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("mcp");
    }
  });

  test("accepts mcp component wrapping http server", () => {
    const result = PluginComponentSchema.safeParse({
      type: "mcp",
      server: { type: "http", name: "api", url: "https://example.com" },
    });
    expect(result.success).toBe(true);
    if (result.success && result.data.type === "mcp") {
      expect(result.data.server.type).toBe("http");
    }
  });

  test("accepts agent component", () => {
    const result = PluginComponentSchema.safeParse(VALID_AGENT_COMPONENT);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.type).toBe("agent");
    }
  });

  test("discriminates on type field", () => {
    expect(PluginComponentSchema.safeParse({ type: "unknown" }).success).toBe(false);
  });
});
