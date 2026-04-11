import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import {
  addPlugin,
  findPlugin,
  loadPlugins,
  manifestToRecord,
  removePlugin,
  savePlugins,
  toggleComponent,
  type PluginInstallMeta,
} from "./state";
import type { PluginRecord, PluginsJson } from "../schemas/plugins";
import type { PluginManifest } from "../schemas/plugin";

let env: TestEnv;

beforeEach(async () => { env = await createTestEnv(); });
afterEach(async () => { await env.cleanup(); });

const VALID_RECORD: PluginRecord = {
  name: "dev-toolkit",
  description: "Dev tools",
  format: "claude-code",
  repo: "https://github.com/nklisch/dev-toolkit",
  ref: "main",
  sha: "abc123",
  scope: "global",
  also: [],
  tap: null,
  components: [
    { type: "skill", name: "code-review", active: true },
    { type: "mcp", name: "database", active: true, command: "npx", args: [], env: {} },
    { type: "agent", name: "reviewer", active: true, platform: "claude-code" },
  ],
  installedAt: "2026-04-10T12:00:00Z",
  updatedAt: "2026-04-10T12:00:00Z",
  active: true,
};

const EMPTY_STATE: PluginsJson = { version: 1, plugins: [] };

describe("loadPlugins", () => {
  test("returns default empty state when file missing", async () => {
    const result = await loadPlugins();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual({ version: 1, plugins: [] });
  });

  test("parses valid plugins.json", async () => {
    const state: PluginsJson = { version: 1, plugins: [VALID_RECORD] };
    const saveResult = await savePlugins(state);
    expect(saveResult.ok).toBe(true);

    const loadResult = await loadPlugins();
    expect(loadResult.ok).toBe(true);
    if (!loadResult.ok) return;
    expect(loadResult.value.plugins).toHaveLength(1);
    expect(loadResult.value.plugins[0]?.name).toBe("dev-toolkit");
  });

  test("returns error for invalid JSON", async () => {
    const configDir = join(env.configDir,"skilltap");
    await Bun.$ `mkdir -p ${configDir}`;
    await Bun.write(join(configDir, "plugins.json"), "not-valid-json{{{");
    const result = await loadPlugins();
    expect(result.ok).toBe(false);
  });

  test("returns error for invalid schema (version 99)", async () => {
    const configDir = join(env.configDir,"skilltap");
    await Bun.$ `mkdir -p ${configDir}`;
    await Bun.write(join(configDir, "plugins.json"), JSON.stringify({ version: 99, plugins: [] }));
    const result = await loadPlugins();
    expect(result.ok).toBe(false);
  });

  test("reads from project path when projectRoot given", async () => {
    const projectDir = join(env.configDir,"myproject");
    const state: PluginsJson = { version: 1, plugins: [VALID_RECORD] };
    const saveResult = await savePlugins(state, projectDir);
    expect(saveResult.ok).toBe(true);

    const loadResult = await loadPlugins(projectDir);
    expect(loadResult.ok).toBe(true);
    if (!loadResult.ok) return;
    expect(loadResult.value.plugins[0]?.name).toBe("dev-toolkit");
  });
});

describe("savePlugins", () => {
  test("writes valid JSON that round-trips through loadPlugins", async () => {
    const state: PluginsJson = { version: 1, plugins: [VALID_RECORD] };
    const saveResult = await savePlugins(state);
    expect(saveResult.ok).toBe(true);

    const loadResult = await loadPlugins();
    expect(loadResult.ok).toBe(true);
    if (!loadResult.ok) return;
    expect(loadResult.value.plugins[0]?.name).toBe("dev-toolkit");
    expect(loadResult.value.plugins[0]?.components).toHaveLength(3);
  });

  test("creates .agents/ dir for project scope", async () => {
    const projectDir = join(env.configDir,"myproject");
    const result = await savePlugins(EMPTY_STATE, projectDir);
    expect(result.ok).toBe(true);
    expect(await Bun.file(join(projectDir, ".agents", "plugins.json")).exists()).toBe(true);
  });
});

describe("addPlugin", () => {
  test("appends a new plugin", () => {
    const result = addPlugin(EMPTY_STATE, VALID_RECORD);
    expect(result.plugins).toHaveLength(1);
    expect(result.plugins[0]?.name).toBe("dev-toolkit");
  });

  test("replaces existing plugin with same name", () => {
    const initial = addPlugin(EMPTY_STATE, VALID_RECORD);
    const updated = { ...VALID_RECORD, description: "Updated" };
    const result = addPlugin(initial, updated);
    expect(result.plugins).toHaveLength(1);
    expect(result.plugins[0]?.description).toBe("Updated");
  });
});

describe("removePlugin", () => {
  test("removes by name", () => {
    const state = addPlugin(EMPTY_STATE, VALID_RECORD);
    const result = removePlugin(state, "dev-toolkit");
    expect(result.plugins).toHaveLength(0);
  });

  test("returns unchanged state if name not found", () => {
    const state = addPlugin(EMPTY_STATE, VALID_RECORD);
    const result = removePlugin(state, "nonexistent");
    expect(result.plugins).toHaveLength(1);
  });
});

