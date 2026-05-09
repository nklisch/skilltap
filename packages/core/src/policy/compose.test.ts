import { describe, expect, test } from "bun:test";
import { type Config, ConfigSchema } from "../schemas/config";
import { composePolicy, composePolicyForSource } from "./compose";

const baseConfig = (): Config => ConfigSchema.parse({});

describe("composePolicy — defaults", () => {
  test("empty config + no flags → fully-defaulted policy", () => {
    const result = composePolicy(baseConfig(), {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual({
      yes: false,
      scope: "",
      also: [],
      scanMode: "static",
      onWarn: "install",
      skipScan: false,
      trusted: false,
    });
  });
});

describe("composePolicy — scan + on_warn", () => {
  test("--deep forces semantic scan", () => {
    const result = composePolicy(baseConfig(), { deep: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("config security.scan respected when no --deep", () => {
    const config: Config = ConfigSchema.parse({
      security: { scan: "semantic" },
    });
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("semantic");
  });

  test("config security.scan = none stays none without --deep", () => {
    const config: Config = ConfigSchema.parse({
      security: { scan: "none" },
    });
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scanMode).toBe("none");
  });

  test("--strict forces on_warn=fail", () => {
    const result = composePolicy(baseConfig(), { strict: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("fail");
  });

  test("config on_warn respected when no --strict", () => {
    const config: Config = ConfigSchema.parse({
      security: { on_warn: "prompt" },
    });
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.onWarn).toBe("prompt");
  });
});

describe("composePolicy — scope", () => {
  test("--scope project takes precedence", () => {
    const config: Config = ConfigSchema.parse({
      defaults: { scope: "global" },
    });
    const result = composePolicy(config, { scope: "project" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("project");
  });

  test("--scope global takes precedence", () => {
    const config: Config = ConfigSchema.parse({
      defaults: { scope: "project" },
    });
    const result = composePolicy(config, { scope: "global" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("global");
  });

  test("config defaults.scope used when no --scope flag", () => {
    const config: Config = ConfigSchema.parse({
      defaults: { scope: "global" },
    });
    const result = composePolicy(config, {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.scope).toBe("global");
  });
});

describe("composePolicy — yes resolution", () => {
  test("--yes enables yes", () => {
    const result = composePolicy(baseConfig(), { yes: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(true);
  });

  test("--no-yes wins over --yes", () => {
    const result = composePolicy(baseConfig(), { yes: true, noYes: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(false);
  });

  test("no flags → yes is false", () => {
    const result = composePolicy(baseConfig(), {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.yes).toBe(false);
  });
});

describe("composePolicy — skipScan", () => {
  test("--skip-scan sets skipScan=true", () => {
    const result = composePolicy(baseConfig(), { skipScan: true });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skipScan).toBe(true);
  });

  test("default skipScan is false", () => {
    const result = composePolicy(baseConfig(), {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skipScan).toBe(false);
  });
});

describe("composePolicyForSource — trust list", () => {
  test("matching tap name → trusted + scan=none", () => {
    const config: Config = ConfigSchema.parse({
      security: { scan: "static", trust: ["home"] },
    });
    const result = composePolicyForSource(
      config,
      {},
      { sourceUrl: "...", tapName: "home" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.trusted).toBe(true);
    expect(result.value.scanMode).toBe("none");
  });

  test("matching URL pattern → trusted + scan=none", () => {
    const config: Config = ConfigSchema.parse({
      security: { scan: "semantic", trust: ["github.com/corp/*"] },
    });
    const result = composePolicyForSource(
      config,
      {},
      { sourceUrl: "github.com/corp/foo" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.trusted).toBe(true);
    expect(result.value.scanMode).toBe("none");
  });

  test("no match → trusted=false, scan unchanged", () => {
    const config: Config = ConfigSchema.parse({
      security: { scan: "static", trust: ["home"] },
    });
    const result = composePolicyForSource(
      config,
      {},
      { sourceUrl: "github.com/other/repo" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.trusted).toBe(false);
    expect(result.value.scanMode).toBe("static");
  });

  test("trust list empty → trusted=false", () => {
    const result = composePolicyForSource(
      baseConfig(),
      {},
      { sourceUrl: "anything" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.trusted).toBe(false);
  });
});
