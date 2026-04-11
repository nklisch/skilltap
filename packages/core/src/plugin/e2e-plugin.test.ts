import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  createClaudePluginRepo,
  createCodexPluginRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { detectPlugin } from "./detect";
import { parseClaudePlugin } from "./parse-claude";
import { parseCodexPlugin } from "./parse-codex";

setDefaultTimeout(30_000);

const cleanups: (() => Promise<void>)[] = [];

afterEach(async () => {
  for (const cleanup of cleanups.splice(0)) await cleanup();
});

// ---------------------------------------------------------------------------
// Journey 1: Claude Code plugin — full detection and component verification
// ---------------------------------------------------------------------------
describe("Claude Code plugin e2e", () => {
  test("detectPlugin on fixture repo finds all component types with correct structure", async () => {
    const repo = await createClaudePluginRepo();
    cleanups.push(repo.cleanup);

    const result = await detectPlugin(repo.path);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).not.toBeNull();

    const manifest = result.value!;
    expect(manifest.name).toBe("test-plugin");
    expect(manifest.format).toBe("claude-code");
    expect(manifest.pluginRoot).toBe(repo.path);

    // --- Verify skills ---
    const skills = manifest.components.filter((c) => c.type === "skill");
    expect(skills.length).toBeGreaterThanOrEqual(1);
    const helper = skills.find((c) => c.type === "skill" && c.name === "helper");
    expect(helper).toBeDefined();
    if (helper?.type === "skill") {
      expect(helper.path).toBe("skills/helper");
      expect(helper.description).toBe("A helper skill");
      // Path must be relative, not absolute
      expect(helper.path.startsWith("/")).toBe(false);
    }

    // --- Verify MCP servers ---
    const mcps = manifest.components.filter((c) => c.type === "mcp");
    expect(mcps.length).toBeGreaterThanOrEqual(1);
    const dbMcp = mcps.find((c) => c.type === "mcp" && c.server.name === "test-db");
    expect(dbMcp).toBeDefined();
    if (dbMcp?.type === "mcp" && dbMcp.server.type === "stdio") {
      expect(dbMcp.server.command).toBe("npx");
      expect(dbMcp.server.args).toEqual(["-y", "test-mcp"]);
    }

    // --- Verify agents ---
    const agents = manifest.components.filter((c) => c.type === "agent");
    expect(agents.length).toBeGreaterThanOrEqual(1);
    const reviewer = agents.find((c) => c.type === "agent" && c.name === "reviewer");
    expect(reviewer).toBeDefined();
    if (reviewer?.type === "agent") {
      expect(reviewer.path).toBe("agents/reviewer.md");
      expect(reviewer.frontmatter.model).toBe("sonnet");
      expect(reviewer.path.startsWith("/")).toBe(false);
    }
  });
});

