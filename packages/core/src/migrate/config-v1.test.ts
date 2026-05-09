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

  test("translates empty config to fully-defaulted V2", () => {
    const result = migrateV1Config({});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.migrated.security.scan).toBe("static");
    expect(result.value.migrated.security.on_warn).toBe("install");
    expect(result.value.migrated.security.trust).toEqual([]);
    expect(result.value.migrated.scanner.agent_cli).toBe("");
    expect(result.value.migrated.scanner.threshold).toBe(5);
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
    expect(result.value.migrated.security.scan).toBe("static");
    // fail > prompt, so picked stricter
    expect(result.value.migrated.security.on_warn).toBe("fail");
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
    expect(result.value.migrated.security.scan).toBe("semantic");
  });

  test("translates v0.x 'off' scan to V2 'none'", () => {
    const result = migrateV1Config({
      security: { human: { scan: "off", on_warn: "allow" } },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.migrated.security.scan).toBe("none");
    // 'allow' on_warn → 'install'
    expect(result.value.migrated.security.on_warn).toBe("install");
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
    expect(result.value.migrated.security.trust).toEqual([
      "my-tap",
      "github.com/corp/*",
    ]);
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
    expect(result.value.migrated.security.trust).toEqual(["my-tap"]);
    expect(
      result.value.warnings.some(
        (w) => w.includes("npm") && w.includes("strict"),
      ),
    ).toBe(true);
  });

  test("[agent-mode] is dropped with a warning, scope salvaged into defaults.scope", () => {
    const result = migrateV1Config({
      "agent-mode": { enabled: true, scope: "project" },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // scope transferred to defaults.scope → warning
    expect(result.value.migrated.defaults.scope).toBe("project");
    expect(result.value.warnings.some((w) => w.includes("agent-mode"))).toBe(
      true,
    );
    expect(result.value.warnings.some((w) => w.includes("scope"))).toBe(true);
  });

  test("[agent-mode].scope = 'global' with empty defaults.scope → defaults.scope set to 'global'", () => {
    const result = migrateV1Config({
      "agent-mode": { enabled: false, scope: "global" },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.migrated.defaults.scope).toBe("global");
    expect(result.value.warnings.some((w) => w.includes("scope"))).toBe(true);
  });

  test("[agent] block (legacy-v2.x) is dropped with a warning", () => {
    const result = migrateV1Config({
      agent: { default: true, block: false },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.warnings.some((w) => w.includes("[agent]"))).toBe(true);
  });

  test("[security.human] on_warn='prompt' + [security.agent] on_warn='fail' → flat on_warn='fail'", () => {
    const result = migrateV1Config({
      security: {
        human: { on_warn: "prompt" },
        agent: { on_warn: "fail" },
      },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.migrated.security.on_warn).toBe("fail");
    expect(result.value.warnings.some((w) => w.includes("on_warn"))).toBe(true);
  });

  test("preserves defaults.also and defaults.scope", () => {
    const result = migrateV1Config({
      defaults: { also: ["claude-code", "cursor"], scope: "project" },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.migrated.defaults.also).toEqual([
      "claude-code",
      "cursor",
    ]);
    expect(result.value.migrated.defaults.scope).toBe("project");
  });

  test("warns and drops defaults.yes", () => {
    const result = migrateV1Config({ defaults: { yes: true } });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.warnings.some((w) => w.includes("defaults].yes"))).toBe(
      true,
    );
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
    expect(result.value.migrated.taps).toEqual([
      { name: "git-tap", url: "https://github.com/u/r", type: "git" },
    ]);
    expect(result.value.httpTapsRejected).toEqual([
      { name: "http-tap", url: "https://api.example.com/v1" },
    ]);
  });

  test("translates v0.x [security.human] operational keys into [scanner]", () => {
    const result = migrateV1Config({
      security: {
        human: {
          agent_cli: "claude",
          threshold: 7,
          max_size: 102400,
          ollama_model: "llama3",
        },
      },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.migrated.scanner.agent_cli).toBe("claude");
    expect(result.value.migrated.scanner.threshold).toBe(7);
    expect(result.value.migrated.scanner.max_size).toBe(102400);
    expect(result.value.migrated.scanner.ollama_model).toBe("llama3");
    const warnings = result.value.warnings.join("\n");
    expect(warnings).toContain("[scanner]");
    expect(warnings).toContain("agent_cli");
    expect(warnings).toContain("threshold");
  });

  test("translates legacy-v2.x flat [security] operational keys into [scanner]", () => {
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
    expect(result.value.migrated.scanner.agent_cli).toBe(
      "/usr/local/bin/claude",
    );
    expect(result.value.migrated.scanner.threshold).toBe(3);
    expect(result.value.migrated.scanner.max_size).toBe(102400);
    expect(result.value.migrated.scanner.ollama_model).toBe("llama3");
  });

  test("drops require_scan with a 'set on_warn=fail' hint warning", () => {
    const result = migrateV1Config({
      security: {
        human: { require_scan: true },
        agent: { require_scan: false },
      },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const warnings = result.value.warnings.join("\n");
    expect(warnings).toContain("require_scan");
    expect(warnings).toContain("on_warn = 'fail'");
  });

  test("preserves builtin_tap, verbose, default_git_host", () => {
    const result = migrateV1Config({
      builtin_tap: false,
      verbose: false,
      default_git_host: "https://gitlab.com",
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.migrated.builtin_tap).toBe(false);
    expect(result.value.migrated.verbose).toBe(false);
    expect(result.value.migrated.default_git_host).toBe("https://gitlab.com");
  });

  test("[registry].allow_npm is dropped silently", () => {
    const result = migrateV1Config({
      registry: { enabled: ["skills.sh"], allow_npm: true },
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.migrated.registry.enabled).toEqual(["skills.sh"]);
    // No warning about allow_npm
    expect(
      result.value.warnings.some((w) => w.toLowerCase().includes("allow_npm")),
    ).toBe(false);
  });

  test("end-to-end: realistic v0.x config produces clean V2 + sensible warnings", () => {
    const result = migrateV1Config({
      defaults: { also: ["claude-code"], yes: false, scope: "project" },
      security: {
        human: {
          scan: "static",
          on_warn: "prompt",
          require_scan: false,
          agent_cli: "claude",
          threshold: 5,
          max_size: 51200,
          ollama_model: "",
        },
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
    expect(result.value.migrated.defaults.also).toEqual(["claude-code"]);
    expect(result.value.migrated.security.scan).toBe("static");
    expect(result.value.migrated.security.on_warn).toBe("fail"); // stricter wins
    expect(result.value.migrated.security.trust).toEqual(["home"]);
    expect(result.value.migrated.scanner.agent_cli).toBe("claude");
    expect(result.value.migrated.scanner.threshold).toBe(5);
    expect(result.value.migrated.taps).toEqual([
      { name: "home", url: "https://gitea.example.com/n/t", type: "git" },
    ]);
    expect(result.value.httpTapsRejected).toEqual([]);
  });
});
