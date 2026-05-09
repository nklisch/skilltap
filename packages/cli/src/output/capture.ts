import type { Output, OutputMode } from "@skilltap/core";

export type CapturedEvent =
  | { kind: "info"; message: string }
  | { kind: "warn"; message: string; hint?: string }
  | { kind: "error"; message: string; hint?: string }
  | { kind: "success"; message: string }
  | { kind: "block"; lines: string[]; stream: "stdout" | "stderr" }
  | { kind: "json"; event: unknown }
  | { kind: "progress:start"; label: string }
  | { kind: "progress:update"; label: string; message: string }
  | { kind: "progress:done"; label: string; message?: string }
  | { kind: "progress:fail"; label: string; message?: string }
  | { kind: "raw"; text: string };

export interface CaptureOutput extends Output {
  events: CapturedEvent[];
}

export function createCaptureOutput(mode: OutputMode = "plain"): CaptureOutput {
  const events: CapturedEvent[] = [];
  const out: CaptureOutput = {
    events,
    mode,
    info(message) {
      events.push({ kind: "info", message });
    },
    warn(message, hint) {
      events.push({ kind: "warn", message, hint });
    },
    error(message, hint) {
      events.push({ kind: "error", message, hint });
    },
    success(message) {
      events.push({ kind: "success", message });
    },
    block(lines, blockOpts) {
      events.push({
        kind: "block",
        lines,
        stream: blockOpts?.stream ?? "stderr",
      });
    },
    json(event) {
      events.push({ kind: "json", event });
    },
    progress(label) {
      events.push({ kind: "progress:start", label });
      return {
        update(message) {
          events.push({ kind: "progress:update", label, message });
        },
        succeed(message) {
          events.push({ kind: "progress:done", label, message });
        },
        fail(message) {
          events.push({ kind: "progress:fail", label, message });
        },
        pause() {},
        resume() {},
      };
    },
    raw(text) {
      events.push({ kind: "raw", text });
    },
  };
  return out;
}
