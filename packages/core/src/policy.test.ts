import { describe, expect, test } from "bun:test";
import {
  composePolicy,
  composePolicyForSource,
  mapAdapterToSourceType,
  resolveOverride,
} from "./policy";
import { ConfigSchema } from "./schemas/config";

const baseConfig = () => ConfigSchema.parse({});

describe("composePolicy", () => {
  test("returns defaults with no flags", () => {
    const result = composePolicy(baseConfig(), {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(false);
    expect(result.value.onWarn).toBe("prompt");
    expect(result.value.requireScan).toBe(false);
    expect(result.value.skipScan).toBe(false);
    expect(result.value.scanMode).toBe("static");
    expect(result.value.scope).toBe("");
    expect(result.value.also).toEqual([]);
  });

  test("no agentMode field on EffectivePolicy", () => {
    const result = composePolicy(baseConfig(), {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect((result.value as Record<string, unknown>).agentMode).toBeUndefined();
  });

  test("--strict overrides on_warn=prompt", () => {
    const result = composePolicy(baseConfig(), { strict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("config security.on_warn=fail applies without flags", () => {
    const config = baseConfig();
    config.security.on_warn = "fail";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("config security.on_warn=allow propagates", () => {
    const config = baseConfig();
    config.security.on_warn = "allow";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("allow");
  });

  test("--no-strict overrides config security.on_warn=fail", () => {
    const config = baseConfig();
    config.security.on_warn = "fail";
    const result = composePolicy(config, { noStrict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("prompt");
  });

  test("security.require_scan=true blocks --skip-scan", () => {
    const config = baseConfig();
    config.security.require_scan = true;
    const result = composePolicy(config, { skipScan: true });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("require_scan");
  });

  test("--skip-scan passes when security.require_scan=false", () => {
    const result = composePolicy(baseConfig(), { skipScan: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skipScan).toBe(true);
  });

  test("--yes flag sets yes=true", () => {
    const result = composePolicy(baseConfig(), { yes: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(true);
  });

  test("config defaults.yes sets yes=true", () => {
    const config = baseConfig();
    config.defaults.yes = true;
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(true);
  });

  test("--project flag sets scope=project", () => {
    const result = composePolicy(baseConfig(), { project: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("project");
  });

  test("--global flag sets scope=global", () => {
    const result = composePolicy(baseConfig(), { global: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("global");
  });

  test("config defaults.scope used when no flag", () => {
    const config = baseConfig();
    config.defaults.scope = "project";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("project");
  });

  test("--project overrides config scope", () => {
    const config = baseConfig();
    config.defaults.scope = "global";
    const result = composePolicy(config, { project: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("project");
  });

  test("--semantic flag upgrades scanMode to semantic", () => {
    const result = composePolicy(baseConfig(), { semantic: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("config security.scan=semantic without flag", () => {
    const config = baseConfig();
    config.security.scan = "semantic";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("config security.scan=off without --semantic stays off", () => {
    const config = baseConfig();
    config.security.scan = "off";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("off");
  });

  test("config also propagated", () => {
    const config = baseConfig();
    config.defaults.also = ["claude-code", "cursor"];
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.also).toEqual(["claude-code", "cursor"]);
  });

  test("omitted CLI flags use config defaults across all resolution paths", () => {
    const config = baseConfig();
    config.defaults.yes = true;
    config.defaults.scope = "global";
    config.security.scan = "semantic";
    config.security.on_warn = "allow";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(true);
    expect(result.value.scope).toBe("global");
    expect(result.value.scanMode).toBe("semantic");
    expect(result.value.onWarn).toBe("allow");
  });
});

describe("resolveOverride", () => {
  test("returns null when no overrides", () => {
    const result = resolveOverride([], { sourceType: "git" });
    expect(result).toBeNull();
  });

  test("returns null when no match", () => {
    const result = resolveOverride(
      [{ match: "my-tap", kind: "tap", preset: "none" }],
      { sourceType: "npm" },
    );
    expect(result).toBeNull();
  });

  test("matches tap by name", () => {
    const result = resolveOverride(
      [{ match: "my-tap", kind: "tap", preset: "none" }],
      { tapName: "my-tap", sourceType: "tap" },
    );
    expect(result).toBe("none");
  });

  test("matches source type", () => {
    const result = resolveOverride(
      [{ match: "npm", kind: "source", preset: "strict" }],
      { sourceType: "npm" },
    );
    expect(result).toBe("strict");
  });

  test("named tap match beats source type match", () => {
    const result = resolveOverride(
      [
        { match: "npm", kind: "source", preset: "strict" },
        { match: "my-tap", kind: "tap", preset: "none" },
      ],
      { tapName: "my-tap", sourceType: "npm" },
    );
    expect(result).toBe("none");
  });

  test("falls back to source match when no tap match", () => {
    const result = resolveOverride(
      [
        { match: "other-tap", kind: "tap", preset: "strict" },
        { match: "npm", kind: "source", preset: "relaxed" },
      ],
      { tapName: "my-tap", sourceType: "npm" },
    );
    expect(result).toBe("relaxed");
  });

  test("matches source type when no tapName provided", () => {
    const result = resolveOverride(
      [{ match: "git", kind: "source", preset: "relaxed" }],
      { sourceType: "git" },
    );
    expect(result).toBe("relaxed");
  });

  test("first tap match wins when multiple tap overrides exist", () => {
    const result = resolveOverride(
      [
        { match: "my-tap", kind: "tap", preset: "none" },
        { match: "my-tap", kind: "tap", preset: "strict" },
      ],
      { tapName: "my-tap", sourceType: "tap" },
    );
    expect(result).toBe("none");
  });

  test("tap override ignores source type match for same source", () => {
    const result = resolveOverride(
      [
        { match: "tap", kind: "source", preset: "strict" },
        { match: "my-tap", kind: "tap", preset: "none" },
      ],
      { tapName: "my-tap", sourceType: "tap" },
    );
    expect(result).toBe("none");
  });
});

describe("composePolicyForSource", () => {
  const baseWithOverrides = () => {
    const config = baseConfig();
    config.security.overrides = [
      { match: "trusted-tap", kind: "tap", preset: "none" },
      { match: "npm", kind: "source", preset: "strict" },
    ];
    return config;
  };

  test("no override falls back to base policy", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(config, {}, { sourceType: "git" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("static");
    expect(result.value.onWarn).toBe("prompt");
  });

  test("tap override applies preset values", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(
      config,
      {},
      { tapName: "trusted-tap", sourceType: "tap" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // "none" preset: scan=off, on_warn=allow, require_scan=false
    expect(result.value.scanMode).toBe("off");
    expect(result.value.onWarn).toBe("allow");
    expect(result.value.requireScan).toBe(false);
  });

  test("source override applies preset values", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(config, {}, { sourceType: "npm" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // "strict" preset: scan=semantic, on_warn=fail, require_scan=true
    expect(result.value.scanMode).toBe("semantic");
    expect(result.value.onWarn).toBe("fail");
    expect(result.value.requireScan).toBe(true);
  });

  test("CLI --strict overrides trust tier preset", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(
      config,
      { strict: true },
      { tapName: "trusted-tap", sourceType: "tap" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("--skip-scan rejected when override preset has require_scan=true", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(
      config,
      { skipScan: true },
      { sourceType: "npm" },
    );
    expect(result.ok).toBe(false);
  });

  test("--semantic flag overrides override preset scan=off", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(
      config,
      { semantic: true },
      { tapName: "trusted-tap", sourceType: "tap" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("--no-strict overrides override preset on_warn=fail", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(
      config,
      { noStrict: true },
      { sourceType: "npm" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("prompt");
  });

  test("--skip-scan allowed when override preset has require_scan=false", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(
      config,
      { skipScan: true },
      { tapName: "trusted-tap", sourceType: "tap" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skipScan).toBe(true);
  });

  test("patches config.security.* directly (not per-mode key)", () => {
    const config = baseWithOverrides();
    config.security.on_warn = "fail";
    // trusted-tap has "none" (on_warn=allow) — should override config on_warn=fail
    const result = composePolicyForSource(
      config,
      {},
      { tapName: "trusted-tap", sourceType: "tap" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("allow");
  });
});

describe("mapAdapterToSourceType", () => {
  test("npm adapter maps to npm", () => {
    expect(mapAdapterToSourceType("npm")).toBe("npm");
  });

  test("local adapter maps to local", () => {
    expect(mapAdapterToSourceType("local")).toBe("local");
  });

  test("tap adapter maps to tap", () => {
    expect(mapAdapterToSourceType("tap")).toBe("tap");
  });

  test("git adapter maps to git", () => {
    expect(mapAdapterToSourceType("git")).toBe("git");
  });

  test("github adapter maps to git", () => {
    expect(mapAdapterToSourceType("github")).toBe("git");
  });

  test("http adapter maps to git", () => {
    expect(mapAdapterToSourceType("http")).toBe("git");
  });

  test("unknown adapter maps to git (default)", () => {
    expect(mapAdapterToSourceType("anything-else")).toBe("git");
  });
});
