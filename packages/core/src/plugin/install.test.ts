import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { installPlugin, type PluginInstallOptions } from "./install";
import type { PluginManifest } from "../schemas/plugin";

let env: TestEnv;

beforeEach(async () => { env = await createTestEnv(); });
afterEach(async () => { await env.cleanup(); });

async function makeContentDir(structure: Record<string, string>): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), "skilltap-content-"));
  for (const [relPath, content] of Object.entries(structure)) {
    const fullPath = join(dir, relPath);
    await mkdir(fullPath.slice(0, fullPath.lastIndexOf("/")), { recursive: true });
    await Bun.write(fullPath, content);
  }
  return dir;
}

const BASE_OPTIONS: PluginInstallOptions = {
  scope: "global",
  also: [],
  skipScan: true,
  repo: "https://github.com/test/test-plugin",
  ref: "main",
  sha: "abc123",
  tap: null,
};

const SKILL_MANIFEST: PluginManifest = {
  name: "test-plugin",
  description: "A test plugin",
  format: "claude-code",
  pluginRoot: ".claude-plugin",
  components: [
    { type: "skill", name: "helper", path: "skills/helper", description: "" },
  ],
};

const FULL_MANIFEST: PluginManifest = {
  name: "test-plugin",
  description: "A test plugin",
  format: "claude-code",
  pluginRoot: ".claude-plugin",
  components: [
    { type: "skill", name: "helper", path: "skills/helper", description: "" },
    {
      type: "mcp",
      server: { type: "stdio", name: "test-db", command: "npx", args: ["-y", "test-mcp"], env: {} },
    },
    { type: "agent", name: "reviewer", path: "agents/reviewer.md", frontmatter: {} },
  ],
};

