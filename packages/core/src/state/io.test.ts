import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { loadState } from "./load";
import { saveState } from "./save";
import type { State } from "./schema";

describe("state load/save", () => {
  let env: TestEnv;
  beforeEach(async () => {
    env = await createTestEnv();
  });
  afterEach(async () => {
    await env.cleanup();
  });

  test("loadState returns default state when file does not exist (global)", async () => {
    const result = await loadState();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.version).toBe(2);
    expect(result.value.skills).toEqual([]);
    expect(result.value.plugins).toEqual([]);
    expect(result.value.mcpServers).toEqual([]);
  });

  test("loadState returns default state when file does not exist (project)", async () => {
    const projectRoot = await mkdtemp(join(tmpdir(), "skilltap-proj-"));
    try {
      const result = await loadState(projectRoot);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.version).toBe(2);
    } finally {
      await rm(projectRoot, { recursive: true, force: true });
    }
  });

  test("save then load round-trips a populated state (global)", async () => {
    const state: State = {
      version: 2,
      skills: [],
      plugins: [],
      mcpServers: [
        {
          name: "skilltap:db",
          source: "github:n/db",
          config: { type: "stdio", command: "node", args: ["s.js"], env: {} },
          targets: ["claude-code"],
          installedAt: "2026-05-05T00:00:00.000Z",
        },
      ],
    };
    const saveResult = await saveState(state);
    expect(saveResult.ok).toBe(true);
    const loadResult = await loadState();
    expect(loadResult.ok).toBe(true);
    if (!loadResult.ok) return;
    expect(loadResult.value.mcpServers).toHaveLength(1);
    expect(loadResult.value.mcpServers[0].name).toBe("skilltap:db");
  });

  test("save then load round-trips a populated state (project)", async () => {
    const projectRoot = await mkdtemp(join(tmpdir(), "skilltap-proj-"));
    try {
      const state: State = {
        version: 2,
        skills: [],
        plugins: [],
        mcpServers: [],
      };
      const saveResult = await saveState(state, projectRoot);
      expect(saveResult.ok).toBe(true);
      const loadResult = await loadState(projectRoot);
      expect(loadResult.ok).toBe(true);
      if (!loadResult.ok) return;
      expect(loadResult.value.version).toBe(2);
    } finally {
      await rm(projectRoot, { recursive: true, force: true });
    }
  });
});
