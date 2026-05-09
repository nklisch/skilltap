import { z } from "zod/v4";

export const UpdateCacheSchema = z.object({
  checkedAt: z.string(),
  latest: z.string(),
});

export type UpdateCacheRaw = z.infer<typeof UpdateCacheSchema>;
