import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  commitAll,
  createTapWithPlugins,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { getConfigDir, loadConfig } from "../config";
import { installSkill } from "../install";
import { loadPlugins } from "./state";
import { addTap, loadTaps, searchTaps, tapDir } from "../taps";

type Env = {
  SKILLTAP_HOME?: string;
  XDG_CONFIG_HOME?: string;
};

let savedEnv: Env;
let homeDir: string;
let configDir: string;

beforeEach(async () => {
  savedEnv = {
    SKILLTAP_HOME: process.env.SKILLTAP_HOME,
    XDG_CONFIG_HOME: process.env.XDG_CONFIG_HOME,
  };
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  if (savedEnv.SKILLTAP_HOME === undefined) delete process.env.SKILLTAP_HOME;
  else process.env.SKILLTAP_HOME = savedEnv.SKILLTAP_HOME;
  if (savedEnv.XDG_CONFIG_HOME === undefined) delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = savedEnv.XDG_CONFIG_HOME;
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

/** Clone a fixture tap repo into the config taps directory, mimicking what addTap does. */
async function cloneTapToConfig(tapName: string, sourcePath: string): Promise<void> {
  const { $ } = await import("bun");
  const dest = tapDir(tapName);
  await mkdir(dest, { recursive: true });
  await $`git clone --depth=1 ${sourcePath} ${dest}`.quiet();
}

// ─── Integration tests: tap plugin install via installSkill ──────────────────

describe("tap plugin install flow", () => {
  test("install tap-name/plugin-name resolves and installs tap plugin", async () => {
    const fixture = await createTapWithPlugins();
    try {
      // Register the tap in config and clone it to the taps dir
      await addTap("test-tap-plugins", fixture.path);

      // The plugin's skill dir must exist in the cloned tap directory
      const clonedTapDir = tapDir("test-tap-plugins");
      const skillSrc = join(clonedTapDir, "plugins/dev-toolkit/skills/code-review");
      await mkdir(skillSrc, { recursive: true });
      await Bun.write(
        join(skillSrc, "SKILL.md"),
        "---\nname: code-review\ndescription: Code review skill from tap plugin\n---\n# Code Review\nReview instructions.\n",
      );
      // Agent file
      const agentDir = join(clonedTapDir, "plugins/dev-toolkit/agents");
      await mkdir(agentDir, { recursive: true });
      await Bun.write(
        join(agentDir, "reviewer.md"),
        "---\nname: reviewer\ndescription: Code reviewer agent\nmodel: sonnet\n---\nYou are a code reviewer.\n",
      );

      const result = await installSkill("test-tap-plugins/dev-toolkit", {
        scope: "global",
        skipScan: true,
        also: ["claude-code"],
      });

      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.pluginRecord).toBeDefined();
      expect(result.value.pluginRecord?.name).toBe("dev-toolkit");
      expect(result.value.pluginRecord?.tap).toBe("test-tap-plugins");
    } finally {
      await fixture.cleanup();
    }
  });

  test("falls through to normal resolution when no taps are configured with that name", async () => {
    // "nonexistent-tap/some-plugin" — no tap named "nonexistent-tap" configured.
    // loadTaps() returns empty, so parseTapPluginRef finds no match and falls through.
    // resolveSource will treat it as GitHub shorthand; git clone will fail — confirm it's a git/network error.
    const result = await installSkill("nonexistent-tap/some-plugin", {
      scope: "global",
      skipScan: true,
      // Supply a local gitHost that immediately fails, avoiding real network calls
      gitHost: "http://localhost:1",
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    // Should be a git or network error — not a tap plugin error
    expect(result.error.message).not.toContain("tap plugin");
  });

  test("pluginRecord recorded in plugins.json with tap field", async () => {
    const fixture = await createTapWithPlugins();
    try {
      await addTap("test-tap-plugins", fixture.path);

      const clonedTapDir = tapDir("test-tap-plugins");
      const skillSrc = join(clonedTapDir, "plugins/dev-toolkit/skills/code-review");
      await mkdir(skillSrc, { recursive: true });
      await Bun.write(
        join(skillSrc, "SKILL.md"),
        "---\nname: code-review\ndescription: Code review skill\n---\n# Code Review\n",
      );
      const agentDir = join(clonedTapDir, "plugins/dev-toolkit/agents");
      await mkdir(agentDir, { recursive: true });
      await Bun.write(
        join(agentDir, "reviewer.md"),
        "---\nname: reviewer\n---\nYou are a reviewer.\n",
      );

      const installResult = await installSkill("test-tap-plugins/dev-toolkit", {
        scope: "global",
        skipScan: true,
        also: ["claude-code"],
      });
      expect(installResult.ok).toBe(true);

      const pluginsResult = await loadPlugins(undefined);
      expect(pluginsResult.ok).toBe(true);
      if (!pluginsResult.ok) return;
      const record = pluginsResult.value.plugins.find((p) => p.name === "dev-toolkit");
      expect(record).toBeDefined();
      expect(record?.tap).toBe("test-tap-plugins");
      expect(record?.format).toBe("skilltap");
    } finally {
      await fixture.cleanup();
    }
  });
});

// ─── Unit tests: loadTaps includes plugin entries with correct fields ─────────

describe("loadTaps — plugin entry fields", () => {
  test("plugin entries have plugin:true badge", async () => {
    const fixture = await createTapWithPlugins();
    try {
      await addTap("plugin-tap", fixture.path);
      const result = await loadTaps();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const pluginEntries = result.value.filter((e) => e.tapPlugin !== undefined);
      expect(pluginEntries.length).toBeGreaterThanOrEqual(1);
      for (const entry of pluginEntries) {
        expect(entry.skill.plugin).toBe(true);
      }
    } finally {
      await fixture.cleanup();
    }
  });

  test("searchTaps finds plugins by name", async () => {
    const fixture = await createTapWithPlugins();
    try {
      await addTap("plugin-tap", fixture.path);
      const loadResult = await loadTaps();
      expect(loadResult.ok).toBe(true);
      if (!loadResult.ok) return;
      const results = searchTaps(loadResult.value, "dev-toolkit");
      expect(results.length).toBeGreaterThanOrEqual(1);
      expect(results[0]?.skill.name).toBe("dev-toolkit");
    } finally {
      await fixture.cleanup();
    }
  });

  test("searchTaps finds plugins by tags", async () => {
    const fixture = await createTapWithPlugins();
    try {
      await addTap("plugin-tap", fixture.path);
      const loadResult = await loadTaps();
      expect(loadResult.ok).toBe(true);
      if (!loadResult.ok) return;
      const results = searchTaps(loadResult.value, "dev");
      expect(results.length).toBeGreaterThanOrEqual(1);
    } finally {
      await fixture.cleanup();
    }
  });
});