// ---------------------------------------------------------------------------
// Journey 2: Codex plugin — detection and no-agents guarantee
// ---------------------------------------------------------------------------
describe("Codex plugin e2e", () => {
  test("detectPlugin on Codex fixture repo finds skills and MCP, never agents", async () => {
    const repo = await createCodexPluginRepo();
    cleanups.push(repo.cleanup);

    const result = await detectPlugin(repo.path);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).not.toBeNull();

    const manifest = result.value!;
    expect(manifest.name).toBe("test-codex");
    expect(manifest.format).toBe("codex");
    expect(manifest.version).toBe("1.0.0");
    expect(manifest.description).toBe("Test Codex plugin");

    // --- Skills present ---
    const skills = manifest.components.filter((c) => c.type === "skill");
    expect(skills.length).toBeGreaterThanOrEqual(1);
    expect(skills.some((c) => c.type === "skill" && c.name === "linter")).toBe(true);

    // --- MCP present ---
    const mcps = manifest.components.filter((c) => c.type === "mcp");
    expect(mcps.length).toBeGreaterThanOrEqual(1);
    expect(mcps.some((c) => c.type === "mcp" && c.server.name === "lint-server")).toBe(true);

    // --- Agents NEVER present (Codex contract) ---
    const agents = manifest.components.filter((c) => c.type === "agent");
    expect(agents).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Journey 3: Plain skill repo — detectPlugin returns null
// ---------------------------------------------------------------------------
describe("plain skill repo", () => {
  test("detectPlugin returns null for a repo with only SKILL.md (no plugin manifest)", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    // Create a plain skill repo with no .claude-plugin or .codex-plugin
    await Bun.write(
      join(dir, "SKILL.md"),
      "---\nname: plain-skill\ndescription: Just a skill\n---\n# Plain\nContent.\n",
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Journey 4: Rich plugin — multiple skills, multiple MCPs, multiple agents
// ---------------------------------------------------------------------------
describe("rich plugin with multiple components", () => {
  test("Claude plugin with 3 skills, 2 MCP servers, 2 agents produces complete manifest", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    // Plugin manifest
    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ name: "dev-toolkit", description: "Full dev toolkit", version: "2.1.0" }),
    );

    // 3 skills
    for (const name of ["code-review", "commit-helper", "test-generator"]) {
      await mkdir(join(dir, "skills", name), { recursive: true });
      await Bun.write(
        join(dir, "skills", name, "SKILL.md"),
        `---\nname: ${name}\ndescription: ${name} skill\n---\n# ${name}\nContent.\n`,
      );
    }

    // 2 MCP servers (mixed stdio + http)
    await Bun.write(
      join(dir, ".mcp.json"),
      JSON.stringify({
        database: { command: "npx", args: ["-y", "@corp/db-mcp"], env: { DB_URL: "postgres://..." } },
        "search-api": { type: "http", url: "https://search.example.com/mcp" },
      }),
    );

    // 2 agents
    await mkdir(join(dir, "agents"), { recursive: true });
    await Bun.write(
      join(dir, "agents", "code-reviewer.md"),
      "---\nname: code-reviewer\ndescription: Thorough code review\nmodel: opus\ntools: Read,Grep,Glob\ncolor: red\n---\nYou are an expert code reviewer.",
    );
    await Bun.write(
      join(dir, "agents", "architect.md"),
      "---\nname: architect\ndescription: Architecture analysis\nmodel: sonnet\n---\nYou are an architecture specialist.",
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).not.toBeNull();

    const manifest = result.value!;
    expect(manifest.name).toBe("dev-toolkit");
    expect(manifest.version).toBe("2.1.0");
    expect(manifest.description).toBe("Full dev toolkit");
    expect(manifest.format).toBe("claude-code");

    // 3 skills
    const skills = manifest.components.filter((c) => c.type === "skill");
    expect(skills).toHaveLength(3);
    const skillNames = skills.map((c) => c.name).sort();
    expect(skillNames).toEqual(["code-review", "commit-helper", "test-generator"]);

    // All skill paths relative
    for (const s of skills) {
      if (s.type === "skill") {
        expect(s.path.startsWith("/")).toBe(false);
        expect(s.path).toStartWith("skills/");
      }
    }

    // 2 MCP servers
    const mcps = manifest.components.filter((c) => c.type === "mcp");
    expect(mcps).toHaveLength(2);

    const dbMcp = mcps.find((c) => c.type === "mcp" && c.server.name === "database");
    expect(dbMcp).toBeDefined();
    if (dbMcp?.type === "mcp" && dbMcp.server.type === "stdio") {
      expect(dbMcp.server.command).toBe("npx");
      expect(dbMcp.server.args).toEqual(["-y", "@corp/db-mcp"]);
      expect(dbMcp.server.env).toEqual({ DB_URL: "postgres://..." });
    }

    const httpMcp = mcps.find((c) => c.type === "mcp" && c.server.name === "search-api");
    expect(httpMcp).toBeDefined();
    if (httpMcp?.type === "mcp" && httpMcp.server.type === "http") {
      expect(httpMcp.server.url).toBe("https://search.example.com/mcp");
    }

    // 2 agents (sorted alphabetically)
    const agents = manifest.components.filter((c) => c.type === "agent");
    expect(agents).toHaveLength(2);
    expect(agents[0]?.name).toBe("architect");
    expect(agents[1]?.name).toBe("code-reviewer");

    // Agent frontmatter preserved
    if (agents[1]?.type === "agent") {
      expect(agents[1].frontmatter.model).toBe("opus");
      expect(agents[1].frontmatter.color).toBe("red");
      expect(agents[1].path).toBe("agents/code-reviewer.md");
    }
  });
});

// ---------------------------------------------------------------------------
// Journey 5: Format priority — Claude Code wins when both exist
// ---------------------------------------------------------------------------
describe("dual-format repo", () => {
  test("repo with both .claude-plugin and .codex-plugin is detected as Claude Code", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    // Claude Code manifest
    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ name: "dual-plugin" }),
    );

    // Codex manifest
    await mkdir(join(dir, ".codex-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".codex-plugin", "plugin.json"),
      JSON.stringify({ name: "dual-codex", version: "1.0.0", description: "Codex version" }),
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).not.toBeNull();
    expect(result.value!.format).toBe("claude-code");
    expect(result.value!.name).toBe("dual-plugin");
  });
});

