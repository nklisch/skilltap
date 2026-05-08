import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { loadConfig, saveConfig } from "@skilltap/core";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { parse, stringify } from "smol-toml";
import {
  ConfigSchema,
  ON_WARN_MODES,
  PRESET_VALUES,
  SECURITY_PRESETS,
  SecurityConfigSchema,
  TrustOverrideSchema,
} from "./config";

describe("ON_WARN_MODES", () => {
  test("includes allow", () => {
    expect(ON_WARN_MODES).toContain("allow");
    expect(ON_WARN_MODES).toContain("prompt");
    expect(ON_WARN_MODES).toContain("fail");
  });
});

describe("PRESET_VALUES", () => {
  test("covers all 4 presets", () => {
    for (const preset of SECURITY_PRESETS) {
      expect(PRESET_VALUES[preset]).toBeDefined();
    }
  });

  test("none preset has correct values", () => {
    expect(PRESET_VALUES.none).toEqual({
      scan: "off",
      on_warn: "allow",
      require_scan: false,
    });
  });

  test("relaxed preset has correct values", () => {
    expect(PRESET_VALUES.relaxed).toEqual({
      scan: "static",
      on_warn: "allow",
      require_scan: false,
    });
  });

  test("standard preset has correct values", () => {
    expect(PRESET_VALUES.standard).toEqual({
      scan: "static",
      on_warn: "prompt",
      require_scan: false,
    });
  });

  test("strict preset has correct values", () => {
    expect(PRESET_VALUES.strict).toEqual({
      scan: "semantic",
      on_warn: "fail",
      require_scan: true,
    });
  });
});

describe("TrustOverrideSchema", () => {
  test("validates valid override", () => {
    const result = TrustOverrideSchema.safeParse({
      match: "my-tap",
      kind: "tap",
      preset: "none",
    });
    expect(result.success).toBe(true);
  });

  test("validates source type override", () => {
    const result = TrustOverrideSchema.safeParse({
      match: "npm",
      kind: "source",
      preset: "standard",
    });
    expect(result.success).toBe(true);
  });

  test("rejects invalid kind", () => {
    const result = TrustOverrideSchema.safeParse({
      match: "my-tap",
      kind: "invalid",
      preset: "none",
    });
    expect(result.success).toBe(false);
  });

  test("rejects invalid preset", () => {
    const result = TrustOverrideSchema.safeParse({
      match: "my-tap",
      kind: "tap",
      preset: "ultra-strict",
    });
    expect(result.success).toBe(false);
  });
});

