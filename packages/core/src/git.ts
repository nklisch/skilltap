import { $ } from "bun"
import { ok, err } from "./types"
import { GitError } from "./types"
import type { Result } from "./types"

export type LogEntry = {
  sha: string
  message: string
  date: string
}

export type CloneOptions = {
  branch?: string
  depth?: number
}

function extractStderr(e: unknown): string {
  if (e instanceof Error && "stderr" in e) {
    const raw = (e as { stderr: unknown }).stderr
    if (raw instanceof Uint8Array) return new TextDecoder().decode(raw).trim()
    return String(raw).trim()
  }
  return String(e)
}

export async function clone(
  url: string,
  dest: string,
  opts?: CloneOptions,
): Promise<Result<void, GitError>> {
  const flags: string[] = ["--depth", String(opts?.depth ?? 1)]
  if (opts?.branch) flags.push("--branch", opts.branch)
  try {
    await $`git clone ${flags} -- ${url} ${dest}`.quiet()
    return ok(undefined)
  } catch (e) {
    return err(new GitError(`git clone failed: ${extractStderr(e)}`, { hint: "Check that the URL is correct and you have access." }))
  }
}

export async function pull(dir: string): Promise<Result<void, GitError>> {
  try {
    await $`git -C ${dir} pull`.quiet()
    return ok(undefined)
  } catch (e) {
    return err(new GitError(`git pull failed: ${extractStderr(e)}`))
  }
}

export async function fetch(dir: string): Promise<Result<void, GitError>> {
  try {
    await $`git -C ${dir} fetch`.quiet()
    return ok(undefined)
  } catch (e) {
    return err(new GitError(`git fetch failed: ${extractStderr(e)}`))
  }
}

export async function diff(
  dir: string,
  from: string,
  to: string,
): Promise<Result<string, GitError>> {
  try {
    const result = await $`git -C ${dir} diff ${from}..${to}`.quiet()
    return ok(result.stdout.toString())
  } catch (e) {
    return err(new GitError(`git diff failed: ${extractStderr(e)}`))
  }
}

export async function revParse(dir: string): Promise<Result<string, GitError>> {
  try {
    const result = await $`git -C ${dir} rev-parse HEAD`.quiet()
    return ok(result.stdout.toString().trim())
  } catch (e) {
    return err(new GitError(`git rev-parse failed: ${extractStderr(e)}`))
  }
}

export async function log(
  dir: string,
  n = 10,
): Promise<Result<LogEntry[], GitError>> {
  // Use unit separator (\x1f) to avoid conflicts with message content
  const SEP = "\x1f"
  const FORMAT = `%H${SEP}%s${SEP}%ai`
  try {
    const result = await $`git -C ${dir} log -${n} --format=${FORMAT}`.quiet()
    const output = result.stdout.toString().trim()
    if (!output) return ok([])
    const entries: LogEntry[] = output.split("\n").map((line) => {
      const [sha, message, date] = line.split(SEP)
      return { sha: sha ?? "", message: message ?? "", date: date ?? "" }
    })
    return ok(entries)
  } catch (e) {
    return err(new GitError(`git log failed: ${extractStderr(e)}`))
  }
}
