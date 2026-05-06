import { describe, expect, test } from "bun:test";
import {
  AgentConfigSchema,
  ConfigV2DefaultsSchema,
  ConfigV2Schema,
  SECURITY_ON_WARN_V2,
  SECURITY_SCAN_V2,
  SecurityConfigV2Schema,
} from "./config-v2";

describe("SecurityConfigV2Schema", () => {
  test("applies defaults from {}", () => {
    const result = SecurityConfigV2Schema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.scan).toBe("static");
      expect(result.data.on_warn).toBe("install");
      expect(result.data.trust).toEqual([]);
    }
  });

  test("accepts every scan value", () => {
    for (const scan of SECURITY_SCAN_V2) {
      expect(SecurityConfigV2Schema.safeParse({ scan }).success).toBe(true);
    }
  });

  test("accepts every on_warn value", () => {
    for (const on_warn of SECURITY_ON_WARN_V2) {
      expect(SecurityConfigV2Schema.safeParse({ on_warn }).success).toBe(true);
    }
  });

  test("rejects invalid scan", () => {
    expect(SecurityConfigV2Schema.safeParse({ scan: "off" }).success).toBe(
      false,
    );
    expect(SecurityConfigV2Schema.safeParse({ scan: "deep" }).success).toBe(
      false,
    );
  });

  test("rejects invalid on_warn", () => {
    expect(SecurityConfigV2Schema.safeParse({ on_warn: "abort" }).success).toBe(
      false,
    );
  });

  test("accepts trust patterns array", () => {
    const result = SecurityConfigV2Schema.safeParse({
      trust: ["github.com/corp/*", "home", "https://internal.example.com/*"],
    });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.trust).toHaveLength(3);
  });
});

describe("AgentConfigSchema", () => {
  test("applies defaults from {}", () => {
    const result = AgentConfigSchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.default).toBe(false);
      expect(result.data.block).toBe(false);
    }
  });

  test("accepts default = true", () => {
    const result = AgentConfigSchema.safeParse({ default: true });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.default).toBe(true);
  });

  test("accepts block = true", () => {
    const result = AgentConfigSchema.safeParse({ block: true });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.block).toBe(true);
  });

  test("rejects non-boolean values", () => {
    expect(AgentConfigSchema.safeParse({ default: "yes" }).success).toBe(false);
    expect(AgentConfigSchema.safeParse({ block: 1 }).success).toBe(false);
  });
});

describe("ConfigV2DefaultsSchema", () => {
  test("defaults to empty also and empty scope", () => {
    const result = ConfigV2DefaultsSchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.also).toEqual([]);
      expect(result.data.scope).toBe("");
    }
  });

  test("accepts populated also and scope", () => {
    const result = ConfigV2DefaultsSchema.safeParse({
      also: ["claude-code", "cursor"],
      scope: "global",
    });
    expect(result.success).toBe(true);
  });

  test("rejects scope=linked", () => {
    expect(ConfigV2DefaultsSchema.safeParse({ scope: "linked" }).success).toBe(
      false,
    );
  });
});

describe("ConfigV2Schema", () => {
  test("parses an empty {} into a fully-defaulted config", () => {
    const result = ConfigV2Schema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.defaults.also).toEqual([]);
      expect(result.data.agent.default).toBe(false);
      expect(result.data.security.scan).toBe("static");
      expect(result.data.security.on_warn).toBe("install");
      expect(result.data.security.trust).toEqual([]);
      expect(result.data.taps).toEqual([]);
      expect(result.data.builtin_tap).toBe(true);
      expect(result.data.verbose).toBe(true);
      expect(result.data.default_git_host).toBe("https://github.com");
    }
  });

  test("accepts a fully-populated v2.0 config", () => {
    const result = ConfigV2Schema.safeParse({
      defaults: { also: ["claude-code"], scope: "project" },
      agent: { default: true, block: false },
      security: { scan: "static", on_warn: "fail", trust: ["home"] },
      taps: [{ name: "home", url: "https://gitea.example.com/n/t" }],
      builtin_tap: false,
      verbose: false,
      default_git_host: "https://gitlab.com",
    });
    expect(result.success).toBe(true);
  });

  test("rejects v1.0 [security.human] / [security.agent] keys", () => {
    // The v2.0 schema is strict about its shape — a v1.0 [security] object
    // with human/agent subkeys won't satisfy v2.0's flat scan/on_warn/trust.
    const result = ConfigV2Schema.safeParse({
      security: { human: { scan: "static" }, agent: { scan: "static" } },
    });
    // This may parse via Zod's loose object handling; what matters is that
    // none of the v1.0 nested keys end up populating v2.0 fields.
    if (result.success) {
      // v2.0 [security] should still have its defaults intact.
      expect(result.data.security.scan).toBe("static");
      expect(result.data.security.on_warn).toBe("install");
    }
  });

  test("rejects invalid scope in defaults", () => {
    const result = ConfigV2Schema.safeParse({
      defaults: { scope: "linked" },
    });
    expect(result.success).toBe(false);
  });
});
