import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { loadConfig, saveConfig } from "@skilltap/core";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { parse, stringify } from "smol-toml";
import {
  ConfigSchema,
  ON_WARN_MODES,
  SCAN_MODES,
  ScannerConfigSchema,
  SecurityConfigSchema,
} from "./config";

describe("SCAN_MODES", () => {
  test("includes semantic, static, none", () => {
    expect(SCAN_MODES).toContain("semantic");
    expect(SCAN_MODES).toContain("static");
    expect(SCAN_MODES).toContain("none");
  });
});

describe("ON_WARN_MODES", () => {
  test("includes prompt, fail, install", () => {
    expect(ON_WARN_MODES).toContain("prompt");
    expect(ON_WARN_MODES).toContain("fail");
    expect(ON_WARN_MODES).toContain("install");
  });
});

describe("SecurityConfigSchema", () => {
  test("applies V2 defaults", () => {
    const result = SecurityConfigSchema.parse({});
    expect(result.scan).toBe("static");
    expect(result.on_warn).toBe("install");
    expect(result.trust).toEqual([]);
  });

  test("accepts all valid scan values", () => {
    expect(SecurityConfigSchema.parse({ scan: "semantic" }).scan).toBe(
      "semantic",
    );
    expect(SecurityConfigSchema.parse({ scan: "static" }).scan).toBe("static");
    expect(SecurityConfigSchema.parse({ scan: "none" }).scan).toBe("none");
  });

  test("rejects legacy scan value 'off'", () => {
    expect(SecurityConfigSchema.safeParse({ scan: "off" }).success).toBe(false);
  });

  test("accepts all valid on_warn values", () => {
    expect(SecurityConfigSchema.parse({ on_warn: "prompt" }).on_warn).toBe(
      "prompt",
    );
    expect(SecurityConfigSchema.parse({ on_warn: "fail" }).on_warn).toBe(
      "fail",
    );
    expect(SecurityConfigSchema.parse({ on_warn: "install" }).on_warn).toBe(
      "install",
    );
  });

  test("rejects legacy on_warn value 'allow'", () => {
    expect(SecurityConfigSchema.safeParse({ on_warn: "allow" }).success).toBe(
      false,
    );
  });

  test("accepts trust glob patterns", () => {
    const result = SecurityConfigSchema.parse({
      trust: ["github.com/me/*", "internal-tap"],
    });
    expect(result.trust).toEqual(["github.com/me/*", "internal-tap"]);
  });

  test("legacy per-mode and override keys silently stripped", () => {
    const result = SecurityConfigSchema.safeParse({
      human: { scan: "semantic" },
      agent: { scan: "off" },
      overrides: [{ match: "x", kind: "tap", preset: "none" }],
      require_scan: true,
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    // Zod strips unknown keys — flat fields use defaults
    expect(result.data.scan).toBe("static");
    expect(result.data.on_warn).toBe("install");
  });
});

describe("ScannerConfigSchema", () => {
  test("applies defaults", () => {
    const result = ScannerConfigSchema.parse({});
    expect(result.agent_cli).toBe("");
    expect(result.ollama_model).toBe("");
    expect(result.threshold).toBe(5);
    expect(result.max_size).toBe(51200);
  });

  test("threshold min and max bounds", () => {
    expect(ScannerConfigSchema.parse({ threshold: 0 }).threshold).toBe(0);
    expect(ScannerConfigSchema.parse({ threshold: 10 }).threshold).toBe(10);
    expect(ScannerConfigSchema.safeParse({ threshold: -1 }).success).toBe(
      false,
    );
    expect(ScannerConfigSchema.safeParse({ threshold: 11 }).success).toBe(
      false,
    );
  });

  test("threshold must be integer", () => {
    expect(ScannerConfigSchema.safeParse({ threshold: 5.5 }).success).toBe(
      false,
    );
  });

  test("accepts agent_cli and ollama_model strings", () => {
    const result = ScannerConfigSchema.parse({
      agent_cli: "claude",
      ollama_model: "llama3",
    });
    expect(result.agent_cli).toBe("claude");
    expect(result.ollama_model).toBe("llama3");
  });
});

describe("ConfigSchema", () => {
  test("applies all defaults when empty", () => {
    const result = ConfigSchema.parse({});
    expect(result.defaults.also).toEqual([]);
    expect(result.defaults.yes).toBe(false);
    expect(result.defaults.scope).toBe("");
    expect(result.security.scan).toBe("static");
    expect(result.security.on_warn).toBe("install");
    expect(result.security.trust).toEqual([]);
    expect(result.scanner.agent_cli).toBe("");
    expect(result.scanner.threshold).toBe(5);
    expect(result.scanner.max_size).toBe(51200);
    expect(result.taps).toEqual([]);
    expect(result.default_git_host).toBe("https://github.com");
  });

  test("config has no agent-mode key", () => {
    const result = ConfigSchema.parse({});
    expect(
      (result as Record<string, unknown>)["agent-mode"],
    ).toBeUndefined();
  });

  test("config has no agent block", () => {
    const result = ConfigSchema.parse({});
    expect((result as Record<string, unknown>).agent).toBeUndefined();
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

  test("accepts full valid V2 config", () => {
    const result = ConfigSchema.parse({
      defaults: { also: ["claude-code", "cursor"], yes: true, scope: "global" },
      security: {
        scan: "semantic",
        on_warn: "fail",
        trust: ["github.com/corp/*"],
      },
      scanner: {
        agent_cli: "claude",
        threshold: 8,
      },
      taps: [{ name: "home", url: "https://example.com/tap.git" }],
    });
    expect(result.defaults.also).toEqual(["claude-code", "cursor"]);
    expect(result.defaults.yes).toBe(true);
    expect(result.defaults.scope).toBe("global");
    expect(result.security.scan).toBe("semantic");
    expect(result.security.on_warn).toBe("fail");
    expect(result.security.trust).toEqual(["github.com/corp/*"]);
    expect(result.scanner.agent_cli).toBe("claude");
    expect(result.scanner.threshold).toBe(8);
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
      security: { trust: ["home"] },
      scanner: { threshold: 7 },
      taps: [{ name: "home", url: "https://example.com/tap.git" }],
    });
    const toml = stringify(config as any);
    const parsed = parse(toml);
    const result = ConfigSchema.parse(parsed);
    expect(result.defaults.also).toEqual(["claude-code"]);
    expect(result.defaults.scope).toBe("global");
    expect(result.security.trust).toEqual(["home"]);
    expect(result.scanner.threshold).toBe(7);
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

  test("legacy v0.x and v2 keys silently stripped (extras ignored)", () => {
    const result = ConfigSchema.safeParse({
      "agent-mode": { enabled: true, scope: "project" },
      agent: { default: true },
      security: {
        human: { scan: "semantic" },
        agent: { scan: "off", on_warn: "allow" },
        overrides: [{ match: "x", kind: "tap", preset: "none" }],
        require_scan: true,
        agent_cli: "claude",
      },
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.security.scan).toBe("static");
    expect(
      (result.data as Record<string, unknown>)["agent-mode"],
    ).toBeUndefined();
    expect(
      (result.data as Record<string, unknown>).agent,
    ).toBeUndefined();
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
        trust: ["github.com/corp/*"],
      },
      scanner: {
        ...firstLoad.value.scanner,
        threshold: 8,
        agent_cli: "claude",
      },
    };

    const saveResult = await saveConfig(config);
    expect(saveResult.ok).toBe(true);

    const reloadResult = await loadConfig();
    expect(reloadResult.ok).toBe(true);
    if (!reloadResult.ok) return;

    expect(reloadResult.value.defaults.also).toEqual([
      "claude-code",
      "cursor",
    ]);
    expect(reloadResult.value.defaults.yes).toBe(true);
    expect(reloadResult.value.defaults.scope).toBe("global");
    expect(reloadResult.value.security.scan).toBe("semantic");
    expect(reloadResult.value.security.on_warn).toBe("fail");
    expect(reloadResult.value.security.trust).toEqual(["github.com/corp/*"]);
    expect(reloadResult.value.scanner.threshold).toBe(8);
    expect(reloadResult.value.scanner.agent_cli).toBe("claude");
  });
});
