import { z } from "zod/v4";

export const AgentResponseSchema = z.object({
  score: z.number().int().min(0).max(10),
  reason: z.string(),
});

export const ResolvedSourceSchema = z.object({
  url: z.string(),
  ref: z.string().optional(),
  adapter: z.string(),
  integrity: z.string().optional(),
  npmPublisher: z.string().optional(),
});

export type AgentResponse = z.infer<typeof AgentResponseSchema>;
export type ResolvedSource = z.infer<typeof ResolvedSourceSchema>;
