import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, pathExists, type TestEnv } from "@skilltap/test-utils";
import { detectPlugin } from "./detect";
import { installPlugin } from "./install";
import { removeInstalledPlugin, toggleInstalledComponent } from "./lifecycle";
import { loadPlugins } from "./state";

setDefaultTimeout(30_000);

let env: TestEnv;

beforeEach(async () => { env = await createTestEnv(); });
afterEach(async () => { await env.cleanup(); });

async function createPluginDir(): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const dir = await mkdtemp(join(tmpdir(), "skilltap-plugin-src-"));

  // Plugin manifest
  await mkdir(join(dir, ".claude-plugin"), { recursive: true });
  await Bun.write(
    join(dir, ".claude-plugin", "plugin.json"),
    JSON.stringify({ name: "lifecycle-plugin", description: "E2e lifecycle test plugin" }),
  );

  // One skill
  await mkdir(join(dir, "skills", "helper"), { recursive: true });
  await Bun.write(
    join(dir, "skills", "helper", "SKILL.md"),
    "---\nname: helper\ndescription: A helper skill\n---\n# Helper\nHelp content.\n",
  );

  // One MCP server
  await Bun.write(
    join(dir, ".mcp.json"),
    JSON.stringify({ "lifecycle-db": { command: "npx", args: ["-y", "lifecycle-mcp"] } }),
  );

  // One agent definition
  await mkdir(join(dir, "agents"), { recursive: true });
  await Bun.write(
    join(dir, "agents", "reviewer.md"),
    "---\nname: reviewer\ndescription: Code reviewer\nmodel: sonnet\n---\nYou are a reviewer.",
  );

  return {
    path: dir,
    cleanup: () => rm(dir, { recursive: true, force: true }),
  };
}

describe("plugin lifecycle e2e", () => {
  test("install → verify state → toggle skill off → toggle skill on → remove → verify clean", async () => {
    const plugin = await createPluginDir();
    try {
      // -----------------------------------------------------------------------
      // Step 1: Detect
      // -----------------------------------------------------------------------
      const detectResult = await detectPlugin(plugin.path);
      expect(detectResult.ok).toBe(true);
      if (!detectResult.ok) return;
      const manifest = detectResult.value!;
      expect(manifest).not.toBeNull();
      expect(manifest.name).toBe("lifecycle-plugin");
      expect(manifest.format).toBe("claude-code");

      // -----------------------------------------------------------------------
      // Step 2: Install
      // -----------------------------------------------------------------------
      const installResult = await installPlugin(plugin.path, manifest, {
        scope: "global",
        also: ["claude-code"],
        skipScan: true,
        repo: null,
        ref: null,
        sha: null,
        tap: null,
      });
      expect(installResult.ok).toBe(true);
      if (!installResult.ok) return;

      const { record, mcpAgents, agentDefsPlaced } = installResult.value;
      expect(record.name).toBe("lifecycle-plugin");
      expect(record.scope).toBe("global");
      expect(record.also).toContain("claude-code");
      expect(agentDefsPlaced).toBe(1);

      // -----------------------------------------------------------------------
      // Step 3: Verify filesystem state after install
      // -----------------------------------------------------------------------
      const skillDir = join(env.homeDir,".agents", "skills", "helper");
      expect(await pathExists(skillDir)).toBe(true);
      expect(await pathExists(join(skillDir, "SKILL.md"))).toBe(true);

      // Agent definition placed
      const agentFile = join(env.homeDir,".claude", "agents", "reviewer.md");
      expect(await pathExists(agentFile)).toBe(true);

      // plugins.json has record
      const pluginsResult = await loadPlugins();
      expect(pluginsResult.ok).toBe(true);
      if (!pluginsResult.ok) return;
      expect(pluginsResult.value.plugins).toHaveLength(1);
      expect(pluginsResult.value.plugins[0]?.name).toBe("lifecycle-plugin");
      expect(pluginsResult.value.plugins[0]?.active).toBe(true);

      // MCP injected into claude-code settings
      const mcpConfig = join(env.homeDir,".claude", "settings.json");
      if (mcpAgents.length > 0) {
        expect(await pathExists(mcpConfig)).toBe(true);
        const settings = await Bun.file(mcpConfig).json();
        expect(settings.mcpServers).toBeDefined();
        const keys = Object.keys(settings.mcpServers as object);
        expect(keys.some((k) => k.includes("lifecycle-plugin"))).toBe(true);
      }

      // -----------------------------------------------------------------------
      // Step 4: Toggle skill off
      // -----------------------------------------------------------------------
      const toggleOffResult = await toggleInstalledComponent(
        "lifecycle-plugin",
        "skill",
        "helper",
      );
      expect(toggleOffResult.ok).toBe(true);
      if (!toggleOffResult.ok) return;
      expect(toggleOffResult.value.nowActive).toBe(false);

      // Skill should now be in .disabled/
      const disabledDir = join(env.homeDir,".agents", "skills", ".disabled", "helper");
      expect(await pathExists(disabledDir)).toBe(true);
      expect(await pathExists(skillDir)).toBe(false);

      // plugins.json reflects disabled state
      const afterToggleOff = await loadPlugins();
      expect(afterToggleOff.ok).toBe(true);
      if (!afterToggleOff.ok) return;
      const skillComp = afterToggleOff.value.plugins[0]?.components.find(
        (c) => c.type === "skill" && c.name === "helper",
      );
      expect(skillComp?.active).toBe(false);

      // -----------------------------------------------------------------------
      // Step 5: Toggle skill back on
      // -----------------------------------------------------------------------
      const toggleOnResult = await toggleInstalledComponent(
        "lifecycle-plugin",
        "skill",
        "helper",
      );
      expect(toggleOnResult.ok).toBe(true);
      if (!toggleOnResult.ok) return;
      expect(toggleOnResult.value.nowActive).toBe(true);

      // Skill should be back in active location
      expect(await pathExists(skillDir)).toBe(true);
      expect(await pathExists(disabledDir)).toBe(false);

      // plugins.json reflects active state again
      const afterToggleOn = await loadPlugins();
      expect(afterToggleOn.ok).toBe(true);
      if (!afterToggleOn.ok) return;
      const skillCompOn = afterToggleOn.value.plugins[0]?.components.find(
        (c) => c.type === "skill" && c.name === "helper",
      );
      expect(skillCompOn?.active).toBe(true);

      // -----------------------------------------------------------------------
      // Step 6: Remove plugin
      // -----------------------------------------------------------------------
      const removeResult = await removeInstalledPlugin("lifecycle-plugin");
      expect(removeResult.ok).toBe(true);
      if (!removeResult.ok) return;
      expect(removeResult.value.name).toBe("lifecycle-plugin");

      // All filesystem artifacts gone
      expect(await pathExists(skillDir)).toBe(false);
      expect(await pathExists(disabledDir)).toBe(false);
      expect(await pathExists(agentFile)).toBe(false);

      // plugins.json is empty
      const afterRemove = await loadPlugins();
      expect(afterRemove.ok).toBe(true);
      if (!afterRemove.ok) return;
      expect(afterRemove.value.plugins).toHaveLength(0);
    } finally {
      await plugin.cleanup();
    }
  });
});
