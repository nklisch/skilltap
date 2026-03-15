import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdtemp, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ensureDirs,
  loadConfig,
  loadInstalled,
  migrateSecurityConfig,
  saveConfig,
  saveInstalled,
} from "./config";
import { ConfigSchema } from "./schemas/config";
import type { InstalledJson } from "./schemas/installed";

let tmpDir: string;
let savedXdg: string | undefined;

beforeEach(async () => {
  tmpDir = await mkdtemp(join(tmpdir(), "skilltap-test-"));
  savedXdg = process.env.XDG_CONFIG_HOME;
  process.env.XDG_CONFIG_HOME = tmpDir;
});

afterEach(async () => {
  if (savedXdg !== undefined) {
    process.env.XDG_CONFIG_HOME = savedXdg;
  } else {
    delete process.env.XDG_CONFIG_HOME;
  }
  await rm(tmpDir, { recursive: true, force: true });
});

async function dirExists(path: string): Promise<boolean> {
  try {
    const s = await stat(path);
    return s.isDirectory();
  } catch {
    return false;
  }
}

async function fileExists(path: string): Promise<boolean> {
  return Bun.file(path).exists();
}

describe("ensureDirs", () => {
  test("creates skilltap config directory", async () => {
    const result = await ensureDirs();
    expect(result.ok).toBe(true);
    expect(await dirExists(join(tmpDir, "skilltap"))).toBe(true);
  });

  test("creates taps subdirectory", async () => {
    await ensureDirs();
    expect(await dirExists(join(tmpDir, "skilltap", "taps"))).toBe(true);
  });

  test("creates cache subdirectory", async () => {
    await ensureDirs();
    expect(await dirExists(join(tmpDir, "skilltap", "cache"))).toBe(true);
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
      expect(result.value.security.human.scan).toBe("static");
      expect(result.value["agent-mode"].enabled).toBe(false);
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
      expect(result.value.security.human.scan).toBe("semantic");
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
  test("returns default when installed.json is missing", async () => {
    const result = await loadInstalled();
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.version).toBe(1);
      expect(result.value.skills).toEqual([]);
    }
  });

  test("parses existing installed.json", async () => {
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
    const result = await loadInstalled();
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.skills).toHaveLength(1);
      expect(result.value.skills[0].name).toBe("commit-helper");
    }
  });

  test("returns error for invalid JSON", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(join(configDir, "installed.json"), "{not valid json");
    const result = await loadInstalled();
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.message).toContain("Invalid JSON");
  });

  test("returns error for invalid schema", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(
      join(configDir, "installed.json"),
      JSON.stringify({ version: 99, skills: [] }),
    );
    const result = await loadInstalled();
    expect(result.ok).toBe(false);
    if (!result.ok)
      expect(result.error.message).toContain("Invalid installed.json");
  });
});

describe("migrateSecurityConfig", () => {
  test("migrates v1 flat config to v2 per-mode", () => {
    const raw = {
      security: {
        scan: "static",
        on_warn: "prompt",
        require_scan: false,
        agent: "claude",
        threshold: 7,
      },
    };
    const result = migrateSecurityConfig(raw);
    const sec = result.security as Record<string, unknown>;
    const human = sec.human as Record<string, unknown>;
    const agent = sec.agent as Record<string, unknown>;

    expect(human.scan).toBe("static");
    expect(human.on_warn).toBe("prompt");
    expect(human.require_scan).toBe(false);
    expect(agent.on_warn).toBe("fail");
    expect(agent.require_scan).toBe(true);
    expect(sec.agent_cli).toBe("claude");
    expect(sec.threshold).toBe(7);
    // Old flat fields should be gone
    expect(sec.scan).toBeUndefined();
    expect(sec.on_warn).toBeUndefined();
    expect(sec.require_scan).toBeUndefined();
  });

  test("migrates v1 with scan=off — agent gets static, not off", () => {
    const raw = { security: { scan: "off", on_warn: "prompt", require_scan: false } };
    const result = migrateSecurityConfig(raw);
    const sec = result.security as Record<string, unknown>;
    const agent = sec.agent as Record<string, unknown>;
    expect(agent.scan).toBe("static");
  });

  test("v2 config passes through unchanged", () => {
    const raw = {
      security: {
        human: { scan: "static", on_warn: "prompt", require_scan: false },
        agent: { scan: "semantic", on_warn: "fail", require_scan: true },
        agent_cli: "claude",
      },
    };
    const result = migrateSecurityConfig(raw);
    const sec = result.security as Record<string, unknown>;
    // Already v2 — no flat scan key
    expect((sec.human as Record<string, unknown>).scan).toBe("static");
    expect((sec.agent as Record<string, unknown>).scan).toBe("semantic");
  });

  test("missing security section passes through unchanged", () => {
    const raw = { defaults: { yes: false } };
    const result = migrateSecurityConfig(raw);
    expect(result).toEqual(raw);
  });

  test("empty security section passes through (no v1 fields)", () => {
    const raw = { security: {} };
    const result = migrateSecurityConfig(raw);
    // No flat scan key — treated as v2
    expect((result.security as Record<string, unknown>).scan).toBeUndefined();
  });

  test("idempotent — running twice produces same result", () => {
    const raw = {
      security: {
        scan: "semantic",
        on_warn: "fail",
        require_scan: true,
        agent: "",
      },
    };
    const once = migrateSecurityConfig(raw);
    const twice = migrateSecurityConfig(once);
    expect(twice).toEqual(once);
  });
});

describe("loadConfig — migration integration", () => {
  test("loads v1 config.toml and migrates to v2", async () => {
    const configDir = join(tmpDir, "skilltap");
    await ensureDirs();
    await Bun.write(
      join(configDir, "config.toml"),
      `[security]\nscan = "semantic"\non_warn = "fail"\nrequire_scan = true\nagent = "claude"\nthreshold = 8\n`,
    );
    const result = await loadConfig();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.security.human.scan).toBe("semantic");
    expect(result.value.security.human.on_warn).toBe("fail");
    expect(result.value.security.human.require_scan).toBe(true);
    expect(result.value.security.agent.on_warn).toBe("fail");
    expect(result.value.security.agent.require_scan).toBe(true);
    expect(result.value.security.agent_cli).toBe("claude");
    expect(result.value.security.threshold).toBe(8);
  });
});

describe("saveInstalled", () => {
  test("writes installed.json to disk", async () => {
    const installed: InstalledJson = { version: 1, skills: [] };
    const result = await saveInstalled(installed);
    expect(result.ok).toBe(true);
    expect(await fileExists(join(tmpDir, "skilltap", "installed.json"))).toBe(
      true,
    );
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
