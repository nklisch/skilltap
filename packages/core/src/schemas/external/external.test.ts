import { describe, expect, test } from "bun:test";
import { GhAttestationSchema } from "./gh-attestation";
import { InTotoStatementSchema } from "./in-toto";
import { McpClientConfigSchema } from "./mcp-client-config";
import { NpmPackageMetadataSchema } from "./npm-registry";
import { RegistryApiResponseSchema } from "./skills-registry";
import { UpdateCacheSchema } from "./self-update";

// ---------------------------------------------------------------------------
// NpmPackageMetadataSchema
// ---------------------------------------------------------------------------
describe("NpmPackageMetadataSchema", () => {
  test("accepts a well-formed npm registry response", () => {
    const raw = {
      name: "my-pkg",
      description: "A package",
      "dist-tags": { latest: "1.2.0" },
      versions: {
        "1.2.0": {
          version: "1.2.0",
          dist: { tarball: "https://example.com/pkg.tgz", integrity: "sha512-abc" },
        },
      },
    };
    expect(NpmPackageMetadataSchema.safeParse(raw).success).toBe(true);
  });

  test("rejects a non-object", () => {
    expect(NpmPackageMetadataSchema.safeParse("bad").success).toBe(false);
  });

  test("rejects when dist.tarball is missing (wrong type)", () => {
    const raw = {
      versions: {
        "1.0.0": {
          version: "1.0.0",
          dist: { tarball: 42 }, // not a string
        },
      },
    };
    expect(NpmPackageMetadataSchema.safeParse(raw).success).toBe(false);
  });

  test("passes through unknown extra fields", () => {
    const raw = { name: "pkg", unknownField: true };
    const result = NpmPackageMetadataSchema.safeParse(raw);
    expect(result.success).toBe(true);
    if (result.success) {
      expect((result.data as Record<string, unknown>).unknownField).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// RegistryApiResponseSchema
// ---------------------------------------------------------------------------
describe("RegistryApiResponseSchema", () => {
  test("accepts a valid skills.sh response", () => {
    const raw = {
      skills: [
        { id: "owner/repo/skill", name: "my-skill", source: "owner/repo", installs: 42 },
      ],
    };
    expect(RegistryApiResponseSchema.safeParse(raw).success).toBe(true);
  });

  test("accepts empty skills array", () => {
    expect(RegistryApiResponseSchema.safeParse({ skills: [] }).success).toBe(true);
  });

  test("accepts missing skills (optional)", () => {
    expect(RegistryApiResponseSchema.safeParse({}).success).toBe(true);
  });

  test("rejects non-object", () => {
    expect(RegistryApiResponseSchema.safeParse(null).success).toBe(false);
  });

  test("rejects when skills entry is missing required id", () => {
    const raw = { skills: [{ name: "bad", source: "x", installs: 0 }] };
    expect(RegistryApiResponseSchema.safeParse(raw).success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// UpdateCacheSchema
// ---------------------------------------------------------------------------
describe("UpdateCacheSchema", () => {
  test("accepts a valid update cache", () => {
    const raw = { checkedAt: "2024-01-01T00:00:00Z", latest: "1.0.0" };
    expect(UpdateCacheSchema.safeParse(raw).success).toBe(true);
  });

  test("rejects when latest is missing", () => {
    expect(UpdateCacheSchema.safeParse({ checkedAt: "2024-01-01T00:00:00Z" }).success).toBe(false);
  });

  test("rejects when checkedAt is missing", () => {
    expect(UpdateCacheSchema.safeParse({ latest: "1.0.0" }).success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// GhAttestationSchema
// ---------------------------------------------------------------------------
describe("GhAttestationSchema", () => {
  test("accepts a fully-formed gh attestation array", () => {
    const raw = [
      {
        verificationResult: {
          statement: {
            predicate: {
              buildDefinition: {
                externalParameters: { workflow: { path: ".github/workflows/release.yml" } },
              },
            },
          },
        },
      },
    ];
    expect(GhAttestationSchema.safeParse(raw).success).toBe(true);
  });

  test("accepts an empty array", () => {
    expect(GhAttestationSchema.safeParse([]).success).toBe(true);
  });

  test("accepts entries with all optional fields absent", () => {
    expect(GhAttestationSchema.safeParse([{}]).success).toBe(true);
  });

  test("rejects a non-array", () => {
    expect(GhAttestationSchema.safeParse({ verificationResult: {} }).success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// InTotoStatementSchema
// ---------------------------------------------------------------------------
describe("InTotoStatementSchema", () => {
  test("accepts a valid SLSA in-toto statement", () => {
    const raw = {
      subject: [{ name: "pkg.tgz", digest: { sha512: "abc123" } }],
      predicateType: "https://slsa.dev/provenance/v1",
      predicate: {
        buildDefinition: {
          externalParameters: {
            workflow: { repository: "https://github.com/owner/repo", path: ".github/workflows/release.yml" },
          },
        },
        runDetails: { builder: { id: "https://github.com/actions/runner" } },
      },
    };
    expect(InTotoStatementSchema.safeParse(raw).success).toBe(true);
  });

  test("accepts an empty object (all fields optional)", () => {
    expect(InTotoStatementSchema.safeParse({}).success).toBe(true);
  });

  test("rejects a non-object", () => {
    expect(InTotoStatementSchema.safeParse("bad").success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// McpClientConfigSchema
// ---------------------------------------------------------------------------
describe("McpClientConfigSchema", () => {
  test("accepts a claude settings.json shape with mcpServers", () => {
    const raw = {
      mcpServers: {
        "skilltap:my-plugin:server": { command: "node", args: ["index.js"] },
      },
    };
    expect(McpClientConfigSchema.safeParse(raw).success).toBe(true);
  });

  test("accepts an empty object", () => {
    expect(McpClientConfigSchema.safeParse({}).success).toBe(true);
  });

  test("passes through extra top-level keys", () => {
    const raw = { mcpServers: {}, permissions: { allow: [] } };
    const result = McpClientConfigSchema.safeParse(raw);
    expect(result.success).toBe(true);
    if (result.success) {
      expect((result.data as Record<string, unknown>).permissions).toBeDefined();
    }
  });

  test("rejects a non-object", () => {
    expect(McpClientConfigSchema.safeParse(null).success).toBe(false);
  });
});
