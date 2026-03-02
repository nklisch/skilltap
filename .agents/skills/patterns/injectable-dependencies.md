# Pattern: Injectable Dependencies

Core functions that call external I/O accept the implementation as an optional parameter with a real default, enabling tests to swap in lightweight mocks without a DI container.

## Rationale

Some core functions depend on external services (network calls, CLI tools, filesystem reads for verification). Making tests require real HTTP or real binaries makes them slow, flaky, and hard to isolate. Rather than a DI container or global mocking, the dependency is accepted as an optional parameter defaulting to the real implementation. Tests pass a mock function; production callers omit it. Private `type` aliases enforce the required signature at compile time.

## Examples

### Example 1: Trust resolver with injectable verifiers
**File**: `packages/core/src/trust/resolve.ts:31`
```typescript
// Private type aliases — enforce exact signature compatibility
type VerifyNpmFn = typeof verifyNpmProvenance;
type VerifyGitHubFn = typeof verifyGitHubAttestation;

export async function resolveTrust(
  params: ResolveTrustParams,
  _verifyNpm: VerifyNpmFn = verifyNpmProvenance,
  _verifyGitHub: VerifyGitHubFn = verifyGitHubAttestation,
): Promise<TrustInfo> {
  // ...
  const npmTrust = await _verifyNpm(name, version, tarballPath);
  // ...
}
```

### Example 2: Test uses mock functions — no real network calls
**File**: `packages/core/src/trust/resolve.test.ts:23`
```typescript
const verifyNpmOk = async (): Promise<NpmTrustData> => ({
  publisher: "acme",
  sourceRepo: "https://github.com/acme/code-review",
  verifiedAt: "2026-01-01T00:00:00.000Z",
});
const verifyNpmFail = async () => null;
const verifyGitHubFail = async () => null;

test("provenance when npm attestation verifies", async () => {
  const trust = await resolveTrust(
    { adapter: "npm", url: "npm:@acme/code-review", tap: null, /* ... */ },
    verifyNpmOk,     // mock — injected as 2nd arg
    verifyGitHubFail // mock — injected as 3rd arg
  );
  expect(trust.tier).toBe("provenance");
});
```

### Example 3: Production call — omit optional params, real implementations used
**File**: `packages/core/src/install.ts:359`
```typescript
// Called with no override fns — uses verifyNpmProvenance and verifyGitHubAttestation
trust = await resolveTrust({
  adapter: resolved.adapter,
  url: effectiveSource,
  tarballPath: resolved.adapter === "npm" ? join(tmpDir, "_pkg.tgz") : undefined,
  npmPackageName: npmInfo?.name,
  // ...
});
```

## When to Use

- A core function has a hard external I/O dependency (HTTP fetch, subprocess, filesystem) that tests need to control
- The dependency has a stable, small interface that can be expressed as a typed function signature
- There is exactly one real implementation and tests need to supply mocks

## When NOT to Use

- For UI decision points (use the callback-driven-options pattern instead)
- When the dependency is always exercised the same way and doesn't vary between callers — just call it directly
- When there are many injected dependencies — at 4+, prefer a named options object

## Common Violations

- Using `jest.mock()` or Bun module mocking instead — works, but harder to see what a test does at a glance
- Accepting the implementation as a required first-class argument — callers must always supply it even when they don't care
- Forgetting the private `type Fn = typeof realFn` alias — without it, signature drift is silent
