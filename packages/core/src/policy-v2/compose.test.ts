import { describe, expect, test } from "bun:test";
import { type ConfigV2, ConfigV2Schema } from "../schemas/config-v2";
import { composeV2, composeV2ForSource } from "./compose";

const baseConfig = (): ConfigV2 => ConfigV2Schema.parse({});

describe("composeV2 — defaults", () => {
  test("empty config + no flags + no env → fully-defaulted v2 policy", () => {
    const result = composeV2(baseConfig(), {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual({
      yes: false,
      agent: false,
      scope: "",
      also: [],
      scanMode: "static",
      onWarn: "install",
      skipScan: false,
      trusted: false,
    });
  });
});

describe("composeV2 — agent resolution", () => {
  test("--agent flag enables agent mode", () => {
    const result = composeV2(baseConfig(), { agent: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.agent).toBe(true);
    expect(result.value.yes).toBe(true); // agent implies yes
  });

  test("--no-agent overrides everything else", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ agent: { default: true } });
    const result = composeV2(config, { noAgent: true, agent: true }, { agent: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.agent).toBe(false);
  });

  test("env SKILLTAP_AGENT enables agent mode", () => {
    const result = composeV2(baseConfig(), {}, { agent: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.agent).toBe(true);
  });

  test("config.agent.default enables agent mode (sticky)", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ agent: { default: true } });
    const result = composeV2(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.agent).toBe(true);
  });

  test("agent.block rejects --agent with helpful error", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ agent: { block: true } });
    const result = composeV2(config, { agent: true });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("blocked");
    expect(result.error.message).toContain("agent.block");
  });

  test("agent.block does NOT trigger when agent isn't requested", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ agent: { block: true } });
    const result = composeV2(config, {});
    expect(result.ok).toBe(true);
  });
});

describe("composeV2 — scan + on_warn", () => {
  test("--deep forces semantic scan", () => {
    const result = composeV2(baseConfig(), { deep: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("config security.scan respected when no --deep", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ security: { scan: "semantic" } });
    const result = composeV2(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("config security.scan = none stays none without --deep", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ security: { scan: "none" } });
    const result = composeV2(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("none");
  });

  test("--strict forces on_warn=fail", () => {
    const result = composeV2(baseConfig(), { strict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("--no-strict reverts to config", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ security: { on_warn: "prompt" } });
    const result = composeV2(config, { strict: true, noStrict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("prompt");
  });
});

describe("composeV2 — scope", () => {
  test("--project takes precedence", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ defaults: { scope: "global" } });
    const result = composeV2(config, { project: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("project");
  });

  test("--global takes precedence", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ defaults: { scope: "project" } });
    const result = composeV2(config, { global: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("global");
  });

  test("config defaults.scope used when no flag", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ defaults: { scope: "global" } });
    const result = composeV2(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("global");
  });
});

describe("composeV2 — yes resolution", () => {
  test("--yes enables yes", () => {
    const result = composeV2(baseConfig(), { yes: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(true);
  });

  test("--no-yes wins over --yes", () => {
    const result = composeV2(baseConfig(), { yes: true, noYes: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(false);
  });

  test("--no-yes overrides agent's implicit yes", () => {
    const result = composeV2(baseConfig(), { agent: true, noYes: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.agent).toBe(true);
    expect(result.value.yes).toBe(false);
  });
});

describe("composeV2ForSource — trust list", () => {
  test("matching tap name → trusted + scan=none", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({
      security: { scan: "static", trust: ["home"] },
    });
    const result = composeV2ForSource(config, {}, { sourceUrl: "...", tapName: "home" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.trusted).toBe(true);
    expect(result.value.scanMode).toBe("none");
  });

  test("matching URL pattern → trusted + scan=none", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({
      security: { scan: "semantic", trust: ["github.com/corp/*"] },
    });
    const result = composeV2ForSource(config, {}, { sourceUrl: "github.com/corp/foo" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.trusted).toBe(true);
    expect(result.value.scanMode).toBe("none");
  });

  test("no match → trusted=false, scan unchanged", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({
      security: { scan: "static", trust: ["home"] },
    });
    const result = composeV2ForSource(config, {}, { sourceUrl: "github.com/other/repo" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.trusted).toBe(false);
    expect(result.value.scanMode).toBe("static");
  });

  test("trust list empty → trusted=false", () => {
    const result = composeV2ForSource(baseConfig(), {}, { sourceUrl: "anything" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.trusted).toBe(false);
  });

  test("propagates agent-block error from base compose", () => {
    const config: ConfigV2 = ConfigV2Schema.parse({ agent: { block: true } });
    const result = composeV2ForSource(
      config,
      { agent: true },
      { sourceUrl: "anything" },
    );
    expect(result.ok).toBe(false);
  });
});
