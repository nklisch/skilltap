import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { mcpConfigPath } from "../../plugin/mcp-inject";
import type { State } from "../../state/schema";
import { checkMcpConsistency } from "./mcp-consistency";

// Minimal valid plugin record for use in state fixtures
function makePluginRecord(overrides: {
  name: string;
  active?: boolean;
  scope?: "global" | "project";
  also?: string[];
  components?: State["plugins"][number]["components"];
}): State["plugins"][number] {
  return {
    name: overrides.name,
    description: "",
    format: "skilltap-v2",
    repo: null,
    ref: null,
    sha: null,
    scope: overrides.scope ?? "global",
    also: overrides.also ?? ["claude-code"],
    tap: null,
    components: overrides.components ?? [],
    installedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    active: overrides.active ?? true,
  };
}

function makeState(plugins: State["plugins"]): State {
  return { version: 2, skills: [], plugins, mcpServers: [] };
}

let env: TestEnv;
let projectRoot: string;
beforeEach(async () => {
  env = await createTestEnv();
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-mcp-test-"));
});
afterEach(async () => {
  await env.cleanup();
  await rm(projectRoot, { recursive: true, force: true });
});

describe("checkMcpConsistency", () => {
  test("returns n/a when state is null", async () => {
    const check = await checkMcpConsistency(null);
    expect(check.status).toBe("pass");
    expect(check.detail).toBe("n/a (no v2 state)");
  });

  test("returns n/a when state has no active MCP components", async () => {
    const state = makeState([
      makePluginRecord({
        name: "skill-only",
        components: [{ type: "skill", name: "my-skill", active: true }],
      }),
    ]);
    const check = await checkMcpConsistency(state);
    expect(check.status).toBe("pass");
    expect(check.detail).toBe("n/a (no active MCP servers in state)");
  });

  test("passes when state-expected MCP entry is present in agent config", async () => {
    // Write the agent config with the expected key
    const configPath = mcpConfigPath("claude-code", "global")!;
    await mkdir(join(env.homeDir, ".claude"), { recursive: true });
    await writeFile(
      configPath,
      JSON.stringify({
        mcpServers: {
          "skilltap:my-plugin:my-server": {
            command: "node",
            args: ["server.js"],
          },
        },
      }),
    );

    const state = makeState([
      makePluginRecord({
        name: "my-plugin",
        also: ["claude-code"],
        components: [
          {
            type: "mcp",
            serverType: "stdio",
            name: "my-server",
            active: true,
            command: "node",
            args: ["server.js"],
            env: {},
          },
        ],
      }),
    ]);

    const check = await checkMcpConsistency(state);
    expect(check.status).toBe("pass");
    expect(check.detail).toBe("1 server entries verified");
    expect(check.issues).toBeUndefined();
  });

  test("warns (not fixable) when state-expected entry is missing from agent config", async () => {
    // Agent config exists but doesn't have the key
    const configPath = mcpConfigPath("claude-code", "global")!;
    await mkdir(join(env.homeDir, ".claude"), { recursive: true });
    await writeFile(configPath, JSON.stringify({ mcpServers: {} }));

    const state = makeState([
      makePluginRecord({
        name: "my-plugin",
        also: ["claude-code"],
        components: [
          {
            type: "mcp",
            serverType: "stdio",
            name: "my-server",
            active: true,
            command: "node",
            args: ["server.js"],
            env: {},
          },
        ],
      }),
    ]);

    const check = await checkMcpConsistency(state);
    expect(check.status).toBe("warn");
    expect(check.issues).toHaveLength(1);
    expect(check.issues![0].message).toContain(
      "Missing in claude-code (global)",
    );
    expect(check.issues![0].message).toContain("skilltap:my-plugin:my-server");
    expect(check.issues![0].fixable).toBe(false);
  });

  test("warns (fixable) when agent config has orphan skilltap: key with no state record, and fix removes it", async () => {
    // Write agent config with an orphan key — no matching state record
    const configPath = mcpConfigPath("claude-code", "global")!;
    await mkdir(join(env.homeDir, ".claude"), { recursive: true });
    await writeFile(
      configPath,
      JSON.stringify({
        mcpServers: {
          "skilltap:orphan-plugin:stale-server": {
            command: "old-binary",
            args: [],
          },
        },
      }),
    );

    // State is empty — no plugins
    const state = makeState([]);

    const check = await checkMcpConsistency(state);
    expect(check.status).toBe("warn");
    expect(check.issues).toHaveLength(1);
    const issue = check.issues![0];
    expect(issue.message).toContain("Orphan in claude-code (global)");
    expect(issue.message).toContain("skilltap:orphan-plugin:stale-server");
    expect(issue.fixable).toBe(true);
    expect(issue.fix).toBeDefined();

    // Verify key is present before fix
    const before = JSON.parse(await readFile(configPath, "utf8"));
    expect(
      before.mcpServers["skilltap:orphan-plugin:stale-server"],
    ).toBeDefined();

    // Run the fix
    await issue.fix!();

    // Verify key is removed after fix
    const after = JSON.parse(await readFile(configPath, "utf8"));
    expect(
      after.mcpServers["skilltap:orphan-plugin:stale-server"],
    ).toBeUndefined();
  });

  test("skips inactive plugins and inactive MCP components", async () => {
    // Neither an inactive plugin nor an inactive component should be expected
    const state = makeState([
      makePluginRecord({
        name: "inactive-plugin",
        active: false,
        also: ["claude-code"],
        components: [
          {
            type: "mcp",
            serverType: "stdio",
            name: "server-a",
            active: true,
            command: "cmd",
            args: [],
            env: {},
          },
        ],
      }),
      makePluginRecord({
        name: "active-plugin",
        active: true,
        also: ["claude-code"],
        components: [
          {
            type: "mcp",
            serverType: "stdio",
            name: "server-b",
            active: false,
            command: "cmd",
            args: [],
            env: {},
          },
        ],
      }),
    ]);

    const check = await checkMcpConsistency(state);
    expect(check.status).toBe("pass");
    expect(check.detail).toBe("n/a (no active MCP servers in state)");
  });
});
