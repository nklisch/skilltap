import { z } from "zod/v4";

// Plugin source can be a relative path string or an object with type-specific fields
const MarketplacePluginSourceSchema = z.union([
  z.string(), // relative path like "./plugins/my-plugin"
  z.object({
    source: z.literal("github"),
    repo: z.string(),
    ref: z.string().optional(),
  }),
  z.object({
    source: z.literal("url"),
    url: z.string(),
    ref: z.string().optional(),
  }),
  z.object({
    source: z.literal("git-subdir"),
    url: z.string(),
    path: z.string(),
    ref: z.string().optional(),
  }),
  z.object({
    source: z.literal("npm"),
    package: z.string(),
    version: z.string().optional(),
  }),
]);

const MarketplacePluginSchema = z.object({
  name: z.string(),
  source: MarketplacePluginSourceSchema,
  description: z.string().optional(),
  version: z.string().optional(),
  category: z.string().optional(),
  tags: z.array(z.string()).optional(),
});

const MarketplaceOwnerSchema = z.object({
  name: z.string(),
  email: z.string().optional(),
});

const MarketplaceMetadataSchema = z
  .object({
    description: z.string().optional(),
    version: z.string().optional(),
    pluginRoot: z.string().optional(),
  })
  .optional();

export const MarketplaceSchema = z.object({
  name: z.string(),
  owner: MarketplaceOwnerSchema,
  metadata: MarketplaceMetadataSchema,
  plugins: z.array(MarketplacePluginSchema),
});

export type MarketplacePluginSource = z.infer<typeof MarketplacePluginSourceSchema>;
export type MarketplacePlugin = z.infer<typeof MarketplacePluginSchema>;
export type Marketplace = z.infer<typeof MarketplaceSchema>;
