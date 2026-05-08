import { describe, expect, test } from "bun:test";
import {
  coerceValue,
  formatConfigValue,
  getConfigValue,
  setConfigValue,
  validateSetKey,
} from "./config-keys";
import { ConfigSchema } from "./schemas/config";

const DEFAULT_CONFIG = ConfigSchema.parse({});

// ---------------------------------------------------------------------------
// getConfigValue
// ---------------------------------------------------------------------------

describe("getConfigValue", () => {
  test("gets a nested string field", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "defaults.scope");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value).toBe("");
  });

  test("gets a nested boolean field", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "defaults.yes");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value).toBe(false);
  });

  test("gets an array field", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "defaults.also");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value).toEqual([]);
  });

  test("gets a number field", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "updates.interval_hours");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value).toBe(24);
  });

  test("gets a whole section", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "defaults");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value).toEqual({ also: [], yes: false, scope: "" });
  });

  test("errors on agent-mode (removed)", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "agent-mode");
    expect(r.ok).toBe(false);
  });

  test("errors on unknown top-level key", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "nonexistent");
    expect(r.ok).toBe(false);
  });

  test("errors on unknown nested key", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "defaults.nonexistent");
    expect(r.ok).toBe(false);
  });

  test("errors on too-deep path", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "defaults.scope.deep");
    expect(r.ok).toBe(false);
  });

  test("errors on security.human (removed per-mode key)", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "security.human");
    expect(r.ok).toBe(false);
  });

  test("errors on security.agent (removed per-mode key)", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "security.agent");
    expect(r.ok).toBe(false);
  });

  test("gets security.scan (flat field)", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "security.scan");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value).toBe("static");
  });

  test("gets security.overrides (empty array)", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "security.overrides");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value).toEqual([]);
  });

  test("gets security.agent_cli", () => {
    const r = getConfigValue(DEFAULT_CONFIG, "security.agent_cli");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value).toBe("");
  });
});

// ---------------------------------------------------------------------------
// validateSetKey
// ---------------------------------------------------------------------------

describe("validateSetKey", () => {
  test("accepts all settable keys", () => {
    const settable = [
      "defaults.scope",
      "defaults.also",
      "defaults.yes",
      "security.agent_cli",
      "security.ollama_model",
      "security.threshold",
      "security.max_size",
      "updates.auto_update",
      "updates.interval_hours",
    ];
    for (const key of settable) {
      const r = validateSetKey(key);
      expect(r.ok).toBe(true);
    }
  });

  test("rejects agent-mode keys with removal message", () => {
    const r = validateSetKey("agent-mode.enabled");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error.hint).toContain("agent-mode has been removed");
  });

  test("rejects telemetry keys with hint", () => {
    const r = validateSetKey("telemetry.enabled");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error.hint).toContain("telemetry");
  });

  test("rejects per-mode security keys with config security hint", () => {
    for (const key of [
      "security.human.scan",
      "security.human.on_warn",
      "security.human.require_scan",
      "security.agent.scan",
      "security.agent.on_warn",
      "security.agent.require_scan",
    ]) {
      const r = validateSetKey(key);
      expect(r.ok).toBe(false);
    }
  });

  test("rejects security.overrides with trust hint", () => {
    const r = validateSetKey("security.overrides");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error.hint).toContain("--trust");
  });

  test("accepts flat security keys (scan, on_warn, require_scan)", () => {
    for (const key of [
      "security.scan",
      "security.on_warn",
      "security.require_scan",
    ]) {
      const r = validateSetKey(key);
      expect(r.ok).toBe(true);
    }
  });

  test("rejects security.agent with agent_cli hint", () => {
    const r = validateSetKey("security.agent");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error.hint).toContain("agent_cli");
  });

  test("rejects taps with hint", () => {
    const r = validateSetKey("taps");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error.hint).toContain("tap add/remove");
  });

  test("rejects unknown keys with settable keys list", () => {
    const r = validateSetKey("foo.bar");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error.hint).toContain("Settable keys");
    expect(r.error.hint).toContain("defaults.scope");
  });
});

// ---------------------------------------------------------------------------
// coerceValue
// ---------------------------------------------------------------------------