describe("toggleComponent", () => {
  test("flips active on a skill component", () => {
    const state = addPlugin(EMPTY_STATE, VALID_RECORD);
    const result = toggleComponent(state, "dev-toolkit", "skill", "code-review");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const comp = result.value.plugins[0]?.components.find(
      (c) => c.type === "skill" && c.name === "code-review",
    );
    expect(comp?.active).toBe(false);
  });

  test("flips active on an mcp component", () => {
    const state = addPlugin(EMPTY_STATE, VALID_RECORD);
    const result = toggleComponent(state, "dev-toolkit", "mcp", "database");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const comp = result.value.plugins[0]?.components.find(
      (c) => c.type === "mcp" && c.name === "database",
    );
    expect(comp?.active).toBe(false);
  });

  test("flips active on an agent component", () => {
    const state = addPlugin(EMPTY_STATE, VALID_RECORD);
    const result = toggleComponent(state, "dev-toolkit", "agent", "reviewer");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const comp = result.value.plugins[0]?.components.find(
      (c) => c.type === "agent" && c.name === "reviewer",
    );
    expect(comp?.active).toBe(false);
  });

  test("updates updatedAt timestamp", () => {
    const before = new Date().toISOString();
    const state = addPlugin(EMPTY_STATE, VALID_RECORD);
    const result = toggleComponent(state, "dev-toolkit", "skill", "code-review");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const plugin = result.value.plugins[0];
    expect(plugin?.updatedAt >= before).toBe(true);
  });

  test("returns error if plugin not found", () => {
    const result = toggleComponent(EMPTY_STATE, "nonexistent", "skill", "foo");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("nonexistent");
  });

  test("returns error if component not found", () => {
    const state = addPlugin(EMPTY_STATE, VALID_RECORD);
    const result = toggleComponent(state, "dev-toolkit", "skill", "nonexistent");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("nonexistent");
  });
});

describe("findPlugin", () => {
  test("returns plugin record by name", () => {
    const state = addPlugin(EMPTY_STATE, VALID_RECORD);
    const found = findPlugin(state, "dev-toolkit");
    expect(found).toBeDefined();
    expect(found?.name).toBe("dev-toolkit");
  });

  test("returns undefined if not found", () => {
    const found = findPlugin(EMPTY_STATE, "nonexistent");
    expect(found).toBeUndefined();
  });
});

describe("manifestToRecord", () => {
  const META: PluginInstallMeta = {
    repo: "https://github.com/nklisch/dev-toolkit",
    ref: "main",
    sha: "abc123",
    scope: "global",
    also: ["cursor"],
    tap: null,
  };

  const MANIFEST: PluginManifest = {
    name: "dev-toolkit",
    description: "Dev tools",
    format: "claude-code",
    pluginRoot: "/tmp/dev-toolkit",
    components: [
      { type: "skill", name: "code-review", path: ".claude/skills/code-review", description: "" },
      {
        type: "mcp",
        server: {
          type: "stdio",
          name: "database",
          command: "npx",
          args: ["-y", "@corp/db-mcp"],
          env: {},
        },
      },
      {
        type: "mcp",
        server: {
          type: "http",
          name: "remote-api",
          url: "https://api.example.com/mcp",
        },
      },
      {
        type: "agent",
        name: "reviewer",
        path: ".claude/agents/reviewer.md",
        frontmatter: {},
      },
    ],
  };

  test("converts manifest with skills, mcp, and agents", () => {
    const record = manifestToRecord(MANIFEST, META);
    // HTTP MCP server is skipped, so 3 components: skill, mcp(stdio), agent
    expect(record.components).toHaveLength(3);
  });

  test("sets correct format and name", () => {
    const record = manifestToRecord(MANIFEST, META);
    expect(record.name).toBe("dev-toolkit");
    expect(record.format).toBe("claude-code");
  });

  test("skips HTTP MCP servers", () => {
    const record = manifestToRecord(MANIFEST, META);
    const httpComp = record.components.find(
      (c) => c.type === "mcp" && c.name === "remote-api",
    );
    expect(httpComp).toBeUndefined();
  });

  test("converts stdio MCP servers correctly", () => {
    const record = manifestToRecord(MANIFEST, META);
    const mcp = record.components.find((c) => c.type === "mcp");
    expect(mcp).toBeDefined();
    if (!mcp || mcp.type !== "mcp") return;
    expect(mcp.name).toBe("database");
    expect(mcp.command).toBe("npx");
    expect(mcp.args).toEqual(["-y", "@corp/db-mcp"]);
  });

  test("converts agents correctly", () => {
    const record = manifestToRecord(MANIFEST, META);
    const agent = record.components.find((c) => c.type === "agent");
    expect(agent).toBeDefined();
    if (!agent || agent.type !== "agent") return;
    expect(agent.name).toBe("reviewer");
    expect(agent.platform).toBe("claude-code");
  });

  test("sets installedAt and updatedAt to current time", () => {
    const before = new Date().toISOString();
    const record = manifestToRecord(MANIFEST, META);
    const after = new Date().toISOString();
    expect(record.installedAt >= before).toBe(true);
    expect(record.installedAt <= after).toBe(true);
    expect(record.updatedAt >= before).toBe(true);
    expect(record.installedAt).toBe(record.updatedAt);
  });

  test("applies meta fields", () => {
    const record = manifestToRecord(MANIFEST, META);
    expect(record.repo).toBe("https://github.com/nklisch/dev-toolkit");
    expect(record.ref).toBe("main");
    expect(record.sha).toBe("abc123");
    expect(record.scope).toBe("global");
    expect(record.also).toEqual(["cursor"]);
    expect(record.tap).toBeNull();
  });
});
