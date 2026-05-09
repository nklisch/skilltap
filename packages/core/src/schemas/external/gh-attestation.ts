import { z } from "zod/v4";

const GhAttestationEntrySchema = z
  .object({
    verificationResult: z
      .object({
        statement: z
          .object({
            predicate: z
              .object({
                buildDefinition: z
                  .object({
                    externalParameters: z
                      .object({
                        workflow: z
                          .object({ path: z.string().optional() })
                          .passthrough()
                          .optional(),
                      })
                      .passthrough()
                      .optional(),
                  })
                  .passthrough()
                  .optional(),
              })
              .passthrough()
              .optional(),
          })
          .passthrough()
          .optional(),
      })
      .passthrough()
      .optional(),
  })
  .passthrough();

export const GhAttestationSchema = z.array(GhAttestationEntrySchema);

export type GhAttestationRaw = z.infer<typeof GhAttestationSchema>;
