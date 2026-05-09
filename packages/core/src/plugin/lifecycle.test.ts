import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { createTestEnv, pathExists, type TestEnv } from "@skilltap/test-utils";
import type { PluginRecord } from "../schemas/plugins";
import { removeInstalledPlugin, toggleInstalledComponent } from "./lifecycle";
import { savePlugins } from "./state";

let env: TestEnv;

beforeEach(async () => {
  env = await createTestEnv();
});
afterEach(async () => {
  await env.cleanup();
});

const NOW = new Date().toISOString();

async function setupPlugin(plugins: PluginRecord[]): Promise<void> {
  const result = await savePlugins(plugins);
  expect(result.ok).toBe(true);
}

async function setupSkillDir(name: string): Promise<string> {
  const dir = join(env.homeDir, ".agents", "skills", name);
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "SKILL.md"), `---\nname: ${name}\n---\n# ${name}`);
  return dir;
}

const BASE_PLUGIN: PluginRecord = {
  name: "my-plugin",
  description: "A test plugin",
  format: "claude-code",
  repo: "https://github.com/test/my-plugin",
  ref: "main",
  sha: "abc123",
  scope: "global",
  also: [],
  tap: null,
  components: [{ type: "skill", name: "helper", active: true }],
  installedAt: NOW,
  updatedAt: NOW,
  active: true,
};

const BASE_STATE: PluginRecord[] = [BASE_PLUGIN];

describe("removeInstalledPlugin", () => {
  test("removes skill directories", async () => {
    await setupPlugin(BASE_STATE);
    const skillDir = await setupSkillDir("helper");

    const result = await removeInstalledPlugin("my-plugin");
    expect(result.ok).toBe(true);

    expect(await pathExists(skillDir)).toBe(false);
  });

  test("removes plugin from plugins.json", async () => {
    await setupPlugin(BASE_STATE);
    await setupSkillDir("helper");

    const result = await removeInstalledPlugin("my-plugin");
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.name).toBe("my-plugin");

    const { loadPlugins } = await import("./state");
    const reloaded = await loadPlugins();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value).toHaveLength(0);
  });

  test("returns error if plugin not found", async () => {
    await setupPlugin([]);

    const result = await removeInstalledPlugin("nonexistent");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("nonexistent");
  });

  test("handles disabled skills in .disabled/", async () => {
    const disabledState: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          components: [{ type: "skill", name: "helper", active: false }],
        },
    ];
    await setupPlugin(disabledState);

    // Create the skill in the disabled dir
    const disabledDir = join(
      env.homeDir,
      ".agents",
      "skills",
      ".disabled",
      "helper",
    );
    await mkdir(disabledDir, { recursive: true });
    await Bun.write(
      join(disabledDir, "SKILL.md"),
      "---\nname: helper\n---\n# helper",
    );

    const result = await removeInstalledPlugin("my-plugin");
    expect(result.ok).toBe(true);

    expect(await pathExists(disabledDir)).toBe(false);
  });

  test("removes agent definition files", async () => {
    const stateWithAgent: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          components: [
            {
              type: "agent",
              name: "reviewer",
              active: true,
              platform: "claude-code",
            },
          ],
        },
    ];
    await setupPlugin(stateWithAgent);

    const agentFile = join(env.homeDir, ".claude", "agents", "reviewer.md");
    await mkdir(join(env.homeDir, ".claude", "agents"), { recursive: true });
    await Bun.write(agentFile, "# Reviewer Agent");

    const result = await removeInstalledPlugin("my-plugin");
    expect(result.ok).toBe(true);

    expect(await pathExists(agentFile)).toBe(false);
  });

  test("handles plugin with only skills (no MCP or agents)", async () => {
    await setupPlugin(BASE_STATE);
    await setupSkillDir("helper");

    const result = await removeInstalledPlugin("my-plugin");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.name).toBe("my-plugin");
  });

  test("removes MCP entries from agent configs", async () => {
    const stateWithMcp: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          also: ["claude-code"],
          components: [
            {
              type: "mcp",
              serverType: "stdio",
              name: "test-server",
              active: true,
              command: "node",
              args: ["server.js"],
              env: {},
            },
          ],
        },
    ];
    await setupPlugin(stateWithMcp);

    // Set up the MCP config
    const configPath = join(env.homeDir, ".claude", "settings.json");
    await mkdir(join(env.homeDir, ".claude"), { recursive: true });
    await Bun.write(
      configPath,
      JSON.stringify({
        mcpServers: {
          "skilltap:my-plugin:test-server": {
            command: "node",
            args: ["server.js"],
          },
        },
      }),
    );

    const result = await removeInstalledPlugin("my-plugin");
    expect(result.ok).toBe(true);

    const config = await Bun.file(configPath).json();
    expect(config.mcpServers["skilltap:my-plugin:test-server"]).toBeUndefined();
  });
});

