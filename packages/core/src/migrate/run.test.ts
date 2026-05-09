import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { createTestEnv, pathExists, type TestEnv } from "@skilltap/test-utils";
import { loadState } from "../state/load";
import { runMigrate } from "./run";

const V1_INSTALLED = {
  version: 1,
  skills: [
    {
      name: "commit-helper",
      repo: "https://github.com/n/r",
      ref: "v1.0",
      sha: "abc",
      scope: "global",
      path: null,
      tap: null,
      also: [],
      installedAt: "2026-05-05T00:00:00.000Z",
      updatedAt: "2026-05-05T00:00:00.000Z",
    },
  ],
};

const V1_PLUGINS = {
  version: 1,
  plugins: [],
};

const V1_CONFIG = `
[defaults]
also = ["claude-code"]

[security.human]
scan = "static"
on_warn = "prompt"

[security.agent]
scan = "static"
on_warn = "fail"

["agent-mode"]
enabled = false
scope = "project"
`;

describe("runMigrate", () => {
  let env: TestEnv;
  beforeEach(async () => {
    env = await createTestEnv();
    // ensure config dir exists for synthetic v1 setup
    await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  });
  afterEach(async () => {
    await env.cleanup();
  });

  test("returns alreadyMigrated when no v1 markers present", async () => {
    const result = await runMigrate({});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.alreadyMigrated).toBe(true);
    expect(result.value.scopes).toEqual([]);
  });

  test("migrates global v1 setup end-to-end", async () => {
    const cfgDir = join(env.configDir, "skilltap");
    const installedPath = join(cfgDir, "installed.json");
    const pluginsPath = join(cfgDir, "plugins.json");
    const configPath = join(cfgDir, "config.toml");

    await writeFile(installedPath, JSON.stringify(V1_INSTALLED, null, 2));
    await writeFile(pluginsPath, JSON.stringify(V1_PLUGINS, null, 2));
    await writeFile(configPath, V1_CONFIG);

    const result = await runMigrate({});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.alreadyMigrated).toBe(false);
    expect(result.value.scopes).toContain("global");

    // state.json was written
    expect(await pathExists(join(cfgDir, "state.json"))).toBe(true);

    // v1 files renamed to .v1.bak
    expect(await pathExists(installedPath)).toBe(false);
    expect(await pathExists(`${installedPath}.v1.bak`)).toBe(true);
    expect(await pathExists(pluginsPath)).toBe(false);
    expect(await pathExists(`${pluginsPath}.v1.bak`)).toBe(true);
    expect(await pathExists(`${configPath}.v1.bak`)).toBe(true);

    // state.json contains the migrated skill
    const stateResult = await loadState();
    expect(stateResult.ok).toBe(true);
    if (!stateResult.ok) return;
    expect(stateResult.value.skills).toHaveLength(1);
    expect(stateResult.value.skills[0].name).toBe("commit-helper");
  });

  test("aborts on HTTP taps without writing", async () => {
    const cfgDir = join(env.configDir, "skilltap");
    const installedPath = join(cfgDir, "installed.json");
    const configPath = join(cfgDir, "config.toml");

    await writeFile(installedPath, JSON.stringify(V1_INSTALLED, null, 2));
    await writeFile(
      configPath,
      `
[[taps]]
name = "http-tap"
url = "https://api.example.com/v1"
type = "http"
`,
    );

    const result = await runMigrate({});
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("HTTP taps");

    // No writes happened
    expect(await pathExists(join(cfgDir, "state.json"))).toBe(false);
    // installed.json was NOT renamed (config check ran first)
    expect(await pathExists(installedPath)).toBe(true);
  });

  test("emits warnings for lossy fields", async () => {
    const cfgDir = join(env.configDir, "skilltap");
    await writeFile(
      join(cfgDir, "installed.json"),
      JSON.stringify(V1_INSTALLED, null, 2),
    );
    await writeFile(
      join(cfgDir, "config.toml"),
      `
[security]
threshold = 3
max_size = 102400
`,
    );

    const result = await runMigrate({});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const warnings = result.value.warnings.join("\n");
    expect(warnings).toContain("threshold");
    expect(warnings).toContain("max_size");
  });

  test("no-op path does not include doctorReport", async () => {
    const result = await runMigrate({});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.alreadyMigrated).toBe(true);
    expect(result.value.doctorReport).toBeUndefined();
  });

  test("end-to-end migration: full v0.x setup translates cleanly", async () => {
    const cfgDir = join(env.configDir, "skilltap");

    const v1Installed = {
      version: 1,
      skills: [
        {
          name: "test-skill",
          repo: "github:owner/repo",
          ref: "v1.0",
          sha: "abc123",
          scope: "global",
          path: null,
          tap: null,
          also: [],
          installedAt: "2026-01-01T00:00:00.000Z",
          updatedAt: "2026-01-01T00:00:00.000Z",
        },
      ],
    };

    const v1Plugins = {
      version: 1,
      plugins: [
        {
          name: "test-plugin",
          description: "a test plugin",
          format: "skilltap",
          repo: "github:owner/plugin-repo",
          ref: "v1.0",
          sha: "def456",
          scope: "global",
          path: null,
          also: [],
          tap: null,
          components: [],
          installedAt: "2026-01-01T00:00:00.000Z",
          updatedAt: "2026-01-01T00:00:00.000Z",
          active: true,
        },
      ],
    };

    const v1Config = `
[defaults]
also = ["claude-code"]

[security.human]
scan = "static"
on_warn = "prompt"

[security.agent]
scan = "static"
on_warn = "fail"

["agent-mode"]
enabled = true
scope = "global"

[[security.overrides]]
match = "trusted-tap"
kind = "tap"
preset = "none"
`;

    const installedPath = join(cfgDir, "installed.json");
    const pluginsPath = join(cfgDir, "plugins.json");
    const configPath = join(cfgDir, "config.toml");

    await writeFile(installedPath, JSON.stringify(v1Installed, null, 2));
    await writeFile(pluginsPath, JSON.stringify(v1Plugins, null, 2));
    await writeFile(configPath, v1Config);

    const result = await runMigrate({});
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.alreadyMigrated).toBe(false);
    expect(result.value.scopes).toContain("global");

    // state.json was written with version 2
    const stateText = await Bun.file(join(cfgDir, "state.json")).text();
    const state = JSON.parse(stateText) as {
      version: number;
      skills: { name: string }[];
      plugins: { name: string }[];
    };
    expect(state.version).toBe(2);
    expect(state.skills.map((s) => s.name)).toContain("test-skill");
    expect(state.plugins.map((p) => p.name)).toContain("test-plugin");

    // config.toml is flat — no per-mode blocks, no [agent-mode]
    const configText = await Bun.file(configPath).text();
    expect(configText).not.toContain("[security.human]");
    expect(configText).not.toContain("[security.agent]");
    expect(configText).not.toContain("[agent-mode]");
    expect(configText).toContain("[security]");

    // .v1.bak files exist for all three originals
    expect(await pathExists(`${installedPath}.v1.bak`)).toBe(true);
    expect(await pathExists(`${pluginsPath}.v1.bak`)).toBe(true);
    expect(await pathExists(`${configPath}.v1.bak`)).toBe(true);

    // doctorReport is populated (doctor ran after migration)
    expect(result.value.doctorReport).toBeDefined();
  });

  test("malformed skilltap.toml produces a warning but migration still succeeds", async () => {
    const cfgDir = join(env.configDir, "skilltap");
    const installedPath = join(cfgDir, "installed.json");

    await writeFile(installedPath, JSON.stringify(V1_INSTALLED, null, 2));

    // Write a malformed skilltap.toml in the project root
    const projectRoot = env.homeDir;
    await writeFile(join(projectRoot, "skilltap.toml"), "[[invalid toml\n");

    const result = await runMigrate({ projectRoot });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.alreadyMigrated).toBe(false);
    const warnings = result.value.warnings.join("\n");
    expect(warnings).toContain("skilltap.toml");
    expect(warnings).toContain("did not parse cleanly");
  });
});
