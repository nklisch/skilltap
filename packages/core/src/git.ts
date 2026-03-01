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

async function wrapGit<T>(
  fn: () => Promise<T>,
  msg: string,
  hint?: string,
): Promise<Result<T, GitError>> {
  try {
    return ok(await fn())
  } catch (e) {
    return err(new GitError(`${msg}: ${extractStderr(e)}`, hint ? { hint } : undefined))
  }
}

export async function clone(
  url: string,
  dest: string,
  opts?: CloneOptions,
): Promise<Result<void, GitError>> {
  const flags: string[] = ["--depth", String(opts?.depth ?? 1)]
  if (opts?.branch) flags.push("--branch", opts.branch)
  return wrapGit(
    () => $`git clone ${flags} -- ${url} ${dest}`.quiet().then(() => undefined),
    "git clone failed",
    "Check that the URL is correct and you have access.",
  )
}

export async function pull(dir: string): Promise<Result<void, GitError>> {
  return wrapGit(
    () => $`git -C ${dir} pull`.quiet().then(() => undefined),
    "git pull failed",
  )
}

export async function fetch(dir: string): Promise<Result<void, GitError>> {
  return wrapGit(
    () => $`git -C ${dir} fetch`.quiet().then(() => undefined),
    "git fetch failed",
  )
}

export async function diff(
  dir: string,
  from: string,
  to: string,
): Promise<Result<string, GitError>> {
  return wrapGit(
    () => $`git -C ${dir} diff ${from}..${to}`.quiet().then((r) => r.stdout.toString()),
    "git diff failed",
  )
}

export async function revParse(dir: string): Promise<Result<string, GitError>> {
  return wrapGit(
    () => $`git -C ${dir} rev-parse HEAD`.quiet().then((r) => r.stdout.toString().trim()),
    "git rev-parse failed",
  )
}

export async function log(
  dir: string,
  n = 10,
): Promise<Result<LogEntry[], GitError>> {
  // Use unit separator (\x1f) to avoid conflicts with message content
  const SEP = "\x1f"
  const FORMAT = `%H${SEP}%s${SEP}%ai`
  return wrapGit(async () => {
    const result = await $`git -C ${dir} log -${n} --format=${FORMAT}`.quiet()
    const output = result.stdout.toString().trim()
    if (!output) return []
    return output.split("\n").map((line) => {
      const [sha, message, date] = line.split(SEP)
      return { sha: sha ?? "", message: message ?? "", date: date ?? "" }
    })
  }, "git log failed")
}
