import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { saveState } from "./save";
import { syncV1ToV2State } from "./sync-from-v1";

let env: TestEnv;
let projectRoot: string;

const SKILL = (name: string) => ({
  name,
  description: "",
  repo: `github:n/${name}`,
  ref: "main",
  sha: "abc123",
  scope: "project" as const,
  path: null,
  tap: null,
  also: ["claude-code"],
  installedAt: "2026-05-06T00:00:00.000Z",
  updatedAt: "2026-05-06T00:00:00.000Z",
  active: true,
});

beforeEach(async () => {
  env = await createTestEnv();
  await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-syncv1-"));
});

afterEach(async () => {
  await env.cleanup();
  await rm(projectRoot, { recursive: true, force: true });
});

describe("syncV1ToV2State", () => {
  test("creates state.json from existing project installed.json", async () => {
    await mkdir(join(projectRoot, ".agents"), { recursive: true });
    await writeFile(
      join(projectRoot, ".agents", "installed.json"),
      JSON.stringify({ version: 1, skills: [SKILL("foo"), SKILL("bar")] }),
    );

    const result = await syncV1ToV2State("project", projectRoot);
    expect(result.ok).toBe(true);

    const stateText = await readFile(
      join(projectRoot, ".agents", "state.json"),
      "utf8",
    );
    const state = JSON.parse(stateText) as {
      version: number;
      skills: Array<{ name: string }>;
      plugins: unknown[];
      mcpServers: unknown[];
    };
    expect(state.version).toBe(2);
    expect(state.skills.map((s) => s.name).sort()).toEqual(["bar", "foo"]);
    expect(state.plugins).toEqual([]);
    expect(state.mcpServers).toEqual([]);
  });

  test("preserves existing state.mcpServers when rebuilding from v0.x", async () => {
    // Pre-seed state.json with an mcpServer entry (e.g. from an mcp: install).
    const seedState = {
      version: 2 as const,
      skills: [],
      plugins: [],
      mcpServers: [
        {
          name: "skilltap:tools:db",
          source: "mcp:user/tools",
          targets: ["claude-code"],
          config: { type: "stdio" as const, command: "node", args: ["s.js"] },
          installedAt: "2026-05-06T00:00:00.000Z",
          updatedAt: "2026-05-06T00:00:00.000Z",
          active: true,
        },
      ],
    };
    const saveResult = await saveState(seedState, projectRoot);
    expect(saveResult.ok).toBe(true);

    // Now write a v0.x installed.json and sync.
    await writeFile(
      join(projectRoot, ".agents", "installed.json"),
      JSON.stringify({ version: 1, skills: [SKILL("foo")] }),
    );

    const result = await syncV1ToV2State("project", projectRoot);
    expect(result.ok).toBe(true);

    const state = JSON.parse(
      await readFile(join(projectRoot, ".agents", "state.json"), "utf8"),
    ) as { skills: Array<{ name: string }>; mcpServers: Array<{ name: string }> };
    expect(state.skills.map((s) => s.name)).toEqual(["foo"]);
    // The mcpServers entry must survive.
    expect(state.mcpServers).toHaveLength(1);
    expect(state.mcpServers[0]?.name).toBe("skilltap:tools:db");
  });

  test("emits an empty state when no v0.x files exist", async () => {
    const result = await syncV1ToV2State("project", projectRoot);
    expect(result.ok).toBe(true);

    const state = JSON.parse(
      await readFile(join(projectRoot, ".agents", "state.json"), "utf8"),
    ) as { version: number; skills: unknown[]; plugins: unknown[] };
    expect(state.version).toBe(2);
    expect(state.skills).toEqual([]);
    expect(state.plugins).toEqual([]);
  });

  test("global scope writes to ~/.config/skilltap/state.json", async () => {
    await writeFile(
      join(env.configDir, "skilltap", "installed.json"),
      JSON.stringify({ version: 1, skills: [SKILL("global-skill")] }),
    );

    const result = await syncV1ToV2State("global");
    expect(result.ok).toBe(true);

    const state = JSON.parse(
      await readFile(join(env.configDir, "skilltap", "state.json"), "utf8"),
    ) as { skills: Array<{ name: string }> };
    expect(state.skills[0]?.name).toBe("global-skill");
  });
});
