import { z } from "zod/v4";

const RegistrySkillEntrySchema = z
  .object({
    id: z.string(),
    name: z.string(),
    description: z.string().optional(),
    source: z.string(),
    installs: z.number().optional(),
  })
  .passthrough();

export const RegistryApiResponseSchema = z
  .object({
    skills: z.array(RegistrySkillEntrySchema).optional(),
  })
  .passthrough();

export type RegistryApiResponseRaw = z.infer<typeof RegistryApiResponseSchema>;
