import { describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir, createClaudePluginRepo } from "@skilltap/test-utils";
import { parseClaudePlugin } from "./parse-claude";

async function makePlugin(dir: string, pluginJson: unknown): Promise<void> {
  await mkdir(join(dir, ".claude-plugin"), { recursive: true });
  await Bun.write(join(dir, ".claude-plugin", "plugin.json"), JSON.stringify(pluginJson));
}

async function makeSkill(dir: string, name: string, description = "A test skill"): Promise<void> {
  await mkdir(join(dir, "skills", name), { recursive: true });
  await Bun.write(
    join(dir, "skills", name, "SKILL.md"),
    `---\nname: ${name}\ndescription: ${description}\n---\n# ${name}\nContent.\n`,
  );
}

describe("parseClaudePlugin", () => {
  test("parses minimal plugin (name only, no components)", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "minimal" });
      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.name).toBe("minimal");
      expect(result.value.format).toBe("claude-code");
      expect(result.value.pluginRoot).toBe(dir);
      expect(result.value.components).toEqual([]);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("discovers skills from skills/ convention", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test" });
      await makeSkill(dir, "helper", "A helper skill");

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const skills = result.value.components.filter((c) => c.type === "skill");
      expect(skills).toHaveLength(1);
      expect(skills[0]?.name).toBe("helper");
      if (skills[0]?.type === "skill") {
        expect(skills[0].path).toBe("skills/helper");
      }
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("discovers mcp from .mcp.json convention", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test" });
      await Bun.write(join(dir, ".mcp.json"), JSON.stringify({
        "my-server": { command: "npx", args: ["-y", "my-mcp"] },
      }));

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const mcps = result.value.components.filter((c) => c.type === "mcp");
      expect(mcps).toHaveLength(1);
      if (mcps[0]?.type === "mcp") {
        expect(mcps[0].server.name).toBe("my-server");
      }
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("discovers agents from agents/ convention", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test" });
      await mkdir(join(dir, "agents"), { recursive: true });
      await Bun.write(join(dir, "agents", "reviewer.md"), "---\nname: reviewer\n---\nContent");

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const agents = result.value.components.filter((c) => c.type === "agent");
      expect(agents).toHaveLength(1);
      expect(agents[0]?.name).toBe("reviewer");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("uses skills path override from manifest", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test", skills: "custom-skills/" });
      await mkdir(join(dir, "custom-skills", "my-skill"), { recursive: true });
      await Bun.write(
        join(dir, "custom-skills", "my-skill", "SKILL.md"),
        "---\nname: my-skill\ndescription: Custom skill\n---\nContent",
      );

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const skills = result.value.components.filter((c) => c.type === "skill");
      expect(skills).toHaveLength(1);
      expect(skills[0]?.name).toBe("my-skill");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("uses mcpServers path override from manifest", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test", mcpServers: "config/mcp.json" });
      await mkdir(join(dir, "config"), { recursive: true });
      await Bun.write(join(dir, "config", "mcp.json"), JSON.stringify({
        "config-server": { command: "node", args: ["server.js"] },
      }));

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const mcps = result.value.components.filter((c) => c.type === "mcp");
      expect(mcps).toHaveLength(1);
      if (mcps[0]?.type === "mcp") {
        expect(mcps[0].server.name).toBe("config-server");
      }
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("uses agents path override from manifest", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test", agents: "custom-agents/" });
      await mkdir(join(dir, "custom-agents"), { recursive: true });
      await Bun.write(join(dir, "custom-agents", "builder.md"), "---\nname: builder\n---\nContent");

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const agents = result.value.components.filter((c) => c.type === "agent");
      expect(agents).toHaveLength(1);
      expect(agents[0]?.name).toBe("builder");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("handles inline mcpServers object in manifest", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, {
        name: "test",
        mcpServers: {
          "inline-server": { command: "npx", args: ["-y", "inline-mcp"] },
        },
      });

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const mcps = result.value.components.filter((c) => c.type === "mcp");
      expect(mcps).toHaveLength(1);
      if (mcps[0]?.type === "mcp") {
        expect(mcps[0].server.name).toBe("inline-server");
      }
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("uses mcpServers as array-of-strings (multiple config files)", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test", mcpServers: ["config/a.json", "config/b.json"] });
      await mkdir(join(dir, "config"), { recursive: true });
      await Bun.write(join(dir, "config", "a.json"), JSON.stringify({
        "server-a": { command: "npx", args: ["-y", "mcp-a"] },
      }));
      await Bun.write(join(dir, "config", "b.json"), JSON.stringify({
        "server-b": { command: "node", args: ["b.js"] },
      }));

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const mcps = result.value.components.filter((c) => c.type === "mcp");
      expect(mcps).toHaveLength(2);
      const names = mcps.map((c) => c.type === "mcp" ? c.server.name : "").sort();
      expect(names).toEqual(["server-a", "server-b"]);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("skills path override to non-existent directory returns empty skills", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test", skills: "nonexistent-dir/" });
      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      // scanner.scan on a non-existent directory returns empty array
      const skills = result.value.components.filter((c) => c.type === "skill");
      expect(skills).toHaveLength(0);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns empty skills when skills/ dir exists but has no SKILL.md files", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test" });
      // Create a skills/ directory with no SKILL.md
      await mkdir(join(dir, "skills", "empty-skill"), { recursive: true });
      await Bun.write(join(dir, "skills", "empty-skill", "README.md"), "Not a skill");

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const skills = result.value.components.filter((c) => c.type === "skill");
      expect(skills).toHaveLength(0);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns err for missing plugin.json", async () => {
    const dir = await makeTmpDir();
    try {
      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(false);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns err for invalid plugin.json (malformed JSON)", async () => {
    const dir = await makeTmpDir();
    try {
      await mkdir(join(dir, ".claude-plugin"), { recursive: true });
      await Bun.write(join(dir, ".claude-plugin", "plugin.json"), "{ not valid }");
      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(false);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns err for invalid plugin.json (missing name)", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { description: "no name field" });
      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(false);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("component paths are relative to plugin root", async () => {
    const dir = await makeTmpDir();
    try {
      await makePlugin(dir, { name: "test" });
      await makeSkill(dir, "my-skill");

      const result = await parseClaudePlugin(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const skill = result.value.components.find((c) => c.type === "skill");
      if (skill?.type === "skill") {
        expect(skill.path).toBe("skills/my-skill");
        expect(skill.path.startsWith("/")).toBe(false);
      }
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("parses fixture claude plugin repo", async () => {
    const { path, cleanup } = await createClaudePluginRepo();
    try {
      const result = await parseClaudePlugin(path);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.name).toBe("test-plugin");
      expect(result.value.format).toBe("claude-code");
      const skills = result.value.components.filter((c) => c.type === "skill");
      expect(skills.length).toBeGreaterThanOrEqual(1);
      const mcps = result.value.components.filter((c) => c.type === "mcp");
      expect(mcps.length).toBeGreaterThanOrEqual(1);
      const agents = result.value.components.filter((c) => c.type === "agent");
      expect(agents.length).toBeGreaterThanOrEqual(1);
    } finally {
      await cleanup();
    }
  });
});
