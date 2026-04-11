import { mkdtemp, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import type { StoredMcpComponent } from "../schemas/plugins";
import {
  injectMcpServers,
  isNamespacedKey,
  listMcpServers,
  MCP_AGENT_CONFIGS,
  mcpConfigPath,
  namespaceMcpServer,
  parseNamespacedKey,
  removeMcpServers,
  substituteMcpVars,
} from "./mcp-inject";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeMcpServer(overrides?: Partial<StoredMcpComponent>): StoredMcpComponent {
  return {
    type: "mcp",
    name: "test-server",
    active: true,
    command: "npx",
    args: ["-y", "my-mcp"],
    env: {},
    ...overrides,
  };
}

async function readJson(path: string): Promise<Record<string, unknown>> {
  return Bun.file(path).json();
}

// ---------------------------------------------------------------------------
// Env isolation for global scope tests
// ---------------------------------------------------------------------------

let tmpDir: string;
let savedHome: string | undefined;

beforeEach(async () => {
  tmpDir = await mkdtemp(join(tmpdir(), "skilltap-test-"));
  savedHome = process.env.SKILLTAP_HOME;
  process.env.SKILLTAP_HOME = tmpDir;
});

afterEach(async () => {
  if (savedHome !== undefined) process.env.SKILLTAP_HOME = savedHome;
  else delete process.env.SKILLTAP_HOME;
  await rm(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// namespaceMcpServer
// ---------------------------------------------------------------------------

describe("namespaceMcpServer", () => {
  test("formats skilltap:plugin:server", () => {
    expect(namespaceMcpServer("dev-toolkit", "database")).toBe("skilltap:dev-toolkit:database");
  });

  test("handles plugin names with hyphens", () => {
    expect(namespaceMcpServer("my-plugin", "my-server")).toBe("skilltap:my-plugin:my-server");
  });
});

// ---------------------------------------------------------------------------
// isNamespacedKey
// ---------------------------------------------------------------------------

describe("isNamespacedKey", () => {
  test("returns true for skilltap-prefixed key", () => {
    expect(isNamespacedKey("skilltap:dev-toolkit:database")).toBe(true);
  });

  test("returns false for plain key", () => {
    expect(isNamespacedKey("my-server")).toBe(false);
  });

  test("returns false for empty string", () => {
    expect(isNamespacedKey("")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// parseNamespacedKey
// ---------------------------------------------------------------------------

describe("parseNamespacedKey", () => {
  test("parses valid namespaced key", () => {
    expect(parseNamespacedKey("skilltap:dev-toolkit:database")).toEqual({
      pluginName: "dev-toolkit",
      serverName: "database",
    });
  });

  test("returns null for non-skilltap key", () => {
    expect(parseNamespacedKey("user-server")).toBeNull();
  });

  test("handles server name containing colons", () => {
    expect(parseNamespacedKey("skilltap:plugin:server:with:colons")).toEqual({
      pluginName: "plugin",
      serverName: "server:with:colons",
    });
  });

  test("returns null for key with only two segments after prefix", () => {
    // "skilltap:plugin" — missing server name segment
    expect(parseNamespacedKey("skilltap:plugin")).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// substituteMcpVars
// ---------------------------------------------------------------------------

describe("substituteMcpVars", () => {
  const ctx = { pluginRoot: "/opt/plugins/foo", pluginData: "/var/data/foo" };

  test("replaces ${CLAUDE_PLUGIN_ROOT} in command", () => {
    const server = makeMcpServer({ command: "${CLAUDE_PLUGIN_ROOT}/bin/server" });
    const result = substituteMcpVars(server, ctx);
    expect(result.command).toBe("/opt/plugins/foo/bin/server");
  });

  test("replaces ${CLAUDE_PLUGIN_ROOT} in args", () => {
    const server = makeMcpServer({ args: ["--root", "${CLAUDE_PLUGIN_ROOT}"] });
    const result = substituteMcpVars(server, ctx);
    expect(result.args).toEqual(["--root", "/opt/plugins/foo"]);
  });

  test("replaces ${CLAUDE_PLUGIN_DATA} in env values", () => {
    const server = makeMcpServer({ env: { DATA_DIR: "${CLAUDE_PLUGIN_DATA}/cache" } });
    const result = substituteMcpVars(server, ctx);
    expect(result.env).toEqual({ DATA_DIR: "/var/data/foo/cache" });
  });

  test("replaces multiple variables in same string", () => {
    const server = makeMcpServer({
      command: "${CLAUDE_PLUGIN_ROOT}/bin",
      args: ["${CLAUDE_PLUGIN_ROOT}:${CLAUDE_PLUGIN_DATA}"],
    });
    const result = substituteMcpVars(server, ctx);
    expect(result.args[0]).toBe("/opt/plugins/foo:/var/data/foo");
  });

  test("returns unchanged component when no variables present", () => {
    const server = makeMcpServer({ command: "npx", args: ["-y", "my-mcp"], env: {} });
    const result = substituteMcpVars(server, ctx);
    expect(result.command).toBe("npx");
    expect(result.args).toEqual(["-y", "my-mcp"]);
  });

  test("handles both variables in same component", () => {
    const server = makeMcpServer({
      command: "${CLAUDE_PLUGIN_ROOT}/server",
      env: { DATA: "${CLAUDE_PLUGIN_DATA}" },
    });
    const result = substituteMcpVars(server, ctx);
    expect(result.command).toBe("/opt/plugins/foo/server");
    expect(result.env.DATA).toBe("/var/data/foo");
  });
});

// ---------------------------------------------------------------------------
// mcpConfigPath
// ---------------------------------------------------------------------------

describe("mcpConfigPath", () => {
  test("returns correct path for claude-code global", () => {
    const path = mcpConfigPath("claude-code", "global");
    expect(path).toBe(join(tmpDir, ".claude/settings.json"));
  });

  test("returns correct path for cursor global", () => {
    const path = mcpConfigPath("cursor", "global");
    expect(path).toBe(join(tmpDir, ".cursor/mcp.json"));
  });

  test("returns correct path for project scope", () => {
    const path = mcpConfigPath("cursor", "project", "/my/project");
    expect(path).toBe("/my/project/.cursor/mcp.json");
  });

  test("returns null for unknown agent", () => {
    expect(mcpConfigPath("unknown-agent", "global")).toBeNull();
  });

  test("covers all 5 registry entries", () => {
    for (const agent of Object.keys(MCP_AGENT_CONFIGS)) {
      const path = mcpConfigPath(agent, "project", "/p");
      expect(path).toBe(join("/p", MCP_AGENT_CONFIGS[agent]));
    }
  });
});

// ---------------------------------------------------------------------------
// injectMcpServers
// ---------------------------------------------------------------------------

describe("injectMcpServers", () => {
  test("creates new mcp.json with mcpServers key", async () => {
    const projectRoot = tmpDir;
    const server = makeMcpServer({ name: "db" });

    const result = await injectMcpServers({
      pluginName: "my-plugin",
      servers: [server],
      agents: ["cursor"],
      scope: "project",
      projectRoot,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toContain("cursor");

    const config = await readJson(join(projectRoot, ".cursor/mcp.json"));
    expect(config.mcpServers).toEqual({
      "skilltap:my-plugin:db": { command: "npx", args: ["-y", "my-mcp"] },
    });
  });

  test("adds to existing settings.json preserving other keys", async () => {
    const projectRoot = tmpDir;
    const settingsPath = join(projectRoot, ".claude/settings.json");
    await Bun.write(
      settingsPath,
      JSON.stringify({ permissions: { allow: ["Bash"] }, mcpServers: {} }, null, 2),
    );

    const result = await injectMcpServers({
      pluginName: "my-plugin",
      servers: [makeMcpServer({ name: "db" })],
      agents: ["claude-code"],
      scope: "project",
      projectRoot,
    });

    expect(result.ok).toBe(true);
    const config = await readJson(settingsPath);
    expect((config.permissions as { allow: string[] }).allow).toEqual(["Bash"]);
    expect((config.mcpServers as Record<string, unknown>)["skilltap:my-plugin:db"]).toBeTruthy();
  });

  test("namespaces server names correctly", async () => {
    const projectRoot = tmpDir;

    await injectMcpServers({
      pluginName: "dev-toolkit",
      servers: [makeMcpServer({ name: "database" })],
      agents: ["cursor"],
      scope: "project",
      projectRoot,
    });

    const config = await readJson(join(projectRoot, ".cursor/mcp.json"));
    expect(Object.keys(config.mcpServers as Record<string, unknown>)[0]).toBe(
      "skilltap:dev-toolkit:database",
    );
  });

  test("creates backup before first modification", async () => {
    const projectRoot = tmpDir;
    const configPath = join(projectRoot, ".cursor/mcp.json");
    await Bun.write(configPath, JSON.stringify({ mcpServers: { "user-key": {} } }, null, 2));

    await injectMcpServers({
      pluginName: "p",
      servers: [makeMcpServer()],
      agents: ["cursor"],
      scope: "project",
      projectRoot,
    });

    const backupExists = await Bun.file(configPath + ".skilltap.bak").exists();
    expect(backupExists).toBe(true);
  });

  test("does not overwrite existing backup", async () => {
    const projectRoot = tmpDir;
    const configPath = join(projectRoot, ".cursor/mcp.json");
    const backupPath = configPath + ".skilltap.bak";
    await Bun.write(configPath, JSON.stringify({ mcpServers: {} }, null, 2));
    await Bun.write(backupPath, JSON.stringify({ original: true }, null, 2));

    await injectMcpServers({
      pluginName: "p",
      servers: [makeMcpServer()],
      agents: ["cursor"],
      scope: "project",
      projectRoot,
    });

    const backup = await readJson(backupPath);
    expect(backup.original).toBe(true);
  });

  test("is idempotent — re-injection produces same result", async () => {
    const projectRoot = tmpDir;
    const opts = {
      pluginName: "p",
      servers: [makeMcpServer({ name: "db" })],
      agents: ["cursor"],
      scope: "project" as const,
      projectRoot,
    };

    await injectMcpServers(opts);
    await injectMcpServers(opts);

    const config = await readJson(join(projectRoot, ".cursor/mcp.json"));
    const keys = Object.keys(config.mcpServers as Record<string, unknown>);
    expect(keys).toHaveLength(1);
    expect(keys[0]).toBe("skilltap:p:db");
  });

  test("only includes env when non-empty", async () => {
    const projectRoot = tmpDir;
    const serverNoEnv = makeMcpServer({ env: {} });
    const serverWithEnv = makeMcpServer({ name: "with-env", env: { TOKEN: "abc" } });

    await injectMcpServers({
      pluginName: "p",
      servers: [serverNoEnv, serverWithEnv],
      agents: ["cursor"],
      scope: "project",
      projectRoot,
    });

    const config = await readJson(join(projectRoot, ".cursor/mcp.json"));
    const mcp = config.mcpServers as Record<string, Record<string, unknown>>;
    expect(mcp["skilltap:p:test-server"].env).toBeUndefined();
    expect(mcp["skilltap:p:with-env"].env).toEqual({ TOKEN: "abc" });
  });

  test("skips unknown agent IDs", async () => {
    const result = await injectMcpServers({
      pluginName: "p",
      servers: [makeMcpServer()],
      agents: ["nonexistent-agent"],
      scope: "project",
      projectRoot: tmpDir,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(0);
  });

  test("applies variable substitution", async () => {
    const projectRoot = tmpDir;

    await injectMcpServers({
      pluginName: "p",
      servers: [
        makeMcpServer({
          command: "${CLAUDE_PLUGIN_ROOT}/bin/server",
          env: { DATA: "${CLAUDE_PLUGIN_DATA}" },
        }),
      ],
      agents: ["cursor"],
      scope: "project",
      projectRoot,
      vars: { pluginRoot: "/plugins/p", pluginData: "/data/p" },
    });

    const config = await readJson(join(projectRoot, ".cursor/mcp.json"));
    const entry = (config.mcpServers as Record<string, Record<string, unknown>>)[
      "skilltap:p:test-server"
    ];
    expect(entry.command).toBe("/plugins/p/bin/server");
    expect((entry.env as Record<string, string>).DATA).toBe("/data/p");
  });

  test("handles multiple servers for multiple agents", async () => {
    const projectRoot = tmpDir;

    const result = await injectMcpServers({
      pluginName: "p",
      servers: [makeMcpServer({ name: "s1" }), makeMcpServer({ name: "s2" })],
      agents: ["cursor", "codex"],
      scope: "project",
      projectRoot,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(2);

    for (const agent of ["cursor", "codex"]) {
      const path = mcpConfigPath(agent, "project", projectRoot)!;
      const config = await readJson(path);
      const keys = Object.keys(config.mcpServers as Record<string, unknown>);
      expect(keys).toHaveLength(2);
    }
  });

  test("creates parent directories when needed", async () => {
    const projectRoot = tmpDir;

    const result = await injectMcpServers({
      pluginName: "p",
      servers: [makeMcpServer()],
      agents: ["gemini"],
      scope: "project",
      projectRoot,
    });

    expect(result.ok).toBe(true);
    const exists = await Bun.file(join(projectRoot, ".gemini/settings.json")).exists();
    expect(exists).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// removeMcpServers
// ---------------------------------------------------------------------------

describe("removeMcpServers", () => {
  test("removes only entries matching plugin name", async () => {
    const projectRoot = tmpDir;
    const configPath = join(projectRoot, ".cursor/mcp.json");
    await Bun.write(
      configPath,
      JSON.stringify(
        {
          mcpServers: {
            "skilltap:plugin-a:server1": { command: "a", args: [] },
            "skilltap:plugin-b:server2": { command: "b", args: [] },
          },
        },
        null,
        2,
      ),
    );

    const result = await removeMcpServers({
      pluginName: "plugin-a",
      agents: ["cursor"],
      scope: "project",
      projectRoot,
    });

    expect(result.ok).toBe(true);
    const config = await readJson(configPath);
    const keys = Object.keys(config.mcpServers as Record<string, unknown>);
    expect(keys).toEqual(["skilltap:plugin-b:server2"]);
  });

  test("preserves user-configured servers", async () => {
    const projectRoot = tmpDir;
    const configPath = join(projectRoot, ".cursor/mcp.json");
    await Bun.write(
      configPath,
      JSON.stringify(
        {
          mcpServers: {
            "user-custom-server": { command: "custom", args: [] },
            "skilltap:p:s": { command: "x", args: [] },
          },
        },
        null,
        2,
      ),
    );

    await removeMcpServers({
      pluginName: "p",
      agents: ["cursor"],
      scope: "project",
      projectRoot,
    });

    const config = await readJson(configPath);
    const mcp = config.mcpServers as Record<string, unknown>;
    expect(mcp["user-custom-server"]).toBeTruthy();
    expect(mcp["skilltap:p:s"]).toBeUndefined();
  });

  test("preserves other skilltap plugin entries", async () => {
    const projectRoot = tmpDir;
    const configPath = join(projectRoot, ".cursor/mcp.json");
    await Bun.write(
      configPath,
      JSON.stringify(
        {
          mcpServers: {
            "skilltap:plugin-a:s": { command: "a", args: [] },
            "skilltap:plugin-b:s": { command: "b", args: [] },
          },
        },
        null,
        2,
      ),
    );

    await removeMcpServers({
      pluginName: "plugin-a",
      agents: ["cursor"],
      scope: "project",
      projectRoot,
    });

    const config = await readJson(configPath);
    const mcp = config.mcpServers as Record<string, unknown>;
    expect(mcp["skilltap:plugin-b:s"]).toBeTruthy();
  });

  test("handles missing config file", async () => {
    const result = await removeMcpServers({
      pluginName: "p",
      agents: ["cursor"],
      scope: "project",
      projectRoot: tmpDir,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // Nothing to remove, but no error
    expect(result.value).toHaveLength(0);
  });

  test("handles config with no mcpServers key", async () => {
    const projectRoot = tmpDir;
    const configPath = join(projectRoot, ".cursor/mcp.json");
    await Bun.write(configPath, JSON.stringify({ other: "data" }, null, 2));

    const result = await removeMcpServers({
      pluginName: "p",
      agents: ["cursor"],
      scope: "project",
      projectRoot,
    });

    expect(result.ok).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// listMcpServers
// ---------------------------------------------------------------------------

describe("listMcpServers", () => {
  test("lists skilltap-namespaced keys", async () => {
    const projectRoot = tmpDir;
    const configPath = join(projectRoot, ".cursor/mcp.json");
    await Bun.write(
      configPath,
      JSON.stringify(
        {
          mcpServers: {
            "skilltap:p:s1": { command: "a", args: [] },
            "skilltap:p:s2": { command: "b", args: [] },
            "user-server": { command: "c", args: [] },
          },
        },
        null,
        2,
      ),
    );

    const result = await listMcpServers("cursor", "project", projectRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.sort()).toEqual(["skilltap:p:s1", "skilltap:p:s2"]);
  });

  test("returns empty array for missing config", async () => {
    const result = await listMcpServers("cursor", "project", tmpDir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(0);
  });

  test("excludes non-skilltap keys", async () => {
    const projectRoot = tmpDir;
    const configPath = join(projectRoot, ".cursor/mcp.json");
    await Bun.write(
      configPath,
      JSON.stringify({ mcpServers: { "user-server": { command: "x", args: [] } } }, null, 2),
    );

    const result = await listMcpServers("cursor", "project", projectRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(0);
  });

  test("returns empty array for unknown agent", async () => {
    const result = await listMcpServers("unknown-agent", "project", tmpDir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Round-trip integration
// ---------------------------------------------------------------------------

describe("round-trip integration", () => {
  test("inject → list → remove → list (empty) for all 5 agents", async () => {
    const projectRoot = tmpDir;
    const agents = Object.keys(MCP_AGENT_CONFIGS);

    const injectResult = await injectMcpServers({
      pluginName: "round-trip",
      servers: [makeMcpServer({ name: "srv" })],
      agents,
      scope: "project",
      projectRoot,
    });

    expect(injectResult.ok).toBe(true);
    if (!injectResult.ok) return;
    expect(injectResult.value).toHaveLength(agents.length);

    // List — each agent should have one key
    for (const agent of agents) {
      const listResult = await listMcpServers(agent, "project", projectRoot);
      expect(listResult.ok).toBe(true);
      if (!listResult.ok) return;
      expect(listResult.value).toEqual(["skilltap:round-trip:srv"]);
    }

    // Remove
    const removeResult = await removeMcpServers({
      pluginName: "round-trip",
      agents,
      scope: "project",
      projectRoot,
    });

    expect(removeResult.ok).toBe(true);

    // List after remove — each agent should have empty array
    for (const agent of agents) {
      const listResult = await listMcpServers(agent, "project", projectRoot);
      expect(listResult.ok).toBe(true);
      if (!listResult.ok) return;
      expect(listResult.value).toHaveLength(0);
    }
  });
});
