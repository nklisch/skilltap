import { describe, expect, test } from "bun:test";
import { createMockAgentBinary } from "@skilltap/test-utils";
import { createCustomAdapter } from "../custom";
import { ScanError } from "../../types";

describe("createCustomAdapter — detect", () => {
  test("returns true when binary file exists", async () => {
    const { binaryPath, cleanup } = await createMockAgentBinary(
      '{"score": 0, "reason": "ok"}',
    );
    try {
      const adapter = createCustomAdapter(binaryPath);
      expect(await adapter.detect()).toBe(true);
    } finally {
      await cleanup();
    }
  });

  test("returns false when binary file missing", async () => {
    const adapter = createCustomAdapter("/nonexistent/path/to/binary-xyz");
    expect(await adapter.detect()).toBe(false);
  });
});

describe("createCustomAdapter — invoke", () => {
  test("valid response → ok(parsed)", async () => {
    const { binaryPath, cleanup } = await createMockAgentBinary(
      '{"score": 8, "reason": "malicious pattern detected"}',
    );
    try {
      const adapter = createCustomAdapter(binaryPath);
      const result = await adapter.invoke("test prompt");
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.score).toBe(8);
      expect(result.value.reason).toBe("malicious pattern detected");
    } finally {
      await cleanup();
    }
  });

  test("garbled output → ok({ score: 0, reason: ... })", async () => {
    const { binaryPath, cleanup } = await createMockAgentBinary(
      "not json at all!!@#$%",
    );
    try {
      const adapter = createCustomAdapter(binaryPath);
      const result = await adapter.invoke("test prompt");
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.score).toBe(0);
      expect(typeof result.value.reason).toBe("string");
    } finally {
      await cleanup();
    }
  });

  test("invoke failure → err(ScanError) with message", async () => {
    const { binaryPath, cleanup } = await createMockAgentBinary("output", 1);
    try {
      const adapter = createCustomAdapter(binaryPath);
      const result = await adapter.invoke("test prompt");
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error).toBeInstanceOf(ScanError);
      expect(result.error.message).toContain("invocation failed");
    } finally {
      await cleanup();
    }
  });
});
