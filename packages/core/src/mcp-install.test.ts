import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { installMcpOnly, parseMcpRef } from "./mcp-install";
import { loadState } from "./state/load";

let env: TestEnv;
let projectRoot: string;
let mcpSourceDir: string;

beforeEach(async () => {
  env = await createTestEnv();
  await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-mcp-proj-"));
  await mkdir(join(projectRoot, ".claude"), { recursive: true });
  mcpSourceDir = await mkdtemp(join(tmpdir(), "skilltap-mcp-source-"));
});

afterEach(async () => {
  await env.cleanup();
  await rm(projectRoot, { recursive: true, force: true });
  await rm(mcpSourceDir, { recursive: true, force: true });
});

describe("parseMcpRef", () => {
  test("returns null for non-mcp source", () => {
    expect(parseMcpRef("github:user/repo")).toBeNull();
    expect(parseMcpRef("https://github.com/u/r")).toBeNull();
    expect(parseMcpRef("npm:@scope/x")).toBeNull();
    expect(parseMcpRef("commit-helper")).toBeNull();
  });

  test("returns null for mcp: with no inner content", () => {
    expect(parseMcpRef("mcp:")).toBeNull();
  });

  test("strips mcp: prefix", () => {
    expect(parseMcpRef("mcp:user/repo")).toEqual({ inner: "user/repo", slug: "repo" });
  });

  test("derives slug from last path segment", () => {
    expect(parseMcpRef("mcp:owner/sub/repo")?.slug).toBe("repo");
    expect(parseMcpRef("mcp:repo-name")?.slug).toBe("repo-name");
  });

  test("strips schemes/protocols when computing slug", () => {
    expect(parseMcpRef("mcp:https://github.com/u/r")?.slug).toBe("r");
    expect(parseMcpRef("mcp:https://github.com/u/r.git")?.slug).toBe("r");
    expect(parseMcpRef("mcp:git@github.com:u/r.git")?.slug).toBe("r");
    expect(parseMcpRef("mcp:github:u/r")?.slug).toBe("r");
  });

  test("npm scoped packages drop @ from slug", () => {
    expect(parseMcpRef("mcp:npm:@scope/db-mcp")?.slug).toBe("db-mcp");
  });
});

describe("installMcpOnly — local source", () => {
  test("installs servers from a .mcp.json at the source root", async () => {
    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({
        mcpServers: {
          db: {
            command: "node",
            args: ["server.js"],
          },
        },
      }),
    );

    const result = await installMcpOnly(`mcp:${mcpSourceDir}`, {
      scope: "project",
      projectRoot,
      agents: ["claude-code"],
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.records).toHaveLength(1);
    const slug = mcpSourceDir.split("/").pop();
    expect(result.value.records[0].name).toBe(`skilltap:${slug}:db`);
    expect(result.value.records[0].config).toMatchObject({
      type: "stdio",
      command: "node",
      args: ["server.js"],
    });
    expect(result.value.records[0].targets).toContain("claude-code");
    expect(result.value.agents).toContain("claude-code");

    // State written to project's state.json
    const stateResult = await loadState(projectRoot);
    expect(stateResult.ok).toBe(true);
    if (!stateResult.ok) return;
    expect(stateResult.value.mcpServers).toHaveLength(1);
    expect(stateResult.value.mcpServers[0].name).toBe(`skilltap:${slug}:db`);

    // Agent config (.claude/settings.json) got the namespaced entry
    const settingsPath = join(projectRoot, ".claude", "settings.json");
    const settings = JSON.parse(await readFile(settingsPath, "utf8"));
    expect(settings.mcpServers).toBeDefined();
    expect(settings.mcpServers[`skilltap:${slug}:db`]).toBeDefined();
  });

  test("installs servers from multiple keys", async () => {
    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({
        mcpServers: {
          db: { command: "node", args: ["db.js"] },
          search: { type: "http", url: "https://search.example.com/mcp" },
        },
      }),
    );

    const result = await installMcpOnly(`mcp:${mcpSourceDir}`, {
      scope: "project",
      projectRoot,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.records).toHaveLength(2);
    const types = result.value.records.map((r) => r.config.type).sort();
    expect(types).toEqual(["http", "stdio"]);
  });

  test("fails when no servers are found", async () => {
    // Empty source directory
    const result = await installMcpOnly(`mcp:${mcpSourceDir}`, {
      scope: "project",
      projectRoot,
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("No MCP servers found");
  });

  test("fails on non-mcp: source", async () => {
    const result = await installMcpOnly("github:u/r", {
      scope: "project",
      projectRoot,
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not an mcp: ref");
  });

  test("re-running replaces existing entries (idempotent)", async () => {
    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({ mcpServers: { db: { command: "node", args: ["v1.js"] } } }),
    );

    const r1 = await installMcpOnly(`mcp:${mcpSourceDir}`, { scope: "project", projectRoot });
    expect(r1.ok).toBe(true);

    // Update the source's mcp.json and re-install
    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({ mcpServers: { db: { command: "node", args: ["v2.js"] } } }),
    );
    const r2 = await installMcpOnly(`mcp:${mcpSourceDir}`, { scope: "project", projectRoot });
    expect(r2.ok).toBe(true);

    const stateResult = await loadState(projectRoot);
    expect(stateResult.ok).toBe(true);
    if (!stateResult.ok) return;
    expect(stateResult.value.mcpServers).toHaveLength(1); // not duplicated
    if (stateResult.value.mcpServers[0].config.type === "stdio") {
      expect(stateResult.value.mcpServers[0].config.args).toEqual(["v2.js"]);
    }
  });
});
