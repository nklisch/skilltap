import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { createTestEnv, pathExists, type TestEnv } from "@skilltap/test-utils";
import {
  ensureDirs,
  loadConfig,
  loadSkillState,
  saveConfig,
  saveSkillState,
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

  test("parses existing V2 config.toml", async () => {
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

[scanner]
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
      expect(result.value.scanner.threshold).toBe(8);
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
      `[scanner]\nthreshold = 99\n`,
    );
    const result = await loadConfig();
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.message).toContain("Invalid config");
  });
});

describe("loadConfig — legacy hard-fail gate (Unit 1.4)", () => {
  const cases: Array<{ marker: string; toml: string }> = [
    { marker: "[security.human]", toml: `[security.human]\nscan = "static"\n` },
    { marker: "[security.agent]", toml: `[security.agent]\nscan = "static"\n` },
    {
      marker: "[[security.overrides]]",
      toml: `[[security.overrides]]\nmatch = "x"\npreset = "none"\n`,
    },
    {
      marker: "security.require_scan",
      toml: `[security]\nrequire_scan = true\n`,
    },
    {
      marker: "security.agent_cli",
      toml: `[security]\nagent_cli = "claude"\n`,
    },
    {
      marker: "security.ollama_model",
      toml: `[security]\nollama_model = "llama3"\n`,
    },
    {
      marker: "security.threshold",
      toml: `[security]\nthreshold = 7\n`,
    },
    {
      marker: "security.max_size",
      toml: `[security]\nmax_size = 99999\n`,
    },
    {
      marker: "[agent-mode]",
      toml: `["agent-mode"]\nenabled = true\n`,
    },
    {
      marker: "[agent]",
      toml: `[agent]\ndefault = true\n`,
    },
  ];

  for (const { marker, toml } of cases) {
    test(`hard-fails on ${marker} with skilltap migrate hint`, async () => {
      const configDir = join(tmpDir, "skilltap");
      await ensureDirs();
      await Bun.write(join(configDir, "config.toml"), toml);
      const result = await loadConfig();
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain(marker);
      expect(result.error.message).toContain("Legacy config detected");
      expect(result.error.hint).toBe("skilltap migrate");
    });
  }

  test("clean V2 config loads to ok", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(
      join(configDir, "config.toml"),
      `[security]\nscan = "static"\non_warn = "install"\ntrust = []\n\n[scanner]\nagent_cli = ""\nthreshold = 5\n`,
    );
    const result = await loadConfig();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.security.scan).toBe("static");
    expect(result.value.scanner.threshold).toBe(5);
  });

  test("empty config dir returns defaulted ConfigSchema.parse({})", async () => {
    // No config file exists yet — loadConfig writes the default template and
    // returns the defaulted parse result.
    const result = await loadConfig();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.security.scan).toBe("static");
    expect(result.value.security.on_warn).toBe("install");
    expect(result.value.scanner.threshold).toBe(5);
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
      scanner: { threshold: 3 },
    });
    await saveConfig(config);
    const loaded = await loadConfig();
    expect(loaded.ok).toBe(true);
    if (loaded.ok) {
      expect(loaded.value.defaults.also).toEqual(["cursor"]);
      expect(loaded.value.defaults.yes).toBe(true);
      expect(loaded.value.defaults.scope).toBe("project");
      expect(loaded.value.scanner.threshold).toBe(3);
    }
  });
});

describe("loadSkillState", () => {
  test("returns empty state when state.json is missing", async () => {
    const result = await loadSkillState();
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
    // state.json is absent — loadSkillState returns empty, not installed.json contents
    const result = await loadSkillState();
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.skills).toHaveLength(0);
    }
  });
});

describe("loadConfig — V2 split security/scanner blocks", () => {
  test("loads V2 [security] + [scanner] blocks", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(
      join(configDir, "config.toml"),
      `[security]\nscan = "semantic"\non_warn = "fail"\ntrust = ["my-tap"]\n\n[scanner]\nagent_cli = "claude"\nthreshold = 8\n`,
    );
    const result = await loadConfig();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.security.scan).toBe("semantic");
    expect(result.value.security.on_warn).toBe("fail");
    expect(result.value.security.trust).toEqual(["my-tap"]);
    expect(result.value.scanner.agent_cli).toBe("claude");
    expect(result.value.scanner.threshold).toBe(8);
  });
});

describe("saveSkillState", () => {
  test("writes state.json to disk (Phase 31c-c-2d-1: state.json is canonical)", async () => {
    const installed: InstalledJson = { version: 1, skills: [] };
    const result = await saveSkillState(installed);
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
    await saveSkillState(installed);
    const loaded = await loadSkillState();
    expect(loaded.ok).toBe(true);
    if (loaded.ok) {
      expect(loaded.value.skills).toHaveLength(1);
      expect(loaded.value.skills[0].name).toBe("my-skill");
      expect(loaded.value.skills[0].tap).toBe("home");
      expect(loaded.value.skills[0].also).toEqual(["claude-code"]);
    }
  });
});
