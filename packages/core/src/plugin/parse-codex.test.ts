import { describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir, createCodexPluginRepo } from "@skilltap/test-utils";
import { parseCodexPlugin } from "./parse-codex";

async function makeCodexPlugin(dir: string, pluginJson: unknown): Promise<void> {
  await mkdir(join(dir, ".codex-plugin"), { recursive: true });
  await Bun.write(join(dir, ".codex-plugin", "plugin.json"), JSON.stringify(pluginJson));
}

async function makeSkill(dir: string, name: string, description = "A test skill"): Promise<void> {
  await mkdir(join(dir, "skills", name), { recursive: true });
  await Bun.write(
    join(dir, "skills", name, "SKILL.md"),
    `---\nname: ${name}\ndescription: ${description}\n---\n# ${name}\nContent.\n`,
  );
}

const VALID_CODEX_MANIFEST = {
  name: "test-codex",
  version: "1.0.0",
  description: "A test Codex plugin",
};

describe("parseCodexPlugin", () => {
  test("parses full codex plugin", async () => {
    const dir = await makeTmpDir();
    try {
      await makeCodexPlugin(dir, VALID_CODEX_MANIFEST);
      const result = await parseCodexPlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.name).toBe("test-codex");
      expect(result.value.format).toBe("codex");
      expect(result.value.version).toBe("1.0.0");
      expect(result.value.description).toBe("A test Codex plugin");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("discovers skills and MCP from conventions", async () => {
    const dir = await makeTmpDir();
    try {
      await makeCodexPlugin(dir, VALID_CODEX_MANIFEST);
      await makeSkill(dir, "linter", "Lints code");
      await Bun.write(join(dir, ".mcp.json"), JSON.stringify({
        "lint-server": { command: "node", args: ["server.js"] },
      }));

      const result = await parseCodexPlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const skills = result.value.components.filter((c) => c.type === "skill");
      expect(skills).toHaveLength(1);
      expect(skills[0]?.name).toBe("linter");
      const mcps = result.value.components.filter((c) => c.type === "mcp");
      expect(mcps).toHaveLength(1);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("never produces agent components", async () => {
    const dir = await makeTmpDir();
    try {
      await makeCodexPlugin(dir, VALID_CODEX_MANIFEST);
      // Add an agents/ directory — it should be ignored
      await mkdir(join(dir, "agents"), { recursive: true });
      await Bun.write(join(dir, "agents", "builder.md"), "---\nname: builder\n---\nContent");

      const result = await parseCodexPlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const agents = result.value.components.filter((c) => c.type === "agent");
      expect(agents).toHaveLength(0);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns err for missing required fields (version)", async () => {
    const dir = await makeTmpDir();
    try {
      await makeCodexPlugin(dir, { name: "test", description: "missing version" });
      const result = await parseCodexPlugin(dir);
      expect(result.ok).toBe(false);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns err for missing required fields (description)", async () => {
    const dir = await makeTmpDir();
    try {
      await makeCodexPlugin(dir, { name: "test", version: "1.0.0" });
      const result = await parseCodexPlugin(dir);
      expect(result.ok).toBe(false);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns err for missing plugin.json", async () => {
    const dir = await makeTmpDir();
    try {
      const result = await parseCodexPlugin(dir);
      expect(result.ok).toBe(false);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("format is 'codex'", async () => {
    const dir = await makeTmpDir();
    try {
      await makeCodexPlugin(dir, VALID_CODEX_MANIFEST);
      const result = await parseCodexPlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.format).toBe("codex");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("parses fixture codex plugin repo", async () => {
    const { path, cleanup } = await createCodexPluginRepo();
    try {
      const result = await parseCodexPlugin(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.name).toBe("test-codex");
      expect(result.value.format).toBe("codex");
      const skills = result.value.components.filter((c) => c.type === "skill");
      expect(skills.length).toBeGreaterThanOrEqual(1);
      const mcps = result.value.components.filter((c) => c.type === "mcp");
      expect(mcps.length).toBeGreaterThanOrEqual(1);
    } finally {
      await cleanup();
    }
  });
});
