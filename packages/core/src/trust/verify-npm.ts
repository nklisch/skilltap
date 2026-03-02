import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import type { Bundle } from "sigstore";
import { VerificationError, verify } from "sigstore";

export interface NpmTrustData {
  publisher: string;
  sourceRepo: string;
  buildWorkflow?: string;
  transparency?: string;
  verifiedAt: string;
}

interface NpmAttestationsResponse {
  attestations?: Array<{
    predicateType: string;
    bundle: Bundle;
  }>;
}

interface InTotoStatement {
  subject?: Array<{
    name?: string;
    // npm SLSA attestations use sha512; older formats may use sha256
    digest?: { sha256?: string; sha512?: string };
  }>;
  predicateType?: string;
  predicate?: SlsaPredicateV1;
}

interface SlsaPredicateV1 {
  buildDefinition?: {
    externalParameters?: {
      workflow?: {
        repository?: string;
        path?: string;
        ref?: string;
      };
    };
  };
  runDetails?: {
    builder?: { id?: string };
    metadata?: { invocationId?: string };
  };
}

const SLSA_PREDICATE_V1 = "https://slsa.dev/provenance/v1";
const NPM_ATTESTATIONS_BASE = "https://registry.npmjs.org/-/npm/v1/attestations";

/**
 * Verify npm provenance for a downloaded tarball.
 * Fetches the npm attestations endpoint, verifies the Sigstore bundle,
 * and checks the tarball SHA-256 against the in-toto statement subject.
 * Returns null on any failure (graceful degradation to publisher tier).
 */
export async function verifyNpmProvenance(
  packageName: string,
  version: string,
  tarballPath: string,
): Promise<NpmTrustData | null> {
  try {
    // Attestations are a public npmjs.org feature — always use the public registry
    const url = `${NPM_ATTESTATIONS_BASE}/${encodeURIComponent(packageName)}@${encodeURIComponent(version)}`;

    let response: Response;
    try {
      response = await fetch(url, { signal: AbortSignal.timeout(10_000) });
    } catch {
      return null;
    }

    if (response.status === 404) return null;
    if (!response.ok) return null;

    let data: NpmAttestationsResponse;
    try {
      data = (await response.json()) as NpmAttestationsResponse;
    } catch {
      return null;
    }

    const attestations = data.attestations ?? [];
    const slsaAttestation = attestations.find(
      (a) => a.predicateType === SLSA_PREDICATE_V1,
    );
    if (!slsaAttestation) return null;

    const bundle = slsaAttestation.bundle;

    // Verify the Sigstore bundle (certificate chain + transparency log + DSSE signature)
    let signer: Awaited<ReturnType<typeof verify>>;
    try {
      signer = await verify(bundle);
    } catch (e) {
      if (e instanceof VerificationError) return null;
      return null;
    }

    // Decode DSSE payload to get the in-toto statement
    const dsseEnvelope = (bundle as Record<string, unknown>)
      .dsseEnvelope as { payload?: string } | undefined;
    const statement = decodeInTotoStatement(dsseEnvelope?.payload);
    if (!statement) return null;

    // Verify the tarball digest matches the in-toto statement subject.
    // npm SLSA attestations use sha512; fall back to sha256 for other formats.
    const digestEntry = statement.subject?.[0]?.digest;
    const subjectDigest512 = digestEntry?.sha512;
    const subjectDigest256 = digestEntry?.sha256;
    if (subjectDigest512 || subjectDigest256) {
      let tarballBytes: Buffer;
      try {
        tarballBytes = await readFile(tarballPath);
      } catch {
        return null;
      }
      if (subjectDigest512) {
        const actual = createHash("sha512").update(tarballBytes).digest("hex");
        if (actual !== subjectDigest512) return null;
      } else if (subjectDigest256) {
        const actual = createHash("sha256").update(tarballBytes).digest("hex");
        if (actual !== subjectDigest256) return null;
      }
    }

    // Extract provenance metadata
    const predicate = statement.predicate;
    const workflow = predicate?.buildDefinition?.externalParameters?.workflow;
    const sourceRepo = workflow?.repository ?? extractRepoFromSan(signer.identity?.subjectAlternativeName ?? "");
    const buildWorkflow = workflow?.path;
    const publisher = extractPublisherFromSan(signer.identity?.subjectAlternativeName ?? "");

    // Rekor log entry URL
    const tlogEntries = (
      (bundle as Record<string, unknown>).verificationMaterial as Record<string, unknown>
    )?.tlogEntries as Array<{ logIndex?: string }> | undefined;
    const logIndex = tlogEntries?.[0]?.logIndex;
    const transparency = logIndex
      ? `https://search.sigstore.dev/?logIndex=${logIndex}`
      : undefined;

    return {
      publisher: publisher || packageName.split("/").slice(0, -1).join("/") || packageName,
      sourceRepo: sourceRepo || "",
      buildWorkflow,
      transparency,
      verifiedAt: new Date().toISOString(),
    };
  } catch {
    return null;
  }
}

function decodeInTotoStatement(payload?: string): InTotoStatement | null {
  if (!payload) return null;
  try {
    const decoded = Buffer.from(payload, "base64").toString("utf-8");
    return JSON.parse(decoded) as InTotoStatement;
  } catch {
    return null;
  }
}

function extractPublisherFromSan(san: string): string {
  // SAN format: https://github.com/owner/repo/.github/workflows/...@ref
  const match = san.match(/https:\/\/github\.com\/([^/]+)/);
  return match?.[1] ?? "";
}

function extractRepoFromSan(san: string): string {
  const match = san.match(/(https:\/\/github\.com\/[^/]+\/[^/@]+)/);
  return match?.[1] ?? "";
}
