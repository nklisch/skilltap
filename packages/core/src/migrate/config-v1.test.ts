import { describe, expect, test } from "bun:test";
import { migrateV1Config } from "./config-v1";

describe("migrateV1Config", () => {
  test("rejects null/undefined", () => {
    expect(migrateV1Config(null).ok).toBe(false);
    expect(migrateV1Config(undefined).ok).toBe(false);
  });

  test("rejects non-object", () => {
    expect(migrateV1Config("foo").ok).toBe(false);
    expect(migrateV1Config([]).ok).toBe(false);
  });

  test("translates empty config to fully-defaulted v2", () => {
    const result = migrateV1Config({});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.security.scan).toBe("static");
    expect(result.value.v2.security.on_warn).toBe("install");
    expect(result.value.v2.security.trust).toEqual([]);
    expect(result.value.v2.agent.default).toBe(false);
    expect(result.value.v2.agent.block).toBe(false);
    expect(result.value.warnings).toEqual([]);
    expect(result.value.httpTapsRejected).toEqual([]);
  });

  test("merges security.human + security.agent into single security block", () => {
    const result = migrateV1Config({
      security: {
        human: { scan: "static", on_warn: "prompt" },
        agent: { scan: "static", on_warn: "fail" },
      },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.security.scan).toBe("static");
    // fail > prompt, so picked stricter
    expect(result.value.v2.security.on_warn).toBe("fail");
    // Different on_warn → warning
    expect(result.value.warnings.some((w) => w.includes("on_warn"))).toBe(true);
  });

  test("picks stricter scan when human and agent differ", () => {
    const result = migrateV1Config({
      security: {
        human: { scan: "static", on_warn: "prompt" },
        agent: { scan: "semantic", on_warn: "prompt" },
      },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // semantic > static
    expect(result.value.v2.security.scan).toBe("semantic");
  });

  test("translates v1 'off' scan to v2 'none'", () => {
    const result = migrateV1Config({
      security: { human: { scan: "off", on_warn: "allow" } },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.security.scan).toBe("none");
    // 'allow' on_warn → 'install'
    expect(result.value.v2.security.on_warn).toBe("install");
  });

  test("translates security.overrides preset='none' into trust array", () => {
    const result = migrateV1Config({
      security: {
        overrides: [
          { match: "my-tap", kind: "tap", preset: "none" },
          { match: "github.com/corp/*", kind: "source", preset: "none" },
        ],
      },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.security.trust).toEqual(["my-tap", "github.com/corp/*"]);
    expect(result.value.warnings).toEqual([]);
  });

  test("warns and drops security.overrides with non-'none' presets", () => {
    const result = migrateV1Config({
      security: {
        overrides: [
          { match: "npm", kind: "source", preset: "strict" },
          { match: "my-tap", kind: "tap", preset: "none" },
        ],
      },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.security.trust).toEqual(["my-tap"]);
    expect(result.value.warnings.some((w) => w.includes("npm") && w.includes("strict"))).toBe(true);
  });

  test("translates agent-mode.enabled into agent.default", () => {
    const result = migrateV1Config({ "agent-mode": { enabled: true, scope: "project" } });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.agent.default).toBe(true);
    // scope dropped → warning
    expect(result.value.warnings.some((w) => w.includes("scope"))).toBe(true);
  });

  test("preserves defaults.also and defaults.scope", () => {
    const result = migrateV1Config({
      defaults: { also: ["claude-code", "cursor"], scope: "project" },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.defaults.also).toEqual(["claude-code", "cursor"]);
    expect(result.value.v2.defaults.scope).toBe("project");
  });

  test("warns and drops defaults.yes", () => {
    const result = migrateV1Config({ defaults: { yes: true } });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.warnings.some((w) => w.includes("defaults].yes"))).toBe(true);
  });

  test("rejects HTTP taps via httpTapsRejected", () => {
    const result = migrateV1Config({
      taps: [
        { name: "git-tap", url: "https://github.com/u/r" },
        { name: "http-tap", url: "https://api.example.com/v1", type: "http" },
      ],
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.taps).toEqual([{ name: "git-tap", url: "https://github.com/u/r" }]);
    expect(result.value.httpTapsRejected).toEqual([
      { name: "http-tap", url: "https://api.example.com/v1" },
    ]);
  });

  test("warns on dropped security fields (agent_cli, threshold, etc.)", () => {
    const result = migrateV1Config({
      security: {
        agent_cli: "/usr/local/bin/claude",
        threshold: 3,
        max_size: 102400,
        ollama_model: "llama3",
      },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const warnings = result.value.warnings.join("\n");
    expect(warnings).toContain("agent_cli");
    expect(warnings).toContain("threshold");
    expect(warnings).toContain("max_size");
    expect(warnings).toContain("ollama_model");
  });

  test("preserves builtin_tap, verbose, default_git_host", () => {
    const result = migrateV1Config({
      builtin_tap: false,
      verbose: false,
      default_git_host: "https://gitlab.com",
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.builtin_tap).toBe(false);
    expect(result.value.v2.verbose).toBe(false);
    expect(result.value.v2.default_git_host).toBe("https://gitlab.com");
  });

  test("end-to-end: realistic v1.0 config produces clean v2 + sensible warnings", () => {
    const result = migrateV1Config({
      defaults: { also: ["claude-code"], yes: false, scope: "project" },
      security: {
        agent_cli: "claude",
        threshold: 5,
        max_size: 51200,
        ollama_model: "",
        human: { scan: "static", on_warn: "prompt", require_scan: false },
        agent: { scan: "static", on_warn: "fail", require_scan: true },
        overrides: [{ match: "home", kind: "tap", preset: "none" }],
      },
      "agent-mode": { enabled: false, scope: "project" },
      taps: [{ name: "home", url: "https://gitea.example.com/n/t" }],
      builtin_tap: true,
      verbose: true,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.v2.defaults.also).toEqual(["claude-code"]);
    expect(result.value.v2.security.scan).toBe("static");
    expect(result.value.v2.security.on_warn).toBe("fail"); // stricter wins
    expect(result.value.v2.security.trust).toEqual(["home"]);
    expect(result.value.v2.agent.default).toBe(false);
    expect(result.value.v2.taps).toEqual([{ name: "home", url: "https://gitea.example.com/n/t" }]);
    expect(result.value.httpTapsRejected).toEqual([]);
  });
});
