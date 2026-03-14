import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { checkForUpdate, downloadAndInstall, fetchLatestVersion, isCompiledBinary } from "./self-update";
import { err, GitError, ok, UserError } from "./types";

type Env = { XDG_CONFIG_HOME?: string };

let savedEnv: Env;
let configDir: string;
let tmpDir: string;

beforeEach(async () => {
  savedEnv = { XDG_CONFIG_HOME: process.env.XDG_CONFIG_HOME };
  configDir = await makeTmpDir();
  tmpDir = await makeTmpDir();
  process.env.XDG_CONFIG_HOME = configDir;
  // Ensure the skilltap config subdir exists (writeCache needs it)
  await mkdir(join(configDir, "skilltap"), { recursive: true });
});

afterEach(async () => {
  if (savedEnv.XDG_CONFIG_HOME === undefined) delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = savedEnv.XDG_CONFIG_HOME;
  await removeTmpDir(configDir);
  await removeTmpDir(tmpDir);
});

async function writeCache(latest: string, hoursAgo = 0): Promise<void> {
  const checkedAt = new Date(Date.now() - hoursAgo * 3_600_000).toISOString();
  await Bun.write(
    join(configDir, "skilltap", "update-check.json"),
    JSON.stringify({ checkedAt, latest }),
  );
}

// ─── checkForUpdate ───────────────────────────────────────────────────────────

describe("checkForUpdate", () => {
  test("returns null when no cache file exists", async () => {
    const result = await checkForUpdate("1.0.0");
    expect(result).toBeNull();
  });

  test("returns null when cached version equals current", async () => {
    await writeCache("1.0.0");
    const result = await checkForUpdate("1.0.0");
    expect(result).toBeNull();
  });

  test("returns null when cached version is older than current", async () => {
    await writeCache("0.9.0");
    const result = await checkForUpdate("1.0.0");
    expect(result).toBeNull();
  });

  test("detects patch update", async () => {
    await writeCache("1.0.1");
    const result = await checkForUpdate("1.0.0");
    expect(result).not.toBeNull();
    expect(result?.type).toBe("patch");
    expect(result?.current).toBe("1.0.0");
    expect(result?.latest).toBe("1.0.1");
  });

  test("detects minor update", async () => {
    await writeCache("1.1.0");
    const result = await checkForUpdate("1.0.5");
    expect(result?.type).toBe("minor");
  });

  test("detects major update", async () => {
    await writeCache("2.0.0");
    const result = await checkForUpdate("1.9.9");
    expect(result?.type).toBe("major");
  });

  test("returns null when cache has malformed version", async () => {
    await writeCache("not-a-version");
    const result = await checkForUpdate("1.0.0");
    expect(result).toBeNull();
  });

  test("returns null when current version is malformed", async () => {
    await writeCache("1.0.1");
    const result = await checkForUpdate("not-a-version");
    expect(result).toBeNull();
  });

  test("returns cached data even when cache is stale (background refresh fires)", async () => {
    // Write a stale cache (25 hours ago)
    await writeCache("1.2.0", 25);
    // Should still return the cached data (background fetch is fire-and-forget)
    const result = await checkForUpdate("1.0.0", 24);
    expect(result).not.toBeNull();
    expect(result?.latest).toBe("1.2.0");
  });

  test("respects custom intervalHours — fresh cache not stale", async () => {
    // Cache is 1 hour old; interval is 2 hours → not stale → no background fetch
    await writeCache("2.0.0", 1);
    const result = await checkForUpdate("1.0.0", 2);
    expect(result).not.toBeNull();
    expect(result?.type).toBe("major");
  });

  test("intervalHours=0 awaits fetch and detects minor update (no prior cache)", async () => {
    const mockFetch = async () => "1.1.0";
    const result = await checkForUpdate("1.0.0", 0, mockFetch);
    expect(result).not.toBeNull();
    expect(result?.type).toBe("minor");
    expect(result?.latest).toBe("1.1.0");
  });

  test("intervalHours=0 returns null when current is already latest", async () => {
    const mockFetch = async () => "1.0.0";
    const result = await checkForUpdate("1.0.0", 0, mockFetch);
    expect(result).toBeNull();
  });

  test("intervalHours=0 returns null when fetch returns null", async () => {
    const mockFetch = async () => null;
    const result = await checkForUpdate("1.0.0", 0, mockFetch);
    expect(result).toBeNull();
  });
});

// ─── fetchLatestVersion ───────────────────────────────────────────────────────

describe("fetchLatestVersion", () => {
  test("returns highest semver from tags", async () => {
    const mockLsRemote = async () => ok(["v0.9.0", "v1.2.0", "v1.0.0"]);
    const result = await fetchLatestVersion(mockLsRemote);
    expect(result).toBe("1.2.0");
  });

  test("returns null when ls-remote fails", async () => {
    const mockLsRemote = async () => err(new GitError("failed"));
    const result = await fetchLatestVersion(mockLsRemote);
    expect(result).toBeNull();
  });

  test("returns null when no tags exist", async () => {
    const mockLsRemote = async () => ok([]);
    const result = await fetchLatestVersion(mockLsRemote);
    expect(result).toBeNull();
  });

  test("ignores malformed tags", async () => {
    const mockLsRemote = async () => ok(["v1.0.0", "release-candidate", "v2.0.0"]);
    const result = await fetchLatestVersion(mockLsRemote);
    expect(result).toBe("2.0.0");
  });
});

