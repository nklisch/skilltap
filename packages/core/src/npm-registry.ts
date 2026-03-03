import { homedir } from "node:os";
import { join } from "node:path";
import { $ } from "bun";
import type { Result } from "./types";
import { err, NetworkError, ok, UserError } from "./types";

export interface NpmVersionInfo {
  version: string;
  dist: {
    tarball: string;
    integrity: string;
    attestations?: { url: string; provenance: unknown };
  };
  npmUser?: string;
}

export interface NpmPackageMetadata {
  name: string;
  description: string;
  distTags: Record<string, string>;
  versions: Record<string, NpmVersionInfo>;
}

/** Parse `npm:<package>[@version]` into package name and version. */
export function parseNpmSource(source: string): { name: string; version: string } {
  const s = source.startsWith("npm:") ? source.slice(4) : source;

  if (s.startsWith("@")) {
    // Scoped: @scope/name or @scope/name@version
    const slashIdx = s.indexOf("/");
    if (slashIdx === -1) return { name: s, version: "latest" };
    const afterSlash = s.slice(slashIdx + 1);
    const atIdx = afterSlash.lastIndexOf("@");
    if (atIdx === -1) return { name: s, version: "latest" };
    const name = s.slice(0, slashIdx + 1 + atIdx);
    return { name, version: afterSlash.slice(atIdx + 1) };
  }

  // Unscoped: name or name@version
  const atIdx = s.lastIndexOf("@");
  if (atIdx === -1) return { name: s, version: "latest" };
  return { name: s.slice(0, atIdx), version: s.slice(atIdx + 1) };
}

/** Read npm registry URL from environment or .npmrc files. */
async function getRegistryUrl(): Promise<string> {
  if (process.env.NPM_CONFIG_REGISTRY) return process.env.NPM_CONFIG_REGISTRY;

  for (const rcPath of [".npmrc", join(homedir(), ".npmrc")]) {
    try {
      const content = await Bun.file(rcPath).text();
      const match = content.match(/^registry\s*=\s*(.+)$/m);
      if (match?.[1]) return match[1].trim();
    } catch {
      // file doesn't exist or can't be read — continue
    }
  }

  return "https://registry.npmjs.org";
}

/** Fetch package metadata from the npm registry. */
export async function fetchPackageMetadata(
  name: string,
  registryUrl?: string,
): Promise<Result<NpmPackageMetadata, NetworkError>> {
  const registry = (registryUrl ?? (await getRegistryUrl())).replace(/\/$/, "");
  const url = `${registry}/${name}`;

  let response: Response;
  try {
    response = await fetch(url, { headers: { Accept: "application/json" } });
  } catch {
    return err(new NetworkError("Could not reach npm registry. Check your connection."));
  }

  if (response.status === 404) {
    return err(new NetworkError(`npm package '${name}' not found on registry.`));
  }
  if (response.status === 401 || response.status === 403) {
    return err(
      new NetworkError(
        `Authentication required for npm package '${name}'.`,
        "Check your .npmrc configuration.",
      ),
    );
  }
  if (!response.ok) {
    return err(new NetworkError(`npm registry returned HTTP ${response.status}`));
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    return err(new NetworkError("Invalid response from npm registry."));
  }

  const raw = data as Record<string, unknown>;
  const distTags = (raw["dist-tags"] as Record<string, string>) ?? {};
  const rawVersions = (raw.versions as Record<string, unknown>) ?? {};

  const versions: Record<string, NpmVersionInfo> = {};
  for (const [ver, info] of Object.entries(rawVersions)) {
    const v = info as Record<string, unknown>;
    const dist = v.dist as Record<string, unknown> | undefined;
    if (!dist?.tarball) continue;
    const npmUser = v._npmUser as { name?: string } | undefined;
    versions[ver] = {
      version: ver,
      dist: {
        tarball: dist.tarball as string,
        integrity: (dist.integrity as string) ?? "",
        attestations: dist.attestations as NpmVersionInfo["dist"]["attestations"],
      },
      npmUser: npmUser?.name,
    };
  }

  return ok({
    name: (raw.name as string) ?? name,
    description: (raw.description as string) ?? "",
    distTags,
    versions,
  });
}

/** Resolve a version string (exact version or dist-tag like "latest") to NpmVersionInfo. */
export function resolveVersion(
  metadata: NpmPackageMetadata,
  version: string,
): Result<NpmVersionInfo, UserError> {
  // Resolve dist-tags (e.g. "latest" → "1.2.0")
  const resolved = metadata.distTags[version] ?? version;

  const info = metadata.versions[resolved];
  if (!info) {
    const available = Object.keys(metadata.versions).join(", ");
    return err(
      new UserError(
        `Version '${version}' not found for npm package '${metadata.name}'.`,
        `Available: ${available}. Use 'npm:${metadata.name}' for the latest version.`,
      ),
    );
  }

  return ok(info);
}

/**
 * Download and extract an npm tarball to a destination directory.
 * Returns the path to the extracted `package/` subdirectory.
 */
export async function downloadAndExtract(
  tarballUrl: string,
  dest: string,
  integrity?: string,
): Promise<Result<string, NetworkError>> {
  let response: Response;
  try {
    response = await fetch(tarballUrl);
  } catch {
    return err(new NetworkError("Could not reach npm registry. Check your connection."));
  }

  if (response.status === 401 || response.status === 403) {
    return err(
      new NetworkError(
        "Authentication required to download npm package.",
        "Check your .npmrc configuration.",
      ),
    );
  }
  if (!response.ok) {
    return err(new NetworkError(`Failed to download npm tarball: HTTP ${response.status}`));
  }

  const buffer = await response.arrayBuffer();

  // Verify integrity (SHA-512 SRI format: "sha512-<base64>")
  if (integrity?.startsWith("sha512-")) {
    const hasher = new Bun.CryptoHasher("sha512");
    hasher.update(new Uint8Array(buffer));
    const digest = hasher.digest("base64");
    const expected = integrity.slice("sha512-".length);
    if (digest !== expected) {
      return err(
        new NetworkError(
          "Tarball integrity check failed. The download may be corrupted.",
        ),
      );
    }
  }

  // Write tarball to disk
  const tarPath = join(dest, "_pkg.tgz");
  await Bun.write(tarPath, buffer);

  // Extract (npm tarballs always extract to a `package/` subdirectory)
  try {
    await $`tar -xzf ${tarPath} -C ${dest}`.quiet();
  } catch {
    return err(new NetworkError("Failed to extract npm package tarball."));
  }

  return ok(join(dest, "package"));
}
