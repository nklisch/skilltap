import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { loadConfig, saveConfig } from "@skilltap/core";
import { parse, stringify } from "smol-toml";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import {
  AgentModeSchema,
  ConfigSchema,
  ON_WARN_MODES,
  PRESET_VALUES,
  SECURITY_PRESETS,
  SecurityConfigSchema,
  SecurityModeSchema,
  TrustOverrideSchema,
} from "./config";

describe("SecurityModeSchema", () => {
  test("applies all defaults", () => {
    const result = SecurityModeSchema.parse({});
    expect(result.scan).toBe("static");
    expect(result.on_warn).toBe("prompt");
    expect(result.require_scan).toBe(false);
  });

  test("accepts all valid scan values", () => {
    expect(SecurityModeSchema.parse({ scan: "static" }).scan).toBe("static");
    expect(SecurityModeSchema.parse({ scan: "semantic" }).scan).toBe("semantic");
    expect(SecurityModeSchema.parse({ scan: "off" }).scan).toBe("off");
  });

  test("rejects invalid scan value", () => {
    expect(SecurityModeSchema.safeParse({ scan: "none" }).success).toBe(false);
    expect(SecurityModeSchema.safeParse({ scan: "both" }).success).toBe(false);
  });

  test("accepts all valid on_warn values", () => {
    expect(SecurityModeSchema.parse({ on_warn: "prompt" }).on_warn).toBe("prompt");
    expect(SecurityModeSchema.parse({ on_warn: "fail" }).on_warn).toBe("fail");
    expect(SecurityModeSchema.parse({ on_warn: "allow" }).on_warn).toBe("allow");
  });

  test("rejects invalid on_warn value", () => {
    expect(SecurityModeSchema.safeParse({ on_warn: "ignore" }).success).toBe(false);
  });
});

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
    expect(PRESET_VALUES.none).toEqual({ scan: "off", on_warn: "allow", require_scan: false });
  });

  test("relaxed preset has correct values", () => {
    expect(PRESET_VALUES.relaxed).toEqual({ scan: "static", on_warn: "allow", require_scan: false });
  });

  test("standard preset has correct values", () => {
    expect(PRESET_VALUES.standard).toEqual({ scan: "static", on_warn: "prompt", require_scan: false });
  });

  test("strict preset has correct values", () => {
    expect(PRESET_VALUES.strict).toEqual({ scan: "semantic", on_warn: "fail", require_scan: true });
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
  test("applies human mode defaults", () => {
    const result = SecurityConfigSchema.parse({});
    expect(result.human.scan).toBe("static");
    expect(result.human.on_warn).toBe("prompt");
    expect(result.human.require_scan).toBe(false);
  });

  test("applies agent mode defaults (strict by default)", () => {
    const result = SecurityConfigSchema.parse({});
    expect(result.agent.scan).toBe("static");
    expect(result.agent.on_warn).toBe("fail");
    expect(result.agent.require_scan).toBe(true);
  });

  test("applies shared defaults", () => {
    const result = SecurityConfigSchema.parse({});
    expect(result.agent_cli).toBe("");
    expect(result.threshold).toBe(5);
    expect(result.max_size).toBe(51200);
    expect(result.ollama_model).toBe("");
    expect(result.overrides).toEqual([]);
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

  test("accepts per-mode settings", () => {
    const result = SecurityConfigSchema.parse({
      human: { scan: "semantic", on_warn: "fail", require_scan: true },
      agent: { scan: "off", on_warn: "allow", require_scan: false },
    });
    expect(result.human.scan).toBe("semantic");
    expect(result.human.on_warn).toBe("fail");
    expect(result.human.require_scan).toBe(true);
    expect(result.agent.scan).toBe("off");
    expect(result.agent.on_warn).toBe("allow");
    expect(result.agent.require_scan).toBe(false);
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
});

describe("AgentModeSchema", () => {
  test("applies all defaults", () => {
    const result = AgentModeSchema.parse({});
    expect(result.enabled).toBe(false);
    expect(result.scope).toBe("project");
  });

  test("accepts all valid scope values", () => {
    expect(AgentModeSchema.parse({ scope: "global" }).scope).toBe("global");
    expect(AgentModeSchema.parse({ scope: "project" }).scope).toBe("project");
  });

  test("rejects invalid scope", () => {
    expect(AgentModeSchema.safeParse({ scope: "local" }).success).toBe(false);
  });
});

describe("ConfigSchema", () => {
  test("applies all defaults when empty", () => {
    const result = ConfigSchema.parse({});
    expect(result.defaults.also).toEqual([]);
    expect(result.defaults.yes).toBe(false);
    expect(result.defaults.scope).toBe("");
    expect(result.security.human.scan).toBe("static");
    expect(result.security.agent.scan).toBe("static");
    expect(result.security.agent.on_warn).toBe("fail");
    expect(result.security.agent.require_scan).toBe(true);
    expect(result["agent-mode"].enabled).toBe(false);
    expect(result["agent-mode"].scope).toBe("project");
    expect(result.taps).toEqual([]);
    expect(result.default_git_host).toBe("https://github.com");
  });

  test("default_git_host defaults to https://github.com", () => {
    const result = ConfigSchema.safeParse({});
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.default_git_host).toBe("https://github.com");
  });

  test("default_git_host accepts custom URL", () => {
    const result = ConfigSchema.safeParse({ default_git_host: "https://gitea.example.com" });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.default_git_host).toBe("https://gitea.example.com");
  });

  test("accepts full valid config with v2 security", () => {
    const result = ConfigSchema.parse({
      defaults: { also: ["claude-code", "cursor"], yes: true, scope: "global" },
      security: {
        human: { scan: "semantic", on_warn: "fail" },
        agent: { scan: "semantic", on_warn: "fail", require_scan: true },
        threshold: 8,
      },
      "agent-mode": { enabled: true, scope: "project" },
      taps: [{ name: "home", url: "https://example.com/tap.git" }],
    });
    expect(result.defaults.also).toEqual(["claude-code", "cursor"]);
    expect(result.defaults.yes).toBe(true);
    expect(result.defaults.scope).toBe("global");
    expect(result.security.human.scan).toBe("semantic");
    expect(result["agent-mode"].enabled).toBe(true);
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
    // biome-ignore lint/suspicious/noExplicitAny: smol-toml stringify types don't accept Config directly
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
      security: { human: { scan: "static" }, unknownSecurity: 99 },
      unknownTopLevel: "ignored",
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.defaults.yes).toBe(true);
    expect(result.data.security.human.scan).toBe("static");
    expect((result.data as Record<string, unknown>).unknownTopLevel).toBeUndefined();
  });

  test("partial config with only [security] block uses defaults elsewhere", () => {
    const result = ConfigSchema.parse({
      security: { human: { scan: "semantic", on_warn: "fail" } },
    });
    expect(result.security.human.scan).toBe("semantic");
    expect(result.security.human.on_warn).toBe("fail");
    expect(result.defaults.also).toEqual([]);
    expect(result.defaults.yes).toBe(false);
    expect(result["agent-mode"].enabled).toBe(false);
    expect(result.taps).toEqual([]);
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
      defaults: { also: ["claude-code", "cursor"], yes: true, scope: "global" as const },
      security: {
        ...firstLoad.value.security,
        human: { scan: "semantic" as const, on_warn: "fail" as const, require_scan: false },
        agent: { scan: "semantic" as const, on_warn: "fail" as const, require_scan: true },
        threshold: 8,
        agent_cli: "claude",
      },
      "agent-mode": { enabled: true, scope: "project" as const },
    };

    const saveResult = await saveConfig(config);
    expect(saveResult.ok).toBe(true);

    const reloadResult = await loadConfig();
    expect(reloadResult.ok).toBe(true);
    if (!reloadResult.ok) return;

    expect(reloadResult.value.defaults.also).toEqual(["claude-code", "cursor"]);
    expect(reloadResult.value.defaults.yes).toBe(true);
    expect(reloadResult.value.defaults.scope).toBe("global");
    expect(reloadResult.value.security.human.scan).toBe("semantic");
    expect(reloadResult.value.security.human.on_warn).toBe("fail");
    expect(reloadResult.value.security.threshold).toBe(8);
    expect(reloadResult.value["agent-mode"].enabled).toBe(true);
  });
});
