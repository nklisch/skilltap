import { describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir, createClaudePluginRepo, createCodexPluginRepo, createStandaloneSkillRepo } from "@skilltap/test-utils";
import { detectPlugin } from "./detect";

describe("detectPlugin", () => {
  test("detects Claude Code plugin", async () => {
    const { path, cleanup } = await createClaudePluginRepo();
    try {
      const result = await detectPlugin(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).not.toBeNull();
      expect(result.value?.format).toBe("claude-code");
    } finally {
      await cleanup();
    }
  });

  test("detects Codex plugin", async () => {
    const { path, cleanup } = await createCodexPluginRepo();
    try {
      const result = await detectPlugin(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).not.toBeNull();
      expect(result.value?.format).toBe("codex");
    } finally {
      await cleanup();
    }
  });

  test("returns null for plain skill repo", async () => {
    const { path, cleanup } = await createStandaloneSkillRepo();
    try {
      const result = await detectPlugin(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toBeNull();
    } finally {
      await cleanup();
    }
  });

  test("prefers Claude Code when both exist", async () => {
    const dir = await makeTmpDir();
    try {
      // Create both plugin types
      await mkdir(join(dir, ".claude-plugin"), { recursive: true });
      await Bun.write(
        join(dir, ".claude-plugin", "plugin.json"),
        JSON.stringify({ name: "claude-plugin" }),
      );
      await mkdir(join(dir, ".codex-plugin"), { recursive: true });
      await Bun.write(
        join(dir, ".codex-plugin", "plugin.json"),
        JSON.stringify({ name: "codex-plugin", version: "1.0.0", description: "Codex" }),
      );

      const result = await detectPlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value?.format).toBe("claude-code");
      expect(result.value?.name).toBe("claude-plugin");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("propagates parse errors from individual parsers", async () => {
    const dir = await makeTmpDir();
    try {
      await mkdir(join(dir, ".claude-plugin"), { recursive: true });
      await Bun.write(join(dir, ".claude-plugin", "plugin.json"), "{ invalid json }");

      const result = await detectPlugin(dir);
      expect(result.ok).toBe(false);
    } finally {
      await removeTmpDir(dir);
    }
  });
});
