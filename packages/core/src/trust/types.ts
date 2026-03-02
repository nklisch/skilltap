import { z } from "zod/v4";

export const TrustTierSchema = z.enum([
  "provenance",
  "publisher",
  "curated",
  "unverified",
]);

export const TrustInfoSchema = z.object({
  tier: TrustTierSchema,
  // Present when tier = "provenance" and source is npm
  npm: z
    .object({
      publisher: z.string(),
      sourceRepo: z.string(),
      buildWorkflow: z.string().optional(),
      transparency: z.string().optional(),
      verifiedAt: z.iso.datetime(),
    })
    .optional(),
  // Present when tier = "provenance" and source is git/github
  github: z
    .object({
      owner: z.string(),
      repo: z.string(),
      workflow: z.string().optional(),
      verifiedAt: z.iso.datetime(),
    })
    .optional(),
  // Present when tier >= "publisher"
  publisher: z
    .object({
      name: z.string(),
      platform: z.enum(["npm", "github"]),
    })
    .optional(),
  // Present when installed from a tap
  tap: z.string().optional(),
});

export type TrustTier = z.infer<typeof TrustTierSchema>;
export type TrustInfo = z.infer<typeof TrustInfoSchema>;
