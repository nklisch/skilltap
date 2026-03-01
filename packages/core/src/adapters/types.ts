import type { Result, UserError } from "../types"
import type { ResolvedSource } from "../schemas"

export interface SourceAdapter {
  readonly name: string
  canHandle(source: string): boolean
  resolve(source: string): Promise<Result<ResolvedSource, UserError>>
}
