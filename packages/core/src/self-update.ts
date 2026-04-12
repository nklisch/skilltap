import { basename, dirname, join } from "node:path";
import { getConfigDir } from "./config";
import { lsRemoteTags } from "./git";
import { extractStderr } from "./shell";
import { err, NetworkError, ok, type Result, UserError } from "./types";

export type UpdateType = "patch" | "minor" | "major";

export interface UpdateCheckResult {
  current: string;
  latest: string;
  type: UpdateType;
}

interface UpdateCache {
  checkedAt: string;
  latest: string;
}

const RELEASE_REPO_URL = "https://github.com/nklisch/skilltap.git";
const CACHE_FILE = "update-check.json";

function parseVersion(v: string): [number, number, number] | null {
  const clean = v.startsWith("v") ? v.slice(1) : v;
  const parts = clean.split(".").map(Number);
  if (parts.length !== 3 || parts.some(Number.isNaN)) return null;
  return [parts[0]!, parts[1]!, parts[2]!];
}

function getUpdateType(current: string, latest: string): UpdateType | null {
  const c = parseVersion(current);
  const l = parseVersion(latest);
  if (!c || !l) return null;
  if (l[0] > c[0]) return "major";
  if (l[0] === c[0] && l[1] > c[1]) return "minor";
  if (l[0] === c[0] && l[1] === c[1] && l[2] > c[2]) return "patch";
  return null;
}

async function readCache(configDir: string): Promise<UpdateCache | null> {
  const file = join(configDir, CACHE_FILE);
  const f = Bun.file(file);
  if (!(await f.exists())) return null;
  try {
    return (await f.json()) as UpdateCache;
  } catch {
    return null;
  }
}

async function writeCache(configDir: string, latest: string): Promise<void> {
  const file = join(configDir, CACHE_FILE);
  try {
    await Bun.write(
      file,
      JSON.stringify({ checkedAt: new Date().toISOString(), latest }),
    );
  } catch {
    // Non-critical — ignore write failures
  }
}

type LsRemoteTagsFn = typeof lsRemoteTags;

export async function fetchLatestVersion(
  _lsRemoteTags: LsRemoteTagsFn = lsRemoteTags,
): Promise<string | null> {
  const result = await _lsRemoteTags(RELEASE_REPO_URL, "v*");
  if (!result.ok) return null;

  const tags = result.value;
  if (tags.length === 0) return null;

  let best: [number, number, number] | null = null;
  let bestTag = "";

  for (const tag of tags) {
    const parsed = parseVersion(tag);
    if (!parsed) continue;
    if (
      !best ||
      parsed[0] > best[0] ||
      (parsed[0] === best[0] && parsed[1] > best[1]) ||
      (parsed[0] === best[0] && parsed[1] === best[1] && parsed[2] > best[2])
    ) {
      best = parsed;
      bestTag = tag;
    }
  }

  if (!bestTag) return null;
  return bestTag.startsWith("v") ? bestTag.slice(1) : bestTag;
}

/**
 * Read cached update result. Kicks off a background refresh if cache is stale.
 * Never throws — returns null when no update info is available yet.
 */
type FetchLatestFn = typeof fetchLatestVersion;

export async function checkForUpdate(
  currentVersion: string,
  intervalHours = 24,
  _fetchLatest: FetchLatestFn = fetchLatestVersion,
): Promise<UpdateCheckResult | null> {
  const configDir = getConfigDir();
  const cache = await readCache(configDir);

  const isStale =
    !cache ||
    Date.now() - new Date(cache.checkedAt).getTime() >
      intervalHours * 3_600_000;

  if (isStale) {
    if (intervalHours === 0) {
      // Caller wants a fresh check — await the fetch instead of fire-and-forget
      const fetched = await _fetchLatest();
      if (fetched) {
        await writeCache(configDir, fetched);
        const type = getUpdateType(currentVersion, fetched);
        if (!type) return null;
        return { current: currentVersion, latest: fetched, type };
      }
      return null;
    }
    // Fire-and-forget — do not block the CLI
    _fetchLatest().then((latest) => {
      if (latest) writeCache(configDir, latest);
    });
  }

  if (!cache?.latest) return null;

  const type = getUpdateType(currentVersion, cache.latest);
  if (!type) return null;

  return { current: currentVersion, latest: cache.latest, type };
}

