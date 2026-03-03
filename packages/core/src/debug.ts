import { appendFile, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { getConfigDir } from "./config";

const MAX_LOG_SIZE = 1_048_576; // 1 MB
const KEEP_BYTES = 524_288; // 500 KB

let enabled: boolean | null = null;
let logPath: string | null = null;
let writeChain: Promise<void> = Promise.resolve();
let rotationDone = false;

function getLogPath(): string {
  if (!logPath) logPath = join(getConfigDir(), "debug.log");
  return logPath;
}

async function rotateIfNeeded(): Promise<void> {
  if (rotationDone) return;
  rotationDone = true;
  try {
    const path = getLogPath();
    const s = await stat(path).catch(() => null);
    if (s && s.size > MAX_LOG_SIZE) {
      const content = await readFile(path);
      await writeFile(path, content.slice(content.length - KEEP_BYTES));
    }
  } catch {
    /* ignore rotation errors */
  }
}

/** Write a timestamped debug message to the log file. No-op when SKILLTAP_DEBUG is not "1". */
export function debug(msg: string, context?: Record<string, unknown>): void {
  if (enabled === null) enabled = process.env.SKILLTAP_DEBUG === "1";
  if (!enabled) return;

  const ts = new Date().toISOString();
  const ctx = context ? ` ${JSON.stringify(context)}` : "";
  const line = `[${ts}] ${msg}${ctx}\n`;

  writeChain = writeChain.then(async () => {
    await rotateIfNeeded();
    const path = getLogPath();
    await mkdir(join(path, ".."), { recursive: true }).catch(() => {});
    await appendFile(path, line).catch(() => {});
  });
}

/**
 * Flush pending debug writes. Only needed in tests — production code uses fire-and-forget.
 * @internal
 */
export function flushDebug(): Promise<void> {
  return writeChain;
}

/**
 * Reset internal state. Only for tests.
 * @internal
 */
export function resetDebug(): void {
  enabled = null;
  logPath = null;
  writeChain = Promise.resolve();
  rotationDone = false;
}
