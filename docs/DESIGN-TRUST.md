# Design: Community Trust Signals

Adds provenance verification and trust metadata to skilltap — **without managing users**. All trust signals piggyback on existing identity systems (npm, GitHub, Sigstore) that package authors already use.

## Motivation

Snyk's ToxicSkills research (February 2026) found that **13.4% of skills on ClawHub/skills.sh contain at least one critical security issue**. 1,467 malicious payloads were discovered. The #1 skill on OpenClaw's marketplace was malware.

skilltap already has two-layer security scanning (static + semantic), which catches known attack patterns. Trust signals add a complementary layer: **who published this, and can we verify it?** This is defense in depth — scanning catches malicious content, provenance catches supply-chain attacks (compromised accounts, tampered packages, spoofed sources).

The key insight: we don't need to build a user database or verification system. npm provenance, GitHub attestations, and Sigstore already provide cryptographically verifiable identity. skilltap just needs to check what's already signed.

## Trust Tiers

Skills display one of four trust states, determined automatically during install:

| Tier | Display | Meaning |
|---|---|---|
| **Provenance verified** | `✓ provenance` | Package/repo has a cryptographic chain from source → build → artifact, verified via Sigstore |
| **Publisher known** | `● publisher` | Source identity is known (npm username, GitHub org) but not cryptographically verified |
| **Tap curated** | `◆ curated` | Skill was installed from a tap (human-curated index) |
| **Unverified** | `○ unverified` | No trust signals available. Not inherently dangerous — just unverified. |

These tiers are **informational, not blocking**. Trust signals don't affect the install flow — security scanning handles the "should this be blocked?" question. Trust signals answer "who published this and can we verify it?"

## Provenance Verification

### npm Provenance

npm's [Trusted Publishing](https://docs.npmjs.com/trusted-publishers/) (GA July 2025) uses OIDC to link packages to their source repo and CI workflow via Sigstore. When a package is published with `--provenance`, npm generates a SLSA Build Level 2 attestation signed with a short-lived Sigstore certificate and recorded on the Rekor transparency log.

**What it proves:**
- This exact tarball was built from this exact commit in this exact repo
- The build ran in a specific CI workflow (GitHub Actions or GitLab CI)
- No human touched the artifact between source and registry

**How skilltap verifies:**

After downloading an npm tarball (see [DESIGN-NPM-ADAPTER.md](./DESIGN-NPM-ADAPTER.md)):

1. Check if the package version has attestations:
   ```
   GET https://registry.npmjs.org/-/npm/v1/attestations/@acme/code-review@1.2.0
   ```
2. If attestations exist, verify the Sigstore bundle:
   - Certificate chain validates against Sigstore's root of trust
   - The `sourceRepositoryUri` in the provenance matches the package's `repository` field
   - The `buildTrigger` is a known CI system (GitHub Actions, GitLab CI)
3. Record the verification result in the trust metadata

**Attestation response** (simplified):

```json
{
  "attestations": [
    {
      "predicateType": "https://slsa.dev/provenance/v1",
      "bundle": {
        "verificationMaterial": {
          "x509CertificateChain": { "certificates": [...] },
          "tlogEntries": [{ "logIndex": "...", "logId": "..." }]
        },
        "dsseEnvelope": {
          "payload": "base64-encoded SLSA provenance",
          "signatures": [{ "sig": "..." }]
        }
      }
    }
  ]
}
```

The SLSA provenance payload (decoded) contains:

```json
{
  "buildDefinition": {
    "externalParameters": {
      "workflow": {
        "ref": "refs/heads/main",
        "repository": "https://github.com/acme/code-review",
        "path": ".github/workflows/publish.yml"
      }
    }
  },
  "runDetails": {
    "builder": { "id": "https://github.com/actions/runner" },
    "metadata": { "invocationId": "https://github.com/acme/code-review/actions/runs/12345" }
  }
}
```

### GitHub Attestations (for git-sourced skills)

GitHub's [Artifact Attestations](https://github.blog/security/supply-chain-security/configure-github-artifact-attestations-for-secure-cloud-native-delivery/) use the same Sigstore infrastructure but for arbitrary artifacts — not just npm packages. A skill author can attest any file in their repo using a GitHub Action.

**How a skill author attests:**

```yaml
# .github/workflows/attest.yml
name: Attest SKILL.md
on:
  release:
    types: [published]
permissions:
  id-token: write
  attestations: write
  contents: read
jobs:
  attest:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/attest-build-provenance@v2
        with:
          subject-path: SKILL.md
```

**How skilltap verifies:**

After cloning a git repo:

