# Pattern: Adapter-Driven Flow Branching

After `resolveSource()` returns a `ResolvedSource`, the `resolved.adapter` value gates source-type-specific paths throughout the install, update, and trust flows.

## Rationale

npm and git sources have fundamentally different lifecycles after resolution: npm downloads a tarball and tracks a version string; git clones a repo and tracks a SHA; local has no remote state at all. Rather than booleans or type unions, the `adapter` string on `ResolvedSource` acts as the discriminator — every place that needs to branch on source type reads it. This keeps the SourceAdapter strategy (detection + URL resolution) cleanly separated from what happens *after* the source is resolved.

## Examples

### Example 1: Install — distinct download and placement paths
**File**: `packages/core/src/install.ts`
```typescript
let contentDir: string;
if (resolved.adapter === "npm") {
  // Tarball path: download, integrity check, extract to tmpDir/package/
  const extractResult = await downloadAndExtract(resolved.url, tmpDir, resolved.integrity);
  contentDir = extractResult.value;    // join(tmpDir, "package")
  sha = null;                          // npm has no SHA
} else {
  // Git path: clone repo, revParse HEAD, find multi-skill or standalone
  await clone(resolved.url, tmpDir, resolved.ref);
  sha = await revParse(tmpDir);
  contentDir = tmpDir;
}
```

### Example 2: Update — separate handler for npm vs git
**File**: `packages/core/src/update.ts`
```typescript
export async function updateSkill(name: string, options: UpdateOptions) {
  const record = installed.skills.find(s => s.name === name);

  if (record.repo?.startsWith("npm:")) {
    return updateNpmSkill(record, options);  // version comparison, tarball download
  }
  // git path: fetch, diff, pull, re-copy multi-skill
}

async function updateNpmSkill(record, options) {
  const { name, version } = parseNpmSource(record.repo!);
  const latest = resolveVersion(meta, "latest");
  if (latest.ref === record.ref) return ok({ updated: false }); // already up-to-date
  // download new tarball, scanStatic, replace install dir
}
```

### Example 3: Trust resolution — adapter selects which verifier branch runs
**File**: `packages/core/src/trust/resolve.ts:45`
```typescript
// npm branch: sigstore + tarball SHA-256 verification
if (params.adapter === "npm" && params.tarballPath && params.npmPackageName) {
  const npmTrust = await _verifyNpm(params.npmPackageName, params.npmVersion, params.tarballPath);
  if (npmTrust) return { tier: "provenance", npm: npmTrust, ... };
}

// git branch: gh attestation verify
if (params.adapter !== "npm" && params.adapter !== "local" && params.skillDir && params.githubRepo) {
  const ghTrust = await _verifyGitHub(params.skillDir, params.githubRepo);
  if (ghTrust) return { tier: "provenance", github: ghTrust, ... };
}

// npm publisher fallback
if (params.adapter === "npm") {
  return { tier: "publisher", publisher: { name: publisherName, platform: "npm" } };
}
```

### Example 4: ResolvedSource carries the adapter name from the adapter itself
**File**: `packages/core/src/adapters/npm.ts:9`
```typescript
export const npmAdapter: SourceAdapter = {
  name: "npm",
  canHandle(source) { return source.startsWith("npm:"); },
  async resolve(source) {
    // ...
    return ok({ url: info.dist.tarball, ref: info.version, adapter: "npm", ... });
    //                                                        ^^^^^^^^^^^^
    //                   adapter sets its own name in the resolved value
  },
};
```

## When to Use

- The install, update, or enrichment step behaves categorically differently between source types
- The branching condition is "what kind of source is this?" — use `resolved.adapter` or `record.repo?.startsWith("npm:")`
- New adapters need distinct post-resolution behavior

## When NOT to Use

- The difference is only a minor flag — don't add a branch for a one-line variation
- Decision is about user intent, not source type — use callback-driven-options instead
- Detecting source type from the URL string directly — always branch on `adapter`, never re-parse `resolved.url`

## Common Violations

- Re-detecting source type from the URL string after resolution (parsing `https://` or `npm:` again) — the adapter name is already in `ResolvedSource`
- Putting source-type-specific logic inside the SourceAdapter's `resolve()` method — adapters resolve to a URL; post-resolution behavior belongs in `install.ts`/`update.ts`
- Storing `resolved.adapter` as anything other than the exact adapter `name` field — must be consistent with how `record.repo` is stored for update detection
