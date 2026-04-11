import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { installSkill } from "../install";
import { createClaudePluginRepo, createCodexPluginRepo, createStandaloneSkillRepo } from "@skilltap/test-utils";

let homeDir: string;
let configDir: string;
let savedHome: string | undefined;
let savedXdg: string | undefined;

beforeEach(async () => {
  homeDir = await mkdtemp(join(tmpdir(), "skilltap-test-"));
  configDir = await mkdtemp(join(tmpdir(), "skilltap-cfg-"));
  savedHome = process.env.SKILLTAP_HOME;
  savedXdg = process.env.XDG_CONFIG_HOME;
  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  if (savedHome !== undefined) process.env.SKILLTAP_HOME = savedHome;
  else delete process.env.SKILLTAP_HOME;
  if (savedXdg !== undefined) process.env.XDG_CONFIG_HOME = savedXdg;
  else delete process.env.XDG_CONFIG_HOME;
  await rm(homeDir, { recursive: true, force: true });
  await rm(configDir, { recursive: true, force: true });
});

describe("installSkill with plugin detection", () => {
  test("detects Claude Code plugin and installs via callback", async () => {
    const repo = await createClaudePluginRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        also: ["claude-code"],
        skipScan: true,
        onPluginDetected: async () => "plugin",
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.pluginRecord).toBeDefined();
      expect(result.value.pluginRecord!.name).toBe("test-plugin");
      expect(result.value.records).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("detects Codex plugin and installs via callback", async () => {
    const repo = await createCodexPluginRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        also: ["codex"],
        skipScan: true,
        onPluginDetected: async () => "plugin",
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.pluginRecord).toBeDefined();
      expect(result.value.records).toHaveLength(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("falls through to skill install when callback returns 'skills-only'", async () => {
    const repo = await createClaudePluginRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        also: ["claude-code"],
        skipScan: true,
        onPluginDetected: async () => "skills-only",
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      // Falls through to normal skill scanning — plugin has skills/helper/SKILL.md
      expect(result.value.pluginRecord).toBeUndefined();
      expect(result.value.records.length).toBeGreaterThan(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("cancels when callback returns 'cancel'", async () => {
    const repo = await createClaudePluginRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        skipScan: true,
        onPluginDetected: async () => "cancel",
      });
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("cancelled");
    } finally {
      await repo.cleanup();
    }
  });

  test("normal skill install when no onPluginDetected callback", async () => {
    const repo = await createClaudePluginRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        also: ["claude-code"],
        skipScan: true,
        // No onPluginDetected — plugin detection silently skipped
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.pluginRecord).toBeUndefined();
      expect(result.value.records.length).toBeGreaterThan(0);
    } finally {
      await repo.cleanup();
    }
  });

  test("plugin record included in InstallResult", async () => {
    const repo = await createClaudePluginRepo();
    try {
      const result = await installSkill(repo.path, {
        scope: "global",
        also: ["claude-code"],
        skipScan: true,
        onPluginDetected: async () => "plugin",
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const record = result.value.pluginRecord!;
      expect(record).toBeDefined();
      expect(record.name).toBe("test-plugin");
      expect(record.scope).toBe("global");
      expect(record.also).toContain("claude-code");
      expect(record.format).toBe("claude-code");
    } finally {
      await repo.cleanup();
    }
  });

  test("non-plugin repo proceeds with normal skill install regardless of callback", async () => {
    const repo = await createStandaloneSkillRepo();
    try {
      let callbackCalled = false;
      const result = await installSkill(repo.path, {
        scope: "global",
        also: ["claude-code"],
        skipScan: true,
        onPluginDetected: async () => {
          callbackCalled = true;
          return "plugin";
        },
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      // No plugin manifest → callback not called → normal install
      expect(callbackCalled).toBe(false);
      expect(result.value.pluginRecord).toBeUndefined();
      expect(result.value.records.length).toBeGreaterThan(0);
    } finally {
      await repo.cleanup();
    }
  });
});
