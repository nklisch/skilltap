import { z } from "zod/v4";

export const McpClientConfigSchema = z
  .object({
    mcpServers: z.record(z.string(), z.unknown()).optional(),
  })
  .passthrough();

export type McpClientConfigRaw = z.infer<typeof McpClientConfigSchema>;
