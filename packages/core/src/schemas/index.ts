import { z } from "zod/v4";
import { err, ok, type Result, UserError } from "../types";

export * from "./agent";
export * from "./config";
export * from "./installed";
export * from "./skill";
export * from "./tap";

export function parseWithResult<T>(
  schema: z.ZodType<T>,
  data: unknown,
  label: string,
): Result<T, UserError> {
  const result = schema.safeParse(data);
  if (!result.success) {
    return err(new UserError(`Invalid ${label}: ${z.prettifyError(result.error)}`));
  }
  return ok(result.data);
}
