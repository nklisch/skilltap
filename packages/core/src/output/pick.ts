import type { OutputMode, OutputOptions } from "./types";

export function pickMode(opts?: OutputOptions): OutputMode {
  if (opts?.json === true) return "json";
  const isTTY =
    opts?.isTTY !== undefined ? opts.isTTY : process.stdout.isTTY === true;
  return isTTY ? "tty" : "plain";
}
