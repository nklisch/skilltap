import { describe, expect, test } from "bun:test";
import { TrustInfoSchema, TrustTierSchema } from "./types";

describe("TrustTierSchema", () => {
  test("accepts valid tiers", () => {
    for (const tier of ["provenance", "publisher", "curated", "unverified"] as const) {
      expect(TrustTierSchema.safeParse(tier).success).toBe(true);
    }
  });

  test("rejects unknown tier", () => {
    expect(TrustTierSchema.safeParse("trusted").success).toBe(false);
    expect(TrustTierSchema.safeParse("").success).toBe(false);
  });
});

describe("TrustInfoSchema", () => {
  test("minimal unverified", () => {
    const result = TrustInfoSchema.safeParse({ tier: "unverified" });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.tier).toBe("unverified");
    expect(result.data.npm).toBeUndefined();
    expect(result.data.github).toBeUndefined();
    expect(result.data.publisher).toBeUndefined();
    expect(result.data.tap).toBeUndefined();
  });

  test("publisher tier", () => {
    const result = TrustInfoSchema.safeParse({
      tier: "publisher",
      publisher: { name: "acme", platform: "npm" },
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.tier).toBe("publisher");
    expect(result.data.publisher?.name).toBe("acme");
    expect(result.data.publisher?.platform).toBe("npm");
  });

  test("curated tier with tap", () => {
    const result = TrustInfoSchema.safeParse({
      tier: "curated",
      tap: "home",
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.tier).toBe("curated");
    expect(result.data.tap).toBe("home");
  });

  test("provenance tier with npm data", () => {
    const result = TrustInfoSchema.safeParse({
      tier: "provenance",
      npm: {
        publisher: "acme",
        sourceRepo: "https://github.com/acme/pkg",
        buildWorkflow: ".github/workflows/publish.yml",
        transparency: "https://search.sigstore.dev/?logIndex=12345",
        verifiedAt: "2026-01-01T00:00:00.000Z",
      },
      publisher: { name: "acme", platform: "npm" },
      tap: "my-tap",
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.tier).toBe("provenance");
    expect(result.data.npm?.sourceRepo).toBe("https://github.com/acme/pkg");
    expect(result.data.npm?.buildWorkflow).toBe(".github/workflows/publish.yml");
    expect(result.data.tap).toBe("my-tap");
  });

  test("provenance tier with github data", () => {
    const result = TrustInfoSchema.safeParse({
      tier: "provenance",
      github: {
        owner: "acme",
        repo: "my-skill",
        workflow: ".github/workflows/attest.yml",
        verifiedAt: "2026-01-01T00:00:00.000Z",
      },
      publisher: { name: "acme", platform: "github" },
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.github?.owner).toBe("acme");
    expect(result.data.github?.repo).toBe("my-skill");
  });

  test("optional fields in npm data", () => {
    const result = TrustInfoSchema.safeParse({
      tier: "provenance",
      npm: {
        publisher: "acme",
        sourceRepo: "https://github.com/acme/pkg",
        verifiedAt: "2026-01-01T00:00:00.000Z",
        // buildWorkflow and transparency omitted
      },
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.npm?.buildWorkflow).toBeUndefined();
    expect(result.data.npm?.transparency).toBeUndefined();
  });

  test("unknown fields ignored (passthrough)", () => {
    // Zod by default strips unknown fields
    const result = TrustInfoSchema.safeParse({
      tier: "unverified",
      unknownField: "value",
    });
    expect(result.success).toBe(true);
  });

  test("rejects missing tier", () => {
    const result = TrustInfoSchema.safeParse({});
    expect(result.success).toBe(false);
  });

  test("rejects invalid publisher platform", () => {
    const result = TrustInfoSchema.safeParse({
      tier: "publisher",
      publisher: { name: "bob", platform: "gitlab" },
    });
    expect(result.success).toBe(false);
  });
});