describe("coerceValue", () => {
  test("coerces boolean true variants", () => {
    for (const v of ["true", "1", "yes", "True", "YES"]) {
      const r = coerceValue([v], { type: "boolean" });
      expect(r.ok).toBe(true);
      if (r.ok) expect(r.value).toBe(true);
    }
  });

  test("coerces boolean false variants", () => {
    for (const v of ["false", "0", "no", "False", "NO"]) {
      const r = coerceValue([v], { type: "boolean" });
      expect(r.ok).toBe(true);
      if (r.ok) expect(r.value).toBe(false);
    }
  });

  test("rejects invalid boolean", () => {
    const r = coerceValue(["maybe"], { type: "boolean" });
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error.hint).toContain("true, false");
  });

  test("coerces integer number", () => {
    const r = coerceValue(["48"], { type: "number" });
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value).toBe(48);
  });

  test("rejects non-integer number", () => {
    expect(coerceValue(["3.5"], { type: "number" }).ok).toBe(false);
  });

  test("rejects non-numeric string as number", () => {
    expect(coerceValue(["abc"], { type: "number" }).ok).toBe(false);
  });

  test("validates enum value", () => {
    const def = { type: "enum" as const, enum: ["off", "patch", "minor"] };
    expect(coerceValue(["patch"], def).ok).toBe(true);
    expect(coerceValue(["major"], def).ok).toBe(false);
  });

  test("enum error shows accepted values", () => {
    const def = { type: "enum" as const, enum: ["off", "patch", "minor"] };
    const r = coerceValue(["major"], def);
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error.hint).toContain("off");
    expect(r.error.hint).toContain("patch");
  });

  test("enum accepts empty string when allowed", () => {
    const def = { type: "enum" as const, enum: ["", "global", "project"] };
    const r = coerceValue([""], def);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value).toBe("");
  });

  test("string[] takes all values", () => {
    const r = coerceValue(["claude-code", "cursor"], { type: "string[]" });
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value).toEqual(["claude-code", "cursor"]);
  });

  test("string[] with single value", () => {
    const r = coerceValue(["claude-code"], { type: "string[]" });
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value).toEqual(["claude-code"]);
  });

  test("string[] with zero values clears to empty array", () => {
    const r = coerceValue([], { type: "string[]" });
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value).toEqual([]);
  });

  test("string passes through", () => {
    const r = coerceValue(["claude"], { type: "string" });
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value).toBe("claude");
  });

  test("rejects multiple values for non-array types", () => {
    expect(coerceValue(["a", "b"], { type: "string" }).ok).toBe(false);
    expect(coerceValue(["1", "2"], { type: "number" }).ok).toBe(false);
    expect(coerceValue(["true", "false"], { type: "boolean" }).ok).toBe(false);
  });

  test("rejects zero values for non-array types", () => {
    expect(coerceValue([], { type: "string" }).ok).toBe(false);
    expect(coerceValue([], { type: "number" }).ok).toBe(false);
    expect(coerceValue([], { type: "boolean" }).ok).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// setConfigValue
// ---------------------------------------------------------------------------

describe("setConfigValue", () => {
  test("sets a nested string field immutably", () => {
    const updated = setConfigValue(DEFAULT_CONFIG, "defaults.scope", "global");
    expect(updated.defaults.scope).toBe("global");
    expect(DEFAULT_CONFIG.defaults.scope).toBe("");
  });

  test("sets an array field", () => {
    const updated = setConfigValue(DEFAULT_CONFIG, "defaults.also", [
      "claude-code",
      "cursor",
    ]);
    expect(updated.defaults.also).toEqual(["claude-code", "cursor"]);
  });

  test("sets a boolean field", () => {
    const updated = setConfigValue(DEFAULT_CONFIG, "defaults.yes", true);
    expect(updated.defaults.yes).toBe(true);
  });

  test("sets a number field", () => {
    const updated = setConfigValue(
      DEFAULT_CONFIG,
      "updates.interval_hours",
      48,
    );
    expect(updated.updates.interval_hours).toBe(48);
  });

  test("preserves other fields in the same section", () => {
    const updated = setConfigValue(DEFAULT_CONFIG, "defaults.scope", "project");
    expect(updated.defaults.yes).toBe(false);
    expect(updated.defaults.also).toEqual([]);
  });

  test("preserves other sections", () => {
    const updated = setConfigValue(DEFAULT_CONFIG, "defaults.scope", "global");
    expect(updated.security).toEqual(DEFAULT_CONFIG.security);
  });
});

// ---------------------------------------------------------------------------
// formatConfigValue
// ---------------------------------------------------------------------------

describe("formatConfigValue", () => {
  test("formats string array space-separated", () => {
    expect(formatConfigValue(["claude-code", "cursor"])).toBe(
      "claude-code cursor",
    );
  });

  test("formats empty array as empty string", () => {
    expect(formatConfigValue([])).toBe("");
  });

  test("formats object array as entry count", () => {
    expect(formatConfigValue([{ name: "home", url: "..." }])).toBe("[1 entry]");
    expect(
      formatConfigValue([
        { name: "a", url: "..." },
        { name: "b", url: "..." },
      ]),
    ).toBe("[2 entries]");
  });

  test("formats boolean", () => {
    expect(formatConfigValue(true)).toBe("true");
    expect(formatConfigValue(false)).toBe("false");
  });

  test("formats number", () => {
    expect(formatConfigValue(24)).toBe("24");
  });

  test("formats string", () => {
    expect(formatConfigValue("global")).toBe("global");
  });

  test("formats empty string", () => {
    expect(formatConfigValue("")).toBe("");
  });

  test("formats null/undefined as empty string", () => {
    expect(formatConfigValue(null)).toBe("");
    expect(formatConfigValue(undefined)).toBe("");
  });
});
