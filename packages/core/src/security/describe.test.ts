import { describe, expect, test } from "bun:test";
import { describeSecurityMode, matchPreset } from "./describe";
import { PRESET_VALUES, SECURITY_PRESETS } from "../schemas/config";

describe("matchPreset", () => {
  test("returns preset name for none preset", () => {
    expect(matchPreset(PRESET_VALUES.none)).toBe("none");
  });

  test("returns preset name for relaxed preset", () => {
    expect(matchPreset(PRESET_VALUES.relaxed)).toBe("relaxed");
  });

  test("returns preset name for standard preset", () => {
    expect(matchPreset(PRESET_VALUES.standard)).toBe("standard");
  });

  test("returns preset name for strict preset", () => {
    expect(matchPreset(PRESET_VALUES.strict)).toBe("strict");
  });

  test("returns null for custom combo", () => {
    expect(matchPreset({ scan: "static", on_warn: "fail", require_scan: false })).toBeNull();
  });

  test("returns null for another custom combo", () => {
    expect(matchPreset({ scan: "off", on_warn: "fail", require_scan: true })).toBeNull();
  });
});

describe("describeSecurityMode", () => {
  test("describes none preset", () => {
    const result = describeSecurityMode(PRESET_VALUES.none);
    expect(result).toBe("none (off + allow)");
  });

  test("describes relaxed preset", () => {
    const result = describeSecurityMode(PRESET_VALUES.relaxed);
    expect(result).toBe("relaxed (static + allow)");
  });

  test("describes standard preset", () => {
    const result = describeSecurityMode(PRESET_VALUES.standard);
    expect(result).toBe("standard (static + prompt)");
  });

  test("describes strict preset with require scan", () => {
    const result = describeSecurityMode(PRESET_VALUES.strict);
    expect(result).toBe("strict (semantic + fail + require scan)");
  });

  test("describes custom combo with 'custom' prefix", () => {
    const result = describeSecurityMode({ scan: "static", on_warn: "fail", require_scan: false });
    expect(result).toBe("custom (static + fail)");
  });

  test("describes custom combo with require_scan", () => {
    const result = describeSecurityMode({ scan: "static", on_warn: "fail", require_scan: true });
    expect(result).toBe("custom (static + fail + require scan)");
  });

  test("all 4 presets labeled correctly", () => {
    for (const preset of SECURITY_PRESETS) {
      const desc = describeSecurityMode(PRESET_VALUES[preset]);
      expect(desc).toStartWith(preset);
    }
  });

  test("near-miss: strict but require_scan=false is custom", () => {
    const result = describeSecurityMode({ scan: "semantic", on_warn: "fail", require_scan: false });
    expect(result).toStartWith("custom");
    expect(result).toContain("semantic");
    expect(result).toContain("fail");
    expect(result).not.toContain("require scan");
  });

  test("near-miss: standard but on_warn=fail is custom", () => {
    const result = describeSecurityMode({ scan: "static", on_warn: "fail", require_scan: false });
    expect(result).toStartWith("custom");
  });

  test("near-miss: relaxed but require_scan=true is custom", () => {
    const result = describeSecurityMode({ scan: "static", on_warn: "allow", require_scan: true });
    expect(result).toStartWith("custom");
    expect(result).toContain("require scan");
  });
});
