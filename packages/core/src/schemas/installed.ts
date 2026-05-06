/**
 * Skill state schemas.
 *
 * `InstalledSkillSchema` / `InstalledSkill` — the canonical record shape
 * for an installed skill. **Still actively used** in v2.1: `state.json`'s
 * `skills[]` array uses this exact shape, and every consumer (install,
 * update, remove, doctor checks, status, sync) references the type.
 *
 * `InstalledJsonSchema` / `InstalledJson` — the v0.x file-wrapper format
 * `{ version: 1, skills: InstalledSkill[] }`. **Legacy in v2.1**: only
 * read by `loadInstalled`'s read-fallback for unmigrated v0.x users
 * (see `core/src/config.ts`) and by `migrate/run.ts` for one-shot upgrades.
 * Never written. Will be removed entirely in v2.2 once the read-fallback
 * is deleted (Phase 31c-c-2d-2-final).
 */
import { z } from "zod/v4";
import { TrustInfoSchema } from "../trust/types";

export const InstalledSkillSchema = z.object({
  name: z.string(),
  description: z.string().default(""),
  repo: z.string().nullable(),
  ref: z.string().nullable(),
  sha: z.string().nullable().default(null),
  scope: z.enum(["global", "project", "linked"]),
  path: z.string().nullable(),
  tap: z.string().nullable().default(null),
  also: z.array(z.string()).default([]),
  installedAt: z.iso.datetime(),
  updatedAt: z.iso.datetime().default("1970-01-01T00:00:00.000Z"),
  trust: TrustInfoSchema.optional(),
  active: z.boolean().default(true),
});

export const InstalledJsonSchema = z.object({
  version: z.literal(1),
  skills: z.array(InstalledSkillSchema),
});

export type InstalledSkill = z.infer<typeof InstalledSkillSchema>;
export type InstalledJson = z.infer<typeof InstalledJsonSchema>;
