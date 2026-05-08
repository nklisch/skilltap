import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { createTestEnv, pathExists, type TestEnv } from "@skilltap/test-utils";
import {
  ensureDirs,
  loadConfig,
  loadInstalled,
  saveConfig,
  saveInstalled,
} from "./config";
import { ConfigSchema } from "./schemas/config";
import type { InstalledJson } from "./schemas/installed";

let env: TestEnv;
let tmpDir: string;

beforeEach(async () => {
  env = await createTestEnv();
  tmpDir = env.configDir;
});

afterEach(async () => {
  await env.cleanup();
});

async function fileExists(path: string): Promise<boolean> {
  return Bun.file(path).exists();
}

describe("ensureDirs", () => {
  test("creates skilltap config directory", async () => {
    const result = await ensureDirs();
    expect(result.ok).toBe(true);
    expect(await pathExists(join(tmpDir, "skilltap"))).toBe(true);
  });

  test("creates taps subdirectory", async () => {
    await ensureDirs();
    expect(await pathExists(join(tmpDir, "skilltap", "taps"))).toBe(true);
  });

  test("creates cache subdirectory", async () => {
    await ensureDirs();
    expect(await pathExists(join(tmpDir, "skilltap", "cache"))).toBe(true);
  });

  test("is idempotent (can be called multiple times)", async () => {
    const r1 = await ensureDirs();
    const r2 = await ensureDirs();
    expect(r1.ok).toBe(true);
    expect(r2.ok).toBe(true);
  });
});

describe("loadConfig", () => {
  test("creates default config file when missing", async () => {
    const result = await loadConfig();
    expect(result.ok).toBe(true);
    expect(await fileExists(join(tmpDir, "skilltap", "config.toml"))).toBe(
      true,
    );
  });

  test("returns default config values when file missing", async () => {
    const result = await loadConfig();
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.defaults.also).toEqual([]);
      expect(result.value.defaults.yes).toBe(false);
      expect(result.value.defaults.scope).toBe("");
      expect(result.value.security.scan).toBe("static");
      expect(result.value.taps).toEqual([]);
    }
  });

  test("parses existing config.toml", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(
      join(configDir, "config.toml"),
      `
[defaults]
yes = true
scope = "global"
also = ["claude-code"]

[security]
scan = "semantic"
threshold = 8
`,
    );
    const result = await loadConfig();
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.defaults.yes).toBe(true);
      expect(result.value.defaults.scope).toBe("global");
      expect(result.value.defaults.also).toEqual(["claude-code"]);
      expect(result.value.security.scan).toBe("semantic");
      expect(result.value.security.threshold).toBe(8);
    }
  });

  test("returns error for invalid TOML", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(join(configDir, "config.toml"), "not = valid toml = [bad]");
    const result = await loadConfig();
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.message).toContain("Invalid TOML");
  });

  test("returns error for invalid schema", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(
      join(configDir, "config.toml"),
      `[security]\nthreshold = 99\n`,
    );
    const result = await loadConfig();
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.message).toContain("Invalid config");
  });
});

describe("saveConfig", () => {
  test("writes config.toml to disk", async () => {
    const config = ConfigSchema.parse({});
    const result = await saveConfig(config);
    expect(result.ok).toBe(true);
    expect(await fileExists(join(tmpDir, "skilltap", "config.toml"))).toBe(
      true,
    );
  });

  test("round-trip: save then load produces equivalent config", async () => {
    const config = ConfigSchema.parse({
      defaults: { also: ["cursor"], yes: true, scope: "project" },
      security: { threshold: 3 },
    });
    await saveConfig(config);
    const loaded = await loadConfig();
    expect(loaded.ok).toBe(true);
    if (loaded.ok) {
      expect(loaded.value.defaults.also).toEqual(["cursor"]);
      expect(loaded.value.defaults.yes).toBe(true);
      expect(loaded.value.defaults.scope).toBe("project");
      expect(loaded.value.security.threshold).toBe(3);
    }
  });
});

describe("loadInstalled", () => {
  test("returns empty state when state.json is missing", async () => {
    const result = await loadInstalled();
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.version).toBe(1);
      expect(result.value.skills).toEqual([]);
    }
  });

  test("installed.json is ignored (v0.x fallback removed)", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(
      join(configDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "commit-helper",
            repo: "https://example.com/repo.git",
            ref: "v1.0.0",
            sha: "abc123",
            scope: "global",
            path: null,
            tap: null,
            also: [],
            installedAt: "2026-02-28T12:00:00.000Z",
            updatedAt: "2026-02-28T12:00:00.000Z",
          },
        ],
      }),
    );
    // state.json is absent — loadInstalled returns empty, not installed.json contents
    const result = await loadInstalled();
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.skills).toHaveLength(0);
    }
  });
});

describe("loadConfig — flat security config", () => {
  test("loads flat [security] block from config.toml", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(
      join(configDir, "config.toml"),
      `[security]\nscan = "semantic"\non_warn = "fail"\nrequire_scan = true\nagent_cli = "claude"\nthreshold = 8\n`,
    );
    const result = await loadConfig();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.security.scan).toBe("semantic");
    expect(result.value.security.on_warn).toBe("fail");
    expect(result.value.security.require_scan).toBe(true);
    expect(result.value.security.agent_cli).toBe("claude");
    expect(result.value.security.threshold).toBe(8);
  });

  test("v1 config with per-mode and agent-mode keys parses without error (extras stripped)", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(
      join(configDir, "config.toml"),
      `[security.human]\nscan = "semantic"\n[security.agent]\nscan = "off"\n["agent-mode"]\nenabled = true\nscope = "project"\n`,
    );
    const result = await loadConfig();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // Per-mode and agent-mode keys are stripped; flat fields use defaults
    expect(result.value.security.scan).toBe("static");
  });
});

describe("saveInstalled", () => {
  test("writes state.json to disk (Phase 31c-c-2d-1: state.json is canonical)", async () => {
    const installed: InstalledJson = { version: 1, skills: [] };
    const result = await saveInstalled(installed);
    expect(result.ok).toBe(true);
    expect(await fileExists(join(tmpDir, "skilltap", "state.json"))).toBe(true);
  });

  test("round-trip: save then load produces equivalent data", async () => {
    const installed: InstalledJson = {
      version: 1,
      skills: [
        {
          name: "my-skill",
          description: "",
          repo: "https://example.com/repo.git",
          ref: "main",
          sha: "deadbeef",
          scope: "global",
          path: null,
          tap: "home",
          also: ["claude-code"],
          installedAt: "2026-02-28T12:00:00.000Z",
          updatedAt: "2026-02-28T12:00:00.000Z",
        },
      ],
    };
    await saveInstalled(installed);
    const loaded = await loadInstalled();
    expect(loaded.ok).toBe(true);
    if (loaded.ok) {
      expect(loaded.value.skills).toHaveLength(1);
      expect(loaded.value.skills[0].name).toBe("my-skill");
      expect(loaded.value.skills[0].tap).toBe("home");
      expect(loaded.value.skills[0].also).toEqual(["claude-code"]);
    }
  });
});