describe("installPlugin", () => {
  test("places skills in .agents/skills/ with correct name", async () => {
    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, BASE_OPTIONS);
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      const skillDir = join(env.homeDir,".agents", "skills", "helper");
      const skillFile = Bun.file(join(skillDir, "SKILL.md"));
      expect(await skillFile.exists()).toBe(true);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("creates agent symlinks for skills when also specified", async () => {
    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, {
        ...BASE_OPTIONS,
        also: ["claude-code"],
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      const symlinkPath = join(env.homeDir,".claude", "skills", "helper");
      const stat = await Bun.file(symlinkPath + "/SKILL.md").exists();
      expect(stat).toBe(true);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("injects MCP servers into agent config files", async () => {
    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
      "agents/reviewer.md": "# Reviewer",
    });
    try {
      const result = await installPlugin(contentDir, FULL_MANIFEST, {
        ...BASE_OPTIONS,
        also: ["claude-code"],
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.mcpAgents).toContain("claude-code");

      const configPath = join(env.homeDir,".claude", "settings.json");
      const config = await Bun.file(configPath).json();
      expect(config.mcpServers).toBeDefined();
      expect(config.mcpServers["skilltap:test-plugin:test-db"]).toBeDefined();
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("places agent .md files in .claude/agents/", async () => {
    const contentDir = await makeContentDir({
      "agents/reviewer.md": "# Reviewer agent content",
    });
    const manifest: PluginManifest = {
      name: "test-plugin",
      description: "",
      format: "claude-code",
      pluginRoot: ".claude-plugin",
      components: [
        { type: "agent", name: "reviewer", path: "agents/reviewer.md", frontmatter: {} },
      ],
    };
    try {
      const result = await installPlugin(contentDir, manifest, BASE_OPTIONS);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.agentDefsPlaced).toBe(1);

      const agentFile = Bun.file(join(env.homeDir,".claude", "agents", "reviewer.md"));
      expect(await agentFile.exists()).toBe(true);
      expect(await agentFile.text()).toBe("# Reviewer agent content");
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("creates .claude/agents/ directory when missing", async () => {
    const contentDir = await makeContentDir({
      "agents/new-agent.md": "# New agent",
    });
    const manifest: PluginManifest = {
      name: "test-plugin",
      description: "",
      format: "claude-code",
      pluginRoot: ".claude-plugin",
      components: [
        { type: "agent", name: "new-agent", path: "agents/new-agent.md", frontmatter: {} },
      ],
    };
    try {
      const result = await installPlugin(contentDir, manifest, BASE_OPTIONS);
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      const agentFile = Bun.file(join(env.homeDir,".claude", "agents", "new-agent.md"));
      expect(await agentFile.exists()).toBe(true);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("records plugin in plugins.json", async () => {
    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, BASE_OPTIONS);
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      // Global plugins.json lives in configDir (XDG_CONFIG_HOME/skilltap/plugins.json)
      const { getConfigDir } = await import("../config");
      const pluginsFile = Bun.file(join(getConfigDir(), "plugins.json"));
      expect(await pluginsFile.exists()).toBe(true);
      const plugins = await pluginsFile.json();
      expect(plugins.plugins).toHaveLength(1);
      expect(plugins.plugins[0].name).toBe("test-plugin");
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("does not write to installed.json", async () => {
    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, BASE_OPTIONS);
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      const installedFile = Bun.file(join(env.homeDir,".agents", "installed.json"));
      expect(await installedFile.exists()).toBe(false);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("skips security scan when skipScan=true", async () => {
    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    let scanCalled = false;
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, {
        ...BASE_OPTIONS,
        skipScan: true,
        onWarnings: async () => {
          scanCalled = true;
          return true;
        },
      });
      expect(result.ok).toBe(true);
      expect(scanCalled).toBe(false);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("aborts when onWarnings returns false", async () => {
    // Create content with invisible unicode to trigger scanner
    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper\u200bContent",
    });
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, {
        ...BASE_OPTIONS,
        skipScan: false,
        onWarnings: async () => false,
      });
      expect(result.ok).toBe(false);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("handles plugin with only skills (no MCP, no agents)", async () => {
    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, BASE_OPTIONS);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.mcpAgents).toHaveLength(0);
      expect(result.value.agentDefsPlaced).toBe(0);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("handles plugin with only MCP servers", async () => {
    const contentDir = await makeContentDir({
      "placeholder.txt": "no skills or agents",
    });
    const manifest: PluginManifest = {
      name: "mcp-only-plugin",
      description: "",
      format: "claude-code",
      pluginRoot: ".claude-plugin",
      components: [
        {
          type: "mcp",
          server: { type: "stdio", name: "my-server", command: "node", args: ["server.js"], env: {} },
        },
      ],
    };
    try {
      const result = await installPlugin(contentDir, manifest, {
        ...BASE_OPTIONS,
        also: ["claude-code"],
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.mcpAgents).toContain("claude-code");
      expect(result.value.agentDefsPlaced).toBe(0);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("handles empty plugin (no components)", async () => {
    const contentDir = await makeContentDir({
      "placeholder.txt": "empty plugin",
    });
    const manifest: PluginManifest = {
      name: "empty-plugin",
      description: "",
      format: "claude-code",
      pluginRoot: ".claude-plugin",
      components: [],
    };
    try {
      const result = await installPlugin(contentDir, manifest, BASE_OPTIONS);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.mcpAgents).toHaveLength(0);
      expect(result.value.agentDefsPlaced).toBe(0);
      expect(result.value.record.name).toBe("empty-plugin");
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("returns correct mcpAgents list", async () => {
    const contentDir = await makeContentDir({
      "placeholder.txt": "mcp plugin",
    });
    const manifest: PluginManifest = {
      name: "test-plugin",
      description: "",
      format: "claude-code",
      pluginRoot: ".claude-plugin",
      components: [
        {
          type: "mcp",
          server: { type: "stdio", name: "svc", command: "bun", args: ["run", "server.ts"], env: {} },
        },
      ],
    };
    try {
      const result = await installPlugin(contentDir, manifest, {
        ...BASE_OPTIONS,
        also: ["claude-code", "cursor"],
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.mcpAgents).toContain("claude-code");
      expect(result.value.mcpAgents).toContain("cursor");
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("returns correct agentDefsPlaced count", async () => {
    const contentDir = await makeContentDir({
      "agents/agent1.md": "# Agent 1",
      "agents/agent2.md": "# Agent 2",
    });
    const manifest: PluginManifest = {
      name: "multi-agent-plugin",
      description: "",
      format: "claude-code",
      pluginRoot: ".claude-plugin",
      components: [
        { type: "agent", name: "agent1", path: "agents/agent1.md", frontmatter: {} },
        { type: "agent", name: "agent2", path: "agents/agent2.md", frontmatter: {} },
      ],
    };
    try {
      const result = await installPlugin(contentDir, manifest, BASE_OPTIONS);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.agentDefsPlaced).toBe(2);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("injects HTTP MCP servers as url entries", async () => {
    const contentDir = await makeContentDir({
      "placeholder.txt": "http server plugin",
    });
    const manifest: PluginManifest = {
      name: "http-plugin",
      description: "",
      format: "claude-code",
      pluginRoot: ".claude-plugin",
      components: [
        {
          type: "mcp",
          server: { type: "http", name: "remote-svc", url: "https://example.com/mcp", headers: {} },
        },
      ],
    };
    try {
      const result = await installPlugin(contentDir, manifest, {
        ...BASE_OPTIONS,
        also: ["claude-code"],
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.mcpAgents).toContain("claude-code");

      const { getConfigDir } = await import("../config");
      const { join } = await import("node:path");
      const configPath = join(env.homeDir, ".claude", "settings.json");
      const config = await Bun.file(configPath).json();
      const entry = config.mcpServers["skilltap:http-plugin:remote-svc"];
      expect(entry).toBeDefined();
      expect(entry.url).toBe("https://example.com/mcp");
      expect(entry.command).toBeUndefined();
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });
});