// ─── isCompiledBinary ─────────────────────────────────────────────────────────

describe("isCompiledBinary", () => {
  test("returns false when running under bun (test env)", () => {
    // Tests run via `bun test`, so process.execPath is the bun binary
    expect(isCompiledBinary()).toBe(false);
  });
});

// ─── downloadAndInstall ───────────────────────────────────────────────────────

const fakeBinary = new Uint8Array([0x7f, 0x45, 0x4c, 0x46]); // ELF magic bytes

function okFetch(_url: string | URL): Promise<Response> {
  return Promise.resolve(new Response(fakeBinary, { status: 200 }));
}

function notFoundFetch(_url: string | URL): Promise<Response> {
  return Promise.resolve(new Response(null, { status: 404 }));
}

function networkErrorFetch(_url: string | URL): Promise<Response> {
  return Promise.reject(new Error("ECONNREFUSED: connection refused"));
}

describe("downloadAndInstall", () => {
  test("returns error on HTTP 404", async () => {
    const execPath = join(tmpDir, "skilltap");
    await Bun.write(execPath, "old");

    const result = await downloadAndInstall("9.9.9", notFoundFetch, execPath);

    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("HTTP 404");
  });

  test("returns error on network failure", async () => {
    const execPath = join(tmpDir, "skilltap");
    await Bun.write(execPath, "old");

    const result = await downloadAndInstall("9.9.9", networkErrorFetch, execPath);

    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Download failed");
  });

  test("replaces binary with downloaded content on success", async () => {
    const execPath = join(tmpDir, "skilltap");
    await Bun.write(execPath, "old content");

    const result = await downloadAndInstall("9.9.9", okFetch, execPath);

    expect(result.ok).toBe(true);
    const written = new Uint8Array(await Bun.file(execPath).arrayBuffer());
    expect(written).toEqual(fakeBinary);
  });

  test("temp file is cleaned up after successful install", async () => {
    const execPath = join(tmpDir, "skilltap");
    await Bun.write(execPath, "old");

    await downloadAndInstall("9.9.9", okFetch, execPath);

    expect(await Bun.file(`${execPath}.update`).exists()).toBe(false);
  });

  test("binary is made executable after install", async () => {
    const execPath = join(tmpDir, "skilltap");
    await Bun.write(execPath, "old");

    await downloadAndInstall("9.9.9", okFetch, execPath);

    const stat = await Bun.file(execPath).stat();
    // Check owner execute bit (0o100)
    expect(stat.mode & 0o100).toBe(0o100);
  });

  test("updates version cache after successful install", async () => {
    const execPath = join(tmpDir, "skilltap");
    await Bun.write(execPath, "old");

    await downloadAndInstall("9.9.9", okFetch, execPath);

    const cacheFile = join(configDir, "skilltap", "update-check.json");
    const cache = (await Bun.file(cacheFile).json()) as { latest: string };
    expect(cache.latest).toBe("9.9.9");
  });

  test("returns error when binary replacement fails", async () => {
    // Put execPath inside a read-only directory so Bun.write to tmpPath (execPath.update) fails
    const roDir = join(tmpDir, "readonly");
    await mkdir(roDir, { recursive: true });
    await Bun.$`chmod 555 ${roDir}`.quiet();
    const execPath = join(roDir, "skilltap");

    const result = await downloadAndInstall("9.9.9", okFetch, execPath);

    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Failed to replace binary");
    expect(result.error.hint).toContain("sudo");
  });

  test("temp file cleaned up when replacement fails", async () => {
    const roDir = join(tmpDir, "readonly2");
    await mkdir(roDir, { recursive: true });
    await Bun.$`chmod 555 ${roDir}`.quiet();
    const execPath = join(roDir, "skilltap");

    await downloadAndInstall("9.9.9", okFetch, execPath);

    // tmpPath = ${execPath}.update — write failed (EACCES), so it should not exist
    expect(await Bun.file(`${execPath}.update`).exists()).toBe(false);
  });

  test("uses gh when available — fetch is not called", async () => {
    const execPath = join(tmpDir, "skilltap-gh");
    await Bun.write(execPath, "old");

    let fetchCalled = false;
    const mockFetch = async () => {
      fetchCalled = true;
      return new Response(fakeBinary, { status: 200 });
    };

    const mockGh = async (_version: string, _asset: string, destPath: string) => {
      await Bun.write(destPath, fakeBinary);
      return ok(undefined);
    };

    const result = await downloadAndInstall("9.9.9", mockFetch, execPath, mockGh);
    expect(result.ok).toBe(true);
    expect(fetchCalled).toBe(false);
  });

  test("falls back to fetch when gh fails", async () => {
    const execPath = join(tmpDir, "skilltap-fallback");
    await Bun.write(execPath, "old");

    const mockGh = async () => err(new UserError("gh not found"));

    const result = await downloadAndInstall("9.9.9", okFetch, execPath, mockGh);
    expect(result.ok).toBe(true);
  });
});
