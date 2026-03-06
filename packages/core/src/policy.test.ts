import { describe, expect, test } from "bun:test";
import { ConfigSchema } from "./schemas/config";
import { composePolicy } from "./policy";

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

  test("config on_warn=fail applies without flags", () => {
    const config = baseConfig();
    config.security.on_warn = "fail";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("--no-strict overrides config on_warn=fail", () => {
    const config = baseConfig();
    config.security.on_warn = "fail";
    const result = composePolicy(config, { noStrict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("prompt");
  });

  test("require_scan=true blocks --skip-scan", () => {
    const config = baseConfig();
    config.security.require_scan = true;
    const result = composePolicy(config, { skipScan: true });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("require_scan");
  });

  test("--skip-scan passes when require_scan=false", () => {
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

  test("config scan=semantic without flag", () => {
    const config = baseConfig();
    config.security.scan = "semantic";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("config scan=off without --semantic stays off", () => {
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
});

describe("composePolicy — agent mode", () => {
  const agentConfig = () => {
    const config = baseConfig();
    config["agent-mode"] = { enabled: true, scope: "project" };
    return config;
  };

  test("forces yes=true, onWarn=fail, requireScan=true", () => {
    const result = composePolicy(agentConfig(), {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(true);
    expect(result.value.onWarn).toBe("fail");
    expect(result.value.requireScan).toBe(true);
    expect(result.value.skipScan).toBe(false);
    expect(result.value.agentMode).toBe(true);
  });

  test("blocks --skip-scan", () => {
    const result = composePolicy(agentConfig(), { skipScan: true });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Agent mode");
    expect(result.error.message).toContain("--skip-scan");
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

  test("promotes scan=off to static", () => {
    const config = agentConfig();
    config.security.scan = "off";
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("static");
  });

  test("preserves scan=semantic", () => {
    const config = agentConfig();
    config.security.scan = "semantic";
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

  test("ignores --strict flag (already forced)", () => {
    const result = composePolicy(agentConfig(), { strict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("ignores --no-strict flag (agent mode overrides)", () => {
    const result = composePolicy(agentConfig(), { noStrict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
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
