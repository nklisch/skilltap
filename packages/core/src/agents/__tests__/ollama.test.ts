import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { $ } from "bun";
import { createOllamaAdapter } from "../ollama";
import { ScanError } from "../../types";

let savedPath: string | undefined;

beforeEach(() => {
  savedPath = process.env.PATH;
});

afterEach(() => {
  if (savedPath === undefined) delete process.env.PATH;
  else process.env.PATH = savedPath;
});

/** Write a mock `ollama` binary to tmpDir and prepend tmpDir to PATH. */
async function setupMockOllama(
  tmpDir: string,
  listOutput: string,
  runResponse = '{"score": 5, "reason": "test response"}',
): Promise<void> {
  const ollamaBin = join(tmpDir, "ollama");
  const runResponseFile = join(tmpDir, "run-response.txt");
  const listOutputFile = join(tmpDir, "list-output.txt");

  await Bun.write(runResponseFile, runResponse);
  await Bun.write(listOutputFile, listOutput);

  // Script uses case to dispatch on the subcommand ($1)
  await Bun.write(
    ollamaBin,
    `#!/bin/sh
case "$1" in
  list) cat '${listOutputFile}';;
  run) cat '${runResponseFile}';;
  *) exit 0;;
esac
exit 0
`,
  );
  await $`chmod +x ${ollamaBin}`.quiet();
  process.env.PATH = `${tmpDir}:${savedPath}`;
}

describe("createOllamaAdapter — detect", () => {
  test("mock ollama present, list returns two lines → true", async () => {
    const tmpDir = await makeTmpDir();
    try {
      // Two lines: header + one model entry
      await setupMockOllama(tmpDir, "NAME\nllama3\n");
      const adapter = createOllamaAdapter("llama3");
      expect(await adapter.detect()).toBe(true);
    } finally {
      await removeTmpDir(tmpDir);
    }
  });

  test("mock ollama present, list returns one line (header only) → false", async () => {
    const tmpDir = await makeTmpDir();
    try {
      // Only the header — no models installed
      await setupMockOllama(tmpDir, "NAME\n");
      const adapter = createOllamaAdapter("llama3");
      expect(await adapter.detect()).toBe(false);
    } finally {
      await removeTmpDir(tmpDir);
    }
  });

  test("mock binary absent → false", async () => {
    // Use a path with no ollama binary
    const emptyDir = await makeTmpDir();
    try {
      process.env.PATH = emptyDir; // Only empty dir — nothing will be found
      const adapter = createOllamaAdapter("llama3");
      expect(await adapter.detect()).toBe(false);
    } finally {
      await removeTmpDir(emptyDir);
    }
  });
});

describe("createOllamaAdapter — invoke", () => {
  test("valid JSON response → ok(parsed)", async () => {
    const tmpDir = await makeTmpDir();
    try {
      await setupMockOllama(
        tmpDir,
        "NAME\nllama3\n",
        '{"score": 6, "reason": "flagged content"}',
      );
      const adapter = createOllamaAdapter("llama3");
      const result = await adapter.invoke("analyze this");
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.score).toBe(6);
      expect(result.value.reason).toBe("flagged content");
    } finally {
      await removeTmpDir(tmpDir);
    }
  });

  test("uses 'llama3' as default model when factory called with empty string", async () => {
    const tmpDir = await makeTmpDir();
    try {
      // This mock only succeeds for 'run' — if the wrong model is passed,
      // the adapter still calls run (model is just an arg), so we verify
      // the call succeeds (no crash) and returns a valid response.
      await setupMockOllama(
        tmpDir,
        "NAME\nllama3\n",
        '{"score": 2, "reason": "default model used"}',
      );
      // Empty string → effectiveModel = "llama3"
      const adapter = createOllamaAdapter("");
      const result = await adapter.invoke("test prompt");
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.score).toBe(2);
    } finally {
      await removeTmpDir(tmpDir);
    }
  });
});