1. Check if the repo has attestations for the current commit:
   ```bash
   gh attestation verify SKILL.md --repo owner/repo --format json
   ```
   Or via the API:
   ```
   GET https://api.github.com/repos/owner/repo/attestations/SHA256_OF_SKILL_MD
   ```
2. If attestations exist and verify, record in trust metadata

**Limitation:** This requires `gh` CLI or a GitHub API token. If neither is available, GitHub attestation verification is skipped silently (no error — just no `provenance` tier).

### Verification Implementation

Sigstore verification is complex (certificate chains, transparency logs, timestamp authorities). Rather than implementing from scratch, use one of:

1. **`sigstore-js`** — Official Sigstore JS SDK. Handles full verification including Rekor log lookup.
2. **Shell out to `cosign verify-blob`** — If cosign is installed. Simpler but adds a binary dependency.
3. **Shell out to `gh attestation verify`** — For GitHub attestations specifically.

**Recommended approach:** Use `sigstore-js` for npm provenance (it's a pure JS library, works in Bun). Use `gh attestation verify` for GitHub attestations (optional — only when `gh` is on PATH).

```
packages/core/src/trust/
  verify-npm.ts       # npm attestation fetch + sigstore-js verification
  verify-github.ts    # GitHub attestation via gh CLI
  types.ts            # TrustInfo type
  index.ts            # barrel export
```

## Trust Metadata

### Schema

New `trust` field on `InstalledSkill`:

```typescript
export const TrustInfoSchema = z.object({
  // Trust tier (computed from available signals)
  tier: z.enum(["provenance", "publisher", "curated", "unverified"]),

  // npm provenance fields (present when tier = "provenance" and source is npm)
  npm: z.object({
    publisher: z.string(),              // npm username
    sourceRepo: z.string(),             // verified source repository URL
    buildWorkflow: z.string().optional(), // CI workflow path
    transparency: z.string().optional(), // Rekor log entry URL
    verifiedAt: z.iso.datetime(),
  }).optional(),

  // GitHub attestation fields (present when tier = "provenance" and source is git)
  github: z.object({
    owner: z.string(),                  // GitHub org or user
    repo: z.string(),                   // Repository name
    workflow: z.string().optional(),     // Workflow that produced the attestation
    verifiedAt: z.iso.datetime(),
  }).optional(),

  // Publisher info (present when tier >= "publisher")
  publisher: z.object({
    name: z.string(),                   // npm username or GitHub login
    platform: z.enum(["npm", "github"]),
  }).optional(),

  // Tap info (present when tier >= "curated")
  tap: z.string().optional(),           // tap name
}).prefault({});
```

Added to `InstalledSkillSchema`:

```typescript
export const InstalledSkillSchema = z.object({
  // ... existing fields ...
  trust: TrustInfoSchema.optional(),
});
```

### Tier Resolution Logic

Computed during install, in order of priority:

```
1. If npm source AND npm attestations verify → tier = "provenance"
2. If git source AND GitHub attestations verify → tier = "provenance"
3. If npm source (publisher known from registry) → tier = "publisher"
4. If git source from GitHub (owner known from URL) → tier = "publisher"
5. If installed from a tap → tier = "curated"
6. Otherwise → tier = "unverified"
```

Tiers are not mutually exclusive in terms of data — a skill can be `provenance` verified AND from a tap. The `tier` field reflects the highest available trust level.

## Display

### `skilltap list`

```
$ skilltap list

Global:
  commit-helper      v1.2.0   home    ✓ provenance   Conventional commit messages
  code-review        1.2.0    npm     ● publisher     Thorough code review
  quick-fix          main     url     ○ unverified    Quick bug fix helper

Project (/home/nathan/dev/app):
  app-dev            main     home    ◆ curated       Development workflow
```

The trust column appears between source and description. When `--json` is used, the full `trust` object is included.

### `skilltap info`

```
$ skilltap info commit-helper

  commit-helper (installed, global)
    Generates conventional commit messages
    Source:     https://github.com/nathan/commit-helper
    Ref:        v1.2.0 (abc123de)
    Tap:        home
    Trust:      ✓ Provenance verified
                  Source: github.com/nathan/commit-helper
                  Build:  .github/workflows/release.yml
                  Log:    https://search.sigstore.dev/?logIndex=12345
    Also:       claude-code
    Size:       12.3 KB (3 files)
    Installed:  2026-03-01

$ skilltap info code-review

  code-review (installed, global)
    Thorough code review skill
    Source:     npm:@acme/code-review
    Version:    1.2.0
    Trust:      ● Publisher known
                  npm: acme
    Also:       claude-code
    Size:       8.2 KB (2 files)
    Installed:  2026-03-01
```

### Agent Mode

In agent mode, trust info is included in the plain text output:

```
OK: Installed commit-helper → .agents/skills/commit-helper/ (v1.2.0) [provenance verified]
OK: Installed code-review → .agents/skills/code-review/ (1.2.0) [publisher: acme]
OK: Installed quick-fix → .agents/skills/quick-fix/ (main) [unverified]
```

## Config

No new config options. Provenance verification runs automatically when available — it's a read-only check with no user decision required. If verification fails or is unavailable, the skill is simply marked as a lower trust tier.

A future option (`security.require_provenance = true`) could block installation of unverified skills, but this is deferred. It would need careful thought about how to handle git-sourced skills from non-GitHub hosts (where attestations aren't available).

## Tap Trust Metadata

Taps can optionally carry trust metadata for their listed skills:

```json
{
  "name": "my tap",
  "skills": [
    {
      "name": "commit-helper",
      "description": "Conventional commit messages",
      "repo": "https://github.com/nathan/commit-helper",
      "tags": ["git"],
      "trust": {
        "verified": true,
        "verifiedBy": "nathan",
        "verifiedAt": "2026-03-01"
      }
    }
  ]
}
```

This is a **social trust signal** — the tap maintainer is asserting they reviewed this skill. It's not cryptographically verified (that's what provenance is for). It's the tap author saying "I looked at this and it's fine."

### TapSkill Schema Update

```typescript
export const TapTrustSchema = z.object({
  verified: z.boolean().default(false),
  verifiedBy: z.string().optional(),
  verifiedAt: z.string().optional(),  // ISO date string
}).optional();

export const TapSkillSchema = z.object({
  name: z.string(),
  description: z.string(),
  repo: z.string(),
  tags: z.array(z.string()).default([]),
  trust: TapTrustSchema,
});
```

When a skill is installed from a tap with `trust.verified = true`, the `curated` tier is strengthened in the display:

```
  commit-helper      v1.2.0   home    ◆ verified     Conventional commit messages
```

vs. without tap verification:

```
  app-helper         v1.0.0   home    ◆ curated      App development helper
```

## `find` Output with Trust

```
$ skilltap find review

  code-review        Thorough code review     ◆ verified     [home]
  termtube-review    Review checklist          ◆ curated      [home]

$ skilltap find --npm review

  @acme/code-review  1.2.0   Thorough code review     ✓ provenance   [npm]
  review-helper      0.3.1   Quick PR review           ● publisher    [npm]
```

## Verification Timing

Provenance is verified **once at install time** and the result is cached in `installed.json`. It is not re-verified on every `list` or `info` call — those read from the stored trust metadata.

On `update`, provenance is re-verified for the new version. If verification fails for the update but succeeded for the original install, the trust tier may degrade:

```
Checking code-review... 1.2.0 → 1.3.0
  ⚠ Provenance not available for 1.3.0 (was verified for 1.2.0)
```

This is informational — it doesn't block the update. But combined with `--strict`, it provides a signal worth paying attention to.

## Dependencies

- **`sigstore-js`** — npm provenance verification. Pure JS, works in Bun. Only runtime dependency added.
- **`gh` CLI** — GitHub attestation verification. Optional — not a package dependency. Used only if present on PATH.

## Error Handling

Trust verification failures are **never fatal**. They degrade gracefully to lower tiers:

| Condition | Behavior |
|---|---|
| npm attestation endpoint returns 404 | Tier = `publisher` (attestations not published) |
| npm attestation fails verification | Tier = `publisher`, log warning |
| GitHub API returns 404 | Tier = `publisher` or `curated` (no attestations) |
| `gh` not on PATH | Skip GitHub attestation check, tier = `publisher` or `curated` |
| Network error during verification | Tier = `publisher` or `curated`, log warning |
| Sigstore root of trust expired | Log warning, treat as unverified provenance |

## New Files

```
packages/core/src/trust/
  types.ts            # TrustInfo schema and types
  verify-npm.ts       # npm attestation verification
  verify-github.ts    # GitHub attestation verification (via gh CLI)
  resolve.ts          # resolveTrust() — compute tier from available signals
  index.ts            # barrel export
```

## Testing

- **Unit tests**: tier resolution logic (all combinations of source type, attestation availability, tap membership)
- **Unit tests**: npm attestation response parsing
- **Unit tests**: trust display formatting
- **Integration test**: install npm package with provenance, verify trust metadata saved
- **Integration test**: install from tap with `trust.verified`, verify display
- **Mock tests**: sigstore verification with known-good and known-bad bundles
- **Test fixture**: pre-built attestation bundle for unit tests (static JSON)
