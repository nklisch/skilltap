import { homedir } from "node:os"
import { resolve } from "node:path"
import { stat } from "node:fs/promises"
import { ok, err, UserError } from "../types"
import type { SourceAdapter } from "./types"

export const localAdapter: SourceAdapter = {
  name: "local",

  canHandle(source: string): boolean {
    return source.startsWith("./") || source.startsWith("/") || source.startsWith("~/")
  },

  async resolve(source: string) {
    const expanded = source.startsWith("~/")
      ? resolve(homedir(), source.slice(2))
      : resolve(source)

    let stats
    try {
      stats = await stat(expanded)
    } catch {
      return err(new UserError(`Path does not exist: ${expanded}`))
    }

    if (!stats.isDirectory()) {
      return err(new UserError(`Path is not a directory: ${expanded}`))
    }

    return ok({ url: expanded, adapter: "local" })
  },
}
