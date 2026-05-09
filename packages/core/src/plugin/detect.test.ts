import { describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  createClaudePluginRepo,
  createCodexPluginRepo,
  createStandaloneSkillRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { detectAllPlugins, detectPlugin } from "./detect";

async function createMultiPluginRepo(): Promise<{
  path: string;
  cleanup: () => Promise<void>;
}> {
  const dir = await makeTmpDir();
  const skilltapDir = join(dir, ".skilltap");
  await mkdir(skilltapDir, { recursive: true });
  await Bun.write(
    join(skilltapDir, "auth.toml"),
    `name = "auth"
version = "0.1.0"
description = "Auth plugin"
publish = true
`,
  );
  await Bun.write(
    join(skilltapDir, "billing.toml"),
    `name = "billing"
version = "0.1.0"
description = "Billing plugin"
publish = true
`,
  );
  return { path: dir, cleanup: () => removeTmpDir(dir) };
}

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
        JSON.stringify({
          name: "codex-plugin",
          version: "1.0.0",
          description: "Codex",
        }),
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

  test("multi-plugin repo with selectName picks the named plugin", async () => {
    const { path, cleanup } = await createMultiPluginRepo();
    try {
      const result = await detectPlugin(path, { selectName: "auth" });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value?.name).toBe("auth");
    } finally {
      await cleanup();
    }
  });

  test("multi-plugin repo without selectName errors with available names", async () => {
    const { path, cleanup } = await createMultiPluginRepo();
    try {
      const result = await detectPlugin(path);
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("auth");
      expect(result.error.message).toContain("billing");
    } finally {
      await cleanup();
    }
  });

  test("multi-plugin repo with bogus selectName errors", async () => {
    const { path, cleanup } = await createMultiPluginRepo();
    try {
      const result = await detectPlugin(path, { selectName: "bogus" });
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("bogus");
      expect(result.error.message).toContain("not found");
    } finally {
      await cleanup();
    }
  });

  test("propagates parse errors from individual parsers", async () => {
    const dir = await makeTmpDir();
    try {
      await mkdir(join(dir, ".claude-plugin"), { recursive: true });
      await Bun.write(
        join(dir, ".claude-plugin", "plugin.json"),
        "{ invalid json }",
      );

      const result = await detectPlugin(dir);
      expect(result.ok).toBe(false);
    } finally {
      await removeTmpDir(dir);
    }
  });
});

describe("detectAllPlugins", () => {
  test("returns all publishable plugins in a multi-plugin repo", async () => {
    const { path, cleanup } = await createMultiPluginRepo();
    try {
      const result = await detectAllPlugins(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const names = result.value.map((m) => m.name).sort();
      expect(names).toEqual(["auth", "billing"]);
    } finally {
      await cleanup();
    }
  });

  test("returns single Claude plugin in a single-plugin repo", async () => {
    const { path, cleanup } = await createClaudePluginRepo();
    try {
      const result = await detectAllPlugins(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(1);
      expect(result.value[0].format).toBe("claude-code");
    } finally {
      await cleanup();
    }
  });

  test("returns empty array for plain skill repo", async () => {
    const { path, cleanup } = await createStandaloneSkillRepo();
    try {
      const result = await detectAllPlugins(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toEqual([]);
    } finally {
      await cleanup();
    }
  });
});
