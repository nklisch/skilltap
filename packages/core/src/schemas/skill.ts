import { z } from "zod/v4"

export const SkillFrontmatterSchema = z.object({
  name: z.string().min(1).max(64).regex(/^[a-z0-9]+(-[a-z0-9]+)*$/),
  description: z.string().min(1).max(1024),
  license: z.string().optional(),
  compatibility: z.string().max(500).optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
})

export type SkillFrontmatter = z.infer<typeof SkillFrontmatterSchema>
