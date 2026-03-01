import type { ResolvedSource } from "../schemas";
import type { Result, UserError } from "../types";

export interface SourceAdapter {
  readonly name: string;
  canHandle(source: string): boolean;
  resolve(source: string): Promise<Result<ResolvedSource, UserError>>;
}