/** Returns true when running as a compiled binary (not via bun run / npm link). */
export function isCompiledBinary(): boolean {
  return !["bun", "bun.exe"].includes(basename(process.execPath));
}

function getPlatformAsset(): string | null {
  const { platform, arch } = process;
  if (platform === "linux" && arch === "x64") return "skilltap-linux-x64";
  if (platform === "linux" && arch === "arm64") return "skilltap-linux-arm64";
  if (platform === "darwin" && arch === "x64") return "skilltap-darwin-x64";
  if (platform === "darwin" && arch === "arm64") return "skilltap-darwin-arm64";
  return null;
}

type FetchFn = (url: string | URL, init?: { signal?: AbortSignal }) => Promise<Response>;

async function ghDownload(
  version: string,
  asset: string,
  destPath: string,
): Promise<Result<void, UserError>> {
  try {
    const whichResult = await Bun.$`which gh`.quiet();
    const ghPath = whichResult.stdout.toString().trim();
    if (!ghPath) return err(new UserError("gh not found"));

    await Bun.$`${ghPath} release download v${version} --repo nklisch/skilltap --pattern ${asset} --dir ${dirname(destPath)} --clobber`.quiet();

    const downloadedPath = join(dirname(destPath), asset);
    if (downloadedPath !== destPath) {
      await Bun.$`mv -f ${downloadedPath} ${destPath}`.quiet();
    }
    return ok(undefined);
  } catch (e) {
    return err(new UserError(`gh download failed: ${extractStderr(e)}`));
  }
}

type GhDownloadFn = typeof ghDownload;

/**
 * Download the specified release from GitHub and atomically replace the
 * running binary. Only works when running as a compiled binary.
 * Tries gh CLI first (inherits auth), falls back to direct HTTP fetch.
 */
export async function downloadAndInstall(
  version: string,
  _fetch: FetchFn = fetch,
  _execPath: string = process.execPath,
  _ghDownload: GhDownloadFn = ghDownload,
): Promise<Result<void, UserError>> {
  const asset = getPlatformAsset();
  if (!asset) {
    return err(
      new UserError(
        "Auto-update is not supported on this platform.",
        "Install manually: npm install -g skilltap",
      ),
    );
  }

  const tmpPath = `${_execPath}.update`;

  const ghResult = await _ghDownload(version, asset, tmpPath);

  if (!ghResult.ok) {
    const url = `https://github.com/nklisch/skilltap/releases/download/v${version}/${asset}`;
    let response: Response;
    try {
      response = await _fetch(url, { signal: AbortSignal.timeout(60_000) });
    } catch (e) {
      return err(
        new NetworkError(`Download failed: ${e}`) as unknown as UserError,
      );
    }

    if (!response.ok) {
      return err(
        new UserError(`Failed to download v${version}: HTTP ${response.status}`),
      );
    }

    const buffer = await response.arrayBuffer();
    try {
      await Bun.write(tmpPath, buffer);
    } catch (e) {
      Bun.$`rm -f ${tmpPath}`.quiet();
      return err(
        new UserError(
          `Failed to replace binary: ${extractStderr(e)}`,
          "Try running with sudo, or install via npm: npm install -g skilltap",
        ),
      );
    }
  }

  try {
    await Bun.$`chmod +x ${tmpPath}`.quiet();
    if (process.platform === "darwin") {
      await Bun.$`xattr -d com.apple.quarantine ${tmpPath} 2>/dev/null || true`.quiet();
      await Bun.$`codesign -s - ${tmpPath} 2>/dev/null || true`.quiet();
    }
    await Bun.$`mv -f ${tmpPath} ${_execPath}`.quiet();
  } catch (e) {
    Bun.$`rm -f ${tmpPath}`.quiet();
    return err(
      new UserError(
        `Failed to replace binary: ${extractStderr(e)}`,
        "Try running with sudo, or install via npm: npm install -g skilltap",
      ),
    );
  }

  await writeCache(getConfigDir(), version);
  return ok(undefined);
}
