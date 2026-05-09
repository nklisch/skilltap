import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { loadManifest } from "./manifest/load";
import { loadLockfile } from "./manifest/lockfile";
import { installMcp, parseMcpRef, removeMcp } from "./mcp-install";
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
    expect(parseMcpRef("mcp:user/repo")).toEqual({
      inner: "user/repo",
      slug: "repo",
    });
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

describe("installMcp — local source", () => {
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

    const result = await installMcp(`mcp:${mcpSourceDir}`, {
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

    const result = await installMcp(`mcp:${mcpSourceDir}`, {
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
    const result = await installMcp(`mcp:${mcpSourceDir}`, {
      scope: "project",
      projectRoot,
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("No MCP servers found");
  });

  test("fails on non-mcp: source", async () => {
    const result = await installMcp("github:u/r", {
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
      JSON.stringify({
        mcpServers: { db: { command: "node", args: ["v1.js"] } },
      }),
    );

    const r1 = await installMcp(`mcp:${mcpSourceDir}`, {
      scope: "project",
      projectRoot,
    });
    expect(r1.ok).toBe(true);

    // Update the source's mcp.json and re-install
    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({
        mcpServers: { db: { command: "node", args: ["v2.js"] } },
      }),
    );
    const r2 = await installMcp(`mcp:${mcpSourceDir}`, {
      scope: "project",
      projectRoot,
    });
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

describe("installMcp — manifest + lockfile sync (Unit 1.14)", () => {
  test("appends [[mcps]] entry to skilltap.toml when manifest exists", async () => {
    // Seed a project manifest
    await writeFile(join(projectRoot, "skilltap.toml"), "");

    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({
        mcpServers: { db: { command: "node", args: ["server.js"] } },
      }),
    );
    const source = `mcp:${mcpSourceDir}`;

    const result = await installMcp(source, {
      scope: "project",
      projectRoot,
    });
    expect(result.ok).toBe(true);

    const manifest = await loadManifest(projectRoot);
    expect(manifest.ok).toBe(true);
    if (!manifest.ok) return;
    expect(manifest.value.mcps).toHaveLength(1);
    expect(manifest.value.mcps[0]).toMatchObject({
      source,
      ref: expect.any(String),
    });
  });

  test("appends [[mcps]] entry to skilltap.lock", async () => {
    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({
        mcpServers: { db: { command: "node", args: ["s.js"] } },
      }),
    );
    const source = `mcp:${mcpSourceDir}`;

    const result = await installMcp(source, {
      scope: "project",
      projectRoot,
    });
    expect(result.ok).toBe(true);

    const lockfile = await loadLockfile(projectRoot);
    expect(lockfile.ok).toBe(true);
    if (!lockfile.ok) return;
    expect(lockfile.value.mcps).toHaveLength(1);
    expect(lockfile.value.mcps[0].source).toBe(source);
  });

  test("removeMcp drops entry from manifest + lockfile", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({
        mcpServers: { db: { command: "node", args: ["s.js"] } },
      }),
    );
    const source = `mcp:${mcpSourceDir}`;

    expect(
      (await installMcp(source, { scope: "project", projectRoot })).ok,
    ).toBe(true);

    const removeResult = await removeMcp(source, {
      scope: "project",
      projectRoot,
    });
    expect(removeResult.ok).toBe(true);

    const manifest = await loadManifest(projectRoot);
    expect(manifest.ok).toBe(true);
    if (manifest.ok) expect(manifest.value.mcps).toHaveLength(0);

    const lockfile = await loadLockfile(projectRoot);
    expect(lockfile.ok).toBe(true);
    if (lockfile.ok) expect(lockfile.value.mcps).toHaveLength(0);
  });

  test("global scope does not write manifest or lockfile", async () => {
    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({
        mcpServers: { db: { command: "node", args: ["s.js"] } },
      }),
    );
    const source = `mcp:${mcpSourceDir}`;

    const result = await installMcp(source, { scope: "global" });
    expect(result.ok).toBe(true);

    // Lockfile should not have been touched at projectRoot.
    const lockfile = await loadLockfile(projectRoot);
    expect(lockfile.ok).toBe(true);
    if (lockfile.ok) expect(lockfile.value.mcps).toHaveLength(0);
  });
});

describe("removeMcp", () => {
  test("removes entries from state and prunes agent config", async () => {
    await writeFile(
      join(mcpSourceDir, ".mcp.json"),
      JSON.stringify({
        mcpServers: {
          db: { command: "node", args: ["server.js"] },
          search: { type: "http", url: "https://search.example.com/mcp" },
        },
      }),
    );
    const source = `mcp:${mcpSourceDir}`;
    const installResult = await installMcp(source, {
      scope: "project",
      projectRoot,
      agents: ["claude-code"],
    });
    expect(installResult.ok).toBe(true);

    // Sanity: settings has the entries
    const settingsPath = join(projectRoot, ".claude", "settings.json");
    const before = JSON.parse(await readFile(settingsPath, "utf8"));
    expect(Object.keys(before.mcpServers ?? {})).toHaveLength(2);

    const result = await removeMcp(source, {
      scope: "project",
      projectRoot,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.removed).toBe(2);
    expect(result.value.agents).toContain("claude-code");
    expect(result.value.names).toHaveLength(2);

    // State pruned
    const stateResult = await loadState(projectRoot);
    expect(stateResult.ok).toBe(true);
    if (!stateResult.ok) return;
    expect(stateResult.value.mcpServers).toHaveLength(0);

    // Agent config pruned
    const after = JSON.parse(await readFile(settingsPath, "utf8"));
    const remaining = Object.keys(after.mcpServers ?? {}).filter((k) =>
      k.startsWith("skilltap:"),
    );
    expect(remaining).toHaveLength(0);
  });

  test("fails when source has no installed entries", async () => {
    const result = await removeMcp("mcp:nope/missing", {
      scope: "project",
      projectRoot,
    });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("No MCP servers installed");
  });

  test("only removes entries matching the given source", async () => {
    const otherDir = await mkdtemp(join(tmpdir(), "skilltap-mcp-other-"));
    try {
      await writeFile(
        join(mcpSourceDir, ".mcp.json"),
        JSON.stringify({
          mcpServers: { db: { command: "node", args: ["a.js"] } },
        }),
      );
      await writeFile(
        join(otherDir, ".mcp.json"),
        JSON.stringify({
          mcpServers: { kept: { command: "node", args: ["b.js"] } },
        }),
      );
      const sourceA = `mcp:${mcpSourceDir}`;
      const sourceB = `mcp:${otherDir}`;
      expect(
        (await installMcp(sourceA, { scope: "project", projectRoot })).ok,
      ).toBe(true);
      expect(
        (await installMcp(sourceB, { scope: "project", projectRoot })).ok,
      ).toBe(true);

      const result = await removeMcp(sourceA, {
        scope: "project",
        projectRoot,
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.removed).toBe(1);

      const stateResult = await loadState(projectRoot);
      expect(stateResult.ok).toBe(true);
      if (!stateResult.ok) return;
      expect(stateResult.value.mcpServers).toHaveLength(1);
      expect(stateResult.value.mcpServers[0].source).toBe(sourceB);
    } finally {
      await rm(otherDir, { recursive: true, force: true });
    }
  });
});