describe("toggleInstalledComponent", () => {
  test("deactivates a skill (moves to .disabled/, returns nowActive=false)", async () => {
    await setupPlugin(BASE_STATE);
    const activeDir = await setupSkillDir("helper");

    const result = await toggleInstalledComponent(
      "my-plugin",
      "skill",
      "helper",
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.nowActive).toBe(false);
    expect(await pathExists(activeDir)).toBe(false);

    const disabledDir = join(
      env.homeDir,
      ".agents",
      "skills",
      ".disabled",
      "helper",
    );
    expect(await pathExists(disabledDir)).toBe(true);
  });

  test("activates a skill (moves from .disabled/, returns nowActive=true)", async () => {
    const disabledState: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          components: [{ type: "skill", name: "helper", active: false }],
        },
    ];
    await setupPlugin(disabledState);

    // Place in disabled dir
    const disabledDir = join(
      env.homeDir,
      ".agents",
      "skills",
      ".disabled",
      "helper",
    );
    await mkdir(disabledDir, { recursive: true });
    await Bun.write(
      join(disabledDir, "SKILL.md"),
      "---\nname: helper\n---\n# helper",
    );

    const result = await toggleInstalledComponent(
      "my-plugin",
      "skill",
      "helper",
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.nowActive).toBe(true);
    expect(await pathExists(disabledDir)).toBe(false);

    const activeDir = join(env.homeDir, ".agents", "skills", "helper");
    expect(await pathExists(activeDir)).toBe(true);
  });

  test("deactivates an MCP server (removes from agent configs)", async () => {
    const stateWithMcp: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          also: ["claude-code"],
          components: [
            {
              type: "mcp",
              serverType: "stdio",
              name: "test-server",
              active: true,
              command: "node",
              args: ["server.js"],
              env: {},
            },
          ],
        },
    ];
    await setupPlugin(stateWithMcp);

    const configPath = join(env.homeDir, ".claude", "settings.json");
    await mkdir(join(env.homeDir, ".claude"), { recursive: true });
    await Bun.write(
      configPath,
      JSON.stringify({
        mcpServers: {
          "skilltap:my-plugin:test-server": {
            command: "node",
            args: ["server.js"],
          },
        },
      }),
    );

    const result = await toggleInstalledComponent(
      "my-plugin",
      "mcp",
      "test-server",
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.nowActive).toBe(false);

    const config = await Bun.file(configPath).json();
    expect(config.mcpServers["skilltap:my-plugin:test-server"]).toBeUndefined();
  });

  test("activates an MCP server (injects into agent configs)", async () => {
    const stateWithMcp: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          also: ["claude-code"],
          components: [
            {
              type: "mcp",
              serverType: "stdio",
              name: "test-server",
              active: false,
              command: "node",
              args: ["server.js"],
              env: {},
            },
          ],
        },
    ];
    await setupPlugin(stateWithMcp);

    const configPath = join(env.homeDir, ".claude", "settings.json");
    await mkdir(join(env.homeDir, ".claude"), { recursive: true });
    await Bun.write(configPath, JSON.stringify({ mcpServers: {} }));

    const result = await toggleInstalledComponent(
      "my-plugin",
      "mcp",
      "test-server",
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.nowActive).toBe(true);
    expect(result.value.mcpAgents).toContain("claude-code");

    const config = await Bun.file(configPath).json();
    expect(config.mcpServers["skilltap:my-plugin:test-server"]).toBeDefined();
  });

  test("deactivates an agent (moves to .disabled/)", async () => {
    const stateWithAgent: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          components: [
            {
              type: "agent",
              name: "reviewer",
              active: true,
              platform: "claude-code",
            },
          ],
        },
    ];
    await setupPlugin(stateWithAgent);

    const agentFile = join(env.homeDir, ".claude", "agents", "reviewer.md");
    await mkdir(join(env.homeDir, ".claude", "agents"), { recursive: true });
    await Bun.write(agentFile, "# Reviewer Agent");

    const result = await toggleInstalledComponent(
      "my-plugin",
      "agent",
      "reviewer",
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.nowActive).toBe(false);
    expect(await pathExists(agentFile)).toBe(false);

    const disabledPath = join(
      env.homeDir,
      ".claude",
      "agents",
      ".disabled",
      "reviewer.md",
    );
    expect(await pathExists(disabledPath)).toBe(true);
  });

  test("activates an agent (moves from .disabled/ back)", async () => {
    const stateWithAgent: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          components: [
            {
              type: "agent",
              name: "reviewer",
              active: false,
              platform: "claude-code",
            },
          ],
        },
    ];
    await setupPlugin(stateWithAgent);

    const disabledPath = join(
      env.homeDir,
      ".claude",
      "agents",
      ".disabled",
      "reviewer.md",
    );
    await mkdir(join(env.homeDir, ".claude", "agents", ".disabled"), {
      recursive: true,
    });
    await Bun.write(disabledPath, "# Reviewer Agent");

    const result = await toggleInstalledComponent(
      "my-plugin",
      "agent",
      "reviewer",
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.nowActive).toBe(true);
    expect(await pathExists(disabledPath)).toBe(false);

    const activePath = join(env.homeDir, ".claude", "agents", "reviewer.md");
    expect(await pathExists(activePath)).toBe(true);
  });

  test("updates plugins.json state", async () => {
    await setupPlugin(BASE_STATE);
    await setupSkillDir("helper");

    const result = await toggleInstalledComponent(
      "my-plugin",
      "skill",
      "helper",
    );
    expect(result.ok).toBe(true);

    const { loadPlugins } = await import("./state");
    const reloaded = await loadPlugins();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;

    const plugin = reloaded.value.find((p) => p.name === "my-plugin");
    expect(plugin).toBeDefined();
    const comp = plugin!.components.find((c) => c.name === "helper");
    expect(comp?.active).toBe(false);
  });

  test("returns error if plugin not found", async () => {
    await setupPlugin([]);

    const result = await toggleInstalledComponent(
      "nonexistent",
      "skill",
      "helper",
    );
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("nonexistent");
  });

  test("returns error if component not found", async () => {
    await setupPlugin(BASE_STATE);

    const result = await toggleInstalledComponent(
      "my-plugin",
      "skill",
      "no-such-component",
    );
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("no-such-component");
  });

  test("toggle HTTP MCP component off removes from agent config", async () => {
    const stateWithHttpMcp: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          also: ["claude-code"],
          components: [
            {
              type: "mcp",
              serverType: "http",
              name: "remote-api",
              active: true,
              url: "https://api.example.com/mcp",
              headers: {},
            },
          ],
        },
    ];
    await setupPlugin(stateWithHttpMcp);

    const configPath = join(env.homeDir, ".claude", "settings.json");
    await mkdir(join(env.homeDir, ".claude"), { recursive: true });
    await Bun.write(
      configPath,
      JSON.stringify({
        mcpServers: {
          "skilltap:my-plugin:remote-api": {
            url: "https://api.example.com/mcp",
          },
        },
      }),
    );

    const result = await toggleInstalledComponent(
      "my-plugin",
      "mcp",
      "remote-api",
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.nowActive).toBe(false);

    const config = await Bun.file(configPath).json();
    expect(config.mcpServers["skilltap:my-plugin:remote-api"]).toBeUndefined();
  });

  test("toggle HTTP MCP component on re-injects { url } entry", async () => {
    const stateWithHttpMcp: PluginRecord[] = [
        {
          ...BASE_PLUGIN,
          also: ["claude-code"],
          components: [
            {
              type: "mcp",
              serverType: "http",
              name: "remote-api",
              active: false,
              url: "https://api.example.com/mcp",
              headers: {},
            },
          ],
        },
    ];
    await setupPlugin(stateWithHttpMcp);

    const configPath = join(env.homeDir, ".claude", "settings.json");
    await mkdir(join(env.homeDir, ".claude"), { recursive: true });
    await Bun.write(configPath, JSON.stringify({ mcpServers: {} }));

    const result = await toggleInstalledComponent(
      "my-plugin",
      "mcp",
      "remote-api",
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.nowActive).toBe(true);
    expect(result.value.mcpAgents).toContain("claude-code");

    const config = await Bun.file(configPath).json();
    const entry = config.mcpServers["skilltap:my-plugin:remote-api"];
    expect(entry).toBeDefined();
    expect(entry.url).toBe("https://api.example.com/mcp");
    expect(entry.command).toBeUndefined();
  });
});
