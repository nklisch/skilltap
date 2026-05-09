import { z } from "zod/v4";

const SlsaPredicateV1Schema = z
  .object({
    buildDefinition: z
      .object({
        externalParameters: z
          .object({
            workflow: z
              .object({
                repository: z.string().optional(),
                path: z.string().optional(),
                ref: z.string().optional(),
              })
              .passthrough()
              .optional(),
          })
          .passthrough()
          .optional(),
      })
      .passthrough()
      .optional(),
    runDetails: z
      .object({
        builder: z.object({ id: z.string().optional() }).passthrough().optional(),
        metadata: z
          .object({ invocationId: z.string().optional() })
          .passthrough()
          .optional(),
      })
      .passthrough()
      .optional(),
  })
  .passthrough();

export const InTotoStatementSchema = z
  .object({
    subject: z
      .array(
        z
          .object({
            name: z.string().optional(),
            digest: z
              .object({
                sha256: z.string().optional(),
                sha512: z.string().optional(),
              })
              .passthrough()
              .optional(),
          })
          .passthrough(),
      )
      .optional(),
    predicateType: z.string().optional(),
    predicate: SlsaPredicateV1Schema.optional(),
  })
  .passthrough();

export type InTotoStatementRaw = z.infer<typeof InTotoStatementSchema>;
