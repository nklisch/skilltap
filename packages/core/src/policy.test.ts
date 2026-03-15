import { describe, expect, test } from "bun:test";
import { ConfigSchema } from "./schemas/config";
import { composePolicy, composePolicyForSource, mapAdapterToSourceType, resolveOverride } from "./policy";

const baseConfig = () => ConfigSchema.parse({});

describe("composePolicy — normal mode", () => {
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
    expect(result.value.agentMode).toBe(false);
  });

  test("--strict overrides on_warn=prompt", () => {
    const result = composePolicy(baseConfig(), { strict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("config human.on_warn=fail applies without flags", () => {
    const config = baseConfig();
    config.security.human.on_warn = "fail";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("config human.on_warn=allow propagates", () => {
    const config = baseConfig();
    config.security.human.on_warn = "allow";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("allow");
  });

  test("--no-strict overrides config human.on_warn=fail", () => {
    const config = baseConfig();
    config.security.human.on_warn = "fail";
    const result = composePolicy(config, { noStrict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("prompt");
  });

  test("human.require_scan=true blocks --skip-scan", () => {
    const config = baseConfig();
    config.security.human.require_scan = true;
    const result = composePolicy(config, { skipScan: true });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("require_scan");
  });

  test("--skip-scan passes when human.require_scan=false", () => {
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

  test("config human.scan=semantic without flag", () => {
    const config = baseConfig();
    config.security.human.scan = "semantic";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("config human.scan=off without --semantic stays off", () => {
    const config = baseConfig();
    config.security.human.scan = "off";
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
});

describe("composePolicy — agent mode", () => {
  const agentConfig = () => {
    const config = baseConfig();
    config["agent-mode"] = { enabled: true, scope: "project" };
    return config;
  };

  test("uses agent.on_warn and agent.require_scan from config", () => {
    const result = composePolicy(agentConfig(), {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // Default agent config is: on_warn=fail, require_scan=true
    expect(result.value.yes).toBe(true);
    expect(result.value.onWarn).toBe("fail");
    expect(result.value.requireScan).toBe(true);
    expect(result.value.skipScan).toBe(false);
    expect(result.value.agentMode).toBe(true);
  });

  test("agent mode with on_warn=allow config uses allow", () => {
    const config = agentConfig();
    config.security.agent.on_warn = "allow";
    config.security.agent.require_scan = false;
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("allow");
    expect(result.value.requireScan).toBe(false);
  });

  test("agent mode with scan=off config uses off (no enforced floor)", () => {
    const config = agentConfig();
    config.security.agent.scan = "off";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("off");
  });

  test("blocks --skip-scan when agent.require_scan=true", () => {
    const result = composePolicy(agentConfig(), { skipScan: true });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Agent mode");
    expect(result.error.message).toContain("--skip-scan");
  });

  test("allows --skip-scan when agent.require_scan=false", () => {
    const config = agentConfig();
    config.security.agent.require_scan = false;
    const result = composePolicy(config, { skipScan: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skipScan).toBe(true);
  });

  test("uses agent-mode.scope when no flag", () => {
    const result = composePolicy(agentConfig(), {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("project");
  });

  test("--global overrides agent-mode.scope", () => {
    const result = composePolicy(agentConfig(), { global: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("global");
  });

  test("--project overrides agent-mode.scope", () => {
    const config = agentConfig();
    config["agent-mode"].scope = "global";
    const result = composePolicy(config, { project: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("project");
  });

  test("preserves agent.scan=semantic", () => {
    const config = agentConfig();
    config.security.agent.scan = "semantic";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("uses defaults.also from config", () => {
    const config = agentConfig();
    config.defaults.also = ["claude-code"];
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.also).toEqual(["claude-code"]);
  });

  test("--strict still forces onWarn=fail", () => {
    const config = agentConfig();
    config.security.agent.on_warn = "allow";
    const result = composePolicy(config, { strict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("--no-strict overrides agent.on_warn=fail", () => {
    const result = composePolicy(agentConfig(), { noStrict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("prompt");
  });

  test("error when agent-mode.scope is empty and no flag passed", () => {
    const config = baseConfig();
    config["agent-mode"] = { enabled: true, scope: "" as "global" };
    const result = composePolicy(config, {});
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("requires a scope");
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
    // tap match wins even though source match also applies
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
    // Has both a tap match and a source:tap match — tap name wins
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
    // no override for git → human defaults
    expect(result.value.scanMode).toBe("static");
    expect(result.value.onWarn).toBe("prompt");
  });

  test("tap override applies preset values", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(config, {}, {
      tapName: "trusted-tap",
      sourceType: "tap",
    });
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
    // trusted-tap has "none" preset, but --strict should still set fail
    const result = composePolicyForSource(config, { strict: true }, {
      tapName: "trusted-tap",
      sourceType: "tap",
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("--skip-scan rejected when override preset has require_scan=true", () => {
    const config = baseWithOverrides();
    const result = composePolicyForSource(config, { skipScan: true }, { sourceType: "npm" });
    // "strict" has require_scan=true
    expect(result.ok).toBe(false);
  });

  test("--semantic flag overrides override preset scan=off", () => {
    const config = baseWithOverrides();
    // trusted-tap has "none" preset (scan=off), but --semantic should upgrade
    const result = composePolicyForSource(config, { semantic: true }, {
      tapName: "trusted-tap",
      sourceType: "tap",
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("--no-strict overrides override preset on_warn=fail", () => {
    const config = baseWithOverrides();
    // npm has "strict" preset (on_warn=fail), but --no-strict should override
    const result = composePolicyForSource(config, { noStrict: true }, { sourceType: "npm" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("prompt");
  });

  test("works in agent mode with override", () => {
    const config = baseWithOverrides();
    config["agent-mode"] = { enabled: true, scope: "project" };
    // Agent mode + trusted-tap "none" override
    const result = composePolicyForSource(config, {}, {
      tapName: "trusted-tap",
      sourceType: "tap",
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // "none" preset from override: scan=off, on_warn=allow
    expect(result.value.scanMode).toBe("off");
    expect(result.value.onWarn).toBe("allow");
    expect(result.value.agentMode).toBe(true);
    expect(result.value.yes).toBe(true);
  });

  test("agent mode without override uses agent mode defaults", () => {
    const config = baseWithOverrides();
    config["agent-mode"] = { enabled: true, scope: "project" };
    // "git" has no override
    const result = composePolicyForSource(config, {}, { sourceType: "git" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // Falls back to agent mode config: on_warn=fail, require_scan=true
    expect(result.value.onWarn).toBe("fail");
    expect(result.value.requireScan).toBe(true);
  });

  test("--skip-scan allowed when override preset has require_scan=false", () => {
    const config = baseWithOverrides();
    // trusted-tap has "none" preset (require_scan=false)
    const result = composePolicyForSource(config, { skipScan: true }, {
      tapName: "trusted-tap",
      sourceType: "tap",
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skipScan).toBe(true);
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
