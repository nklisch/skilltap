import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { basename, dirname } from "node:path";
import { createMockAgentBinary } from "@skilltap/test-utils";
import { $ } from "bun";
import { createCliAdapter } from "../factory";
import { ScanError } from "../../types";

let savedPath: string | undefined;

beforeEach(() => {
  savedPath = process.env.PATH;
});

afterEach(() => {
  if (savedPath === undefined) delete process.env.PATH;
  else process.env.PATH = savedPath;
});

describe("createCliAdapter — invoke", () => {
  test("valid JSON object → ok({ score, reason })", async () => {
    const { binaryPath, cleanup } = await createMockAgentBinary(
      '{"score": 7, "reason": "suspicious content"}',
    );
    try {
      const adapter = createCliAdapter("Test", "test", (_prompt) =>
        $`${binaryPath}`.quiet(),
      );
      const result = await adapter.invoke("test prompt");
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.score).toBe(7);
      expect(result.value.reason).toBe("suspicious content");
    } finally {
      await cleanup();
    }
  });

  test("JSON in code block → extracted correctly", async () => {
    const { binaryPath, cleanup } = await createMockAgentBinary(
      "```json\n{\"score\": 3, \"reason\": \"looks fine\"}\n```",
    );
    try {
      const adapter = createCliAdapter("Test", "test", (_prompt) =>
        $`${binaryPath}`.quiet(),
      );
      const result = await adapter.invoke("test prompt");
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.score).toBe(3);
      expect(result.value.reason).toBe("looks fine");
    } finally {
      await cleanup();
    }
  });

  test("unparseable output → ok({ score: 0, reason: ... })", async () => {
    const { binaryPath, cleanup } = await createMockAgentBinary(
      "I cannot analyze this content, sorry.",
    );
    try {
      const adapter = createCliAdapter("Test", "test", (_prompt) =>
        $`${binaryPath}`.quiet(),
      );
      const result = await adapter.invoke("test prompt");
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.score).toBe(0);
      expect(typeof result.value.reason).toBe("string");
    } finally {
      await cleanup();
    }
  });

  test("non-zero exit code → err(ScanError)", async () => {
    const { binaryPath, cleanup } = await createMockAgentBinary("error output", 1);
    try {
      const adapter = createCliAdapter("Test", "test", (_prompt) =>
        $`${binaryPath}`.quiet(),
      );
      const result = await adapter.invoke("test prompt");
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error).toBeInstanceOf(ScanError);
    } finally {
      await cleanup();
    }
  });

  test("non-existent binary path → err(ScanError)", async () => {
    const adapter = createCliAdapter("Test", "test", (_prompt) =>
      $`/nonexistent/path/to/binary-xyz`.quiet(),
    );
    const result = await adapter.invoke("test prompt");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error).toBeInstanceOf(ScanError);
  });
});

describe("createCliAdapter — detect", () => {
  test("binary on PATH → true", async () => {
    const { binaryPath, cleanup } = await createMockAgentBinary(
      '{"score": 0, "reason": "ok"}',
    );
    try {
      // Prepend the binary's directory so `which <name>` finds it
      process.env.PATH = `${dirname(binaryPath)}:${savedPath}`;
      const cliName = basename(binaryPath); // "mock-agent"
      const adapter = createCliAdapter("Test", cliName, (_prompt) =>
        $`${binaryPath}`.quiet(),
      );
      expect(await adapter.detect()).toBe(true);
    } finally {
      await cleanup();
    }
  });

  test("binary not on PATH → false", async () => {
    const adapter = createCliAdapter(
      "Test",
      "definitely-not-installed-xyzabc123",
      (_prompt) => $`/nonexistent`.quiet(),
    );
    expect(await adapter.detect()).toBe(false);
  });
});