describe("SecurityConfigSchema", () => {
  test("applies flat defaults", () => {
    const result = SecurityConfigSchema.parse({});
    expect(result.scan).toBe("static");
    expect(result.on_warn).toBe("prompt");
    expect(result.require_scan).toBe(false);
    expect(result.agent_cli).toBe("");
    expect(result.threshold).toBe(5);
    expect(result.max_size).toBe(51200);
    expect(result.ollama_model).toBe("");
    expect(result.overrides).toEqual([]);
  });

  test("accepts all valid scan values", () => {
    expect(SecurityConfigSchema.parse({ scan: "static" }).scan).toBe("static");
    expect(SecurityConfigSchema.parse({ scan: "semantic" }).scan).toBe("semantic");
    expect(SecurityConfigSchema.parse({ scan: "off" }).scan).toBe("off");
  });

  test("rejects invalid scan value", () => {
    expect(SecurityConfigSchema.safeParse({ scan: "none" }).success).toBe(false);
    expect(SecurityConfigSchema.safeParse({ scan: "both" }).success).toBe(false);
  });

  test("accepts all valid on_warn values", () => {
    expect(SecurityConfigSchema.parse({ on_warn: "prompt" }).on_warn).toBe("prompt");
    expect(SecurityConfigSchema.parse({ on_warn: "fail" }).on_warn).toBe("fail");
    expect(SecurityConfigSchema.parse({ on_warn: "allow" }).on_warn).toBe("allow");
  });

  test("rejects invalid on_warn value", () => {
    expect(SecurityConfigSchema.safeParse({ on_warn: "ignore" }).success).toBe(false);
  });

  test("threshold min and max bounds", () => {
    expect(SecurityConfigSchema.parse({ threshold: 0 }).threshold).toBe(0);
    expect(SecurityConfigSchema.parse({ threshold: 10 }).threshold).toBe(10);
    expect(SecurityConfigSchema.safeParse({ threshold: -1 }).success).toBe(false);
    expect(SecurityConfigSchema.safeParse({ threshold: 11 }).success).toBe(false);
  });

  test("threshold must be integer", () => {
    expect(SecurityConfigSchema.safeParse({ threshold: 5.5 }).success).toBe(false);
  });

  test("accepts trust overrides array", () => {
    const result = SecurityConfigSchema.parse({
      overrides: [
        { match: "my-tap", kind: "tap", preset: "none" },
        { match: "npm", kind: "source", preset: "strict" },
      ],
    });
    expect(result.overrides).toHaveLength(2);
    expect(result.overrides[0].match).toBe("my-tap");
    expect(result.overrides[1].preset).toBe("strict");
  });

  test("legacy per-mode keys silently stripped (v1 back-compat)", () => {
    const result = SecurityConfigSchema.safeParse({
      human: { scan: "semantic" },
      agent: { scan: "off" },
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    // Zod strips unknown keys — flat fields use defaults
    expect(result.data.scan).toBe("static");
    expect(result.data.on_warn).toBe("prompt");
  });
});

describe("ConfigSchema", () => {
  test("applies all defaults when empty", () => {
    const result = ConfigSchema.parse({});
    expect(result.defaults.also).toEqual([]);
    expect(result.defaults.yes).toBe(false);
    expect(result.defaults.scope).toBe("");
    expect(result.security.scan).toBe("static");
    expect(result.security.on_warn).toBe("prompt");
    expect(result.security.require_scan).toBe(false);
    expect(result.taps).toEqual([]);
    expect(result.default_git_host).toBe("https://github.com");
  });

  test("config has no agent-mode key", () => {
    const result = ConfigSchema.parse({});
    expect((result as Record<string, unknown>)["agent-mode"]).toBeUndefined();
  });

  test("default_git_host defaults to https://github.com", () => {
    const result = ConfigSchema.safeParse({});
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.default_git_host).toBe("https://github.com");
  });

  test("default_git_host accepts custom URL", () => {
    const result = ConfigSchema.safeParse({
      default_git_host: "https://gitea.example.com",
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.default_git_host).toBe("https://gitea.example.com");
  });

  test("accepts full valid config with flat security", () => {
    const result = ConfigSchema.parse({
      defaults: { also: ["claude-code", "cursor"], yes: true, scope: "global" },
      security: {
        scan: "semantic",
        on_warn: "fail",
        require_scan: true,
        threshold: 8,
      },
      taps: [{ name: "home", url: "https://example.com/tap.git" }],
    });
    expect(result.defaults.also).toEqual(["claude-code", "cursor"]);
    expect(result.defaults.yes).toBe(true);
    expect(result.defaults.scope).toBe("global");
    expect(result.security.scan).toBe("semantic");
    expect(result.security.on_warn).toBe("fail");
    expect(result.taps[0].name).toBe("home");
  });

  test("defaults scope accepts empty string", () => {
    const result = ConfigSchema.parse({ defaults: { scope: "" } });
    expect(result.defaults.scope).toBe("");
  });

  test("rejects invalid defaults scope", () => {
    expect(
      ConfigSchema.safeParse({ defaults: { scope: "local" } }).success,
    ).toBe(false);
  });

  test("TOML round-trip preserves values", () => {
    const config = ConfigSchema.parse({
      defaults: { also: ["claude-code"], yes: false, scope: "global" },
      security: { threshold: 7 },
      taps: [{ name: "home", url: "https://example.com/tap.git" }],
    });
    const toml = stringify(config as any);
    const parsed = parse(toml);
    const result = ConfigSchema.parse(parsed);
    expect(result.defaults.also).toEqual(["claude-code"]);
    expect(result.defaults.scope).toBe("global");
    expect(result.security.threshold).toBe(7);
    expect(result.taps[0].name).toBe("home");
    expect(result.taps[0].url).toBe("https://example.com/tap.git");
  });

  test("unknown keys silently ignored", () => {
    const result = ConfigSchema.safeParse({
      defaults: { also: [], yes: true, scope: "global", unknownDefault: "x" },
      security: { scan: "static", unknownSecurity: 99 },
      unknownTopLevel: "ignored",
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.defaults.yes).toBe(true);
    expect(result.data.security.scan).toBe("static");
    expect(
      (result.data as Record<string, unknown>).unknownTopLevel,
    ).toBeUndefined();
  });

  test("partial config with only [security] block uses defaults elsewhere", () => {
    const result = ConfigSchema.parse({
      security: { scan: "semantic", on_warn: "fail" },
    });
    expect(result.security.scan).toBe("semantic");
    expect(result.security.on_warn).toBe("fail");
    expect(result.defaults.also).toEqual([]);
    expect(result.defaults.yes).toBe(false);
    expect(result.taps).toEqual([]);
  });

  test("v1 config with agent-mode and per-mode security keys parses without error (extras stripped)", () => {
    const result = ConfigSchema.safeParse({
      "agent-mode": { enabled: true, scope: "project" },
      security: {
        human: { scan: "semantic" },
        agent: { scan: "off", on_warn: "allow" },
      },
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    // Flat fields use defaults; per-mode and agent-mode keys stripped
    expect(result.data.security.scan).toBe("static");
    expect((result.data as Record<string, unknown>)["agent-mode"]).toBeUndefined();
  });
});

describe("Config I/O round-trip", () => {
  let configDir: string;

  beforeEach(async () => {
    configDir = await makeTmpDir();
    process.env.XDG_CONFIG_HOME = configDir;
  });

  afterEach(async () => {
    delete process.env.XDG_CONFIG_HOME;
    await removeTmpDir(configDir);
  });

  test("save config with all optional fields and reload produces same values", async () => {
    const firstLoad = await loadConfig();
    expect(firstLoad.ok).toBe(true);
    if (!firstLoad.ok) return;

    const config = {
      ...firstLoad.value,
      defaults: {
        also: ["claude-code", "cursor"],
        yes: true,
        scope: "global" as const,
      },
      security: {
        ...firstLoad.value.security,
        scan: "semantic" as const,
        on_warn: "fail" as const,
        require_scan: true,
        threshold: 8,
        agent_cli: "claude",
      },
    };

    const saveResult = await saveConfig(config);
    expect(saveResult.ok).toBe(true);

    const reloadResult = await loadConfig();
    expect(reloadResult.ok).toBe(true);
    if (!reloadResult.ok) return;

    expect(reloadResult.value.defaults.also).toEqual(["claude-code", "cursor"]);
    expect(reloadResult.value.defaults.yes).toBe(true);
    expect(reloadResult.value.defaults.scope).toBe("global");
    expect(reloadResult.value.security.scan).toBe("semantic");
    expect(reloadResult.value.security.on_warn).toBe("fail");
    expect(reloadResult.value.security.threshold).toBe(8);
  });
});
