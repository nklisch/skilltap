# Pattern: Graceful Fallback Chain

Optional verification functions return `null` on any failure, letting the caller descend through a priority-ordered tier list without error propagation.

## Rationale

Trust verification is *enrichment* — if sigstore or `gh` attestation fails, the system should fall back to a lower trust tier (publisher, curated, unverified) rather than blocking installation. This is fundamentally different from core operations like `clone` or `scanStatic`, where failure stops the operation and returns a `Result<_, E>`. Using `T | null` return instead of `Result<T, E>` signals "this step is optional" and keeps the caller's tier-cascade logic simple: `if (result) return highTier; /* else fall through */`.

The top-level function wraps the entire chain in `try/catch` and returns `{ tier: "unverified" }` as the ultimate fallback — it never throws and never returns a `Result`.

## Examples

### Example 1: Verifier returns null on any failure — never throws
**File**: `packages/core/src/trust/verify-npm.ts:55`
```typescript
export async function verifyNpmProvenance(
  packageName: string,
  version: string,
  tarballPath: string,
): Promise<NpmTrustData | null> {
  try {
    const response = await fetch(url);
    if (!response.ok) return null;          // network issue → null

    const signer = await verify(bundle);    // sigstore verify
    // ...
    if (actualDigest !== subjectDigest) return null;  // integrity mismatch → null

    return { publisher, sourceRepo, verifiedAt: ... };
  } catch {
    return null;  // unexpected error → null
  }
}
```

### Example 2: Caller descends the tier list
**File**: `packages/core/src/trust/resolve.ts:45`
```typescript
export async function resolveTrust(params, _verifyNpm, _verifyGitHub): Promise<TrustInfo> {
  try {
    // Tier 1: cryptographic provenance
    if (params.adapter === "npm" && params.tarballPath) {
      const npmTrust = await _verifyNpm(name, version, tarballPath);
      if (npmTrust) return { tier: "provenance", npm: npmTrust, ... };
      // null → fall through to publisher
    }

    if (params.adapter !== "npm" && params.skillDir && params.githubRepo) {
      const ghTrust = await _verifyGitHub(skillDir, githubRepo);
      if (ghTrust) return { tier: "provenance", github: ghTrust, ... };
      // null → fall through to publisher
    }

    // Tier 2: identity known, not cryptographic
    if (params.adapter === "npm") {
      return { tier: "publisher", publisher: { name: publisherName, platform: "npm" }, ... };
    }

    // Tier 3: human-curated index
    if (params.tap) {
      return { tier: "curated", tap: params.tap };
    }

    // Tier 4: unknown
    return { tier: "unverified" };
  } catch {
    return { tier: "unverified" };  // ultimate fallback
  }
}
```

### Example 3: GitHub verifier — same null-on-failure convention
**File**: `packages/core/src/trust/verify-github.ts:29`
```typescript
export async function verifyGitHubAttestation(
  skillDir: string,
  githubRepo: string,
): Promise<GitHubTrustData | null> {
  try {
    const result = await $`which gh`.quiet();
    if (!result.stdout.toString().trim()) return null;  // gh not installed → null

    const raw = await $`${ghPath} attestation verify ${skillMd} --repo ${owner}/${repo} --format json`.quiet();
    const results = JSON.parse(raw.stdout.toString());
    if (!results.length) return null;  // no attestation → null

    return { owner, repo, workflow, verifiedAt: ... };
  } catch {
    return null;  // any failure → null
  }
}
```

## When to Use

- The operation is enrichment — success adds metadata, failure means "use less-trusted default"
- There is a pre-defined fallback chain with ordered priority levels
- The caller must always produce a valid result (never an error) for this dimension

## When NOT to Use

- The operation is required — if it fails, the whole operation should fail (use `Result<T, E>`)
- There is no meaningful fallback — use `Result<T, E>` and surface the error to the user
- The null return would be ambiguous — if callers need to distinguish "not found" from "failed", use `Result`

## Common Violations

- Throwing from a verifier instead of returning `null` — the cascade fails with an uncaught exception
- Returning `Result<T, E>` from a verifier — forces every call site to handle the error explicitly, even when it should just degrade
- Forgetting the outer `try/catch` in the chain function — any unexpected bug in the cascade leaks as an unhandled rejection
