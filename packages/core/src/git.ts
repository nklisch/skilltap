import { $ } from "bun";
import type { Result } from "./types";
import { err, GitError, ok } from "./types";

export type LogEntry = {
  sha: string;
  message: string;
  date: string;
};

export type CloneOptions = {
  branch?: string;
  depth?: number;
};

function extractStderr(e: unknown): string {
  if (e instanceof Error && "stderr" in e) {
    const raw = (e as { stderr: unknown }).stderr;
    if (raw instanceof Uint8Array) return new TextDecoder().decode(raw).trim();
    return String(raw).trim();
  }
  return String(e);
}

async function wrapGit<T>(
  fn: () => Promise<T>,
  msg: string,
  hint?: string,
): Promise<Result<T, GitError>> {
  try {
    return ok(await fn());
  } catch (e) {
    return err(
      new GitError(`${msg}: ${extractStderr(e)}`, hint ? { hint } : undefined),
    );
  }
}

export async function checkGitInstalled(): Promise<Result<void, GitError>> {
  return wrapGit(
    () => $`git --version`.quiet().then(() => undefined),
    "git is not installed or not on PATH",
    "Install git: https://git-scm.com/downloads",
  );
}

export async function clone(
  url: string,
  dest: string,
  opts?: CloneOptions,
): Promise<Result<void, GitError>> {
  const flags: string[] = ["--depth", String(opts?.depth ?? 1)];
  if (opts?.branch) flags.push("--branch", opts.branch);
  try {
    await $`git clone ${flags} -- ${url} ${dest}`.quiet();
    return ok(undefined);
  } catch (e) {
    const stderr = extractStderr(e);
    if (stderr.includes("Authentication failed")) {
      return err(
        new GitError(
          `Authentication failed for '${url}'. Check your git credentials or SSH keys.`,
        ),
      );
    }
    if (stderr.includes("not found") || stderr.includes("does not exist")) {
      return err(new GitError(`Repository not found: '${url}'.`));
    }
    return err(
      new GitError(
        `git clone failed: ${stderr}`,
        "Check that the URL is correct and you have access.",
      ),
    );
  }
}

export async function pull(dir: string): Promise<Result<void, GitError>> {
  return wrapGit(
    () => $`git -C ${dir} pull`.quiet().then(() => undefined),
    "git pull failed",
  );
}

export async function fetch(dir: string): Promise<Result<void, GitError>> {
  return wrapGit(
    () => $`git -C ${dir} fetch`.quiet().then(() => undefined),
    "git fetch failed",
  );
}

export async function diff(
  dir: string,
  from: string,
  to: string,
  pathSpec?: string,
): Promise<Result<string, GitError>> {
  const extra = pathSpec ? ["--", pathSpec] : [];
  return wrapGit(
    () =>
      $`git -C ${dir} diff ${from}..${to} ${extra}`
        .quiet()
        .then((r) => r.stdout.toString()),
    "git diff failed",
  );
}

export type DiffFileStat = {
  status: "M" | "A" | "D" | "R";
  path: string;
  insertions: number;
  deletions: number;
};

export type DiffStat = {
  filesChanged: number;
  insertions: number;
  deletions: number;
  files: DiffFileStat[];
};

export async function diffStat(
  dir: string,
  from: string,
  to: string,
  pathSpec?: string,
): Promise<Result<DiffStat, GitError>> {
  const extra = pathSpec ? ["--", pathSpec] : [];
  return wrapGit(async () => {
    const numstatOut = await $`git -C ${dir} diff --numstat ${from}..${to} ${extra}`
      .quiet()
      .then((r) => r.stdout.toString().trim());
    const nameStatusOut = await $`git -C ${dir} diff --name-status ${from}..${to} ${extra}`
      .quiet()
      .then((r) => r.stdout.toString().trim());

    // Parse --numstat: "<ins>\t<del>\tfilename" per line
    const numstatMap = new Map<string, { ins: number; del: number }>();
    if (numstatOut) {
      for (const line of numstatOut.split("\n")) {
        const parts = line.split("\t");
        if (parts.length >= 3) {
          const ins = parseInt(parts[0] ?? "0", 10);
          const del = parseInt(parts[1] ?? "0", 10);
          const file = parts[2] ?? "";
          numstatMap.set(file, {
            ins: Number.isNaN(ins) ? 0 : ins,
            del: Number.isNaN(del) ? 0 : del,
          });
        }
      }
    }

    // Parse --name-status: "<STATUS>\tfilename" per line
    const files: DiffFileStat[] = [];
    if (nameStatusOut) {
      for (const line of nameStatusOut.split("\n")) {
        const parts = line.split("\t");
        if (parts.length >= 2) {
          const statusChar = (parts[0] ?? "M")[0] ?? "M";
          const filePath = parts[parts.length - 1] ?? "";
          const status = (["M", "A", "D", "R"].includes(statusChar)
            ? statusChar
            : "M") as "M" | "A" | "D" | "R";
          const counts = numstatMap.get(filePath) ?? { ins: 0, del: 0 };
          files.push({
            status,
            path: filePath,
            insertions: counts.ins,
            deletions: counts.del,
          });
        }
      }
    }

    const insertions = files.reduce((s, f) => s + f.insertions, 0);
    const deletions = files.reduce((s, f) => s + f.deletions, 0);
    return { filesChanged: files.length, insertions, deletions, files };
  }, "git diff stat failed");
}

export async function revParse(
  dir: string,
  ref = "HEAD",
): Promise<Result<string, GitError>> {
  return wrapGit(
    () =>
      $`git -C ${dir} rev-parse ${ref}`
        .quiet()
        .then((r) => r.stdout.toString().trim()),
    "git rev-parse failed",
  );
}

export async function log(
  dir: string,
  n = 10,
): Promise<Result<LogEntry[], GitError>> {
  // Use unit separator (\x1f) to avoid conflicts with message content
  const SEP = "\x1f";
  const FORMAT = `%H${SEP}%s${SEP}%ai`;
  return wrapGit(async () => {
    const result = await $`git -C ${dir} log -${n} --format=${FORMAT}`.quiet();
    const output = result.stdout.toString().trim();
    if (!output) return [];
    return output.split("\n").map((line) => {
      const [sha, message, date] = line.split(SEP);
      return { sha: sha ?? "", message: message ?? "", date: date ?? "" };
    });
  }, "git log failed");
}
