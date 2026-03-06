import { basename, join } from "node:path";
import { getConfigDir } from "./config";
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

const GITHUB_OWNER = "nklisch";
const GITHUB_REPO = "skilltap";
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

export async function fetchLatestVersion(): Promise<string | null> {
  try {
    const response = await fetch(
      `https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/latest`,
      {
        headers: { Accept: "application/vnd.github.v3+json" },
        signal: AbortSignal.timeout(5000),
      },
    );
    if (!response.ok) return null;
    const data = (await response.json()) as { tag_name?: string };
    const tag = data.tag_name;
    if (!tag) return null;
    return tag.startsWith("v") ? tag.slice(1) : tag;
  } catch {
    return null;
  }
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

/**
 * Download the specified release from GitHub and atomically replace the
 * running binary. Only works when running as a compiled binary.
 */
export async function downloadAndInstall(
  version: string,
  _fetch: FetchFn = fetch,
  _execPath: string = process.execPath,
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

  const url = `https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}/releases/download/v${version}/${asset}`;
  const tmpPath = `${_execPath}.update`;

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

  try {
    const buffer = await response.arrayBuffer();
    await Bun.write(tmpPath, buffer);
    await Bun.$`chmod +x ${tmpPath}`.quiet();
    await Bun.$`mv -f ${tmpPath} ${_execPath}`.quiet();
  } catch (e) {
    // Clean up temp file if possible
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
