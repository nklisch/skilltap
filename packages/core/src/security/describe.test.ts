import { describe, expect, test } from "bun:test";
import { describeSecurityMode } from "./describe";

describe("describeSecurityMode", () => {
  test("formats scan + on_warn", () => {
    expect(describeSecurityMode({ scan: "static", on_warn: "install" })).toBe(
      "static + install",
    );
  });

  test("formats semantic + fail", () => {
    expect(describeSecurityMode({ scan: "semantic", on_warn: "fail" })).toBe(
      "semantic + fail",
    );
  });

  test("formats none + prompt", () => {
    expect(describeSecurityMode({ scan: "none", on_warn: "prompt" })).toBe(
      "none + prompt",
    );
  });
});
