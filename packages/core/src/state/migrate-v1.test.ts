import { describe, expect, test } from "bun:test";
import type { InstalledJson } from "../schemas/installed";
import type { PluginsJson } from "../schemas/plugins";
import { migrateV1State } from "./migrate-v1";

const SKILL = {
  name: "commit-helper",
  repo: "https://github.com/n/r",
  ref: "v1",
  sha: "abc",
  scope: "global" as const,
  path: null,
  tap: null,
  also: [],
  installedAt: "2026-05-05T00:00:00.000Z",
  updatedAt: "2026-05-05T00:00:00.000Z",
  active: true,
};

const PLUGIN = {
  name: "dev-toolkit",
  description: "",
  format: "skilltap" as const,
  repo: "https://github.com/c/d",
  ref: "main",
  sha: "def",
  scope: "global" as const,
  also: [],
  tap: null,
  components: [],
  installedAt: "2026-05-05T00:00:00.000Z",
  updatedAt: "2026-05-05T00:00:00.000Z",
  active: true,
};

describe("migrateV1State", () => {
  test("merges empty installed + plugins to empty v2 state", () => {
    const state = migrateV1State({ version: 1, skills: [] }, { version: 1, plugins: [] });
    expect(state.version).toBe(2);
    expect(state.skills).toEqual([]);
    expect(state.plugins).toEqual([]);
    expect(state.mcpServers).toEqual([]);
  });

  test("preserves skill records verbatim", () => {
    const installed: InstalledJson = { version: 1, skills: [SKILL] };
    const plugins: PluginsJson = { version: 1, plugins: [] };
    const state = migrateV1State(installed, plugins);
    expect(state.skills).toEqual([SKILL]);
  });

  test("preserves plugin records verbatim", () => {
    const installed: InstalledJson = { version: 1, skills: [] };
    const plugins: PluginsJson = { version: 1, plugins: [PLUGIN] };
    const state = migrateV1State(installed, plugins);
    expect(state.plugins).toEqual([PLUGIN]);
  });

  test("merges both into one state", () => {
    const state = migrateV1State(
      { version: 1, skills: [SKILL] },
      { version: 1, plugins: [PLUGIN] },
    );
    expect(state.skills).toHaveLength(1);
    expect(state.plugins).toHaveLength(1);
    expect(state.mcpServers).toEqual([]);
  });
});