// ---------------------------------------------------------------------------
// Journey 6: Error resilience — malformed manifests
// ---------------------------------------------------------------------------
describe("error handling", () => {
  test("malformed Claude plugin.json returns descriptive error", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(join(dir, ".claude-plugin", "plugin.json"), "{ broken json!!!}");

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Invalid JSON");
  });

  test("Claude plugin.json missing name returns error", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ description: "no name field" }),
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("name");
  });

  test("Codex plugin.json missing required fields returns error", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".codex-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".codex-plugin", "plugin.json"),
      JSON.stringify({ name: "incomplete" }),
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("required");
  });

  test("plugin with invalid MCP config propagates error through detection", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ name: "bad-mcp-plugin" }),
    );
    // Invalid: not a JSON object
    await Bun.write(join(dir, ".mcp.json"), "not json at all");

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("JSON");
  });
});

// ---------------------------------------------------------------------------
// Journey 7: Plugin with only some component types
// ---------------------------------------------------------------------------
describe("partial component plugins", () => {
  test("plugin with only skills (no MCP, no agents)", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ name: "skills-only" }),
    );
    await mkdir(join(dir, "skills", "my-skill"), { recursive: true });
    await Bun.write(
      join(dir, "skills", "my-skill", "SKILL.md"),
      "---\nname: my-skill\ndescription: Just a skill\n---\n# My Skill\n",
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const manifest = result.value!;
    expect(manifest.components.filter((c) => c.type === "skill")).toHaveLength(1);
    expect(manifest.components.filter((c) => c.type === "mcp")).toHaveLength(0);
    expect(manifest.components.filter((c) => c.type === "agent")).toHaveLength(0);
  });

  test("plugin with only MCP servers (no skills, no agents)", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ name: "mcp-only" }),
    );
    await Bun.write(
      join(dir, ".mcp.json"),
      JSON.stringify({ server: { command: "node", args: ["srv.js"] } }),
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const manifest = result.value!;
    expect(manifest.components.filter((c) => c.type === "skill")).toHaveLength(0);
    expect(manifest.components.filter((c) => c.type === "mcp")).toHaveLength(1);
    expect(manifest.components.filter((c) => c.type === "agent")).toHaveLength(0);
  });

  test("plugin with only agents (no skills, no MCP)", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ name: "agents-only" }),
    );
    await mkdir(join(dir, "agents"), { recursive: true });
    await Bun.write(
      join(dir, "agents", "helper.md"),
      "---\nname: helper\ndescription: Helper agent\n---\nHelp instructions.",
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const manifest = result.value!;
    expect(manifest.components.filter((c) => c.type === "skill")).toHaveLength(0);
    expect(manifest.components.filter((c) => c.type === "mcp")).toHaveLength(0);
    expect(manifest.components.filter((c) => c.type === "agent")).toHaveLength(1);
  });

  test("empty plugin (manifest only, no components at all)", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ name: "empty-plugin" }),
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value!.name).toBe("empty-plugin");
    expect(result.value!.components).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Journey 8: MCP format variations in real plugin context
// ---------------------------------------------------------------------------
describe("MCP format variations through plugin detection", () => {
  test("wrapped .mcp.json format (mcpServers key) works through full detection", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ name: "wrapped-mcp" }),
    );
    await Bun.write(
      join(dir, ".mcp.json"),
      JSON.stringify({
        mcpServers: {
          "my-server": { command: "npx", args: ["-y", "my-mcp"] },
        },
      }),
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const mcps = result.value!.components.filter((c) => c.type === "mcp");
    expect(mcps).toHaveLength(1);
    if (mcps[0]?.type === "mcp") {
      expect(mcps[0].server.name).toBe("my-server");
    }
  });

  test("HTTP MCP server works through full detection", async () => {
    const dir = await makeTmpDir();
    cleanups.push(() => removeTmpDir(dir));

    await mkdir(join(dir, ".claude-plugin"), { recursive: true });
    await Bun.write(
      join(dir, ".claude-plugin", "plugin.json"),
      JSON.stringify({ name: "http-mcp" }),
    );
    await Bun.write(
      join(dir, ".mcp.json"),
      JSON.stringify({
        "remote-api": { type: "http", url: "https://api.example.com/mcp" },
      }),
    );

    const result = await detectPlugin(dir);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const mcps = result.value!.components.filter((c) => c.type === "mcp");
    expect(mcps).toHaveLength(1);
    if (mcps[0]?.type === "mcp" && mcps[0].server.type === "http") {
      expect(mcps[0].server.url).toBe("https://api.example.com/mcp");
    }
  });
});
